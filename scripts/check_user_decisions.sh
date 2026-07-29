#!/usr/bin/env bash
# 离线会记 · 用户决策迁移守门 v5 (生产版)
# 过滤规则:
#   - session_meta.cwd 在 Documents/离线会记 或 Documents/meetily
#   - 排除 AGENTS.md 模板 / 开发者消息 / 环境注入
#   - 业务决策只对含离线会记专有词的块匹配
#   - 关键业务常量 (22 项) repo grep 全检
set -e

CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
VAULT="$HOME/Documents/Obsidian Vault"
REPO="$HOME/Documents/meetily"

echo "=== 离线会记 · 决策守门 v5 ==="
echo

# 1) 限定 session
SESSIONS=$(find "$CODEX_HOME/sessions" -name 'rollout-*.jsonl' -mtime -14 2>/dev/null | while read f; do
  CWD=$(head -1 "$f" 2>/dev/null | jq -r '.payload.cwd // ""' 2>/dev/null)
  if echo "$CWD" | grep -qE "Documents/(离线会记|meetily)"; then echo "$f"; fi
done | sort -u | head -30)

if [ -z "$SESSIONS" ]; then
  echo "⚠️  无相关 session"
  exit 0
fi
echo "📂 命中 $(echo "$SESSIONS" | wc -l | tr -d ' ') 个相关 session"
echo

# 2) 提取 + 离线会记过滤
TMP=$(mktemp)
for f in $SESSIONS; do
  # 跳过每个 session 第 1 条 user message (那是 AGENTS.md 模板 + 环境注入)
  FIRST_DONE=0
  jq -r '
    select(.type=="response_item") |
    select(.payload.type=="message" and .payload.role=="user") |
    .payload.content |
    map(select(type=="object")) |
    map(.text // .input // "") |
    join("\n")
  ' "$f" 2>/dev/null | while read -r line; do
    [ -z "$line" ] && continue
    if [ "$FIRST_DONE" = "0" ]; then FIRST_DONE=1; continue; fi
    echo "$line" | grep -qE "^# AGENTS\.md|<INSTRUCTIONS>|Codex 全局记忆|<environment_context>|^# Files mentioned" && continue
    echo "$line" | grep -qE "离线会记|meetily|meeting_minutes|sherpa|sensevoice|parakeet|funasr|summary|摘要|转录|transcription|录音|导入|激活码|quota|配额|会员|API|token|UI" && echo "$line" >> "$TMP"
  done
done
NONEMPTY=$(grep -cE '\S' "$TMP" 2>/dev/null || echo 0)
echo "📋 离线会记相关用户消息: $NONEMPTY 条"

# 3) 业务决策短语扫描
DECISION_RE='导入限制|应该|永远|不要|禁止|加一个|去掉|改成|增加|删除|上限是|下限是|超过|小于|>=|<=|≤|≥|保证|撤回|显式|铁律|强制|至少|不要超过|永久|必须|确保|记得|记住|卡点|坑'

DECISIONS=$(grep -nE "$DECISION_RE" "$TMP" 2>/dev/null | head -50 || true)
if [ -z "$DECISIONS" ]; then
  echo "✅ 业务决策短语: 0"
else
  echo "🚨 业务决策短语: $(echo "$DECISIONS" | wc -l | tr -d ' ') 条"
  echo "$DECISIONS" | head -30
fi
echo

# 4) 关键业务常量
echo "=== 关键业务常量 ==="
KW=("5 GB" "MAX_FILE_SIZE_BYTES" "ratelimit" "60s.*5 次" "FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT" "FREE_MONTHLY_MEETING_LIMIT" "MAX_DIAR_AUDIO_SECONDS" "MEMORY_PRESSURE_THRESHOLD_MB" "ANONYMOUS_FREE_RECORDINGS" "truncate_segments_for_tier" "is_placeholder_title" "bound_machine_id" "chunk_transcript_by_token" "diar_pickup_loop" "user_redeem_activation_code" "HardwareOnboardingModal" "redeem/page" "user_id = ?1" "MEMORY_PRESSURE" "text_boundary_overlap" "shutdown_global_daemon")
MISSING=0
for k in "${KW[@]}"; do
  if rg -l "$k" "$REPO/frontend/src-tauri/src" "$REPO/frontend/src" 2>/dev/null | head -1 > /dev/null; then
    echo "  ✓ $k"
  else
    echo "  ✗ $k"
    MISSING=$((MISSING+1))
  fi
done
echo "→ 缺 $MISSING"
echo

# 5) Obsidian 决策日志
DECISION_LOG="$VAULT/00-收件箱/决策日志.md"
if [ -f "$DECISION_LOG" ]; then
  echo "=== Obsidian 决策日志最后 5 行 ==="
  tail -5 "$DECISION_LOG"
fi
AGENTS="$HOME/.codex/AGENTS.md"
if [ -f "$AGENTS" ]; then
  echo
  echo "=== AGENTS.md 最后 5 行 ==="
  tail -5 "$AGENTS"
fi

rm -f "$TMP"
echo
echo "=== 守门完成 ==="
