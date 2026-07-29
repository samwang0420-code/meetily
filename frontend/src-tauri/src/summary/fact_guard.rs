use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::BTreeSet;

// Match monetary amounts or unit-bearing numbers. Plain bare integers (e.g. "第二"
// rendered as "2", or "38" from time fragments) are excluded because they are
// indistinguishable from sequence numbers, problem indices, or split tokens.
static NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    (?:
        # 1. Currency-prefixed: ¥12800, ￥12800, $99, USD99
        (?:¥|￥|\$|USD)\s*\d[\d,]*(?:\.\d+)?
        |
        # 2. Unit-suffixed: 3000元, 12800元, 99块, 5万, 1.2千
        \d[\d,]*(?:\.\d+)?\s*(?:元|块|万美元|美元|人民币|dollars?)
        |
        # 3. Chinese large units: 3000万, 1.2亿, 99百万
        \d[\d,]*(?:\.\d+)?\s*(?:万|亿|百万|千)
    )
    "
).unwrap());
static DATE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    20\d{2}年(?:1[0-2]|0?[1-9])月(?:(?:3[01]|[12]\d|0?[1-9])[日号])?
    |
    20\d{2}年
    |
    (?:1[0-2]|0?[1-9])月(?:3[01]|[12]\d|0?[1-9])[日号]?
    |
    (?:1[0-2]|0?[1-9])/(?:3[01]|[12]\d|0?[1-9])
    "
).unwrap());
static EVIDENCE_PREFIX_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"^\s*\[evidence:\d+\s+start=(?:unknown|[-+]?\d+(?:\.\d+)?s)\s+end=(?:unknown|[-+]?\d+(?:\.\d+)?s)\]\s*"
).unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactGuardReport {
    pub unexpected_numbers: Vec<String>,
    pub unexpected_dates: Vec<String>,
    pub overclaimed_decision: bool,
}

impl FactGuardReport {
    pub fn is_safe(&self) -> bool { self.unexpected_numbers.is_empty() && self.unexpected_dates.is_empty() && !self.overclaimed_decision }
    pub fn is_severe(&self) -> bool { !self.is_safe() }

    /// Number of issues the user-visible UI should surface.
    pub fn issue_count(&self) -> usize {
        self.unexpected_numbers.len() + self.unexpected_dates.len() + if self.overclaimed_decision { 1 } else { 0 }
    }

    /// True when the report has issues worth a UI banner, even if not severe enough to replace the summary.
    pub fn needs_review(&self) -> bool { self.issue_count() > 0 }
}

fn normalized_tokens(re: &Regex, text: &str) -> BTreeSet<String> {
    re.find_iter(text).map(|m| m.as_str().split_whitespace().collect::<String>().replace(',', "")).filter(|v| !v.is_empty()).collect()
}

pub fn validate_summary(transcript: &str, summary: &str) -> FactGuardReport {
    let source_numbers = normalized_tokens(&NUMBER_RE, transcript);
    let summary_numbers = normalized_tokens(&NUMBER_RE, summary);
    let source_dates = normalized_tokens(&DATE_RE, transcript);
    let summary_dates = normalized_tokens(&DATE_RE, summary);
    let proposal_language = ["提案", "暂定", "需要确认", "没有结论", "未确定"].iter().any(|v| transcript.contains(v));
    let decision_language = ["已确定", "确定了", "最终决定", "会议决定", "确定执行", "确定采用"].iter().any(|v| summary.contains(v))
        && !summary.contains("不是最终决定")
        && !summary.contains("尚未确定")
        && !summary.contains("未确定");
    FactGuardReport {
        unexpected_numbers: summary_numbers.difference(&source_numbers).cloned().collect(),
        unexpected_dates: summary_dates.difference(&source_dates).cloned().collect(),
        overclaimed_decision: proposal_language && decision_language,
    }
}

/// Build a conservative meeting minutes body from the transcript, organised by the
/// evidence actually present in the source. Anything the AI summary claimed but the
/// transcript does not support is surfaced as a "需要确认" bullet rather than
/// being silently kept.
pub fn conservative_fallback(transcript: &str, report: &FactGuardReport) -> String {
    let mut output = String::from("## ⚠️ 纪要质量复核（已自动降级）\n\n");
    output.push_str("> AI 生成的纪要包含未被原文证据支持的内容，系统已自动改为基于原文重建的安全版。请人工核对下方「确认项」。\n\n");

    output.push_str("### 核心要点（基于原文）\n\n");
    output.push_str(&extract_evidence_lines(transcript, report));
    output.push_str("\n");

    output.push_str("### 确认项（请人工核对）\n\n");
    let mut issues = 0;
    if !report.unexpected_numbers.is_empty() {
        output.push_str(&format!(
            "- 以下金额在原文中未找到，可能为 AI 编造: {}\n",
            join_preview(&report.unexpected_numbers)
        ));
        issues += 1;
    }
    if !report.unexpected_dates.is_empty() {
        output.push_str(&format!(
            "- 以下日期在原文中未找到，可能为 AI 编造: {}\n",
            join_preview(&report.unexpected_dates)
        ));
        issues += 1;
    }
    if report.overclaimed_decision {
        output.push_str("- 原文是提案/未决信息，AI 误表述为最终决定。请按原文措辞修改。\n");
        issues += 1;
    }
    if issues == 0 {
        output.push_str("- 未识别到具体错误字段。建议重新生成。\n");
    }
    output
}

