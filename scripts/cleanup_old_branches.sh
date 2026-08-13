#!/usr/bin/env bash
# §115: 24h 自动清理机制
# 任何 (cleanup-recommended-* / backup/* / 长期未动的 perf/*) 24h 后自动删

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

FORCE="${1:-dry-run}"

echo "=== §115 cleanup scan (mode: $FORCE) ==="

# 1. 列出 cleanup-recommended-* 标记的分支
echo ""
echo "--- 1. cleanup-recommended tagged branches ---"
TAGS=$(git ls-remote origin | grep -oE 'refs/tags/cleanup-recommended-[0-9-]+' | sort -u || true)
if [ -z "$TAGS" ]; then
  echo "  (none)"
fi

# 2. 列出 backup/* 和 perf/* 旧分支, 用 last commit date 判断
echo ""
echo "--- 2. old backup/perf/feature branches (last commit > 24h) ---"
NOW=$(date +%s)
THRESHOLD=86400  # 24h

for b in $(git branch -r | grep -E "(backup/|perf/|feature/)" | tr -d ' '); do
  bn=$(echo "$b" | sed 's|origin/||')
  LAST=$(git log -1 --format=%ct "origin/$bn" 2>/dev/null || echo 0)
  if [ "$LAST" -eq 0 ]; then continue; fi
  AGE=$(( (NOW - LAST) ))
  if [ "$AGE" -gt "$THRESHOLD" ]; then
    HOURS=$(( AGE / 3600 ))
    echo "  $bn: ${HOURS}h ago"
    if [ "$FORCE" = "--force" ]; then
      git push origin --delete "$bn" 2>&1 | head -2 || echo "    → delete failed"
    fi
  fi
done

if [ "$FORCE" != "--force" ]; then
  echo ""
  echo "(dry-run. Use 'bash scripts/cleanup_old_branches.sh --force' to actually delete)"
fi
