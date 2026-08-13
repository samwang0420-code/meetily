#!/bin/zsh
# 离线会记 — 一键启动脚本(W1.13 release 模式)
# 用 release binary(直接 embed 静态资源)而非 debug binary(需要 dev server)
# 用法: bash /Users/wangwei/Documents/离线会记/outputs/start-meetily.sh

set -e
REPO=/Users/wangwei/Documents/meetily

echo "=== 1. 启 Ollama(后台) ==="
if curl -sL --max-time 2 http://localhost:11434/api/version > /dev/null 2>&1; then
  echo "  ✅ Ollama 已在跑"
else
  nohup /opt/homebrew/opt/ollama/bin/ollama serve > /tmp/ollama.log 2>&1 &
  echo "  PID: $!"
  sleep 4
  curl -sL --max-time 2 http://localhost:11434/api/version > /dev/null 2>&1 || { echo "  ❌ Ollama 启动失败"; tail /tmp/ollama.log; exit 1; }
  echo "  ✅ Ollama 起来了"
fi

echo ""
echo "=== 2. 检查 release binary ==="
BIN=${REPO}/target/release/meetily
if [ ! -f "$BIN" ]; then
  echo "  ⚠️ release binary 不存在,使用 debug binary (需要 next dev 配合)"
  BIN=${REPO}/target/debug/meetily
  NEED_NEXT=1
fi
ls -lh "$BIN" | awk '{print "  binary:",$5,$9}'

echo ""
echo "=== 3. 启 Tauri/Meetily(GUI) ==="
if pgrep -f "target/release/meetily\|target/debug/meetily" > /dev/null 2>&1; then
  pgrep -f "target/release/meetily\|target/debug/meetily" | xargs kill -9 2>/dev/null
  sleep 1
fi
nohup "$BIN" > /tmp/meetily.log 2>&1 &
echo "  PID: $!"
sleep 5
if pgrep -f "target/release/meetily\|target/debug/meetily" > /dev/null 2>&1; then
  echo "  ✅ Meetily 起来了"
else
  echo "  ❌ Meetily 启动失败,看 /tmp/meetily.log"
  tail -20 /tmp/meetily.log
  exit 1
fi

echo ""
echo "=== ✅ 启动完成 ==="
echo ""
echo "GUI 应该在桌面上"
echo "如果看不到: open -a $BIN"
echo ""
echo "11:30 检查清单:"
echo "  1. 看 GUI 是否正常(无白屏)"
echo "  2. 点开始录音"
echo "  3. 录 30 秒中文"
echo "  4. 看转录是否正常"
echo "  5. 截图给我"