/// Split the transcript into coarse sentences (Chinese full-width and English
/// punctuation) and keep the ones that carry numbers, dates, or proposal / decision
/// language. Keeps the user able to read the actual evidence in seconds.
fn extract_evidence_lines(transcript: &str, report: &FactGuardReport) -> String {
    let mut output = String::new();
    let mut kept = 0usize;
    for raw in split_sentences(transcript) {
        let line = strip_evidence_prefix(raw.trim());
        if line.is_empty() {
            continue;
        }
        let has_number = NUMBER_RE.is_match(&line);
        let has_date = DATE_RE.is_match(&line);
        let has_proposal = ["提案", "暂定", "需要确认", "没有结论", "未确定", "提议"]
            .iter()
            .any(|v| line.contains(v));
        let has_decision = ["决定", "确定", "确认执行", "拍板"]
            .iter()
            .any(|v| line.contains(v));
        // Skip empty / filler lines, keep evidence-rich lines.
        if !(has_number || has_date || has_proposal || has_decision) {
            continue;
        }
        output.push_str("- ");
        output.push_str(&line);
        output.push_str("\n");
        kept += 1;
        if kept >= 20 {
            output.push_str("- ……（更多原文请向下滚动）\n");
            break;
        }
    }
    if kept == 0 {
        // Defensive fallback: when no informative sentence was detected, return the
        // first 800 chars of the transcript verbatim so the user can still review.
        let clean_transcript = transcript
            .lines()
            .map(|line| strip_evidence_prefix(line.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        let snippet: String = clean_transcript.chars().take(800).collect();
        output.push_str(&snippet);
        if clean_transcript.chars().count() > 800 {
            output.push_str("……");
        }
        output.push_str("\n");
    }
    // Tag issues for the UI to highlight: numeric tokens in the transcript that the
    // summary quoted but the source does not carry.
    let _ = report; // report is referenced via conservative_fallback signature only
    output
}

fn strip_evidence_prefix(line: &str) -> Cow<'_, str> {
    EVIDENCE_PREFIX_RE.replace(line, "")
}

fn split_sentences(transcript: &str) -> impl Iterator<Item = &str> {
    transcript.split(|c: char| matches!(c, '。' | '！' | '？' | '\n' | ';' | '；' | '.' | '!' | '?'))
}

fn join_preview(items: &[String]) -> String {
    let head: Vec<&str> = items.iter().take(3).map(String::as_str).collect();
    let suffix = if items.len() > 3 { " ……" } else { "" };
    format!("{}{}", head.join("、"), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invented_facts() {
        let report = validate_summary("预算暂定3000元，具体金额需要确认。", "预算确定为12800元，日期为2026年7月16日，会议确定执行。");
        assert!(report.unexpected_numbers.contains(&"12800元".to_string()));
        assert!(!report.unexpected_dates.is_empty());
        assert!(report.unexpected_dates.iter().any(|date| date.contains("2026年")));
        assert!(report.overclaimed_decision);
        assert!(report.is_severe());
    }
    #[test]
    fn accepts_supported_proposal() {
        let report = validate_summary("目前只是提案，不是最终决定，预算暂定3000元。", "预算暂定3000元，目前不是最终决定。");
        assert!(report.is_safe(), "{report:?}");
    }

    #[test]
    fn fallback_keeps_evidence_and_marks_review() {
        let source = "预算暂定3000元，具体金额需要确认。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("3000元"));
        assert!(fallback.contains("需要确认"));
        assert!(fallback.contains("以下金额在原文中未找到"));
    }

    #[test]
    fn fallback_extracts_only_informative_lines() {
        let source = "今天天气不错。预算暂定3000元，具体金额需要确认。下次会议是7月20日。好的没问题。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("3000元"), "evidence sentence with number kept");
        assert!(fallback.contains("7月20日"), "evidence sentence with date kept");
        assert!(!fallback.contains("今天天气不错"), "filler line dropped");
        assert!(!fallback.contains("好的没问题"), "filler line dropped");
    }

    #[test]
    fn temperature_range_is_not_treated_as_a_date() {
        let report = validate_summary(
            "气温将在45-95摄氏度之间波动。",
            "气温将在45-95摄氏度之间波动。",
        );
        assert!(report.unexpected_dates.is_empty(), "{report:?}");
    }

    #[test]
    fn fallback_hides_internal_evidence_markers() {
        let source = "[evidence:36 start=unknown end=unknown] 6月10日气温升高。\n[evidence:47 start=12.50s end=18.25s] 预算暂定3000元。";
        let report = FactGuardReport {
            unexpected_numbers: vec!["12800元".to_string()],
            unexpected_dates: vec![],
            overclaimed_decision: false,
        };
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("6月10日气温升高"));
        assert!(fallback.contains("预算暂定3000元"));
        assert!(!fallback.contains("[evidence:"));
        assert!(!fallback.contains("start=unknown"));
    }

    #[test]
    fn fallback_includes_unknown_issues_placeholder() {
        let source = "今天讨论一下。";
        let empty = FactGuardReport {
            unexpected_numbers: vec![],
            unexpected_dates: vec![],
            overclaimed_decision: false,
        };
        let fallback = conservative_fallback(source, &empty);
        assert!(fallback.contains("未识别到具体错误字段"));
    }

    #[test]
    fn number_regex_ignores_bare_indices_and_time_fragments() {
        // "第二个问题", "下一阶段", "[00:38]" must not match NUMBER_RE.
        let transcript_text = "[00:34] 第二个问题，[00:38] 呃下一阶段讨论。";
        let matched: Vec<&str> = NUMBER_RE.find_iter(transcript_text).map(|m| m.as_str()).collect();
        assert!(matched.is_empty(), "expected no amounts, got {matched:?}");

        // Real amounts must still match.
        let real = "预算暂定3000元，¥12800，5万人民币。";
        let matched_real: Vec<&str> = NUMBER_RE.find_iter(real).map(|m| m.as_str()).collect();
        assert!(matched_real.iter().any(|s| s.contains("3000元")), "3000元 missing: {matched_real:?}");
        assert!(matched_real.iter().any(|s| s.contains("¥12800")), "¥12800 missing: {matched_real:?}");
        assert!(matched_real.iter().any(|s| s.contains("5万")), "5万 missing: {matched_real:?}");
    }

    #[test]
    fn chinese_real_transcript_no_false_positive() {
        let transcript_text = "[00:34] 目前只是提案呢，不是最终决定，呃预算暂定3000元吧。";
        let safe_summary = "目前是提案，最终预算3000元。";
        let report = validate_summary(transcript_text, safe_summary);
        assert!(report.is_safe(), "3000元 is in transcript, summary must be safe: {report:?}");

        let unsafe_summary = "预算确定为12800元。";
        let report2 = validate_summary(transcript_text, unsafe_summary);
        assert!(report2.is_severe());
        assert!(report2.unexpected_numbers.iter().any(|n| n.contains("12800")));
    }

    #[test]
    fn fallback_uses_friendly_chinese_labels() {
        let source = "预算暂定3000元。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("⚠️ 纪要质量复核"));
        assert!(fallback.contains("AI 生成的纪要包含未被原文证据支持的内容"));
        assert!(fallback.contains("核心要点（基于原文）"));
        assert!(fallback.contains("确认项（请人工核对）"));
    }

    #[test]
    fn issue_count_and_needs_review() {
        let report = validate_summary("预算暂定3000元。", "预算确定为12800元，2026年7月16日交付。");
        // Transcript has no proposal/decision language, so only the two unsupported facts count.
        assert_eq!(report.issue_count(), 2);
        assert_eq!(report.unexpected_numbers, vec!["12800元".to_string()]);
        assert_eq!(report.unexpected_dates, vec!["2026年7月16日".to_string()]);
        assert!(!report.overclaimed_decision);
        assert!(report.needs_review());
        assert!(report.is_severe());
        assert!(report.needs_review());
        assert!(report.is_severe());

        let safe = validate_summary("预算3000元。", "预算3000元。");
        assert_eq!(safe.issue_count(), 0);
        assert!(!safe.needs_review());
        assert!(safe.is_safe());
    }

    #[test]
    fn report_serializes_to_json() {
        let report = validate_summary("3000元。", "12800元。");
        let json = serde_json::to_value(&report).expect("serialize");
        let arr = json.get("unexpected_numbers").and_then(|v| v.as_array()).expect("array");
        assert!(arr.iter().any(|v| v.as_str() == Some("12800元")));
        assert_eq!(json.get("overclaimed_decision").and_then(|v| v.as_bool()), Some(false));
    }
}
