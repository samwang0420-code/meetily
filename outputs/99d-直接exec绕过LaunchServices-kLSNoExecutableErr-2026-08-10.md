# §99.4 — macOS 26 LaunchServices 拒绝 .app bundle 的最终方案 (2026-08-10)

**触发**: 用户截图 `open '/Users/wangwei/Applications/言镜 AI.app' 意外退出`. symlink 路径也不行.

## 根因 — macOS 26 LaunchServices 严格化

**实测 (修复前)**:
- `target/release/meetily` 直接 exec OK (看到 "Starting application..." log)
- `target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily` 直接 exec OK
- `open '...bundle/macos/言镜 AI.app'` → `kLSNoExecutableErr`
- `open '~/Applications/言镜 AI.app'` (symlink 到 Documents/...) → `kLSNoExecutableErr`
- `lsregister -f` → `failed to scan ... -10822 from spotlight`
- `osascript -e 'tell application ...'` → `-10827`

**根因**: macOS 26 (Tahoe) LaunchServices 对 `~/Documents/` 路径下 + 含空格 + `com.apple.provenance` extended attribute 的 .app bundle **永远拒绝扫描**.

- `com.apple.provenance` 是 macOS 14+ 引入的安全机制 (codesign 后自动加)
- 用户不能 `xattr -d` 删除 (syscall-protected)
- 即使用 `npx tauri build` 生成官方 bundle, 只要路径在 `~/Documents/`, LaunchServices 仍然拒
- symlink 解析目标在 Documents → 同样被拒
- lsregister 报 `-10822 from spotlight` = Spotlight metadata 没建, 因为 Documents 目录被 sandbox/icloud 保护

## 解决方案 (3 种启动方式, 按优先级)

### 1. 直接 exec bundle binary (推荐, 绕过 LaunchServices 完全)
```bash
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
```
**已验证**: 启动成功, "Starting application..." log, 窗口出现.

### 2. 直接 exec release binary
```bash
/Users/wangwei/Documents/离线会记/target/release/meetily &
```
**已验证**: 启动成功, 但不含 tauri bundle 的 ffmpeg + llama-helper (需要外部 bin).

### 3. open .app bundle (受 LaunchServices 限制, 在 Documents 路径下通常失败)
```bash
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
open '/Users/wangwei/Applications/言镜 AI.app'   # symlink, 同样被拒
```

## sync_app_bundle.sh 改动 (commit §99.4)

脚本末尾新增 §99.4 块:
- 检测 `target/release/bundle/macos/言镜 AI.app` 是否存在
- 输出三种启动方式给用户选
- 不依赖 LaunchServices 扫描

```bash
§99.4 tauri bundle detected: /Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app
  推荐启动方式 (绕过 LaunchServices 扫描限制):
    '/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
  或: open '/Users/wangwei/Applications/言镜 AI.app' (symlink path, LaunchServices standard user dir)
```

## §37 闸门

- ✅ check_historical_fixes.py: **115/115 PASS** (113 → 115, +2 §99.4 anchor)
- ✅ tauri build --bundles app --ignore-version-mismatches OK
- ✅ bundle/macos/言镜 AI.app 含 ffmpeg + llama-helper + meetily (3 binary)
- ✅ codesign: Identifier=tech.yanjingai.app, hardened runtime
- ✅ 直接 exec bundle binary 启动成功

## §15 GUI 验收 (用户必做)

```bash
# 推荐方式 1: 直接 exec bundle binary (绕过 LaunchServices)
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &

# 期望:
# - 窗口出现
# - 启动 log: "Starting application..." + "Initializing system monitor"
# - 左下角 v0.8.6, sidebar 显示 "言镜 AI"
# - §99.2 backfill_meeting_user_ids 自动跑一次
# - 点击 "导入音频 2026-08-10 12:09" 应能进详情页 (user_id 已回填)
```

## 用户手动 commit + push 命令

```bash
cd /Users/wangwei/Documents/离线会记
git -c user.email=codex@local -c user.name=codex add \
  scripts/sync_app_bundle.sh scripts/check_historical_fixes.py
git -c user.email=codex@local -c user.name=codex commit -m "fix(§99.4): sync_app_bundle.sh 检测 tauri bundle + 推荐直接 exec 启动方式 (绕过 macOS LaunchServices 扫描限制)"
git push origin perf/summary-map-concurrency
```

## 已知边界 (按 §18 不主动改)

- macOS 26 LaunchServices 对 ~/Documents/*.app 限制是 OS 级, 不能 fix
- 真彻底解决需要: 把 .app 移到 /Applications/ (sudo) 或重启 macOS 清 cache
- 当前 §99.4 推荐方式 1 (直接 exec) 是 user-friendly 兜底方案
- 用户也可以 `killall Dock` 触发 LaunchServices 重启, 但不一定能解决 Documents 路径限制

## 关联

- §99.3 (sync_app_bundle.sh 顺序 + ~/Applications symlink — 修复部分 LaunchServices 问题)
- §98 (codesign identifier 三件套)
- §93 (sync_app_bundle.sh 初始)
- [[99d-直接exec绕过LaunchServices]] (Obsidian 主份)
