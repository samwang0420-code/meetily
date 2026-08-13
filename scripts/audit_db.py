#!/usr/bin/env python3
"""
meetily transcript DB auditor (2026-07-31)
Independent of cargo/next build. Runs against the live SQLite DB
to surface real production issues from historical recordings.

§15 §17 §32 §33 §34 §36 §37 compliance: pure read-only SQL,
no GUI / no binary required. Output is text the user can read
and decide on before touching code.
"""
from __future__ import annotations
import argparse
import os
import sqlite3
import sys
from dataclasses import dataclass

DEFAULT_DB = os.path.expanduser(
    "~/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite"
)


@dataclass
class Issue:
    meeting_id: str
    title: str
    severity: str
    metric: str
    detail: str


def fetch_meetings(con):
    return list(con.execute(
        "SELECT m.id, m.title, m.created_at, "
        "(SELECT COUNT(*) FROM transcripts t WHERE t.meeting_id=m.id) AS segs, "
        "(SELECT COALESCE(SUM(LENGTH(t.transcript)),0) FROM transcripts t WHERE t.meeting_id=m.id) AS chars, "
        "(SELECT COALESCE(SUM(t.duration),0.0) FROM transcripts t WHERE t.meeting_id=m.id) AS sum_dur, "
        "(SELECT COALESCE(MAX(t.audio_end_time),0.0) FROM transcripts t WHERE t.meeting_id=m.id) AS max_end, "
        "(SELECT COALESCE(MIN(t.audio_start_time),0.0) FROM transcripts t WHERE t.meeting_id=m.id) AS min_start, "
        "(SELECT status FROM summary_processes p WHERE p.meeting_id=m.id) AS sum_status, "
        "(SELECT chunk_count FROM summary_processes p WHERE p.meeting_id=m.id) AS sum_chunks "
        "FROM meetings m ORDER BY m.created_at DESC"
    ))


