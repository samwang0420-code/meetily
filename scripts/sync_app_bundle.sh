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

# §97 (2026-08-10): 同步 Info.plist 的 CFBundleIdentifier 跟 tauri.conf.json 一致.
# 不同步会触发 macOS 启动 app 时 "意外退出" — LaunchServices 按 Info.plist identifier
# 加载 sandbox/entitlements/钥匙串, 跟 binary 内的 identifier 不匹配就闪退.
TAURI_CONF="$REPO_ROOT/frontend/src-tauri/tauri.conf.json"
if [[ -f "$TAURI_CONF" ]]; then
    EXPECTED_ID=$(plutil -extract identifier raw "$TAURI_CONF" 2>/dev/null || python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['identifier'])" "$TAURI_CONF")
    PLIST_ID=$(plutil -extract CFBundleIdentifier raw "$APP_DIR/Contents/Info.plist" 2>/dev/null || grep -A1 CFBundleIdentifier "$APP_DIR/Contents/Info.plist" | tail -1 | sed -E 's/.*<string>(.*)<\/string>.*/\1/')
    if [[ -n "$EXPECTED_ID" && "$PLIST_ID" != "$EXPECTED_ID" ]]; then
        echo -e "${YELLOW}§97 plist sync${NC}  CFBundleIdentifier  $PLIST_ID  ->  $EXPECTED_ID"
        # 用 sed 替换 Info.plist 里 CFBundleIdentifier 的 <string> 值
        sed -i.bak -E "s|<string>$PLIST_ID</string>|<string>$EXPECTED_ID</string>|" "$APP_DIR/Contents/Info.plist"
        rm -f "$APP_DIR/Contents/Info.plist.bak"
        # 验证
        NEW_ID=$(plutil -extract CFBundleIdentifier raw "$APP_DIR/Contents/Info.plist")
        if [[ "$NEW_ID" != "$EXPECTED_ID" ]]; then
            echo -e "${RED}ERROR${NC}: Info.plist 同步失败 (expected=$EXPECTED_ID, got=$NEW_ID)"
            exit 1
        fi
        plutil -lint "$APP_DIR/Contents/Info.plist" >/dev/null
        echo -e "${GREEN}OK${NC}: §97 Info.plist identifier synced"
    fi
fi

# Sync (先把新 binary 复制到 .app, 然后再 codesign 整个 .app)
# §99.3 (2026-08-10): 之前 sync_app_bundle.sh 顺序错了 — 先 codesign 然后 cp, cp 把刚签的 binary
# 覆盖回未签名的 release/meetily, 每次都说 OK 但下次启动还是闪退 (meetily-f4d07fa731b148b3 identifier
# 跟 Info.plist tech.yanjingai.app 不匹配 → launchd 162 Launch failed).
SRC_HASH=$(shasum "$SRC_BINARY" | awk '{print $1}')
cp -f "$SRC_BINARY" "$DST_BINARY"
DST_HASH=$(shasum "$DST_BINARY" | awk '{print $1}')

if [[ "$SRC_HASH" != "$DST_HASH" ]]; then
    echo -e "${RED}ERROR${NC}: sync 后 hash 不一致 (src=$SRC_HASH, dst=$DST_HASH)"
    exit 1
fi

DST_SIZE=$(stat -f "%z" "$DST_BINARY")
echo -e "${GREEN}OK${NC}: synced  $DST_SIZE bytes  sha=${DST_HASH:0:12}"

