#!/usr/bin/env bash
# §94 §6.2: meetily release 前硬闸门, 顺序跑 7 步.
#
# 用法:
#   ./scripts/pre_release_check.sh
#   MEETILY_SHERPA_DAEMONS=2 ./scripts/pre_release_check.sh
#
# 任何 step exit != 0 中止. 用于 release 前必跑, 避免"代码漏".

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

# 颜色
if [[ -t 1 ]]; then
    GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
else
    GREEN=''; YELLOW=''; RED=''; BLUE=''; NC=''
fi

step=0
fail=0

banner() {
    step=$((step+1))
    echo ""
    echo -e "${BLUE}==== Step $step: $1 ====${NC}"
}

check_pass() {
    echo -e "${GREEN}✓ PASS${NC}  $1"
}

check_fail() {
    echo -e "${RED}✗ FAIL${NC}  $1"
    fail=$((fail+1))
}

# Step 1: guard 脚本 (历史 fix 守卫)
banner "guard: check_historical_fixes.py 76+ anchor"
if python3 scripts/check_historical_fixes.py 2>&1 | tail -2; then
    check_pass "guard script"
else
    check_fail "guard script"
    exit 1
fi

# Step 2: audit (全面代码审计, 死代码/孤儿模块/悬空命令/版本号)
banner "audit: audit_codebase.py 0 errors"
if python3 scripts/audit_codebase.py 2>&1 | tail -3; then
    check_pass "audit script"
else
    check_fail "audit script"
    exit 1
fi

# Step 3: cargo test (跳过 system_audio macOS SCK 弹窗测试)
banner "cargo test --lib (skip 3 system_audio SCK 弹窗测试)"
if (cd frontend/src-tauri && cargo test --lib -- --skip audio::system_audio_* --test-threads=1 2>&1 | tail -5); then
    check_pass "cargo test"
else
    check_fail "cargo test"
    exit 1
fi

# Step 4: next build (Next.js 前端)
banner "frontend next build"
if (cd frontend && pnpm run build 2>&1 | tail -5); then
    check_pass "next build"
else
    check_fail "next build"
    exit 1
fi

# Step 5: cargo build --release (Tauri 后端)
banner "cargo build --release"
if (cd frontend/src-tauri && cargo build --release 2>&1 | tail -5); then
    check_pass "cargo build --release"
else
    check_fail "cargo build --release"
    exit 1
fi

# Step 6: sync .app bundle (用户用 `open 言镜 AI.app` 必须跑)
banner "sync_app_bundle.sh (macOS .app bundle sync)"
if [[ "$(uname)" == "Darwin" ]]; then
    if [[ -d "target/release/言镜 AI.app" ]]; then
        if ./scripts/sync_app_bundle.sh 2>&1 | tail -3; then
            check_pass "sync_app_bundle"
        else
            check_fail "sync_app_bundle"
            exit 1
        fi
    else
        echo -e "${YELLOW}⊘ SKIP${NC}  no .app bundle in target/release/ (用户没手造 bundle)"
    fi
else
    echo -e "${YELLOW}⊘ SKIP${NC}  not macOS"
fi

# Step 7: i18n 校验
banner "i18n verify (zh/en keys 对齐)"
if (cd frontend && node ../scripts/verify_i18n.mjs 2>&1 | tail -3); then
    check_pass "i18n"
else
    check_fail "i18n"
    exit 1
fi

echo ""
echo -e "${BLUE}=========================================${NC}"
if [[ $fail -eq 0 ]]; then
    echo -e "${GREEN}✓ ALL ${step} STEPS PASSED${NC}"
    echo ""
    echo "release ready. run:"
    echo "  killall meetily 2>/dev/null"
    echo "  open '$REPO_ROOT/target/release/言镜 AI.app'"
    exit 0
else
    echo -e "${RED}✗ ${fail} STEPS FAILED${NC}"
    exit 1
fi
