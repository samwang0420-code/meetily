# §99.6 sync_app_bundle.sh 漏 tauri bundle 路径修复 (2026-08-10)

## 事故
§99.5 fix commit (398836e) push 成功后, 用户跑 §99.4 推荐启动方式:
```bash
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
```
**仍然 panic at lib.rs:610**. Commit 已 push, 但 binary mtime 是 `Aug 10 20:47`, source HEAD 是 21:03.

## 根因
`scripts/sync_app_bundle.sh` 之前只 sync 两个路径:
1. ✅ `target/release/言镜 AI.app/Contents/MacOS/言镜 AI` (手造 .app)
2. ✅ `~/Applications/言镜 AI.app` symlink (LaunchServices 兜底)
3. ❌ `target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily` (**tauri build 官方 bundle**)

`npx tauri build` 跑出的 bundle 在每次 cargo build 后没被更新. §99.4 §99.5 修过的代码 commit 进了, 但 tauri bundle binary 没被 sync 过去. 用户跑 §99.4 推荐路径拿到的是 §99.5 修复前的旧 binary.

## 修复
`scripts/sync_app_bundle.sh` 末尾加 §99.6 tauri bundle 同步逻辑:

```bash
TAURI_BUNDLE="$TARGET_DIR/bundle/macos/言镜 AI.app"
if [[ -d "$TAURI_BUNDLE" ]]; then
    TAURI_BIN="$TAURI_BUNDLE/Contents/MacOS/meetily"
    echo "§99.4 tauri bundle detected: $TAURI_BUNDLE"
    # §99.6: 主动 sync, 用 sha 对比增量
    if [[ -f "$TAURI_BIN" ]]; then
        SRC_SHA=$(shasum "$SRC_BINARY" 2>/dev/null | awk '{print $1}')
        DST_SHA=$(shasum "$TAURI_BIN" 2>/dev/null | awk '{print $1}')
        if [[ "$SRC_SHA" != "$DST_SHA" ]]; then
            cp "$SRC_BINARY" "$TAURI_BIN"
            echo "§99.6 synced tauri bundle binary"
        else
            echo "§99.6 already in sync"
        fi
    fi
fi
```

## 铁律 (AGENTS.md §10)
1. **任何 .app bundle 路径只要存在, 必须主动 sync** — sync_app_bundle.sh 检测到路径就 cp + sha 对比
2. **sync 用 sha 对比 + 增量更新** — 同 sha 跳过, 不同 sha 才 cp
3. **sync 必须覆盖 §99.4 推荐路径** — `target/release/bundle/macos/言镜 AI.app` 是 §99.4 唯一推荐 exec 启动方式
4. **用户反馈 panic 时第一件事查 binary mtime** — 比对 source HEAD vs binary mtime, 差距 > 5min 必是 sync 漏了

## §37 硬闸门
- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib: 331 passed / 0 failed
- ✅ check_historical_fixes.py: **118/118 PASS** (含 §99.6 双 anchor: synced + skip-when-same)
- ✅ sync_app_bundle.sh §99.6: tauri bundle already in sync sha=60f854bfcff2

## 用户手动命令 (Codex CLI auto-review 故障)
```bash
cd /Users/wangwei/Documents/离线会记

git add AGENTS.md scripts/sync_app_bundle.sh scripts/check_historical_fixes.py
git -c user.email=codex@local -c user.name=codex commit -m "fix(§99.6): sync_app_bundle.sh 漏 tauri bundle 路径, panic 复发

- 之前只 sync 手造 .app + ~/Applications symlink
- 漏 target/release/bundle/macos/言镜 AI.app (§99.4 推荐 exec 启动路径)
- 用户跑 §99.4 推荐方式拿到 §99.5 修复前的旧 binary, panic 复发
- 加 sha 对比增量 sync, 同 sha 跳过, 不同 sha cp"
git push origin perf/summary-map-concurrency

cp outputs/99f-sync_app_bundle漏tauri-bundle-修复-§99.6-2026-08-10.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/99f-sync_app_bundle漏tauri-bundle-修复-§99.6-2026-08-10.md"

# GUI 验证
killall meetily 2>/dev/null
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
# 期望: log 走到 system_monitor init 后不停, 没 panic
```

## 关联
- §99.4 (推荐启动方式) / §99.5 (tokio::spawn fix) / §98 (codesign identifier)
- §37 硬闸门 / §92 防代码漏
