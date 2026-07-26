#!/usr/bin/env bash
# install-sherpa-onnx.sh - 下载并安装 sherpa-onnx v1.13.4 macOS arm64 静态库
#
# 用法: bash outputs/scripts/install-sherpa-onnx.sh
# 前置: 已 brew install cmake
#
# 国内推荐镜像顺序:huggingface hf-mirror → github release(直连太慢)
# 若 hf-mirror 失败则尝试 gitee 镜像,最后兜底为官方 github release

set -e

LIBS_DIR="${HOME}/Documents/离线会记/sherpa-libs"
mkdir -p "${LIBS_DIR}"
cd "${LIBS_DIR}"

VER="v1.13.4"
ARCHIVE="sherpa-onnx-${VER}-osx-arm64-static-no-tts.tar.bz2"

# 检查是否已安装
if [ -f "${LIBS_DIR}/lib/libsherpa-onnx.a" ]; then
    echo "✅ sherpa-onnx 静态库已存在: ${LIBS_DIR}/lib/libsherpa-onnx.a"
    exit 0
fi

# 尝试多个镜像(GitHub 直连国内慢,以 hf-mirror 优先)
URLS=(
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/${VER}/${ARCHIVE}"
    "https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/${VER}/${ARCHIVE}"
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.10.28/sherpa-onnx-v1.10.28-osx-universal2-static.tar.bz2"
)

echo "📥 下载 sherpa-onnx ${VER} macOS arm64 ..."
for url in "${URLS[@]}"; do
    echo "  → try: $url"
    if curl -fL --connect-timeout 10 --max-time 1800 -o "${ARCHIVE}" "$url" 2>&1 | tail -1; then
        if [ -s "${ARCHIVE}" ] && [ "$(stat -f%z ${ARCHIVE})" -gt 1000000 ]; then
            echo "  ✅ 下载成功: $(du -h ${ARCHIVE} | awk '{print $1}')"
            break
        fi
    fi
    echo "  ⚠️ 当前源失败,尝试下一个"
    rm -f "${ARCHIVE}"
done

if [ ! -f "${ARCHIVE}" ]; then
    echo "❌ 所有镜像均失败,请手动下载:"
    echo "   1) 浏览器打开 https://github.com/k2-fsa/sherpa-onnx/releases/tag/${VER}"
    echo "   2) 下 ${ARCHIVE}"
    echo "   3) 放到 ${LIBS_DIR}/"
    exit 1
fi

echo "📦 解压 ${ARCHIVE} ..."
tar xjf "${ARCHIVE}"
rm -f "${ARCHIVE}"

# 移动到统一 layout:libsherpa-onnx.a 放到 lib/,头文件放到 include/
LATEST_DIR="$(find "${LIBS_DIR}" -maxdepth 1 -type d -name 'sherpa-onnx-*' | head -1)"
if [ -n "${LATEST_DIR}" ]; then
    mkdir -p "${LIBS_DIR}/lib"
    mkdir -p "${LIBS_DIR}/include"
    if [ -d "${LATEST_DIR}/lib" ]; then
        cp -f "${LATEST_DIR}/lib/"*.a "${LIBS_DIR}/lib/" 2>/dev/null || true
    fi
    if [ -d "${LATEST_DIR}/include" ]; then
        cp -rf "${LATEST_DIR}/include/"* "${LIBS_DIR}/include/" 2>/dev/null || true
    fi
    if [ -d "${LATEST_DIR}/bin" ]; then
        cp -rf "${LATEST_DIR}/bin/"* "${LIBS_DIR}/lib/" 2>/dev/null || true
    fi
fi

if [ -f "${LIBS_DIR}/lib/libsherpa-onnx.a" ]; then
    echo "✅ 静态库已就绪: ${LIBS_DIR}/lib/libsherpa-onnx.a"
else
    echo "❌ 解压后未找到 libsherpa-onnx.a,请检查 archive 内容"
    ls -la "${LIBS_DIR}/"
    exit 1
fi

echo ""
echo "下一步:"
echo "  在仓库根目录执行: cargo check -p meetily"
echo "  build.rs 会自动读 SHERPA_LIB_PATH 或默认 \${LIBS_DIR}"
