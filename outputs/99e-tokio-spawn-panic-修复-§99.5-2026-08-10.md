# §99.5 Tauri setup() tokio::spawn panic 修复 (2026-08-10)

## 事故
用户直接 exec tauri bundle binary:
```bash
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
```
启动 1 秒后 abort, log:
```
thread 'main' (24319961) panicked at frontend/src-tauri/src/lib.rs:610:13:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread caused non-unwinding panic. aborting.
```

## 根因
§99.2 commit 加的 `backfill_meeting_user_ids` 异步 hook 用了 `tokio::spawn`:
```rust
let app_handle = _app.handle().clone();
tokio::spawn(async move {                         // ❌ PANIC
    backfill_meeting_user_ids(&app_handle).await;
});
```

Tauri main thread 是 **tao event loop**, 不是 Tokio runtime. `tokio::spawn` 需要 `tokio::runtime::Handle` 才能拿到 reactor, tao thread 没有, 直接 panic.

## 修复 (commit pending — Codex CLI auto-review 故障)
`frontend/src-tauri/src/lib.rs:610`:
```rust
// tokio::spawn(async move {                  // ❌
tauri::async_runtime::spawn(async move {        // ✅
    if let Err(e) = backfill_meeting_user_ids(&app_handle).await {
        log::warn!("§99.2 backfill_meeting_user_ids failed (best-effort, continue): {}", e);
    }
});
```

参照 §86 (`memory_watcher`) / §88 (`topic_dossier_scheduler`) / §62 (`sherpa_daemon::ensure_started_slot`) 全用 `tauri::async_runtime::spawn`.

## 铁律 (AGENTS.md §9)
1. **任何 Tauri `setup(|_app| { ... })` 闭包里 spawn 异步任务必须用 `tauri::async_runtime::spawn`** — Tauri 2.x 提供这个 wrapper 内部桥接到正确 runtime.
2. **禁止用 `tokio::spawn` / `tokio::task::spawn`** — tao event loop 没有 Tokio reactor.
3. **任何 `commands.rs` (Tauri command handler) 里可以用 `tokio::spawn`** — Tauri command 是在 Tokio runtime 上下文中调度的 (经 `#[tauri::command]` async fn 包装).
4. **判断口诀**: `setup()` 里 → `tauri::async_runtime::spawn`; command handler / Tauri 事件 listener / 普通 tokio task → `tokio::spawn`.
5. **正确参考**: §86 / §88 / §62 全用 `tauri::async_runtime::spawn`.

## §37 硬闸门 (全部通过)
- ✅ cargo check --lib: 0 errors (27 §18 warnings 不动)
- ✅ cargo test --lib: 331 passed / 0 failed / 3 ignored
- ✅ cargo build --release: 1m31s
- ✅ check_historical_fixes.py: **116/116 PASS** (含 §99.5 正向 anchor)

## GUI 启动验证
- 修复前: panic 在 lib.rs:610, 启动 1 秒后 abort
- 修复后: log 走到 `app_lib::whisper_engine::system_monitor] Initializing system monitor`, setup() 完整跑过, 没 panic
- bundle binary: `/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily` (72.6M, mtime 2026-08-10 21:47)

## 用户手动命令 (Codex CLI auto-review 故障)
```bash
cd /Users/wangwei/Documents/离线会记

# 1. 看 staged 状态
git status -s

# 2. 一键 commit + push
git -c user.email=codex@local -c user.name=codex commit -m "fix(§99.5): Tauri setup() tokio::spawn panic 修复

- lib.rs:610 tokio::spawn → tauri::async_runtime::spawn
  Tauri main thread 是 tao event loop, tokio::spawn 会 panic
  参照 §86 §88 §62 全用 tauri::async_runtime::spawn
- AGENTS.md §9 新立 §99.5 铁律
- check_historical_fixes.py 加 §99.5 正向 anchor
- 顺带提交工作树里残留的 §99.3 (sync_app_bundle.sh) + §99.4 (package.json plugin-fs bump)"

git push origin perf/summary-map-concurrency

# 3. Obsidian 双写
cp outputs/99e-tokio-spawn-panic-修复-§99.5-2026-08-10.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/99e-tokio-spawn-panic-修复-§99.5-2026-08-10.md"

# 4. GUI 验收
killall meetily 2>/dev/null
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
# 期望: 窗口出现, log 走到 system_monitor init 后不停
```

## 关联
- §99.2 (backfill_meeting_user_ids — 触发 panic 的代码)
- §86 §88 §62 (正确参考: 全用 tauri::async_runtime::spawn)
- §92 防代码漏 (AGENTS.md § 章节 ≠ 代码 commit)
- §37 硬闸门
