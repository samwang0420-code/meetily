#!/usr/bin/env python3
# 离线会记 · 用户决策迁移守门 v6 (Python 版, 精确过滤)
# - 只看 session_meta.cwd 在 Documents/离线会记 或 Documents/meetily
# - 跳过每个 session 第 1 条 user message (AGENTS.md 模板)
# - 跳过纯 AGENTS.md / 环境 / Files mentioned 块
# - 离线会记专有词过滤
# - 关键业务常量 22 项 repo grep
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

CODEX_HOME = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
VAULT = Path.home() / "Documents" / "Obsidian Vault"
REPO = Path.home() / "Documents" / "meetily"
DECISION_LOG = VAULT / "00-收件箱" / "决策日志.md"
AGENTS = Path.home() / ".codex" / "AGENTS.md"

print("=== 离线会记 · 决策守门 v6 (Python) ===\n")

# 1) 找相关 sessions
sessions = []
_CUTOFF = time.time() - 14 * 86400
for f in sorted(CODEX_HOME.glob("sessions/**/rollout-*.jsonl")):
    try:
        if f.stat().st_mtime < _CUTOFF:
            continue
    except Exception:
        continue
    try:
        with f.open() as fp:
            for line in fp:
                d = json.loads(line)
                if d.get("type") == "session_meta":
                    cwd = d.get("payload", {}).get("cwd", "")
                    if "Documents/离线会记" in cwd or "Documents/meetily" in cwd:
                        sessions.append(f)
                    break
    except Exception:
        continue

if not sessions:
    print("⚠️  无相关 session (找 cwd 在 Documents/离线会记 或 meetily)")
    sys.exit(0)
print(f"📂 命中 {len(sessions)} 个相关 session\n")

# 2) 提取用户消息
import time
LINUX_KEYWORDS = r"离线会记|meetily|meeting_minutes|sherpa|sensevoice|parakeet|funasr|summary|摘要|转录|transcription|录音|导入|激活码|quota|配额|会员|API|UI|0\.7\.0|0\.6\.|v0\."
DECISION_RE = re.compile(
    r"导入限制|应该|永远|不要|禁止|加一个|去掉|改成|增加|删除|"
    r"上限是|下限是|超过|小于|>=|<=|≤|≥|保证|撤回|显式|铁律|"
    r"强制|至少|不要超过|永久|必须|确保|记得|记住|卡点|坑|"
    r"不超过|不能|应当|确保|不要忘记|记得|"
    r"上线|发布|打包|部署|测试|验收|回归"
)
SKIP_RE = re.compile(
    r"^# AGENTS\.md|<INSTRUCTIONS>|Codex 全局记忆|<environment_context>|^# Files mentioned"
)
excluded_by_session = {}

all_msgs = []
for f in sessions:
    msgs = []
    try:
        with f.open() as fp:
            for line in fp:
                try:
                    d = json.loads(line)
                except Exception:
                    continue
                if d.get("type") != "response_item":
                    continue
                p = d.get("payload", {})
                if p.get("type") != "message" or p.get("role") != "user":
                    continue
                content = p.get("content", [])
                text = ""
                for c in content:
                    if isinstance(c, dict):
                        text += c.get("text", "") or c.get("input", "") or ""
                    elif isinstance(c, str):
                        text += c
                msgs.append((d.get("timestamp", ""), text.strip()))
    except Exception:
        continue
    # 跳过每个 session 第 1 条 (AGENTS.md 模板)
    msgs = msgs[1:] if len(msgs) > 1 else []
    # 排除 AGENTS.md / env / Files mentioned
    msgs = [(ts, t) for ts, t in msgs if not SKIP_RE.search(t)]
    # 离线会记关键词过滤
    msgs = [(ts, t) for ts, t in msgs if re.search(LINUX_KEYWORDS, t, re.IGNORECASE)]
    all_msgs.extend(msgs)

print(f"📋 离线会记相关用户消息: {len(all_msgs)} 条")

# 3) 业务决策短语扫描
decisions = []
for ts, t in all_msgs:
    for m in DECISION_RE.finditer(t):
        decisions.append((ts, m.group(0), t[:300]))
