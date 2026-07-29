#!/bin/zsh
# 离线会记 v0.7.0-rc1 启动脚本
# 双击此文件启动 (在 Finder 里)
# Tag: v0.7.0-rc1 (commit a668d1f)

set -e

BIN="/Users/wangwei/Documents/meetily/target/release/meetily"

if [[ ! -x "$BIN" ]]; then
  osascript -e 'display dialog "Binary not found:\n'"$BIN"'\n\n请先运行:\ncd /Users/wangwei/Documents/meetily/frontend/src-tauri && cargo build --release" buttons {"OK"} default button "OK" with icon stop'
  exit 1
fi

# 通过 launchd GUI session 启动 (避免 CLI silent abort)
nohup "$BIN" >/dev/null 2>&1 &
disown

# 给 Finder 一个友好的通知
osascript -e 'display notification "v0.7.0-rc1 已启动" with title "离线会记" subtitle "检查托盘图标"' 2>/dev/null || true

exit 0
