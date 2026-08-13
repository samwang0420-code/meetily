#!/bin/zsh
# 离线会记 — 健康检查(11:30 跑这个看哪挂了)
echo "=== 离线会记健康检查 ==="
echo ""

# 1. Ollama
echo "1. Ollama 端口 11434"
if curl -sL --max-time 2 http://localhost:11434/api/version > /dev/null 2>&1; then
  echo "   ✅ Ollama 跑着: $(curl -sL http://localhost:11434/api/version)"
  if command -v ollama &> /dev/null; then
    echo "   模型: $(ollama list 2>/dev/null | head -3 | tail -1 | awk '{print $1, $3}')"
  fi
else
  echo "   ❌ Ollama 没跑,跑: /opt/homebrew/opt/ollama/bin/ollama serve &"
fi
echo ""

# 2. Whisper 模型
echo "2. Whisper ggml-small.bin"
MODEL="/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/models/ggml-small.bin"
if [[ -f "$MODEL" ]]; then
  SIZE=$(stat -f%z "$MODEL")
  if [[ $SIZE -gt 400000000 ]]; then
    echo "   ✅ 模型在,$((SIZE/1024/1024))MB,位置正确"
  else
    echo "   ⚠️  模型存在但只有 $((SIZE/1024/1024))MB,可能损坏,重新下:"
    echo "      curl -sL --max-time 60 -o '$MODEL' 'https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin'"
  fi
else
  echo "   ❌ 模型不在,下:"
  echo "      mkdir -p '/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/models/'"
  echo "      curl -sL --max-time 60 -o '$MODEL' 'https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin'"
fi
echo ""

# 3. 数据库配置
echo "3. SQLite 配置"
DB="/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite"
if [[ -f "$DB" ]]; then
  echo "   ✅ DB 在"
  echo "   settings: $(sqlite3 "$DB" "SELECT provider||','||model||','||whisperModel FROM settings")"
  echo "   transcript: $(sqlite3 "$DB" "SELECT provider||','||model FROM transcript_settings")"
else
  echo "   ❌ DB 不在,meetily 第一次启会自动建"
fi
echo ""

# 4. Next dev
echo "4. Next dev 端口 3118"
if lsof -i :3118 > /dev/null 2>&1; then
  echo "   ✅ Next dev 跑着"
else
  echo "   ❌ Next dev 没跑,跑: cd /Users/wangwei/Documents/meetily/frontend && pnpm dev &"
fi
echo ""

# 5. Meetily 二进制
echo "5. Meetily 二进制"
if [[ -f /Users/wangwei/Documents/meetily/target/debug/meetily ]]; then
  if pgrep -f "target/debug/meetily" > /dev/null 2>&1; then
    echo "   ✅ Meetily 跑着(PID: $(pgrep -f 'target/debug/meetily'))"
  else
    echo "   ⚠️  Meetily 二进制在,但没跑,跑: /Users/wangwei/Documents/meetily/target/debug/meetily &"
  fi
else
  echo "   ❌ Meetily 二进制不在,需要 cargo build"
fi
echo ""

echo "=== 总结 ==="
echo "若以上 5 项全 ✅:打开 meetily 窗口测录音"
echo "若某项 ❌:跑 /Users/wangwei/Documents/离线会记/outputs/start-meetily.sh 自动重启"