print(f"🚨 业务决策短语: {len(decisions)} 条 (前 30)")
seen = set()
for ts, kw, snip in decisions[:30]:
    key = (ts, kw)
    if key in seen:
        continue
    seen.add(key)
    print(f"  [{ts}] [{kw}] {snip[:150].replace(chr(10),' ')}")
print()

# 3.5) i18n 完整性 + 死代码扫描
print("\n=== i18n 完整性 (zh vs en) ===")
def keys_in_ts(path):
    txt = open(path).read()
    return set(re.findall(r"^\s{4,}([a-zA-Z_]\w*)\s*:", txt, re.M))
_ZH = keys_in_ts(REPO/"frontend/src/i18n/locales/zh.ts")
_EN = keys_in_ts(REPO/"frontend/src/i18n/locales/en.ts")
print(f"  zh={len(_ZH)} en={len(_EN)} only_zh={len(_ZH-_EN)} only_en={len(_EN-_ZH)}")
if _ZH != _EN:
    only_zh = sorted(_ZH-_EN)[:10]
    only_en = sorted(_EN-_ZH)[:10]
    print(f"  ⚠️  diff only_zh={only_zh} only_en={only_en}")

print("\n=== 死代码 i18n key 扫描 ===")
_zh_txt = (REPO/"frontend/src/i18n/locales/zh.ts").read_text()
_dead = []
for _m in re.finditer(r"^\s{4,}([a-zA-Z_]\w*)\s*:\s*['\"]([^'\"]+)['\"]", _zh_txt, re.M):
    _k = _m.group(1); _v = _m.group(2)
    if _v.endswith("{") or len(_v) < 2:
        continue
    # 用 t('key') 或 t("key") 模式精确匹配
    _res = subprocess.run(
        ["rg", "-l", "--", f"t\([\'\"]{_k}[\'\"]\)", str(REPO/"frontend/src")],
        capture_output=True, text=True
    )
    _files = [f for f in _res.stdout.strip().split("\n") if "/i18n/locales/" not in f]
    if not _files:
        _dead.append((_k, _v[:40]))
if _dead:
    print(f"  ⚠️  dead i18n: {len(_dead)}")
    for _k, _v in _dead[:10]:
        print(f"    {_k}: {_v!r}")
else:
    print("  ✓ 0 dead keys")

# 4) 关键业务常量
print("\n=== 关键业务常量 ===")
KW = [
    "5 GB", "MAX_FILE_SIZE_BYTES", "ratelimit", "60s 内最多 5 次",
    "FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT", "FREE_MONTHLY_MEETING_LIMIT",
    "MAX_DIAR_AUDIO_SECONDS", "MEMORY_PRESSURE_THRESHOLD_MB",
    "ANONYMOUS_FREE_RECORDINGS", "truncate_segments_for_tier",
    "is_placeholder_title", "bound_machine_id", "chunk_transcript_by_token",
    "diar_pickup_loop", "user_redeem_activation_code", "HardwareOnboardingModal",
    "redeem/page", "user_id = ?1", "MEMORY_PRESSURE", "text_boundary_overlap",
    "shutdown_global_daemon",
]
# 路径关键字 (用 glob 而不是 rg)
PATH_KW = {"redeem/page": REPO/"frontend/src/app/redeem/page.tsx"}
missing = []
for k in KW:
    if k in PATH_KW:
        ok = PATH_KW[k].exists()
    else:
        # SQL 字面 ? 转义给 ripgrep
        kw = k
        res = subprocess.run(
            ["rg", "-l", "-F", kw, str(REPO/"frontend/src-tauri/src"), str(REPO/"frontend/src-tauri/scripts"), str(REPO/"frontend/src"), str(REPO/"frontend/src/app")],
            capture_output=True, text=True
        )
        ok = bool(res.stdout.strip())
    if ok:
        print(f"  ✓ {k}")
    else:
        print(f"  ✗ {k}")
        missing.append(k)
print(f"→ 缺 {len(missing)}")
print()

# 5) Obsidian / AGENTS.md 末尾
if DECISION_LOG.exists():
    print("=== Obsidian 决策日志最后 5 行 ===")
    print("\n".join(DECISION_LOG.read_text().splitlines()[-5:]))
if AGENTS.exists():
    print("\n=== AGENTS.md 最后 5 行 ===")
    print("\n".join(AGENTS.read_text().splitlines()[-5:]))

print("\n=== 守门完成 ===")
