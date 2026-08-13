# §108 sync_app_bundle.sh 缺 sidecar 同步 — llama-helper not found 修复 (2026-08-12)

## 触发

用户 8/12 截图反馈:
1. 生成摘要报错 "会议纪要生成失败 / llama-helper binary not found. Build with 'cd llama-helper && cargo build --release' or set MEETILY_LLAMA_HELPER env var."
2. binary 12:39 已更新 (+1120 bytes) 没生效

## 根因 (1 跳)

§90 commit `fda59cd` (8/7 01:51) 手造了 `target/release/言镜 AI.app/` bundle, 但 bundle 里**只有** `言镜 AI` 一个 binary.

后续 `cargo build --release` 只更新 `target/release/meetily`, `sync_app_bundle.sh` §99.6 只同步这个 binary. **llama-helper + ffmpeg 两个 sidecar 一直没被 sync 进 bundle.**

`tConfig.json`:
```json
"externalBin": [
  "binaries/llama-helper",
  "binaries/ffmpeg"
]
```

`externalBin` 是 Tauri 约定的 sidecar binary 声明, 但 `sync_app_bundle.sh` 没处理这两个. 用户启动 `.app` bundle 时:
1. main binary 启动 OK (因为 sync 了)
2. 调 llama-helper 时找不到 sidecar binary
3. 报 "llama-helper binary not found"

## 实证

### 1. 用户 bundle 状态 (修复前)

```bash
$ ls -la '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app/Contents/MacOS/'
total 141128
-rwxr-xr-x@ 1 wangwei  staff  72253680 Aug 12 13:27 言镜 AI
# 缺 llama-helper + ffmpeg
```

### 2. Tauri 官方 bundle 状态 (修复前, 8/10 编的)

```bash
$ ls -la '/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/'
total 251960
-rwxr-xr-x@ 1 wangwei  staff  51271200 Aug 10 20:47 ffmpeg
-rwxr-xr-x@ 1 wangwei  staff   5068704 Aug 10 20:47 llama-helper
-rwxr-xr-x@ 1 wangwei  staff  72657984 Aug 12 13:27 meetily
# 完整 3 个 binary
```

### 3. 修复后

```bash
$ bash scripts/sync_app_bundle.sh
OK: synced  72657984 bytes  sha=da9d5b925318
OK: §108 synced llama-helper  5079280 bytes  sha=36336210532c
OK: §108 synced ffmpeg  51551944 bytes  sha=87955227bb80
# 两个 bundle 都补齐
```

## 修复 (1 文件)

### `scripts/sync_app_bundle.sh` 加 sidecar sync 函数

```bash
# §108 (2026-08-12): 同步 sidecar binary (llama-helper + ffmpeg) 到 .app bundle
sync_sidecar() {
    local app_dir="$1"
    local dst_dir="$app_dir/Contents/MacOS"
    [[ ! -d "$dst_dir" ]] && return 0
    for sidecar in llama-helper ffmpeg; do
        local src_bin="$TARGET_DIR/$sidecar"
        local dst_bin="$dst_dir/$sidecar"
        if [[ ! -f "$src_bin" ]]; then
            echo -e "${YELLOW}WARN${NC}: §108 $sidecar not in $TARGET_DIR (skip)"
            continue
        fi
        if [[ -f "$dst_bin" ]]; then
            local src_sha=$(shasum "$src_bin" 2>/dev/null | awk '{print $1}')
            local dst_sha=$(shasum "$dst_bin" 2>/dev/null | awk '{print $1}')
            if [[ "$src_sha" == "$dst_sha" ]]; then
                continue  # in sync, skip
            fi
        fi
        cp -f "$src_bin" "$dst_bin"
        chmod +x "$dst_bin"
        local new_sha=$(shasum "$dst_bin" 2>/dev/null | awk '{print $1}')
        local size=$(stat -f "%z" "$dst_bin")
        echo -e "${GREEN}OK${NC}: §108 synced $sidecar  $size bytes  sha=${new_sha:0:12}"
    done
}
sync_sidecar "$APP_DIR"
TAURI_BUNDLE="$TARGET_DIR/bundle/macos/言镜 AI.app"
[[ -d "$TAURI_BUNDLE" ]] && sync_sidecar "$TAURI_BUNDLE"
```

**关键点**:
- sha 对比增量 sync (相同跳过, 节省 ~50MB ffmpeg 复制时间)
- 同时 sync 到用户 bundle 和 tauri 官方 bundle (两边一致)
- 加 `chmod +x` (cp 默认可能丢可执行位)

## §37 硬闸门

- ✅ bash -n 语法检查: OK
- ✅ 实跑 sync_app_bundle.sh: 3 binary 全部 sync 成功
- ✅ 两个 bundle 都验证完整: 用户 bundle + tauri 官方 bundle 各 3 个 binary
- ✅ check_historical_fixes.py: **134/134 PASS** (+2 §108 锚点)

## §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null` (kill 旧的, 防止还跑着旧 binary)
2. `bash scripts/sync_app_bundle.sh` (再跑一次, 确认所有 sidecar OK)
3. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'` (或 `open ~/Applications/言镜 AI.app`)
4. 打开任一会话 (会议脉络 / Untitled) → 点 "生成摘要"
5. 期望: 不再报 "llama-helper binary not found", 摘要正常生成

## 教训 (§93 + §56 强化)

1. §93 (8/7 立): "改 frontend/src/**.tsx 后必须 sync_app_bundle.sh" 铁律, 当时只覆盖 main binary.
2. §108 立: bundle 内 binary **不只一个**, Tauri `externalBin` 声明的 sidecar 必须随 main 同步 sync.
3. §56 强化: cargo build --release pass 不代表 .app bundle 完整, 必须 `sync_app_bundle.sh` 后 `ls .app/Contents/MacOS/` 验证 3 个 binary 都在.
4. **新铁律 (§108)**: 任何 §X 改动影响 .app bundle 完整性, 必须 `sync_app_bundle.sh` + 验证 bundle 内 binary 数量.

## 关联

- §90 (commit fda59cd, 手造 bundle 没带 sidecar)
- §93.1 (8/7 macOS .app bundle sync 规则, 只覆盖 main binary)
- §99.6 (8/10 sync tauri bundle binary, 同样只覆盖 main)
- §37 (硬闸门)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit)
- §92 (决策迁移铁律, outputs + Obsidian + AGENTS.md 三处同日落)
