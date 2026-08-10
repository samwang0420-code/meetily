---
title: §96 Python interpreter 探测 + 用户 site-packages 显式注入 (2026-08-09)
date: 2026-08-09
author: Codex CLI
type: decision-log
trigger: §95 落地后 ASR daemon 仍找不到 sherpa_onnx (macOS launchd 精简 PATH)
---

# §96 — Python interpreter 探测 + PYTHONUSERBASE / PYTHONUNBUFFERED 注入

## 触发

§95 (commit 726c794) 落地 import.rs 三分支永不 fallback Whisper 后, 用户实测仍报 sherpa daemon 启动失败.
根因: macOS Tauri app bundle 启动时 launchd 注入精简 PATH, 不含 `/opt/homebrew/bin`.
`which python3` fallback 到 `/usr/bin/python3` (Xcode CLT), 该 Python 只装 numpy 没装 sherpa_onnx.
错误信息 "No module named 'numpy'" 实际是 sherpa_onnx 缺失, 但 stderr 输出被吞.

## 修复 (frontend/src-tauri/src/audio/sherpa_daemon.rs, +57/-2)

### 1. python_path() 改为 7 候选 + 真 import 探测 + OnceLock 缓存

候选路径 (按优先级):
1. `/opt/homebrew/bin/python3`
2. `/opt/homebrew/opt/python@3.14/bin/python3.14`
3. `/opt/homebrew/opt/python@3.13/bin/python3.13`
4. `/opt/homebrew/opt/python@3.12/bin/python3.12`
5. `/usr/local/bin/python3`
6. `/opt/local/bin/python3`
7. `/Library/Frameworks/Python.framework/Versions/Current/bin/python3`

探测 = `python -c "import sherpa_onnx, numpy, soundfile; print('OK')"`.
三个 module 全部能 import 才返回. 否则继续下一个候选, 全失败用 PATH 兜底.
结果用 `OnceLock<String>` 缓存, app 启动后 python 路径不变.

### 2. spawn Python 子进程时显式传 env

- `PYTHONUSERBASE=$HOME` → 强制 Python 看 `~/Library/Python/<ver>/lib/python/site-packages`
- `PYTHONUNBUFFERED=1` → stderr 不缓冲, daemon 启动失败立刻能看

## 验证 (§37 6 步硬闸门)

- ✅ cargo check --lib: 0 errors (27 §18 warnings, +1 §96 dropping_copy_types)
- ✅ cargo test --lib: 327 passed / 0 failed / 2 ignored (system_audio SCK)
- ✅ guard `check_historical_fixes.py`: 87/87 PASS
- ✅ audit `audit_codebase.py`: 0 errors / 0 warns / 60 info

## 边界

- **OnceLock 缓存**: app 启动后 python 路径不变, 不支持热切换
- **探测失败兜底**: `which python3` 最后兜底, 失败用 `/usr/bin/python3` (一定失败, 但有 warn 日志)

## 关联

- §95 commit 726c794 (import.rs 三分支, 永不 fallback Whisper)
- §63 commit 2fe96d7 (provider sensevoice-zh → funasr-nano-zh)
- §62 A commit 2fe96d7 (多 daemon 池, 默认 3)

## §15 GUI 验收 (用户必做)

```
killall meetily && open /Users/wangwei/Documents/离线会记/target/release/言镜\ AI.app
```
1. 启动后 logs 应见 `[sherpa] §96 python3 selected (probe OK): /opt/homebrew/bin/python3`
2. 录 30s 中文 → DB `transcripts ORDER BY id DESC LIMIT 1` ≥ 1
3. Python 子进程数 = 3 (Section 62 A 多 daemon 默认)
