#!/usr/bin/env bash
# §115: 24h 自动清理机制
# 用法:
#   bash scripts/cleanup_old_branches.sh          # dry-run
#   bash scripts/cleanup_old_branches.sh --force  # 实际清理

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

FORCE="${1:-dry-run}"

# 同步远端 cache
git fetch origin --prune --quiet 2>/dev/null || true

echo "=== §115 cleanup scan (mode: $FORCE) ==="
echo ""

# 1. 列出 cleanup-recommended-* 标记的分支
echo "--- 1. cleanup-recommended-* 标记的分支 (24h 到期) ---"

# Tag → Branch 映射 (用数组)
TAGS_BRANCHES=(
  "cleanup-recommended-2026-08-14-perf:perf/summary-map-concurrency"
  "cleanup-recommended-2026-08-14-backup:backup/main-v0.8.2-pre-v0.8.6"
  "cleanup-recommended-2026-08-14-feature-w1:feature/w1-no-cloud"
)

for pair in "${TAGS_BRANCHES[@]}"; do
  tag="${pair%:*}"
  branch="${pair#*:}"
  # 确认 tag 存在
  if ! git ls-remote origin | grep -q "refs/tags/$tag\$"; then
    echo "  tag $tag: 远端不存在 (skip)"
    continue
  fi
  # 确认 branch 存在
  if ! git ls-remote origin | grep -q "refs/heads/$branch\$"; then
    echo "  tag $tag: branch $branch 远端不存在 (单独清 tag)"
    if [ "$FORCE" = "--force" ]; then
      git push origin --delete "refs/tags/$tag" 2>&1 | head -1 || true
    fi
    continue
  fi
  echo "  tag $tag → branch $branch (待清理)"
  if [ "$FORCE" = "--force" ]; then
    git push origin --delete "$branch" 2>&1 | head -2 || echo "    → delete branch failed"
    git push origin --delete "refs/tags/$tag" 2>&1 | head -1 || true
  fi
done

# 2. 列出 backup/* / perf/* / feature/* 旧分支 (last commit > 24h)
echo ""
echo "--- 2. backup/* / perf/* / feature/* 旧分支 (last commit > 24h, 远端真实存在) ---"
NOW=$(date +%s)
THRESHOLD=86400  # 24h

# 用 git ls-remote origin 真实存在的分支
for b in $(git ls-remote origin | grep -E "refs/heads/(backup/|perf/|feature/)" | awk '{print $2}' | sed 's|refs/heads/||'); do
  LAST=$(git log -1 --format=%ct "origin/$b" 2>/dev/null || echo 0)
  if [ "$LAST" -eq 0 ]; then continue; fi
  AGE=$(( NOW - LAST ))
  if [ "$AGE" -gt "$THRESHOLD" ]; then
    HOURS=$(( AGE / 3600 ))
    echo "  $b: ${HOURS}h ago"
    if [ "$FORCE" = "--force" ]; then
      git push origin --delete "$b" 2>&1 | head -2 || echo "    → delete failed"
    fi
  fi
done

if [ "$FORCE" != "--force" ]; then
  echo ""
  echo "(dry-run. Use 'bash scripts/cleanup_old_branches.sh --force' to actually delete)"
fi
