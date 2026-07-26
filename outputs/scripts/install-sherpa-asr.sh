#!/usr/bin/env bash
# install-sherpa-asr.sh - 下载国产 ASR 模型(Paraformer + SenseVoice,均 INT8 量化)
#
# 国内强制走 hf-mirror.com(GitHub 国内慢,ModelScope 找不到精确命名的 repo)
# 默认路径: ~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/
#
# 254MB+228MB+~400KB 总计约 482MB,家宽 5-10 分钟

set -e

MODELS_DIR="${HOME}/Library/Application Support/cn.lixianhuiji.app/models/sherpa"
mkdir -p "${MODELS_DIR}"
cd "${MODELS_DIR}"

# 默认免费:Paraformer-zh INT8 (217MB onnx + 74KB tokens)
PARAFORMER_DIR="${MODELS_DIR}/paraformer-zh-int8"
PARAFORMER_MODEL="${PARAFORMER_DIR}/model.int8.onnx"
PARAFORMER_TOKENS="${PARAFORMER_DIR}/tokens.txt"

# Pro:SenseVoice 多语种 INT8 (228MB onnx + 308KB tokens)
SENSEVOICE_DIR="${MODELS_DIR}/sense-voice-zh-int8"
SENSEVOICE_MODEL="${SENSEVOICE_DIR}/model.int8.onnx"
SENSEVOICE_TOKENS="${SENSEVOICE_DIR}/tokens.txt"

BASE_URL="https://hf-mirror.com/csukuangfj"

download() {
    local name="$1"
    local url="$2"
    local target="$3"
    if [ -f "${target}" ] && [ "$(stat -f%z ${target})" -gt 1000000 ]; then
        echo "  ✅ ${name} 已存在: $(du -h ${target} | awk '{print $1}')"
        return 0
    fi
    echo "  📥 ${name} ← ${url}"
    curl -fL --connect-timeout 10 --max-time 1800 -o "${target}" "${url}" 2>&1 | tail -1
    if [ ! -s "${target}" ]; then
        echo "  ❌ ${name} 下载失败"
        return 1
    fi
    echo "  ✅ ${name}: $(du -h ${target} | awk '{print $1}')"
}

# 1) Paraformer (默认 - 免费用户即可用)
echo "=== 步骤 1/2: Paraformer-zh INT8 模型 ==="
mkdir -p "${PARAFORMER_DIR}"
download "paraformer-model" "${BASE_URL}/sherpa-onnx-paraformer-zh-2024-03-09/resolve/main/model.int8.onnx" "${PARAFORMER_MODEL}"
download "paraformer-tokens" "${BASE_URL}/sherpa-onnx-paraformer-zh-2024-03-09/resolve/main/tokens.txt" "${PARAFORMER_TOKENS}"

# 2) SenseVoice (Pro 独占 - 需开通 Pro 后再下)
echo ""
echo "=== 步骤 2/2: SenseVoice-zh INT8 模型(Pro 会员独占)==="
mkdir -p "${SENSEVOICE_DIR}"
download "sensevoice-model" "${BASE_URL}/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx" "${SENSEVOICE_MODEL}"
download "sensevoice-tokens" "${BASE_URL}/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt" "${SENSEVOICE_TOKENS}"

echo ""
echo "✅ 模型安装完成"
echo "   Paraformer (免费): ${PARAFORMER_DIR}"
echo "   SenseVoice (Pro):  ${SENSEVOICE_DIR}"
echo ""
echo "下次启动 meetily 时会自动检测可用模型,在 TranscriptSettings 切换即可"
