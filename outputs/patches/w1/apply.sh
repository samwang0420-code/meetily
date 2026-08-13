#!/usr/bin/env bash
# 离线会记 — W1 改造应用脚本
# 在已经 fork/clone 的 meetily 仓库根目录(或 frontend/ 目录)运行
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPLACE="$HERE/replacement"

# 推断仓库根
if [[ -d "frontend/src-tauri/src" ]]; then
  ROOT="$(pwd)"
elif [[ -d "src-tauri/src" ]]; then
  ROOT="$(cd .. && pwd)"
else
  echo "!! 必须在 meetily 仓库根或 frontend/ 子目录运行" >&2
  exit 1
fi
echo "==> 仓库根: $ROOT"

# 备份
BACKUP="$HERE/backup-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP"

backup() {
  local rel="$1"
  if [[ -f "$ROOT/$rel" ]]; then
    mkdir -p "$BACKUP/$(dirname "$rel")"
    cp "$ROOT/$rel" "$BACKUP/$rel"
  fi
}

apply_replace() {
  local rel="$1"
  backup "$rel"
  cp "$REPLACE/$rel" "$ROOT/$rel"
  echo "  [REPLACE] $rel"
}

# --- 1. tauri.conf.json: identifier / productName / CSP / 删 updater ---
apply_replace "frontend/src-tauri/tauri.conf.json"

# --- 2. analytics 模块: 整套改 noop ---
apply_replace "frontend/src-tauri/src/analytics/analytics.rs"
apply_replace "frontend/src-tauri/src/analytics/commands.rs"

# --- 3. llm_client.rs: 砍云端 provider ---
backup "frontend/src-tauri/src/summary/llm_client.rs"
python3 "$REPLACE/scripts/llm_client.patch.py" "$ROOT/frontend/src-tauri/src/summary/llm_client.rs"

# --- 4. Cargo.toml: 删 posthog-rs 依赖 ---
backup "frontend/src-tauri/Cargo.toml"
python3 - <<PY
import pathlib
p = pathlib.Path("$ROOT/frontend/src-tauri/Cargo.toml")
s = p.read_text()
old = 'posthog-rs = "0.3.7"\n'
if old in s:
    p.write_text(s.replace(old, "", 1))
    print("  [EDIT] Cargo.toml: 删 posthog-rs = 0.3.7")
else:
    print("  [SKIP] Cargo.toml: posthog-rs 未找到(可能已删)")
PY

echo ""
echo "==> W1 改造应用完成"
echo "    备份: $BACKUP"
echo ""
echo "下一步:"
echo "  cd $ROOT/frontend"
echo "  pnpm install"
echo "  ./clean_run.sh            # Mac dev"
echo "  ./clean_run_windows.bat   # Win dev"
echo "  ./clean_build.sh          # Mac 生产构建"
