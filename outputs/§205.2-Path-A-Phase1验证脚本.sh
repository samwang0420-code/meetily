#!/bin/bash
# §205.2 Path A — XHToken llama.cpp fork 编译 + Qwen3.5 tok/s 验证
# 用户在 Mac 终端执行 (sandbox 拦不住 git clone / cmake)
# 输出: /tmp/spark_phase1_a_result.log
# 用法: bash outputs/§205.2-Path-A-Phase1验证脚本.sh

set -e

LOG=/tmp/spark_phase1_a_result.log
: > "$LOG"
exec > >(tee -a "$LOG") 2>&1

echo "========================================"
echo "§205.2 Path A — Phase 1 验证"
echo "时间: $(date)"
echo "========================================"

cd /tmp

# Step 1: 拉 XHToken fork (含 spark2_5 架构支持)
echo ""
echo "[1/5] git clone XHToken/llama.cpp fork ..."
if [[ -d xhtoken-llama.cpp ]]; then
    echo "  复用已存在目录: cd xhtoken-llama.cpp && git pull"
    cd xhtoken-llama.cpp && git pull --depth 1
else
    git clone --depth 1 https://github.com/XHToken/llama.cpp.git xhtoken-llama.cpp
    cd xhtoken-llama.cpp
fi

# Step 2: cmake build with Metal
echo ""
echo "[2/5] cmake build with Metal (8 核并行) ..."
cmake -S . -B build -DGGML_METAL=ON 2>&1 | tail -5
cmake --build build --parallel 8 2>&1 | tail -10

# Step 3: 验证 binary
echo ""
echo "[3/5] 验证 build 输出 ..."
ls -la build/bin/llama-cli
./build/bin/llama-cli --version 2>&1 | head -3 || true

# Step 4: 跑 Qwen3.5-2B baseline 对比 §197 (7.44 tok/s 实测)
echo ""
echo "[4/5] Qwen3.5-2B tok/s 验证 (与 §197 baseline 对比) ..."
MODEL="$HOME/Library/Application Support/tech.yanjingai.app/models/summary/Qwen3.5-2B-Q4_K_M.gguf"
if [[ ! -f "$MODEL" ]]; then
    echo "  ⚠️ 模型不存在: $MODEL"
    echo "  找一下:"
    find "$HOME/Library/Application Support/tech.yanjingai.app/models/" -name "Qwen3.5*.gguf" 2>/dev/null
    exit 1
fi

./build/bin/llama-cli -m "$MODEL" -p "请用一句话总结刑事案件庭审记录的核心争议焦点" -n 200 --metal -ngl 24 -t 8 2>&1 | tail -30

# Step 5: 提取关键数字
echo ""
echo "[5/5] 结果评估 ..."
TOK_S=$(./build/bin/llama-cli -m "$MODEL" -p "测试" -n 100 --metal -ngl 24 -t 8 2>&1 | grep -oE "[0-9]+\.[0-9]+ tokens per second" | tail -1 | awk '{print $1}')

echo ""
echo "========================================"
echo "📊 关键数字"
echo "========================================"
echo "XHToken fork Qwen3.5 tok/s: ${TOK_S:-未提取到 (查日志最后一行的 eval time 行)}"
echo "§197 baseline (Qwen3.5 + 0.1.146): 7.44 tok/s"
echo ""
echo "========================================"
echo "🎯 决策矩阵"
echo "========================================"
if [[ -n "$TOK_S" ]]; then
    if (( $(echo "$TOK_S > 7.44" | bc -l) )); then
        echo "  ✓ $TOK_S > 7.44 — XHToken fork 更优"
        echo "    → 撤回 §197 llama-cpp-2 0.1.146 限制"
        echo "    → 用 XHToken fork 编 Spark (1.7B, F16 3.27GB 或量化 Q4_K_M ~1.1GB)"
    else
        echo "  ✗ $TOK_S ≤ 7.44 — §197 baseline 仍是 Apple Silicon Q4_K 最优"
        echo "    → 走 §205.2 Path B (MLX) 或 Path C (暂停 Spark)"
    fi
fi

echo ""
echo "========================================"
echo "完整日志: $LOG"
echo "完成后把 $LOG 的 tok/s 数字告诉 Codex 即可"
echo "========================================"