# §98 (2026-08-10): codesign identifier 跟 CFBundleIdentifier 不一致 → launchd 162 Launch failed.
# cargo build --release 不重新签名 .app bundle, 旧 binary codesign identifier 还嵌在 Mach-O 内.
# 即使 Info.plist 已经匹配, 每次 sync 都要 re-sign 让 binary 内 codesign identifier 跟新 bundle 一致.
# §99.3: 必须 cp 之后再 codesign, 否则 cp 会覆盖刚签好的 binary.
if [[ -n "$EXPECTED_ID" ]]; then
    # codesign -dv 输出到 stderr 不是 stdout, 必须 2>&1
    CURRENT_BIN_ID=$(codesign -dv "$APP_DIR" 2>&1 | awk -F= '/^Identifier/ {print $2; exit}')
    if [[ "$CURRENT_BIN_ID" != "$EXPECTED_ID" ]]; then
        echo -e "${YELLOW}§98 codesign fix${NC}  current=$CURRENT_BIN_ID  expected=$EXPECTED_ID"
        codesign --remove-signature "$DST_BINARY" 2>/dev/null || true
        codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1
        NEW_BIN_ID=$(codesign -dv "$APP_DIR" 2>&1 | awk -F= '/^Identifier/ {print $2; exit}')
        if [[ "$NEW_BIN_ID" != "$EXPECTED_ID" ]]; then
            echo -e "${RED}ERROR${NC}: codesign 后 identifier 仍不匹配 (expected=$EXPECTED_ID, got=$NEW_BIN_ID)"
            exit 1
        fi
        echo -e "${GREEN}OK${NC}: §98 codesign identifier=$NEW_BIN_ID"
    else
        echo -e "${GREEN}OK${NC}: §98 codesign identifier 已对齐 ($CURRENT_BIN_ID)"
    fi
fi



echo ""
echo "用法: open '$APP_DIR'"

# §99.3 (2026-08-10): 创建 ~/Applications/言镜 AI.app symlink
# 原因: macOS 26 LaunchServices 对 ~/Documents/.../*.app 拒绝扫描
# (com.apple.provenance + 路径含空格 + 用户保护目录 → kLSNoExecutableErr).
# ~/Applications 是 LaunchServices 标准用户目录, symlink 让 .app 可被 `open` 启动.
USER_APPS_DIR="$HOME/Applications"
APP_LINK="$USER_APPS_DIR/言镜 AI.app"
mkdir -p "$USER_APPS_DIR" 2>/dev/null || true
if [[ -L "$APP_LINK" ]] || [[ -e "$APP_LINK" ]]; then
    CURRENT_TARGET=$(readlink "$APP_LINK" 2>/dev/null || echo "")
    if [[ "$CURRENT_TARGET" != "$APP_DIR" ]]; then
        rm -f "$APP_LINK" 2>/dev/null || true
        ln -s "$APP_DIR" "$APP_LINK" 2>/dev/null || true
    fi
else
    ln -s "$APP_DIR" "$APP_LINK" 2>/dev/null || true
fi
if [[ -L "$APP_LINK" ]]; then
    echo -e "${GREEN}OK${NC}: §99.3 symlink ready: $APP_LINK → $(readlink "$APP_LINK")"
    echo "  用法 (LaunchServices 标准目录, 避免 kLSNoExecutableErr): open '$APP_LINK'"
else
    echo -e "${YELLOW}WARN${NC}: §99.3 symlink 创建失败 (sandbox 限制)"
    echo "  请用户手动跑:"
    echo "    ln -sfn '$APP_DIR' '$APP_LINK'"
    echo "  然后: open '$APP_LINK'"
fi

# §99.4 (2026-08-10): 检测 tauri build 官方 bundle 路径
# (target/release/bundle/macos/言镜 AI.app) 优先用这个 — tauri build 跑的 codesign 含 hardened runtime,
# 比 cargo build 输出的更完整. 用户可以三种方式启动:
#   1. 直接 binary:       target/release/meetily
#   2. 直接 bundle exec: target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily
#   3. open .app (受 LaunchServices 扫描限制, ~/Documents 路径常被拒)
TAURI_BUNDLE="$TARGET_DIR/bundle/macos/言镜 AI.app"
if [[ -d "$TAURI_BUNDLE" ]]; then
    echo ""
    echo "§99.4 tauri bundle detected: $TAURI_BUNDLE"
    echo "  推荐启动方式 (绕过 LaunchServices 扫描限制):"
    echo "    '$TAURI_BUNDLE/Contents/MacOS/meetily' &"
    echo "  或: open '$APP_LINK' (symlink path, LaunchServices standard user dir)"
fi
