#!/usr/bin/env bash
# §93: 把 `target/release/meetily` 同步到 `target/release/言镜 AI.app/Contents/MacOS/言镜 AI`
# 用途: macOS 上, .app bundle (用户用 `open '言镜 AI.app'`) 是独立 binary,
#       `cargo build --release` 默认只更新 `target/release/meetily`. 不跑这个脚本,
#       用户跑 .app bundle 时看到的是旧 binary.
#
# 用法:
#   ./scripts/sync_app_bundle.sh            # 默认 sync release profile
#   ./scripts/sync_app_bundle.sh --debug    # sync debug profile
#   MEETILY_SHERPA_DAEMONS=2 ./scripts/sync_app_bundle.sh   # 自定义 daemon 数
#
# §37 闸门: scripts/check_historical_fixes.py 包含 `93_app_bundle_synced` anchor,
#           它检查 .app bundle 内部 binary mtime >= target/release/meetily mtime.

set -euo pipefail

PROFILE="${1:-release}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
# §93: src-tauri 是 workspace member, cargo 用 workspace root target/, 不是 frontend/target/
# Workspace root: /Users/wangwei/Documents/离线会记/Cargo.toml
TARGET_DIR="$REPO_ROOT/target/$PROFILE"
SRC_BINARY="$TARGET_DIR/meetily"
APP_DIR="$TARGET_DIR/言镜 AI.app"
DST_BINARY="$APP_DIR/Contents/MacOS/言镜 AI"

# 颜色 (macOS bash 不一定支持, fallback)
if [[ -t 1 ]]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    RED='\033[0;31m'
    NC='\033[0m'
else
    GREEN=''; YELLOW=''; RED=''; NC=''
fi

echo -e "${YELLOW}§93 sync_app_bundle${NC}  profile=$PROFILE"
echo "  src: $SRC_BINARY"
echo "  dst: $DST_BINARY"

# 检查源 binary
if [[ ! -f "$SRC_BINARY" ]]; then
    echo -e "${RED}ERROR${NC}: 源 binary 不存在: $SRC_BINARY"
    echo "请先跑: cd frontend && cargo build --$PROFILE"
    exit 1
fi

# 检查 .app bundle
if [[ ! -d "$APP_DIR" ]]; then
    echo -e "${YELLOW}SKIP${NC}: .app bundle 不存在: $APP_DIR"
    echo "如果你用 `cargo run` 或 `target/release/meetily` 直接跑, 不需要这个脚本."
    echo "如果你想用 macOS .app bundle (open 言镜 AI.app), 跑: ./scripts/make_app_bundle.sh 首次造 bundle"
    exit 0
fi

# 检查 Info.plist + Resources
if [[ ! -f "$APP_DIR/Contents/Info.plist" ]]; then
    echo -e "${RED}ERROR${NC}: $APP_DIR/Contents/Info.plist 不存在"
    echo "  .app bundle 不完整. 删了重建: rm -rf '$APP_DIR' && ./scripts/make_app_bundle.sh"
    exit 1
fi
if [[ ! -d "$APP_DIR/Contents/Resources" ]]; then
    echo -e "${RED}ERROR${NC}: $APP_DIR/Contents/Resources 不存在"
    exit 1
fi

# Sync
SRC_HASH=$(shasum "$SRC_BINARY" | awk '{print $1}')
cp -f "$SRC_BINARY" "$DST_BINARY"
DST_HASH=$(shasum "$DST_BINARY" | awk '{print $1}')

if [[ "$SRC_HASH" != "$DST_HASH" ]]; then
    echo -e "${RED}ERROR${NC}: sync 后 hash 不一致 (src=$SRC_HASH, dst=$DST_HASH)"
    exit 1
fi

DST_SIZE=$(stat -f "%z" "$DST_BINARY")
echo -e "${GREEN}OK${NC}: synced  $DST_SIZE bytes  sha=${DST_HASH:0:12}"
echo ""
echo "用法: open '$APP_DIR'"
