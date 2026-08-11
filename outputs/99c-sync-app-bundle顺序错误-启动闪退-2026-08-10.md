# §99.3 — sync_app_bundle.sh 顺序错 + LaunchServices 闪退修复 (2026-08-10)

**触发**: 用户 `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'` 报"意外退出".

## 根因 — 两个叠加 bug

### Bug 1: sync_app_bundle.sh 顺序错

§93 脚本原本顺序:
1. 检查 Info.plist (line 87)
2. 检查 codesign identifier (line 96) — 可能重签整个 .app
3. **`cp -f SRC_BINARY DST_BINARY`** (line 113) — **覆盖 binary 回到未签名状态!**

所以每次跑 sync 都"修"一次 codesign (打印 OK), 但 cp 又覆盖回去.
下次 cargo build --release 后, 旧 binary identifier 又浮现.

**修复**: cp 必须在 codesign 之前. 脚本重排序:
- §97 Info.plist 同步
- **§93 cp binary** (新位置)
- §98 codesign identifier 同步

### Bug 2: macOS LaunchServices 拒绝扫描 .app

`com.apple.provenance` extended attribute (macOS 14+ 安全机制) + 路径在 `~/Documents/...` (用户保护目录) + 路径含空格 (`言镜 AI.app`) →
LaunchServices 拒绝注册 → `kLSNoExecutableErr: The executable is missing` → `open` 命令启动后 panic at `_RegisterApplication`.

Crash backtrace (从 `~/Library/Logs/DiagnosticReports/言镜 AI-*.ips`):
```
+  191736  ___RegisterApplication_block_invoke
+   111792  _dispatch_client_callout
+    17968  _dispatch_once_callout
+     6348  _RegisterApplication
+     5864  GetCurrentProcess
+ 16683772  -[NSMenuBarPresentationInstance _getAggregateUIMode:withOptions:]
+     33340  _NSInitializeAppContext
+     25100  -[NSApplication init]
+     23808  +[NSApplication sharedApplication]
+   7676552  tauri_runtime_wry::Wry::new
+   8209180  tauri::app::Builder::build
```

**修复**: 创建 symlink 到 LaunchServices 标准用户目录 `~/Applications/`:
```bash
ln -sfn '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app' \
        '/Users/wangwei/Applications/言镜 AI.app'
open '/Users/wangwei/Applications/言镜 AI.app'
```

## 修改 (commit §99.3, 待用户 push)

### 1. `scripts/sync_app_bundle.sh`
- 把 `cp -f` 块从底部移到 §98 codesign 块之前
- §99.3 末尾加 symlink 创建逻辑 (失败时给用户清楚提示)
- ln 加 `|| true` 防 `set -e` 中断

### 2. `scripts/check_historical_fixes.py`
3 个新 §99.3 anchor:
- `99_3_sync_cp_before_codesign`: cp 在 codesign 之前
- `99_3_apps_dir_symlink`: `USER_APPS_DIR=$HOME/Applications`
- `99_3_open_symlink_hint`: 提示用 symlink 路径 open

**guard**: 113/113 PASS (110 → 113)

## 用户手动跑命令 (Codex CLI sandbox auto-review 持续故障)

```bash
cd /Users/wangwei/Documents/离线会记

# 1. 跑 sync (会建 symlink + codesign)
bash scripts/sync_app_bundle.sh

# 2. 如果 symlink 没建成功, 手动建
ln -sfn '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app' \
        '/Users/wangwei/Applications/言镜 AI.app'

# 3. commit + push (§99.3 修复)
git -c user.email=codex@local -c user.name=codex add \
  scripts/sync_app_bundle.sh scripts/check_historical_fixes.py
git -c user.email=codex@local -c user.name=codex commit -m "fix(§99.3): sync_app_bundle.sh 顺序修复 + ~/Applications symlink"
git push origin perf/summary-map-concurrency

# 4. 验证 (用 symlink 路径)
open '/Users/wangwei/Applications/言镜 AI.app'
```

期望:
- §93 sync + §98 codesign + §99.3 symlink 全 OK
- `open '/Users/wangwei/Applications/言镜 AI.app'` 启动成功, 窗口出现
- 不再 "意外退出"

## §37 闸门

- ✅ cargo check --lib: 0 errors (27 §18 warnings 不动)
- ✅ cargo test --lib: 331 passed / 0 failed / 3 ignored
- ✅ check_historical_fixes.py: **113/113 PASS**
- ✅ sync_app_bundle.sh: cp 在 codesign 之前, 末尾建 symlink
- ✅ codesign -dvv: Identifier=tech.yanjingai.app (不是 meetily-f4d07fa731b148b3)
- ✅ binary mtime 12:53, codesign identifier 对齐

## 已知边界

- macOS 偶发 launchd 162 "Launch failed": 等 3-5s 再 open (与 §92 一致)
- 如果 `~/Applications/言镜 AI.app` symlink 已存在但指向旧路径, sync 自动更新
- codesign 是 adhoc (无 Team ID), 首次启动可能弹权限框
- LaunchServices 缓存可能需要重启 macOS 才完全重建 (兜底方案)

## 关联

- §93 (sync_app_bundle.sh 初始)
- §97 (identifier 改造 + Info.plist 同步)
- §98 (sqlx checksum + codesign identifier 三件套)
- §99 / §99.2 (前两个 import bug 修复)
- [[99c-sync-app-bundle顺序错误-启动闪退]] (Obsidian 主份)