def audit_one(meeting_id, title, segs, max_end, sum_dur, min_start, chars,
              sum_status, sum_chunks, con):
    issues = []
    if segs == 0:
        return issues

    if max_end and max_end > 30 and sum_dur and sum_dur < max_end * 0.3:
        drift = max_end - sum_dur
        issues.append(Issue(
            meeting_id=meeting_id, title=title, severity='critical',
            metric='time_drift',
            detail=f'max_end={max_end:.1f}s vs sum_dur={sum_dur:.1f}s '
                   f'(drift={drift:.1f}s, content {sum_dur/max_end*100:.0f}% of timeline); '
                   f'user sees {int(max_end/60)+1}min timeline but only {int(sum_dur)}s of real content. '
                   f'Section 33 VAD timestamp double-count regression.'
        ))

    overlaps = con.execute(
        "WITH o AS (SELECT audio_start_time AS s, audio_end_time AS e "
        "FROM transcripts WHERE meeting_id=? AND audio_start_time IS NOT NULL "
        "ORDER BY audio_start_time), "
        "l AS (SELECT s, e, LAG(e) OVER (ORDER BY s) AS pe FROM o) "
        "SELECT COUNT(*) AS overlaps, COALESCE(SUM(pe - s),0) AS total_overlap "
        "FROM l WHERE pe IS NOT NULL AND s + 0.1 < pe",
        (meeting_id,),
    ).fetchone()
    if overlaps['overlaps'] and overlaps['overlaps'] >= 2:
        issues.append(Issue(
            meeting_id=meeting_id, title=title, severity='high',
            metric='segment_overlap',
            detail=f'{overlaps["overlaps"]} overlapping segments, '
                   f'total {overlaps["total_overlap"]:.1f}s of duplicated audio. '
                   f'Section 34 force-split does not suppress SpeechEnd.samples re-emission.'
        ))

    short = con.execute(
        "SELECT COUNT(*) AS n, AVG(duration) AS avg_d "
        "FROM transcripts WHERE meeting_id=? AND duration IS NOT NULL",
        (meeting_id,),
    ).fetchone()
    if segs >= 20 and short['avg_d'] and short['avg_d'] < 2.0:
        issues.append(Issue(
            meeting_id=meeting_id, title=title, severity='critical',
            metric='over_segmentation',
            detail=f'{segs} segments with avg duration {short["avg_d"]:.2f}s; '
                   f'VAD is splitting on syllables, not utterances. '
                   f'Section 32 8s force-split threshold not effective (or set to 0.6s). '
                   f'User sees "1min->4min" jumps in playback.'
        ))

    # §50 fix: ASR 文本重复识别 (同一句话被录了多次)
    # 02c7f2d9 / a2851054 等 — §34 SpeechEnd.samples 重复 emit 残留
    dup = con.execute(
        "SELECT COUNT(*) AS dup_groups, SUM(c) AS dup_segs "
        "FROM (SELECT transcript, COUNT(*) AS c FROM transcripts "
        "WHERE meeting_id=? AND LENGTH(transcript)>20 "
        "GROUP BY transcript HAVING c>=2)",
        (meeting_id,),
    ).fetchone()
    if dup['dup_segs'] and dup['dup_segs'] >= 2:
        issues.append(Issue(
            meeting_id=meeting_id, title=title, severity='high',
            metric='asr_text_repeat',
            detail=f'{dup["dup_groups"]} text patterns repeated {dup["dup_segs"]} times in total. '
                   f'Section 34 SpeechEnd.samples repeat-emit residual. '
                   f'Wastes summary tokens, may pollute LLM input.'
        ))

    big_segs = con.execute(
        "SELECT COUNT(*) AS n, MAX(duration) AS max_d "
        "FROM transcripts WHERE meeting_id=? AND duration>20",
        (meeting_id,),
    ).fetchone()
    if big_segs['n'] and big_segs['n'] >= 2:
        if not (short['avg_d'] and short['avg_d'] < 5.0):
            issues.append(Issue(
                meeting_id=meeting_id, title=title, severity='medium',
                metric='long_segment',
                detail=f'{big_segs["n"]} segments >20s (max={big_segs["max_d"]:.1f}s); '
                       f'SpeechEnd not firing or not flushed. '
                       f'Check Silero min_speech_time / post_speech_pad config.'
            ))

    if sum_status == 'completed' and sum_chunks is not None:
        if chars > 1800 * 4 and sum_chunks == 1:
            issues.append(Issue(
                meeting_id=meeting_id, title=title, severity='high',
                metric='map_reduce_skipped',
                detail=f'{chars} chars but chunk_count=1; Section 36 1800-token cap not enforced. '
                       f'BuiltInAI/Ollama provider uses model.context_size - 300, '
                       f'which is always >1800, so Map-Reduce never triggers for medium-length meetings.'
            ))

    # §49: summary 完成但 processing_time > 前端 polling 阈值 (600s = 10 min)
    # 用户报"摘要超时"实际是前端 polling 超时, 后端仍然 completed
    if sum_status == 'completed' and meeting_id:
        row = con.execute(
            "SELECT processing_time FROM summary_processes WHERE meeting_id=?",
            (meeting_id,),
        ).fetchone()
        if row and row['processing_time'] and row['processing_time'] > 600:
            issues.append(Issue(
                meeting_id=meeting_id, title=title, severity='high',
                metric='polling_timeout_disconnect',
                detail=f'status=completed in {row["processing_time"]:.0f}s '
                       f'({row["processing_time"]/60:.1f}min), but frontend MAX_POLLS=300 '
                       f'(\u003d10min) — user sees "timeout 15min" but backend succeeded. '
                       f'i18n string says "15 minutes" but real threshold is 10. '
                       f'\u00a749 root cause.'
            ))

    if sum_status == 'PENDING':
        issues.append(Issue(
            meeting_id=meeting_id, title=title, severity='medium',
            metric='summary_pending_stuck',
            detail='summary_processes.status=PENDING but never resolved. No timeout / cleanup path.'
        ))

    return issues


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--db', default=DEFAULT_DB)
    ap.add_argument('--json', action='store_true')
    args = ap.parse_args()

    if not os.path.exists(args.db):
        print(f"DB not found: {args.db}", file=sys.stderr)
        return 2

    con = sqlite3.connect(args.db)
    con.row_factory = sqlite3.Row
    meetings = fetch_meetings(con)
    issues = []
    for m in meetings:
        issues.extend(audit_one(
            m['id'], m['title'], m['segs'], m['max_end'], m['sum_dur'],
            m['min_start'], m['chars'], m['sum_status'], m['sum_chunks'], con,
        ))

    if args.json:
        import json
        print(json.dumps([vars(i) for i in issues], ensure_ascii=False, indent=2))
        return 0

    sev_order = {'critical': 0, 'high': 1, 'medium': 2, 'info': 3}
    issues.sort(key=lambda i: (sev_order.get(i.severity, 9), -len(i.detail)))

    print(f"=== meetily DB audit  ({len(meetings)} meetings, {len(issues)} issues) ===\n")
    print(f"{'SEV':<10}{'METRIC':<26}{'MEETING':<46}")
    print('-' * 82)
    by_metric = {}
    for i in issues:
        print(f"{i.severity:<10}{i.metric:<26}{i.meeting_id:<46}")
        print(f"          {(i.title or '')[:60]}")
        print(f"          {i.detail[:120]}")
        print()
        by_metric[i.metric] = by_metric.get(i.metric, 0) + 1

    print("--- summary by metric ---")
    for k, v in sorted(by_metric.items(), key=lambda x: -x[1]):
        print(f"  {k:<28}{v}")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
