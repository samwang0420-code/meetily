#!/usr/bin/env bash
# 离线会记 — W1 改造推送脚本
# 在 samwang0420-code/meetily 的本地 clone 根目录运行
# 1) 应用 w1-fork-changes.diff
# 2) 提交 commit
# 3) 推送到 fork 的 feature 分支
set -euo pipefail

BRANCH="feature/w1-no-cloud"
COMMIT_MSG="feat(w1): 屏蔽云端 + CSP 重写 + 砍 posthog + 砍云端 LLM (W1 改造)

- identifier: com.meetily.ai → cn.lixianhuiji.app
- productName: meetily → 离线会记
- 删 tauri.conf.json 的 updater plugin block (GitHub releases endpoint)
- CSP connect-src 删 https://api.ollama.ai / 5167 / 8178,只留 localhost:11434
- Cargo.toml 删 posthog-rs = 0.3.7
- src/analytics/analytics.rs: 521 行 → 75 行 (PostHog Client → 本地 noop)
- src/analytics/commands.rs: 373 行 → 109 行 (25 个 tauri::command 内部全 noop)
- src/summary/llm_client.rs:
  - LLMProvider::from_str: 砍 OpenAI/Claude/Groq/OpenRouter/CustomOpenAI,只接受 Ollama/BuiltInAI
  - generate_summary: 入口加硬守卫,非本地 provider 直接返错

lib.rs 中 25 个 analytics::commands::* invoke_handler 引用保持不变,编译零破坏。

详细改造日志见: outputs/02-W1-改造-屏蔽云端+CSS重写.md
上游 base: Zackriya-Solutions/meetily @ 0281737d (v0.4.0)
"

DIFF_FILE="$(cd "$(dirname "$0")" && pwd)/w1-fork-changes.diff"

# 检查 diff 文件
if [[ ! -f "$DIFF_FILE" ]]; then
  echo "!! 找不到 diff: $DIFF_FILE" >&2
  exit 1
fi

# 必须在 fork 仓库根运行(看 .git + 顶层 Cargo.toml)
if [[ ! -d ".git" ]] || [[ ! -f "Cargo.toml" ]]; then
  echo "!! 必须在 samwang0420-code/meetily 仓库根目录运行" >&2
  exit 1
fi

# 1. 切新分支
git checkout -b "$BRANCH" 2>/dev/null || git checkout "$BRANCH"

# 2. 应用 diff
echo "==> 应用 diff..."
if git apply --check "$DIFF_FILE" 2>&1; then
  git apply "$DIFF_FILE"
  echo "  ✅ diff 应用成功"
else
  echo "  ❌ diff 应用失败,可能已应用过或冲突" >&2
  exit 1
fi

# 3. 提交
echo "==> 提交 commit..."
git add -A
git commit -m "$COMMIT_MSG" -q
echo "  ✅ commit 完成: $(git rev-parse --short HEAD)"

# 4. 推送(可选,先 dry-run)
echo ""
echo "==> 准备推送到 origin (feature/w1-no-cloud)..."
echo "    如需推送,请手动执行:"
echo "    git push -u origin $BRANCH"
echo ""
echo "==> 完成后,在 GitHub 上:"
echo "    https://github.com/samwang0420-code/meetily/tree/$BRANCH"
