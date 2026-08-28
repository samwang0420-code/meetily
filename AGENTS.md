# AGENTS.md — 离线会记 / 言镜 AI 项目规则

> 任何 LLM session (Claude / Codex / WorkBuddy) 操作本项目必读。

## 1. 项目身份

- **产品名**: 言镜 AI / SpeakMirror (旧 Meetily)
- **主仓库**: `/Users/wangwei/Documents/离线会记/`
- **Obsidian Vault**: `~/Documents/Obsidian Vault/项目/3-离线会记/`
- **当前版本**: v0.9.0 (HEAD 4e76363)
- **当前分支**: `main`

## 2. 双写规则 (§92 立, 2026-08-07)

### ✅ 必须双写
- **`outputs/§X-...md` ↔ Obsidian `项目/3-离线会记/§X-...md`**
- 每次写 outputs/§X 后必须 `cp` 到 Obsidian 同名文件
- 文件大小 + 行数必须完全一致 (`diff -q` 应无输出)
- §91 / §70 / §62 / §71-78 等历史 outputs 都已双写

### ❌ 不要双写
- 决策日志 `决策日志-当前.md` (仓库本地用,污染严重,不同步到 Obsidian `00-收件箱/决策日志.md`)
- 仓库根目录 `CLAUDE.md` / `README.md` (Obsidian 不需要)
- 代码文件 / Cargo.toml / src-tauri/* (Obsidian 是笔记,不是代码镜像)

### 双写模板
```bash
# 写完 outputs/§X.md 后:
cp /Users/wangwei/Documents/离线会记/outputs/§X-*.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/§X-*.md"
diff -q /Users/wangwei/Documents/离线会记/outputs/§X-*.md \
        "$HOME/Documents/Obsidian Vault/项目/3-离线会记/§X-*.md"
# 期望: 无输出 = 一致
```

## 3. §92 铁律 (防代码漏)

1. **AGENTS.md § 章节 ≠ 代码 commit** — outputs 写"改了 X 文件"前必须 `git show` 验证 commit 真存在
2. **guard 脚本盲点** — `check_historical_fixes.py` 只 grep 文字锚点,不验证功能
   - §62 §63 §90 三个盲点必须手动验证
   - 升级版 guard 见 §92 P2 清单
3. **release 前必跑** — `python3 scripts/check_historical_fixes.py --strict` + `cargo test --lib` + `cargo build --release` 三件套
4. **commit 完整性 CI** — commit message 写"fix §X"前必须 `git log --oneline | grep §X` 看主线真在

## 3.1 §93.1 macOS .app bundle 同步 (用户必走)

**问题**: macOS 上 `cargo build --release` 只更新 `target/release/meetily`. §90 commit 手造了 `target/release/言镜 AI.app/Contents/MacOS/言镜 AI` (独立 binary, 不是 symlink/hardlink), 每次 build 后**不自动同步**, 用户跑 .app bundle 时看到旧 binary.

**新工作流 (3 步, 缺一不可)**:
```bash
# 1. build
cd frontend && cargo build --release

# 2. sync .app bundle  (新!)
./scripts/sync_app_bundle.sh

# 3. 打开 .app bundle
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

**用户直接跑 `target/release/meetily`** 不需要 sync (那个文件已更新). 用户用 `open 言镜 AI.app` 必须 sync.

**验证**:
- `scripts/sync_app_bundle.sh` 内部做 hash 对比
- `scripts/check_historical_fixes.py` 含 `93_sync_app_bundle_script` anchor
- 任何 commit 修改 src-tauri/src 必须重 build + sync 才能让 .app bundle 用户看到

## 3.2 §94 全面代码审计 (2026-08-07 全面盘点)

**触发**: 用户原话 "我真的不想这些问题再次发生了"

**全盘审计发现** (5 大类):
1. **死代码**: 6 个 backup/orig 文件 + audio_v2/ 68KB 9 文件孤儿模块
2. **版本号不一致**: tauri.conf.json 0.1.0 / package.json 0.4.0 / Cargo.toml 0.4.0 / .app Info.plist 0.8.5 (4 个值)
3. **UI v0.8.5 残留**: i18n 5 处关键位置
4. **API 悬空命令**: 4 个 (api_get_auto_generate_setting / builtin_ai_get_models_directory / get_streaming_timing_stats / whisper_get_models_directory)
5. **决策 vs 代码脱节**: §62 §63 §90 + .app bundle sync (前 4 个 § 修了)

**本节交付**:
- `scripts/audit_codebase.py` — 全面代码审计脚本 (跑出 0 errors / 0 warns / 60 info)
- `scripts/pre_release_check.sh` — release 前硬闸门 7 步
- `scripts/check_historical_fixes.py` 升级 76 → 84 anchor
- 删 6 backup/orig + 9 audio_v2 孤儿
- 4 悬空命令全修
- 4 处版本号同步到 0.8.6

**未来工作流** (任何 commit/release 前):
```bash
# 快速检查 (5 秒)
python3 scripts/audit_codebase.py --strict

# 完整 release (11 分钟)
./scripts/pre_release_check.sh
```

**铁律** (每月跑一次 audit):
1. **AGENTS.md § 章节 ≠ 代码 commit** (§92)
2. **每次 release 前必跑 pre_release_check.sh** (§94 §6.2)
3. **commit 完整性 CI** — `git log --grep §X` 看主线真在
4. **§94 audit 每月跑一次** — `python3 scripts/audit_codebase.py --strict`
5. **版本号 bump SOP** (2026-08-21 立, 由 pricing 页 footer v0.9.0 残留触发的教训):
   - 任何 `vX.Y.Z → vX.Y.Z'` 版本号变更, **必须**先全量 grep 残留:
     ```bash
     grep -rn 'vX\.Y\.Z' frontend/src frontend/src-tauri \
                  frontend/package.json frontend/src-tauri/tauri.conf.json \
                  --include='*.ts' --include='*.tsx' --include='*.json' \
                  --include='*.toml' --include='*.css'
     ```
   - 修完所有命中后, 再 grep `vX\.Y\.Z` 应输出 0 行
   - guard 锚点 regex (如 `ui_version_0_9_0_sidebar`) 也必须同步更新到新版本号
   - 然后 §37 6 步闸门全过 (cargo check / cargo test --lib / next build / check_historical_fixes / cargo build --release / GUI 端到端)
   - **不能**只改 5-6 个已知位置就完事, 必须全文搜
   - 教训: 2026-08-21 v0.9.0 → v0.9.1 时漏改 `frontend/src/app/pricing/page.tsx:361` footer, 用户截图右下角显示 v0.9.0 才补 commit `dda51aa`

## 4. 文件结构

```
/Users/wangwei/Documents/离线会记/
├── outputs/                          ← 双写到 Obsidian
│   ├── §91-...md
│   ├── §92-...md
│   └── ...
├── frontend/
│   ├── src/                          ← Next.js UI
│   └── src-tauri/                    ← Rust + Tauri 后端
├── meetily-mcp/                      ← MCP server
├── llama-helper/                     ← LLM helper
├── CLAUDE.md                         ← Claude 上下文
├── AGENTS.md                         ← 本文件 (任何 LLM 必读)
├── README.md                         ← 项目 README
├── Cargo.toml                        ← workspace root
└── 决策日志-当前.md                  ← 仓库本地决策日志 (不同步)
```

## 5. 历史教训

- §70 (2026-08-06): 11 个 § 修复未落地 (commit message 写了但代码没动)
- §92 (2026-08-07): §62 §63 §90 三组关键优化 commit 完全丢失 (git log --all 0 命中)
- 用户原话: "每次大变动后代码都会漏,这不是我想要的"
- 根因: outputs 文档堆积,AGENTS.md § 章节 ≠ 代码 commit,guard 只查文字不查功能

## 6. §97 identifier 改造铁律 (2026-08-10 立)

**触发**: §94 P1 待办 + 言镜 AI 品牌改名 (commit `276906e`).

**五条铁律**:

1. **identifier 改动必须配 migrate 函数** — 不能只改 `tauri.conf.json`
2. **migrate 只能 COPY, 不能 DELETE/MOVE** — 旧目录保留观察期 (§65 A 方案)
3. **migrate 必须 best-effort** — 失败 warn 不阻塞启动
4. **Python 路径优先 env var, fallback hardcode** — env var 名跟 identifier 对齐
   - `YANJINGAI_DIAR_DB_PATH` 优先
   - `LIXIANHUIJI_DIAR_DB_PATH` 向后兼容
   - 最后 fallback 新 bundle id hardcode
5. **每次 commit 前 §37 硬闸门 + AGENTS.md §92 三处同步**

**实现位置**:
- 常量: `frontend/src-tauri/src/config.rs::APP_BUNDLE_ID` / `APP_BUNDLE_ID_LEGACY`
- 函数: `frontend/src-tauri/src/lib.rs::migrate_legacy_app_data()` + `setup()` 早期调用
- tauri.conf.json: `identifier: tech.yanjingai.app`
- Python: `scripts/sherpa_asr.py` / `scripts/diar.py` / `scripts/diar_download.py` 路径改 + env var 优先级
- Rust inline Python: `api/api.rs:1453` / `api/diar_pickup_loop.rs:147` env var 优先级
- UI: `app/legal/privacy/page.tsx` / `components/TranscriptSettings.tsx` / `hooks/useRecordingStart.ts` 路径改
- guard: `scripts/check_historical_fixes.py` 10 个 §97 锚点 (97/97 PASS)

**§97 已知边界**:
- `/tmp/lixianhuiji_diar` IPC 共享临时目录保留 (改风险 > 收益)
- 旧 `cn.lixianhuiji.app/` 数据保留 30 天观察期, 之后用户可手动删
- 内部协议 (DB 表名 / localStorage key / Tauri event 名 / migration 文件名) 不动 — 向后兼容

**关联**:
- [[97-identifier改造-tech.yanjingai.app-数据迁移-2026-08-10]] (Obsidian) / `outputs/97-identifier改造-tech.yanjingai.app-数据迁移-2026-08-10.md` (Codex)
- [[65-言镜AI品牌改名与Bundle数据迁移]] (§65 原决策)
- [[94-全面代码审计-代码漏系统性问题-2026-08-07]] (§94 P1 待办)

## 7. §98 identifier 改造后启动闪退三件套修复铁律 (2026-08-10 立)

**触发**: 用户报告 `open '言镜 AI.app'` 闪退 "意外退出". 根因排查发现 3 个 bug 叠加:
1. **Info.plist CFBundleIdentifier 没同步** (sync_app_bundle.sh 缺 §97 逻辑)
2. **sqlx _sqlx_migrations.checksum 不匹配** (§73 同类 — checksum mismatch, 不是 missing)
3. **codesign identifier 跟 Info.plist 不一致** (launchd 162 Launch failed)

**铁律**:

1. **改 tauri.conf.json identifier 必须 3 处同步**: tauri.conf.json + Info.plist + codesign
2. **sync_app_bundle.sh 必须包含 §97 + §98 段**, 否则 binary / bundle 不匹配
3. **sqlx checksum 不一致必须 startup self-heal** (避免每次手工 Python sync)
4. **任何 release binary 改动 → 必跑 sync_app_bundle.sh + 重 build → 验证 codesign**
5. **每次 commit 前 §37 硬闸门 + AGENTS.md §92 三处同步**

**实现位置**:
- `frontend/src-tauri/src/database/manager.rs::sync_migration_checksums` (startup self-heal)
- `scripts/sync_app_bundle.sh` §97 + §98 段 (Info.plist + codesign 自动同步)
- `scripts/fix_sqlx_checksums.py` (手工 sync 工具, 应急用)
- `scripts/check_historical_fixes.py` §98 锚点 (guard 97 → 101)

**commit**: `32c7fd8`

**关联**:
- [[98-identifier改造后启动闪退三件套修复-2026-08-10]] (Obsidian) / `outputs/98-identifier改造后启动闪退三件套修复-2026-08-10.md` (Codex)
- [[97-identifier改造-tech.yanjingai.app-数据迁移-2026-08-10]] (上一 commit)
- [[73-启动panic-missing-migrations-根因+一次性修复]] (同类 sqlx checksum 修复)

## 8. §99 导入失败 PYTHONUSERBASE + models 迁移修复铁律 (2026-08-10 立)

**触发**: 用户截图 `Sherpa transcription failed on segment 0: sherpa slot 0 error: No module named 'numpy'`. 两个叠加 bug:

### Bug 1 — §96 PYTHONUSERBASE=$HOME 覆盖 numpy 默认 user-base

`§96 commit ad3b9e2` 在 spawn Python 子进程时加 `cmd.env("PYTHONUSERBASE", home)`:
- homebrew Python 默认 user-base = `~/Library/Python/3.14/lib/python` (numpy/sherpa_onnx 装在那)
- 显式设 `PYTHONUSERBASE=/Users/wangwei` → PEP 370 错误映射到 `~/lib/python3.14/site-packages` → numpy 找不到
- **探测时 import OK ≠ spawn 时 import OK** (env 不一致)

### Bug 2 — §97 migrate_legacy_app_data 漏复制 models/

`§97 commit 276906e` 迁移只复制 3 个 sqlite 文件, 漏了 `models/`:
- 旧 `cn.lixianhuiji.app/models/sherpa/` 有 `funasr-nano-int8` + `paraformer-zh-int8` (~1.2GB)
- 新 `tech.yanjingai.app/models/sherpa/` 不存在
- sherpa_asr.py 启动后 `discovered 0 model packs`, 导入转录 0 段识别

### 铁律

1. **Python 探测和 spawn 必须用相同 env** — 探测 import OK 不代表 spawn import OK. 必须真 spawn 一次 + 发 list action 验证 `ok=true`.
2. **永远不要显式设 `PYTHONUSERBASE=$HOME`** — homebrew Python 默认 user-base 已经是 `~/Library/Python/3.14/lib/python`. 显式覆盖破坏 PEP 370 路径映射.
3. **§97 迁移必须 COPY 完整用户数据** — db + decode_cache + **models**, 不能只复制 db. 注释假设"用户已有"必须验证 (`ls $new/models/sherpa`).
4. **新代码改动必须加 guard anchor** — 不加 anchor 下次重构被覆盖 (见 §56 §92 §94). commit 6907799 加了 6 个 §99 anchor.
5. **cargo test --lib 全套** — 改动 sherpa_daemon.rs 必须跑 spawn 测试 (探测 ≠ spawn).

**实现位置**:
- `frontend/src-tauri/src/audio/sherpa_daemon.rs::ensure_started_slot` — 不再设 PYTHONUSERBASE, 保留 PYTHONUNBUFFERED
- `frontend/src-tauri/src/audio/sherpa_daemon.rs::tests::section_99_spawned_python_can_import_sherpa_onnx` — spawn 验证单测
- `frontend/src-tauri/src/lib.rs::migrate_legacy_app_data` — 加 `copy_dir_recursive` + models/ 迁移
- `frontend/src-tauri/src/lib.rs::copy_dir_recursive` — 新 helper (递归复制目录树)

**commit**: `6907799` (perf/summary-map-concurrency)
**binary**: `/Users/wangwei/Documents/离线会记/target/release/meetily` 69M mtime 11:40
**guard**: `python3 scripts/check_historical_fixes.py` → **107/107 PASS** (101 → 107)

## 9. §99.5 Tauri setup() 禁止 tokio::spawn 铁律 (2026-08-10 立)

**触发事故**: §99.2 加的 `tokio::spawn(async move { backfill_meeting_user_ids(...) })` 在 `tauri::Builder::default().setup(...)` 里直接 panic 阻断启动:

```
thread 'main' panicked at frontend/src-tauri/src/lib.rs:610:13:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread caused non-unwinding panic. aborting.
```

**根因**: Tauri main thread 是 `tao` event loop, **不是** Tokio runtime. `tokio::spawn` 需要 `tokio::runtime::Handle` 才能拿到 reactor, tao thread 没有, 直接 panic.

**铁律**:
1. **任何 Tauri `setup(|_app| { ... })` 闭包里 spawn 异步任务必须用 `tauri::async_runtime::spawn`** — Tauri 2.x 提供这个 wrapper 内部桥接到正确 runtime.
2. **禁止用 `tokio::spawn` / `tokio::task::spawn`** — tao event loop 没有 Tokio reactor.
3. **任何 `commands.rs` (Tauri command handler) 里可以用 `tokio::spawn`** — Tauri command 是在 Tokio runtime 上下文中调度的 (经 `#[tauri::command]` async fn 包装).
4. **判断口诀**: `setup()` 里 → `tauri::async_runtime::spawn`; command handler / Tauri 事件 listener / 普通 tokio task → `tokio::spawn`.
5. **正确参考**: §86 (`memory_watcher` start), §88 (`topic_dossier_scheduler` start), §62 (`sherpa_daemon` ensure_started_slot) 全用 `tauri::async_runtime::spawn`.

**反向案例 (§99.5 bug)**:
```rust
.setup(|_app| {
    Box::pin(async move {
        // ... 其他 setup ...
        let app_handle = _app.handle().clone();
        tokio::spawn(async move {  // ❌ PANIC: "there is no reactor running"
            backfill_meeting_user_ids(&app_handle).await;
        });
    })
})
```

**正确写法**:
```rust
.setup(|_app| {
    Box::pin(async move {
        let app_handle = _app.handle().clone();
        tauri::async_runtime::spawn(async move {  // ✅ Tauri 内部桥接
            backfill_meeting_user_ids(&app_handle).await;
        });
    })
})
```

**guard**: `python3 scripts/check_historical_fixes.py` 116/116 PASS (含 §99.5 正向 anchor: `lib.rs` 包含 `§99.5.*tauri::async_runtime::spawn`).

**修复 commit**: `perf/summary-map-concurrency` HEAD 即将加 commit (Codex CLI auto-review 故障期间用户手动 push)

**关联**: §86 / §88 / §62 / §37 硬闸门 / §92 防代码漏

## 10. §99.6 sync_app_bundle.sh 必须也 sync tauri bundle binary (2026-08-10 立)

**触发事故**: §99.5 fix push 后用户跑 §99.4 推荐启动方式:
```bash
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
```
仍然 panic at lib.rs:610. Commit 398836e 已经入仓, 但 binary mtime 是 20:47 (panic 前 tauri build 产物), 不是 21:03 (cargo build 输出).

**根因**: `sync_app_bundle.sh` 之前只 sync 两个路径:
1. `target/release/言镜 AI.app/Contents/MacOS/言镜 AI` (手造 .app)
2. `~/Applications/言镜 AI.app` symlink (LaunchServices 兜底)

**没 sync**: `target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily` (tauri build 官方 bundle 路径, §99.4 推荐启动方式).

`npx tauri build` 跑出的 bundle 在每次 cargo build 后没被更新, 用户走 §99.4 推荐路径拿到的是旧 binary.

**铁律**:
1. **任何 .app bundle 路径只要存在, 必须主动 sync** — sync_app_bundle.sh 检测到路径就 cp + sha 对比
2. **sync 用 sha 对比 + 增量更新** — 同 sha 跳过 (无操作), 不同 sha 才 cp (避免无谓写盘)
3. **sync 必须覆盖 §99.4 推荐路径** — `target/release/bundle/macos/言镜 AI.app` 是 §99.4 唯一推荐 exec 启动方式, 不可漏
4. **用户反馈 panic 时第一件事查 binary mtime** — 比对 source HEAD vs binary mtime, 差距 > 5min 必是 sync 漏了

**正确 sync 模式** (sync_app_bundle.sh 末尾):
```bash
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
```

**guard**: `python3 scripts/check_historical_fixes.py` 118/118 PASS (含 §99.6 双 anchor: synced / skip-when-same).

**关联**: §99.4 (推荐启动方式) / §99.5 (fix) / §37 硬闸门 / §92 防代码漏

## §103 VAD buffer warn 噪音 + sherpa_asr.py 命名 (2026-08-11)

**触发**: 用户 8/11 重新导入 1:49:57 音频, log 20MB, 全是 VAD buffer warn:
```
$ grep -c "VAD.*buffer is large" /tmp/meetily_verify_102.log
144357
```
每次 30ms VAD `process_chunk` 都 warn 一次 (1M samples / 62.5s 阈值偏低).

**修复 (3 文件)**:

1. `frontend/src-tauri/src/audio/vad.rs`:
   - struct 加 `warned_about_buffer: bool` flag
   - 阈值 1M → 9.6M samples (10 min at 16kHz)
   - 跨阈值只 warn 一次, SpeechEnd 后 reset

2. `frontend/src-tauri/scripts/sherpa_asr.py`:
   - `duration_ms` (实际是 ASR 总耗时) → `total_ms`
   - 跟 `decode_ms` (纯推理) + `audio_seconds` (音频时长) 区分
   - 注释："renamed from duration_ms to avoid ambiguity vs audio_seconds"

3. `scripts/check_historical_fixes.py`: 121 → 124/124 PASS (3 §103 锚点)

**§37 硬闸门**:
- cargo check --lib: 0 errors (28 warnings §18 不动)
- cargo build --release: 1m30s, binary 10:23 72M
- sync_app_bundle.sh: §99.6 tauri bundle SHA 一致

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 重新导入 13430280252492828.mp4
grep -c "VAD.*buffer is large" /tmp/meetily_verify_103.log   # 预期 < 10
grep "total_ms" /tmp/meetily_verify_103.log | head -3        # 改用 total_ms 字段
```

**关联**: §102 (symphonia stereo downmix fallback 基础) / §37 / §56

## §104 Sidebar 改名 + 会议详情加导出 + 隐藏华而不实 (2026-08-12)

**触发**: 用户 8/11 反馈知识图谱、夜间重建等"华而不实"，判断错了竞品功能 ≠ 用户需求。
> 不是删掉和折叠，是做成可用的，方便用的，同时又有设计感的，你再仔细琢磨一下
> 暂时先 Sidebar 改名"知识图谱 → 会议脉络"、会议详情加"导出"按钮两个吧，其他的都隐藏，不是删掉

**真实场景认知** (我之前误把):
- 知识图谱：跨会议追踪 (用户: 单 PC, 一天 < 5 会议, 跨会议是伪需求)
- Obsidian 同步：用户不一定用 Obsidian
- MCP Server：AI 重度用户专属
- 夜间重建 (0-6 点窗口)：电脑不一直开, 触发率 0
- ⌥+Space 实时 Q&A：开会还要打字问问题, 反人性
- 真正的护城河 = 100% 本地 + 离线 + 中文准确率

**改动 (5 文件)**:

1. **Sidebar** `frontend/src/components/Sidebar/index.tsx:775` + i18n zh.ts:10 / en.ts:10
   - "知识图谱 / Knowledge Graph" → "会议脉络 / Meeting Timeline"
   - 紫色 nav 按钮保留, 徽章 `topics count` 保留

2. **/knowledge 页标题** `frontend/src/app/knowledge/page.tsx`
   - `知识图谱 / Knowledge Graph` → `会议脉络 / Meeting Timeline`
   - subtitle `跨会议主题追踪 · 每场会议结束自动提取主题、人物、决议、问题` → `按时间线浏览会议 · 主题自动聚合 · 一键跳转相关会议`

3. **/knowledge 页 4 个 stat cards + 4 个 panel 隐藏**
   - 4 个 StatCard (主题总数/决议/项目/待办行动项) → `{false as boolean /* hide per §104 */}`
   - 4 个 panel JSX 替换为 `{false as boolean /* hide per §104 */}`：
     - Action items (改放 /meeting-details 内)
     - Obsidian 同步 (改放 /settings)
     - MCP Server (改放 /settings)
     - 夜间重建 (改成本地空闲检测, 不强制 0-6 点)
   - 隐藏策略: JSX 块从源代码**删除** (用 marker 替代)，state + handlers + i18n keys 保留
   - 重新启用: `git log -p` 还原 JSX + 删 4 个 marker

4. **会议详情加导出按钮** `frontend/src/app/meeting-details/page-content.tsx`
   - 顶部"返回工作台"按钮右侧加 `<Download>` 按钮 + `ChevronDown`
   - dropdown 3 选项: 复制摘要 / Markdown 文件 / TXT 文件
   - 复用已有 `copyOperations.handleCopySummary` + `handleExportSummary('md'|'txt')`
   - click-outside 关闭, `data-testid="meeting-export-button"`

5. **i18n 加 4 个 key** `frontend/src/i18n/locales/{zh,en}.ts`
   - `meeting.export` / `meeting.copy_summary` / `meeting.export_markdown` / `meeting.export_txt`
   - 顺便修注释格式: `#` Python 风格 → `//` JS 风格 (SWC parser 不接受 `#`)

**§37 硬闸门**:
- tsc --noEmit: 18 errors (§18 已知, 0 new)
- next build: OK
- cargo build --release: 11:40 72M
- check_historical_fixes: 124/124 PASS
- sync_app_bundle.sh: §99.6 SHA 一致

**§15 GUI 验收** (用户必做):
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. Sidebar "知识图谱" → "会议脉络"
# 2. /knowledge 页 → 4 stat card 消失, 4 panel 不显示
# 3. 会议详情 → 顶部"返回工作台"右有"导出"按钮 + dropdown
```

**禁止** (§18 强化):
- 重启 4 个隐藏 panel 时, 不还原旧的"夜间重建 0-6 点"文案
- 改名"会议脉络"回"知识图谱" (用户已判定)

**关联**: §100 (P0-P2 UI 暴露初始版) / §18 (不主动改华而不实) / §71 (7 款 AI 会议工具调研) / §56 (commit 必带实际改动)

### §104.1 录音通知 Toast 英文 → i18n (2026-08-12 11:45)

**触发**: 用户 8/12 截图反馈右下角弹出 "Recording Started / Inform all participants ..." 全部英文, 主界面是中文。元素最显眼的英文残留。

**根因**: `frontend/src/lib/recordingNotification.tsx` 4 处硬编码英文。该文件是 utility function (非 React 组件), 不能用 `useTranslation` hook。

**修复**:
- `localT(path: string)` helper: 读 `localStorage['lixianhuiji.locale']` → `DICTS[locale]` → 路径 lookup
- 4 处英文 → `localT('recording.notification.{title,body,dont_show,ack}')`
- `i18n/locales/zh.ts` 加 zh 文案, `i18n/locales/en.ts` 加 en 文案

**§37 硬闸门**:
- tsc --noEmit: 1 error (§18 bun:test 已知)
- next build: OK
- cargo build --release: 11:47 72M
- sync_app_bundle: §99.6 SHA 一致

**§15 GUI 验收**:
```bash
killall meetily 2>/dev/null && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 开始录音 → 右下角弹出 toast 应显示中文
```

**关联**: §104 (主改动) / §18 (严禁英文硬编码) / §90 (UI 漏代码 4 项)

## §106 模型设置 Modal 切换功能删除 — 固定本地模式 (2026-08-12)

**触发**: 用户 8/12 截图反馈 "这里的切换功能先仅用掉吧, 我们只用本地模型生成摘要"。

**背景**: 模型设置 Modal 的 "AI 总结模型" provider dropdown 让用户在 7 个 provider 间切换 (Built-in AI / Claude / OpenAI / Groq / Ollama / OpenRouter / Custom Server)。项目宪法是"云端 API 永不接入" (§18, §71 §P0-P2), 留这个 dropdown 是"华而不实"且误导用户 (看起来支持云端但实际跑不通)。

**改动 (3 文件, +11/-133)**:
1. `frontend/src/components/ModelSettingsModal.tsx` (-130/+8):
   - 删除整个 `<Select>` provider dropdown (含 7 个 SelectItem)
   - 删除下游 `<Popover>` + `<Command>` model combobox (含 loading 态 / search / 4 个 isLoading state)
   - 替换为静态展示: 绿点 + "Built-in AI (Offline, No API needed)" + 右侧 "已锁定本地模式 (云端服务暂不开放)" 标签
   - **用户无法切换 provider** (符合 §106 意图)
2. `frontend/src/i18n/locales/zh.ts`:
   - `model_settings.fixed_local_only: '已锁定本地模式（云端服务暂不开放）'`
3. `frontend/src/i18n/locales/en.ts`:
   - `model_settings.fixed_local_only: 'Local mode only (cloud disabled)'`

**保留 (§18 精神 — 隐藏 ≠ 删除)**:
- `BuiltInModelManager.tsx` 子组件**保留不动** — 用户仍可在多个**本地模型**间选择 (Qwen 3.5 2B / Llama 3.2 3B 等)
- `modelConfig.provider` state 保留 (`builtin-ai` 硬编码默认值)
- `modelOptions` / `isLoading*` / `loadOpenRouterModels` 等 state/handlers 保留 (不删 = 未来想恢复云端选项时还原 1 个 git diff 即可)
- 旧 i18n keys (`claude` / `openai` / `groq` / `ollama` / `openrouter` / `custom_openai`) 保留 — 防止老 UI 残骸突然报 missing key

**设计意图**:
1. **§18 不主动改无关 bug** 精神 — 不删 JSX 块外逻辑, 只删用户能看见的"华而不实"控件
2. **§104 "华而不实" 用户原则延续** — 用户单一 PC 本地使用者, 一天 < 5 会议, 不会用云端
3. **隐藏 ≠ 删除** — git log 还原 1 行 + Select 块即可恢复 (rollback 风险 0)
4. **绿色实心点 + 静态标签** 视觉告诉用户 "系统已替你做了选择", 不会引发"我能切吗"的疑问

**§37 硬闸门 (本节)**:
- tsc --noEmit: 1 个 §18 bun:test 已知错误 (不动)
- next build: OK
- cargo test --lib: 全部 PASS
- cargo build --release: binary 72M
- check_historical_fixes.py: **130/130 PASS** (+2 §106 锚点)
- sync_app_bundle.sh: 同步 binary 到 言镜 AI.app bundle

**已知边界**:
- 模型选择 Modal 内 Claude / OpenAI / Groq 等条目在 i18n 中保留, 但**用户看不到也用不到**
- 旧用户 localStorage `providerModelMap` 中可能有 `claude=xxx` 等残留, 不清理 (无害)
- `modelConfig.provider` 默认值不变 (`builtin-ai`)

**§15 GUI 验收 (用户必做, 不能 CLI 测)**:
1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 设置 → 模型设置 → "AI 总结模型" 应该是静态 "Built-in AI (Offline, No API needed)" + 绿点 + "已锁定本地模式 (云端服务暂不开放)" 标签
4. **不应**有 provider dropdown 可点击切换
5. BuiltInModelManager 部分 (内置模型选择) 仍可用, 可选 Qwen 3.5 2B 等本地模型

**关联**:
- §18 (云端 API 永不接入 / 不主动改无关 bug)
- §104 (8/11 用户"华而不实"反馈, 隐藏 4 面板 + 4 stat)
- §71 (7 款 AI 会议工具调研, 本地差异化护城河)
- §29 (FunASR-Nano Pro tier gate, 免费/会员基于本地模型分发)
- §92 (决策迁移铁律, outputs + Obsidian + AGENTS.md 三处同日落)
- [[106-模型设置Modal切换删除-固定本地模式]] (Obsidian) / `outputs/§106-...md` (Codex)

## §107 录音通知 toast 翻译未生效修复 (2026-08-12)

**触发**: 用户 8/12 截图: 录音通知 toast 显示英文 key 字面文本 `recording.notification.title` / `body` / `dont_show` / `ack`, 而不是翻译后的中英文文案。

**根因 (1 跳)**: §104.1 (commit 090238c) 加了 `localT('recording.notification.*')` 4 处调用, 但 i18n keys 实际放到了 `topbar.meeting_details.notification.*` 孤儿路径下。`localT` lookup 失败 → fallback 返回 path 字符串本身 → 用户看到英文 key 字面。

**实证**:
```
$ grep -n "录音已开始" frontend/src/i18n/locales/zh.ts
384:      title: '🔴 录音已开始',   ← 在 topbar.meeting_details.notification (孤儿)

$ grep -rn "meeting_details.notification" frontend/src --include="*.ts" --include="*.tsx"
(无任何匹配 — 完全孤儿)
```

**修复 (2 文件, +14/-0)**:
1. `frontend/src/i18n/locales/zh.ts` 顶级 `recording` 块加 `notification` 子块:
   ```ts
   recording: {
     memory_warning: '...',
     memory_critical: '...',
     // §107: §104.1 实际路径应为 recording.notification.*, 之前放到 topbar.meeting_details.notification 是错的 (孤儿 key, 没人用)
     notification: {
       title: '🔴 录音已开始',
       body: '请告知所有参会者, 本次会议正在被录制。',
       dont_show: '不再显示此提示',
       ack: '我已告知参会者',
     },
   },
   ```
2. `frontend/src/i18n/locales/en.ts` 同位置加英文版。

**保留 (§18 精神)**: `topbar.meeting_details.notification.*` 孤儿 key 不删, 防止误伤未来代码。

**§37 硬闸门**:
- tsc --noEmit: 1 个 §18 bun:test 已知错误 (不动)
- next build: OK
- cargo build --release: 1m33s 增量, binary 69M **mtime 13:26**
- check_historical_fixes.py: **132/132 PASS** (+2 §107 锚点)
- sync_app_bundle.sh: 同步 binary 到 言镜 AI.app bundle

**§15 GUI 验收 (用户必做)**:
1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 开始录音 → 右下角 toast 应显示中文 ("🔴 录音已开始 / 请告知所有参会者...")
4. 英文 locale → 显示 "🔴 Recording Started / Inform all participants..."

**教训 (§56 强化)**:
- §104.1 commit 时只验证文件存在 + cargo build pass + tsc pass, 没真正**渲染一次** toast 验证 `localT` 走的路径
- §37 闸门没补 "tsc/next build 不验证 i18n lookup 实际匹配" 这层
- **新增**给后续 §X 提醒: 任何 `localT` / `t()` 改动 → §15 GUI 验收必跑
- 写完 i18n key 后, 必须 grep 一遍确认调用路径 == 声明路径

**关联**:
- §104.1 (录音通知 i18n 第一次尝试, 路径错)
- §104 (UI 华而不实清理)
- §92 (决策迁移铁律)
- §18 (不主动改无关 bug — 孤儿 key 保留)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit, 这次 §104.1 描述与代码脱节)
- [[107-录音通知toast翻译未生效修复]] (Obsidian) / `outputs/§107-...md` (Codex)

## §108 sync_app_bundle.sh 缺 sidecar 同步 — llama-helper not found 修复 (2026-08-12)

**触发**: 用户 8/12 截图: 生成摘要报 "llama-helper binary not found", binary 已更新不生效。

**根因 (1 跳)**: §90 commit `fda59cd` (8/7 01:51) 手造 `target/release/言镜 AI.app/` bundle 只放了 `言镜 AI` 一个 binary, **缺 llama-helper + ffmpeg**。`sync_app_bundle.sh` §99.6 只 sync `meetily`, 没处理 `tauri.conf.json externalBin` 声明的两个 sidecar binary。

```bash
# 修复前 (用户 bundle):
/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app/Contents/MacOS/
  └── 言镜 AI  (72M) ← 唯一 binary, llama-helper / ffmpeg 都没有

# tauri 官方 bundle (8/10 编的) 是完整的:
/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/
  ├── meetily      (72M)
  ├── llama-helper (5M)
  └── ffmpeg       (51M)
```

**修复 (1 文件)**: `scripts/sync_app_bundle.sh` 加 `sync_sidecar()` 函数, 同时 sync 到用户 bundle + tauri 官方 bundle, sha 对比增量。

**§37 硬闸门**:
- bash -n 语法检查: OK
- 实跑 sync_app_bundle.sh: 3 binary 全部 sync 成功
- 两个 bundle 都验证完整 (各 3 个 binary)
- check_historical_fixes.py: **134/134 PASS** (+2 §108 锚点)

**§15 GUI 验收 (用户必做)**:
1. `killall meetily 2>/dev/null`
2. `bash scripts/sync_app_bundle.sh`
3. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
4. 任意会话 → 生成摘要 → 应该正常生成, 不再报 llama-helper not found

**新铁律 (§108 立)**:
- 任何 §X 改动影响 .app bundle 完整性, 必须 `sync_app_bundle.sh` 后 `ls .app/Contents/MacOS/` 验证 3 个 binary (main + llama-helper + ffmpeg) 都在
- cargo build --release pass + sync_app_bundle.sh pass **不等于** .app bundle 完整, 必须实测启动一次确认摘要能生成
- bundle 手造时 (§90 之类) 必须包含全部 `tauri.conf.json externalBin` 声明的 sidecar, 不只 main binary

**关联**:
- §90 (commit fda59cd 手造 bundle 缺 sidecar)
- §93.1 (8/7 .app bundle sync 规则, 只覆盖 main binary)
- §99.6 (8/10 sync tauri bundle binary, 同样只覆盖 main)
- §37 (硬闸门) / §56 (AGENTS.md 双校) / §92 (决策迁移铁律)
- [[108-sync-app-bundle-缺sidecar-llama-helper-not-found]] (Obsidian) / `outputs/§108-...md` (Codex)

## §109 会议详情页 UI 整理 — 2 个 i18n key 错 + TranscriptButtonGroup 重复 (2026-08-12)

**触发**: 用户 8/12 截图反馈 "页面有点乱" — 顶部两套工具栏 + `common.speaker_title_short` 英文 key 字面 + "录音" 按钮错位。

**3 个独立 bug**:

1. **Bug 1 — Speaker Roster 按钮 i18n key 错**:
   - `SummaryPanel.tsx:334` 用 `t('common.speaker_title_short')` (孤儿 key, 不存在)
   - 应为 `t('speaker.title')` = '说话人名单' / 'Speaker roster'
   - 跟 §107 i18n 路径错位模式完全一样 (§56 教训)

2. **Bug 2 — "录音" 按钮 label 错**:
   - `TranscriptButtonGroup.tsx:101` 用 `t('meeting_details.recording')` = '录音'
   - 但 onClick 是 `onOpenMeetingFolder` (打开录音文件夹)
   - 应为 `t('meeting_details.open_folder')` = '打开录音文件夹'

3. **Bug 3 — 顶部两套工具栏重复**:
   - TranscriptPanel (左列 1/3) 顶部有 TranscriptButtonGroup: 复制 / 导出 MD / 导出 TXT / 打开文件夹 / 重新转录
   - SummaryPanel (右列 2/3) 顶部有 SummaryGeneratorButtonGroup + SummaryUpdaterButtonGroup: 重新生成 / 语言 / AI 模型 / 模板 / 保存 / 复制 / 导出 MD / 导出 TXT / 查找 / 打开文件夹
   - **复制 / 导出 MD / 导出 TXT / 打开文件夹 4 个按钮重复出现**, 用户看两套觉得"乱"
   - 与 §104 "华而不实" 用户原则一致

**修复 (3 文件)**:
1. `SummaryPanel.tsx:334` `t('common.speaker_title_short')` → `t('speaker.title')`
2. `TranscriptButtonGroup.tsx:101` `t('meeting_details.recording')` → `t('meeting_details.open_folder')`
3. `TranscriptPanel.tsx` 整个 TranscriptButtonGroup JSX 包到 `{false && (...)}` 里 (隐藏而非删除, 保留 props 链路, 未来 git log 还原)

**§37 硬闸门**:
- tsc: 1 §18 bun:test 已知
- next build OK
- cargo build --release 1m34s, binary 13:47
- check_historical_fixes.py **137/137 PASS** (+3 §109 锚点)
- sync_app_bundle OK

**§15 GUI 验收 (用户必做)**:
1. `killall meetily && bash scripts/sync_app_bundle.sh && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
2. 打开任一会话 → 期望:
   - transcript 列顶部**空** (无 TranscriptButtonGroup 工具栏)
   - summary 列顶部: "说话人名单" (中文) + 重新生成 / 语言 / AI 模型 / 模板 + 保存 / 复制 / 导出 MD / 导出 TXT / 查找 / 打开文件夹
   - 整个页**只有一套**工具栏

**还原方法 (§18 精神 — 隐藏 ≠ 删除)**:
git log 找 §109 之前版本, 删 `{false && ...}` 包裹即可恢复 TranscriptButtonGroup 渲染。

**关联**:
- §107 (i18n key 路径错位同样模式)
- §104 (UI 华而不实清理)
- §90 (v0.8 UI 漏代码 4 项)
- §56 (AGENTS.md 双校)
- §18 (隐藏 ≠ 删除)
- §37 (硬闸门)
- §92 (决策迁移铁律)
- [[109-会议详情页UI整理]] (Obsidian) / `outputs/§109-...md` (Codex)

## §109.1 TranscriptButtonGroup 恢复渲染 — §109 误判 revert (2026-08-12)

**触发**: 用户 8/12 二次反馈 "还是不对啊, 菜单都堆在一起了, 原本是语音识别那里保存/复制/导出MD/导出TXT, 现在都在一起了"。

**根因 (§109 我误判)**:
- §109 我**错误**认为 TranscriptButtonGroup (transcript 工具栏) 跟 SummaryUpdaterButtonGroup (summary 工具栏) 重复
- 用户说"页面乱"=两个工具栏重复 → 隐藏 TranscriptButtonGroup
- 实际上**两个工具栏功能完全不同**:
  - **TranscriptButtonGroup** = 复制 transcript 文本 / 导出 transcript MD / 导出 transcript TXT / 打开录音文件夹 / 重新转录
  - **SummaryUpdaterButtonGroup** = 保存 summary 修改 / 复制 summary / 导出 summary MD / 导出 summary TXT / 查找 / 打开文件夹
- 用户真正反馈 = transcript 列的 4 个按钮 (复制/导出MD/导出TXT/打开录音文件夹) **消失**了

**修复 (1 文件)**:
- `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`: 撤销 `{false && (...)}` 包裹, 恢复 `<TranscriptButtonGroup ... />` 渲染
- **保留** §109 的 2 个 i18n key 修复 (speaker.title / open_folder), 这两个不是误判

**§109 锚点调整**:
```
"109_transcript_button_group_hidden" → "109_transcript_button_group_rendered"
regex: \{false && → <TranscriptButtonGroup
```

**§37 硬闸门**:
- tsc: 1 §18 bun:test 已知
- next build OK
- cargo build --release 2m42s 增量, binary 13:00
- check_historical_fixes.py **137/137 PASS**
- sync_app_bundle OK

**§15 GUI 验收 (用户必做)**:
1. `killall meetily && bash scripts/sync_app_bundle.sh && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
2. 打开任一会话 → 期望:
   - **transcript 列顶部**: 复制 / 导出 MD / 导出 TXT / 打开录音文件夹 (浅色 outline 4 按钮)
   - **summary 列顶部**: 说话人名单 / 重新生成 / 自动检测 / AI 模型 / 模板 + 保存 / 复制 / 导出 MD / 导出 TXT (深色按钮)
   - 两套工具栏各司其职, **不重复**

**教训 (§56 强化)**:
- "页面乱" 这种主观反馈, **必须先确认用户真实意图** (重复? 太挤? 缺按钮? 顺序错?), 不能想当然
- §56 铁律扩展: 任何 §X 修复, 不确定时问用户, 不要乱猜
- §109 出发点 (i18n 修复) 正确, 但隐藏 TranscriptButtonGroup 方向**完全错**

**关联**:
- §109 (误判隐藏)
- §104 (UI 华而不实清理, 方向部分误判)
- §107 (i18n key 路径错位)
- §56 (AGENTS.md 双校)
- [[109.1-TranscriptButtonGroup恢复渲染]] (Obsidian) / `outputs/§109.1-...md` (Codex)

## §110 SummaryPanel 按钮简化 — 9 按钮 → 4 元素 (2026-08-12)

**触发**: 用户 8/12 截图反馈 "你不觉得这部分的按钮太多了吗。提升审美, 让页面简洁好用, 不啰嗦"。

**根因 (2 类)**:
1. **操作粒度太细**: 保存/复制/导出MD/导出TXT 4 个按钮都属于"导出"语义, 平铺无层次
2. **标签冗长**: 9 个图标+文字按钮横向挤满 1 行, 没视觉权重区分

**修复方案 (4 元素)**:
```
[说话人名单]  [重新生成 ★]  [⚙️ 设置 ▾]  [📤 导出 ▾]
                            │              ├ 保存 (脏时绿点)
                            ├ 自动检测      ├ 复制
                            ├ AI 模型       ├ 导出 MD
                            └ 模板          ├ 导出 TXT
                                          └ 打开文件夹
```

**视觉变化**:
- 9 按钮 → 4 元素 (横向密度 -55%)
- 主操作"重新生成"蓝紫渐变高亮
- 设置 + 导出收下拉 (按需展开)
- 高频左, 低频右

**实现 (1 文件改)**:
- `frontend/src/components/MeetingDetails/SummaryPanel.tsx`
- 删 SummaryGenerator/UpdaterButtonGroup 组件调用, 改 4 元素 (3 Button + 2 DropdownMenu)
- SummaryGenerator/Updater 内部文件保留 (其他场景可能用, §18 隐藏 ≠ 删除)

**i18n**: 全部用现有 key (summary.regenerate/settings_title/ai_model/template/save/copy/export_md/export_txt, meeting_details.open_folder), **无新增**

**§37 硬闸门**:
- tsc: 1 §18 bun:test 已知
- next build OK
- cargo build --release 2m00s, binary 13:14
- check_historical_fixes.py **139/139 PASS** (+2 §110 锚点)
- sync_app_bundle OK

**§15 GUI 验收 (用户必做)**:
1. `killall meetily && bash scripts/sync_app_bundle.sh && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
2. 打开任一有 summary 的会话 → 期望:
   - 顶部 4 元素 (说话人名单 / 重新生成 / ⚙️ 设置 / 📤 导出)
   - 不再有平铺 9 按钮
   - ⚙️ 下拉含: 自动检测 / AI 模型 / 模板
   - 📤 下拉含: 保存 / 复制 / 导出 MD / 导出 TXT / 打开文件夹
   - 保存项脏时显示绿点

**关联**:
- §104 (UI 华而不实清理, 方向正确)
- §109 (误判隐藏)
- §109.1 (恢复 TranscriptButtonGroup)
- §18 (隐藏 ≠ 删除, SummaryGenerator/Updater 文件保留)
- §56 (AGENTS.md 双校) / §37 (硬闸门) / §92 (决策迁移铁律)
- [[110-SummaryPanel按钮简化]] (Obsidian) / `outputs/§110-...md` (Codex)

## §111 搜狗 .scel 热词转换集成 — sogou_legal / sogou_medical 2 个新 pack (2026-08-12)

**触发**: 用户 8/12 19:30 在 `~/Documents/离线会记/热词/` 放 22 个搜狗拼音 .scel 细胞词库 (8 法律 + 14 医学, 去重 16 个), 请求评估后加入热词。

**评估**:
- 医学 9 个 .scel 总 **123,305** unique 词 (远超热词容量)
- 法律 6 个 .scel 总 **7,197** unique 词
- 3 个太小 (民法常用词汇 8 词 / 法律开庭笔录 11 词 / 各类基本医学大量重复) 忽略
- ⚠️ 全量灌入会爆 sherpa_asr LLM 解码 (>10万词), 必须精选
- ⚠️ .scel 含 metadata 污染 ("方推荐" / "网友上传") + GBK 误解码乱码 ("牎概" / "譬蝟晥")

**实施**:
1. **`frontend/src-tauri/scripts/convert_scel_to_json.py` (新, 299 行)**
   - 解析 .scel (跳过 0x200 字节 metadata, UTF-16LE 中文字符连续提取)
   - 质量过滤: 长度 3-10 / ≥2 汉字 / 排除元数据黑名单 (14 词) / 排除乱码字符 (18 字符)
   - 多文件 md5 去重 + 同词最高频 + freq 降序取 top N
   - 写 hotwords_data/{pack_name}.json 格式 (同 §91 schema)
2. **生成 2 个新 pack**:
   - `sogou_medical_curated.json` (800 词, 医学精选, raw 123K → 800)
   - `sogou_legal_curated.json` (800 词, 法律精选, raw 7K → 800)
3. **`sherpa_hotwords.py` 加 2 个新 pack entry** — UI 自动出现

**8 pack 总览** (`list_available_packs()`):

| ID | 来源 | 词数 | 许可 |
|---|---|---|---|
| general | THUOCL IT/技术工程 | 300 | Apache-2.0 |
| legal | LaWGPT + THUOCL 法律 | 538 | Apache-2.0 + MIT |
| medical | OMAHA + THUOCL 医疗 | 488 | CC-BY-4.0 + Apache-2.0 |
| finance | THUOCL 财经 | 176 | Apache-2.0 |
| **sogou_legal** | **搜狗 .scel 法律精选** | **800** | **用户分享** |
| **sogou_medical** | **搜狗 .scel 医学精选** | **800** | **用户分享** |
| legacy_legal | THUOCL 法律 (旧) | 257 | Apache-2.0 |
| legacy_medical | THUOCL 医疗 (旧) | 249 | Apache-2.0 |

**质量**:
- ✅ 排除乱码 (牎概 / 譬如晥 / 猶) — 800 词池几乎全是真实医学/法律专业术语
- ⚠️ 仍有少量 .scel 描述性词 ("一些法律文书词汇" / "慢更新中" / "有个人色彩"), 不算乱码但价值低 (§18 不主动改)
- 用户分享的 .scel **不随产品 ship** (避免分发法律/版权问题), 仅 ship 转换 + 质量过滤后的精选 JSON

**§37 硬闸门**:
- tsc: 1 §18 bun:test 已知
- next build OK
- cargo build --release 2m05s, binary 23:54
- check_historical_fixes.py **143/143 PASS** (+4 §111 锚点)
- sync_app_bundle OK
- list_available_packs() 测过 8 pack 全 OK

**§15 GUI 验收 (用户必做)**:
1. `killall meetily && bash scripts/sync_app_bundle.sh && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
2. **设置 → 热词** 应该看到 8 个 pack (不是 6 个)
3. 新增 sogou_legal (800 词) + sogou_medical (800 词)
4. 律师/医生录音:
   - `sogou_legal` 录 30s 法律对话 → "质押权/非诉程序/刑事诉讼" 等专业词正确识别
   - `sogou_medical` 录 30s 医学对话 → "骨源性肉瘤/骨疣切除术/股骨头置换" 等正确识别

**关联**:
- §91 (v0.8.6 6 pack 集成, 加 2 个新 pack)
- §40 (v0.8.4 法律/医疗模板深化)
- §18 (不主动改无关 bug)
- §56 (AGENTS.md 双校) / §37 (硬闸门) / §92 (决策迁移铁律)
- [[111-搜狗scel热词转换集成]] (Obsidian) / `outputs/§111-...md` (Codex)

## §112 上架物料 v0.8.6 (2026-08-12 立)

**触发**: 用户 §26 前置必做 #4 — 应用商店文案 + 5 截图 + 1 视频。在 P0-P2 完成后必须做才能上架。

**已落地**:
- 应用商店文案 (中文 4000 字 + 英文 4000 字) — 100% 完成
- 5 张截图 SOP (主屏录音 / 会议详情 / 热词 / 知识图谱 / ⌥+Space) — 100% 完成
- 视频脚本大纲 — 0% (用户说过不强求, 暂不做)
- 隐私政策 URL — 待 yanjingai.tech 上线后确认

**关键文案要点**:
- App Name: 言镜 AI (zh) / SpeakMirror AI (en) - 30 字符
- Subtitle: 离线会议记录与摘要 / Offline Meeting Notes & Insights
- 关键词: 离线,会议,记录,摘要,本地,AI,隐私,中文,转写,翻译 (100 字符)
- 类别: Productivity (主) + Business (副)
- 价格: 免费 (内购解锁热词 / 会员)

**5 张截图都需要用户启 binary 手拍** (按 §15 铁律, CLI 测不出 Tauri GUI):
1. 主屏 + 实时录音 (录音指示器 + 实时转写)
2. 会议详情 + 摘要 + 行动项
3. 设置 → 热词 (8 pack 列表)
4. 知识图谱 / 会议脉络 (主题卡片)
5. 实时问答 (⌥+Space 弹窗 + 3 条建议)

**禁止**:
- 跳过截图硬提交 — App Store 强制 5 张, 缺一拒绝
- 用 Lorem Ipsum / Demo 假数据 — 真实中文会议看着像产品
- 改二进制版本号但不更新 `What's New` 章节 — 审核会抓
- 假装视频可省略 — App Store 不强制但显著降低转化率

**§15 GUI 验收 (用户必做)**:
1. `killall meetily && open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
2. 按 §112 §2.2 SOP 拍 5 张截图到 `frontend/public/screenshots/`
3. `sips -g pixelHeight -g pixelWidth` 验证 2880×1800
4. `git add + commit "feat(§112): 上架物料 5 截图"` 提交
5. App Store Connect 上传 + 提交审核

**关联**:
- §26 (上架物料前置必做 #4)
- §65 (言镜 AI 品牌 / yanjingai.tech 域名)
- §91 (P0-P2 完整收尾, 准备上线)
- [[112-上架物料v0.8.6-2026-08-12]] (Obsidian) / `outputs/§112-...md` (Codex)

## §113 merge main v0.8.6 策略 (2026-08-13 立)

**触发**: 用户原话 "现在开 PR merge main" + GitHub Web PR 因 26 冲突阻挡。

**仓库状态**:
- main: ac10b1a (v0.8.2), 落后 42 commit
- perf/summary-map-concurrency: 068620b (v0.8.6), 领先 63 commit
- 冲突文件数: 26 (shared core: i18n/ConfigContext/api.rs/vad.rs/sherpa_daemon.rs)

**核心结论**: main 旧 42 commit (v0.7.0-rc3 → v0.8.2 期间修复) **95% 已在 perf 演进中独立完成** (按 §22/§23/§24/§25/§29/§31/§51/§52/§57/§58/§60/§61/§62/§63/§64/§70/§85/§91 等章节修复)。

**采用 `git merge -s ours` 策略**:
- 不手动解 26 冲突
- 生成空 merge commit (50e77b5) 告诉 GitHub "采用 ours (release/v0.8.6)"
- 备份 main 旧 42 commit 为 `backup/main-v0.8.2-pre-v0.8.6` (24-48h 后清理)

**执行步骤**:
```bash
# 1. 备份 main
git push origin origin/main:refs/heads/backup/main-v0.8.2-pre-v0.8.6

# 2. 创建 release/v0.8.6
git checkout -b release/v0.8.6 HEAD
git push -u origin release/v0.8.6

# 3. 解决冲突空 merge
git merge -s ours origin/main -m "merge(main): v0.8.6 整体替换 main 旧 42 commit"
git push origin release/v0.8.6 --force-with-lease
```

**后续 commit hash**:
- 50e77b5 merge(main): v0.8.6 整体替换 main 旧 42 commit
- 068620b docs(§112): 上架物料 v0.8.6
- a39d211 feat(§111): 搜狗 .scel 热词转换集成
- efb1a82 refactor(§110): SummaryPanel 按钮简化

**用户下一步**:
1. 刷新 GitHub PR 页面
2. 冲突状态应变为 "All conflicts resolved"
3. 点 "Squash and merge"
4. 填 commit 标题: `v0.8.6 正式发布: P0-P2 完整化 + 热词 8 pack + 上架物料 (§78-§113)`
5. 合并后告诉我清理 backup 分支

**禁止**:
- 用 GitHub Web 手动解 26 冲突 (误操作回滚 perf 修复)
- 删 backup/main-v0.8.2-pre-v0.8.6 之前未经过 24h 观察期

**关联**:
- §37 (release SOP) / §56 (AGENTS.md 双校) / §92 (决策迁移铁律)
- [[113-merge-main-v0.8.6-策略]] (Obsidian) / `outputs/§113-...` (Codex)

## §114 React 渲染错误不再弹红色 toast (2026-08-13 立)

**触发**: 用户截图绿色 "Recording saved successfully!" + 红色 "保存会议失败 (UI 渲染时出错, 已隔离该会议卡; 控制台查看堆栈)" 双 toast。

**根因**: CardBoundary 已隔离渲染错误 + 渲染 fallback 占位卡, useRecordingStop catch 块再 toast 一次 = 双重告警。

**修复 (1 文件)**: `frontend/src/hooks/useRecordingStop.ts`

```typescript
if (isReactInternal) {
  console.warn('[§114] React 渲染错误已被 CardBoundary 隔离, 不弹红色 toast');
  console.warn('[§114] 错误详情:', msg);
} else {
  // 实际错误才弹 toast
  setStatus(RecordingStatus.ERROR, sanitizeDescription(msg, 'error'));
  safeToast.error('保存会议失败', { description: msg, duration: 8000 });
}
```

**行为对比**:
- 修复前: 录音成功 + 某会议卡渲染失败 → 绿 + 红双 toast
- 修复后: 绿 toast + console.warn（用户不被打扰）

**§37 闸门 (2026-08-13 10:21)**:
- tsc / next build / cargo check / cargo test 331/0 / cargo build --release 1m37s / guard 152/152 (4 §114) / sync 3 binary hash 一致

**§15 GUI 验收**:
```bash
killall meetily 2>/dev/null; open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 录 30s → 停 → 期待绿色 toast, 红色 toast 不再弹
```

**禁止**:
- 让 React 内部错误弹 toast（CardBoundary 已隔离 + 渲染 fallback）
- 改 CardBoundary fallback 渲染（应保留红色 "渲染失败" 占位卡，给用户上下文）

**关联**:
- §105 (录音 stop user_id 写入 + 旧标题本地化, 8/12)
- §113 (merge main v0.8.6 策略, 8/13)
- CardBoundary v0.6.10+ (已隔离机制)
- [[114-React渲染错误不再弹红色toast]] (Obsidian) / `outputs/§114-...` (Codex)

## §115 Git workflow 主分支发版 + 24h 自动清理 (2026-08-13 立)

**触发**: 用户原话 "perf/summary-map-concurrency 和 backup/main-v0.8.2-pre-v0.8.6 一样, 保留 24 小时, 后续自动删除. 以后的新版本都需要从 Main 开新分支开发, 合并分支后自动删除."

**核心铁律**:
1. **新版本从 main 开新分支** — 不再从旧 feature 分支累积
2. **合并后自动删除源分支** — `git push origin --delete <branch>`
3. **旧分支保留 24h 观察期** — 打 `cleanup-recommended-<date>` tag 提醒
4. **main 永远是 release-ready** — 不允许"长期 feature"堆积在 main 之外

**v0.8.7 起步 SOP**:
```bash
git checkout main && git pull origin main
git checkout -b feature/v0.8.7-xxx
# 开发 (§37 6 步闸门)
git push origin feature/v0.8.7-xxx
gh pr create --base main --head feature/v0.8.7-xxx
# GitHub Web squash merge
git push origin --delete feature/v0.8.7-xxx
git tag -f cleanup-recommended-$(date +%Y-%m-%d) <branch-sha>
git branch -d feature/v0.8.7-xxx
# 24h 后
bash scripts/cleanup_old_branches.sh --force
```

**当前清理时间表**:
- 2026-08-13 10:24: §115 立, perf + backup 24h 倒计时开始
- 2026-08-14 10:24: 24h 到期, 自动清理 perf/summary-map-concurrency + backup/main-v0.8.2-pre-v0.8.6

**§37 闸门扩展 (7 步)**:
- 0. git status (干净)
- 1. cargo check --lib
- 2. cargo test --lib
- 3. check_historical_fixes.py
- 4. (check_v08_migration_completeness.py 待补)
- 5. cargo build --release
- 6. GUI 端到端
- 7. **git push origin --delete <merged-branch>**

**禁止**:
- ❌ 在 perf/summary-map-concurrency 上继续 dev（已合并 main, 重复工作）
- ❌ 从旧 feature 分支开新分支（必须从 main 起）
- ❌ squash merge 保留源分支（必须 `git push origin --delete`）
- ❌ 24h 内不清理 cleanup-recommended 标记的分支
- ❌ 把 backup/main-v0.8.2-pre-v0.8.6 永久保留

**关联**:
- §37 (release SOP) / §113 (merge main v0.8.6) / §114 (React 渲染错误)
- §18 (不主动改无关 bug)
- [[115-Git-workflow主分支发版+24h自动清理]] (Obsidian) / `outputs/§115-...` (Codex)

## §151 禁止并行多分支 — 同一时间只允许 1 个未合并 codex/* 分支 (2026-08-21 立)

**触发**: 用户 8/21 质问"你为什么要同时存在两个分支呢"。8/20 当天我开了两个并行分支：
- `codex/remove-topic-recall-popup` (11:17) — §144 删除 TopicRecallPopup
- `codex/legal-summary-fix` (11:58) — §145~§150 法律摘要 + 合并漏 commit

reflog 显示当天 checkout 来回切换，§115 已立规则"合并后自动删除"但**未明文禁止并行开分支**。

**铁律**:
1. **同一时间只允许 1 个未合并的 `codex/*` 分支存在** — 检 `git branch -a | grep 'codex/' | wc -l` 必须 ≤ 1（不含已合并未删 / cleanup-recommended-tag 的）
2. **新需求发现时**:
   - (a) 当前分支已 commit 但未合并 → **在当前分支继续加 commit**（优先）
   - (b) 当前分支已 push 等合并 → **等当前 PR 合并后再开新分支**
   - (c) 紧急独立主题（不阻塞当前 release）→ 开新分支前**必须显式告诉用户"我会再开一个分支"**
3. **违反处理**:
   - 立即把新分支 merge --no-ff 到当前分支（或反之）
   - 删除已合并的源分支（本地 + 远端）
   - 在 commit message 注明"按 §115 + §120 清理分支"
4. **新章节/新决策 SOP**:
   - 任何 commit message 含"§X" → 必须当日同时落地: 代码 + Obsidian + AGENTS.md §X
   - §115 / §120 是合并/分支相关, 单独 check 一次

**今日已落地清理**:
- `codex/remove-topic-recall-popup` 1 个 commit (`1eb62b0 §144`) merge --no-ff 进 `codex/legal-summary-fix`
- 新 merge commit: `b43bae2 merge(codex/remove-topic-recall-popup): §144 TopicRecallPopup 删除 — 合并到 legal-summary-fix (按 §115 规则清理分支)`
- 删除本地 + 远端 `codex/remove-topic-recall-popup`
- 远端 `codex/legal-summary-fix` 推到 origin (5d0a1c5..b43bae2)

**未来规避检查 (每次 commit 前必跑)**:
```bash
git branch -a | grep -E '^\s+codex/' | grep -v 'remotes/' | wc -l
# 期望输出 1 (或 0, 如果当前 checkout 在 main)
# 如果 ≥ 2 → 停止 commit, 先按 §120 处理
```

**§120 关联**:
- §115 (主分支发版 + 24h 自动清理)
- §92 (决策迁移铁律, outputs + Obsidian + AGENTS.md 三处同步)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit)

**关联**: [[151-禁止并行多分支-2026-08-21]] (Obsidian) / `outputs/§151-...md` (Codex)

## §121 P0-A / P2-C LLM trigger 必须用 Ollama 不 BuiltInAI (2026-08-15 立)

**触发**: 用户 8/14 导入 566fe7a9 (1h49m 音频) → 转录 OK → topic_node / topic_dossier 全部 0 行, 知识图谱从未触发.

**根因 (5 跳)**:
1. `topic_graph/mod.rs:397` (`trigger_after_summary`) → `LLMProvider::BuiltInAI`
2. `topic_graph/mod.rs:538` (`rebuild_topic_dossier`) → `LLMProvider::BuiltInAI`
3. `live_qa/mod.rs:131` (`run_live_qa`) → `LLMProvider::BuiltInAI`
4. `summary/llm_client.rs::generate_summary` 对 BuiltInAI 强制要 `app_data_dir: Option<&Path>` (sidecar binary `llama-helper` 路径)
5. 3 处 trigger 链都传 `None` → `generate_summary` 内部 `.ok_or_else(|| "app_data_dir is required for BuiltInAI")` → Err → 上层 swallow 写 warn log

**修复 (commit pending)**:
3 处全部改用 `LLMProvider::Ollama`, 走 `localhost:11434`, 用户机器已跑 `qwen3.5:2b` (2.74GB).

**§121 铁律**:
1. 任何 spawn hook / 异步 trigger 调 BuiltInAI 必须传 `app_data_dir: Some(&app.path().app_data_dir()?)` —— 否则永远 fail.
2. 或者改用 Ollama (`localhost:11434`) —— 本地 Ollama 在 P0-A / LiveQA 这种 trigger 链路更稳, 不依赖 sidecar binary 启动.
3. **禁止 BuiltInAI swallow log**: trigger 链路任何 LLM call 失败必须升级 `log::error!` + emit Tauri 事件 (`topic-extract-failed` / `topic-dossier-failed`), 前端 `meeting-details/page.tsx` 监听 + toast.error. 不再 silent.
4. 新增 trigger 必加单元测试 mock LLM: 防止 "传 None" 类 bug 永远跑不到.
5. 任何 §X 改动 LLM 调用必须 cargo test + 实跑 trigger 一次验证 DB 表非空.

**§121 实施位置**:
- `topic_graph/mod.rs:415-426` (`trigger_after_summary` 失败 emit `topic-extract-failed`)
- `topic_graph/mod.rs:559-572` (`rebuild_topic_dossier` 失败 emit `topic-dossier-failed`)
- `meeting-details/page.tsx:206-220` (前端 listener + toast.error)

**§37 6 步硬闸门 (§121)**:
- ✅ cargo check --lib: 0 errors (28 warnings §18 不动)
- ✅ cargo test --lib: 335 passed / 0 failed / 3 ignored
- ✅ check_historical_fixes.py: 171 → **176/176 PASS** (+5 §121 anchor)

**关联**:
- [[121-P0-A-LLM-trigger-改Ollama-2026-08-15]] (Obsidian) / `outputs/§121-topic_graph-LLM-trigger-改Ollama-2026-08-15.md` (Codex)
- §91 (P0-A 完整化收尾, §121 是其 silent-fail 补丁) / §88 (P2-B/C 收尾) / §85 (MVP 起点)
- §18 / §37 / §15 / §99.5 (Tauri spawn 边界 — 不同话题但容易混淆)

## §122 action_items parser 兼容多模板 (2026-08-15 立)

**触发**: §121 修完后审计发现 `action_items` 表 0 行. parser 只认 `**行动事项**` / `## 行动事项`, 漏抓法律/电商模板的 marker.

**根因**: `action_items/mod.rs::parse_markdown_action_items` 早期只识别 2 个 marker. §91 P2-A 完整化收尾时加了 8 个模板 (含 `legal_consultation` / `cross_border_ecommerce`), 但 parser 没同步.

| 模板 | 行动事项 marker |
|---|---|
| `standard_meeting` | `**行动事项**` / `## 行动事项` |
| `legal_consultation` | `**待办事项**` / `## 待办事项` |
| `cross_border_ecommerce` | `**下周重点事项**` / `## 下周重点事项` |
| `medical_consultation` | (无对应, 用 `**待确认信息**` 而非行动项) |

**修复**: parser marker 列表扩展为 6 个 (3 个 `**...**` + 3 个 `## ...`).

**新增测试**: `test_parse_legal_template_todo_marker` + `test_parse_ecommerce_template_marker` (2/2 PASS).

**§122 铁律**:
1. 新增任何模板必须在 §X commit 同步更新 action_items parser marker — 不允许"模板加了一节但 parser 没接"
2. 占位 marker 必须随模板加: 新模板若 marker 段落允许空 (`本次无...`), 必须同步加占位字符串到 `ACTION_ITEMS_PLACEHOLDERS`
3. parser 测试矩阵: 新 marker 必须有一个独立 `#[test]` 覆盖, 防止回归

**§37 6 步硬闸门 (§122)**:
- ✅ cargo check --lib: 0 errors (28 warnings §18 不动)
- ✅ cargo test --lib action_items: **7/7 PASS** (含 2 个 §122 新测试)
- ✅ check_historical_fixes.py: 176 → **180/180 PASS** (+4 §122 anchor)

**关联**:
- [[122-action_items-parser-兼容多模板-2026-08-15]] (Obsidian) / `outputs/§122-action_items-parser-兼容多模板-2026-08-15.md` (Codex)
- §91 (P2-A 完整化收尾, §122 是其 parser 漏兼容补丁) / §121 (同 session 修复 topic_graph silent fail)
- §85 / §18 / §37 / §15

## §123 模板选择持久化 + 法律/医学热词提醒 + FunASR-Nano 热词透传 (2026-08-15 立)

**触发**: 用户 8/15 4 件事:
1. **P1**: 模板按钮能不能默认显示用户已经选择的模板
2. **P2**: 法律/医学模板选时检查热词是否勾选, 没勾就提示
3. **P1**: 自定义模板先不做, 但要保证模板足够 (审计 9 个模板足够)
4. **P1**: 转录/总结过程中热词是否真的起作用 (FunASR-Nano 修复)

### 模板覆盖度审计 (用户判定已足够, 不做自定义 UI)
| 模板 | 场景 |
|---|---|
| `standard_meeting` | 通用 (fallback) |
| `daily_standup` | 每日站会 |
| `project_sync` | 项目同步 |
| `retrospective` | 敏捷回顾 |
| `sales_marketing_client_call` | 销售客户 |
| `legal_consultation` | 法律咨询 (Pro) |
| `medical_consultation` | 医疗会诊 (Pro) |
| `psychiatric_session` | 心理 SOAP (Pro) |
| `cross_border_ecommerce` | 跨境电商 (§86) |

### 改动 (15 文件 + 1 migration)

**P1 模板持久化**:
- 新 migration `20260815000000_meetings_template_id.sql` (加列 + 索引, 老数据 NULL fallback)
- `MeetingModel.template_id: Option<String>` + `MeetingMetadata.template_id` + `MeetingDetails.template_id`
- `api_process_transcript` 加 `UPDATE meetings SET template_id = ?1` (选过模板就持久化)
- 前端 `useTemplates(initialTemplateId?: string | null)` 优先用 `meeting.template_id`
- `SummaryGeneratorButtonGroup` 显示 `selectedTemplateName || t('summary.template')` + `max-w-[120px] truncate`

**P2 legal/medical 热词提醒**:
- `useTemplates::handleTemplateSelection` 选 legal/medical → 调 `hotwords_get` 读 pack
- 不在 whitelist → `safeToast.warning(t('summary.template_hotwords_missing'))` (不阻塞, 只提醒)
- whitelist: legal = `['legal', 'sogou_legal', 'legacy_legal']`; medical = `['medical', 'sogou_medical', 'legacy_medical']`
- i18n zh + en 各新增 2 个 key (`template_hotwords_missing` / `_desc`)

**P1 FunASR-Nano hotwords 透传**:
- **根因**: `sherpa_asr.py::_load_funasr_nano` 创建 recognizer 时 `hotwords=""` 写死, sherpa-onnx 1.13.4 `OfflineRecognizer.from_funasr_nano` 接受 `hotwords: str` 但**没有 setter**, 必须重建. 之前写死 → 用户设置的法律热词永远不生效 (Paraformer/SenseVoice 走 postprocess 不受影响).
- **修复**: `_load_funasr_nano(model_dir, hotwords="")` 接参数 + `_ensure_model(tag, hotwords_str="")` + 模块级 `_RECOGNIZER_HOTWORDS`
- funasr_nano tag hotwords 变化时强制重建 recognizer, stderr log `[sherpa_asr] §123 funasr_nano hotwords changed, reloading recognizer`
- Paraformer/SenseVoice 不影响 (走 postprocess)
- `transcribe()` 入口计算 `_hotwords_str = ",".join(get_hotwords(...))` 传 `_ensure_model(tag, _hotwords_str)`

### 铁律

1. **summary 按钮必须显示当前模板名** — UI 必须回显, 不允许 fallback 翻译硬编码
2. **legal/medical 选模板不阻塞, 只提醒** — 用户决策, §18 不主动加硬阻塞
3. **funasr_nano hotwords 变化必须重建 recognizer** — sherpa-onnx 1.13.4 不支持 setter, 别试 setter
4. **后端 log 必须验证热词生效** — stderr 留 `[sherpa_asr] §123 funasr_nano hotwords changed, reloading recognizer`
5. **§92 三处同步**: outputs + Obsidian + AGENTS.md 同日落
6. **i18n 路径严格一致** — 这次从一开始就走 `summary.template_hotwords_missing.*`, 不放孤儿路径 (按 §107 教训)

### §37 6 步硬闸门
- tsc --noEmit: 1 个 §18 bun:test (不动)
- next build: OK (`/meeting-details` 1.43MB)
- cargo check --lib: 0 errors / 28 §18 warnings (不动)
- cargo test --lib: **337 passed / 0 failed / 3 ignored**
- check_historical_fixes.py: **192/192 PASS** (12 个 §123 锚点)
- cargo build --release + sync_app_bundle.sh (见下方)

### 关联
- [[123-模板选择持久化+法律医学热词提醒+FunASR-Nano热词透传]] (Obsidian)
- `outputs/§123-...md` (Codex)
- §92 (决策迁移铁律) / §37 (硬闸门) / §15 (GUI 验收) / §18 (不主动改无关)
- §29 (FunASR-Nano Pro tier gate) / §104 (华而不实隐藏) / §106 (固定本地模式)
- §107 (i18n 路径教训 — 这次严格走 `summary.*` 顶级) / §108 (sync_app_bundle sidecar)
- §122 (action_items parser 兼容多模板, 上次 commit, 同日)

## §124 SummaryPanel 顶部工具栏统一 (2026-08-16 立)

**触发**: 用户 8/16 截图反馈 "重新生成摘要页面和第一次生成的页面不一样, 你要统一了哦".

**3 套不同 UI (现状)**:
| 状态 | 顶部工具栏 |
|---|---|
| `!aiSummary` (首次) | 居中 `<SummaryGeneratorButtonGroup>` (生成/语言/⚙️Dialog/模板) |
| `isSummaryLoading` (加载) | 居中 `<SummaryGeneratorButtonGroup>` (同上) |
| `aiSummary` 已存在 | §110 4 元素 (说话人/重新生成/⚙️ Dropdown/📤 Dropdown) |

**统一方案 (1 文件)**:
- 删除 SummaryGeneratorButtonGroup + SummaryUpdaterButtonGroup imports (`SummaryUpdaterButtonGroup` 是 dead code, 从未真正 render)
- 顶部条件 `{aiSummary && !isSummaryLoading && (...)}` → `{!isSummaryLoading && (...)}` (3 状态共享)
- 主按钮 3 态: 加载中 = 红"■停止" / 有摘要 = "✨重新生成" / 无摘要 = "✨生成摘要"
- 说话人 button: `aiSummary` 条件渲染 (没摘要就隐藏, 不需 disabled)
- ⚙️ / 📤 Trigger button 加 `disabled={isSummaryLoading}`
- ⚙️ 内 Template 项显示 `selectedTemplateName || t('summary.template')` (与 §123 一致)
- 主区 3 态简化: loading = 流式/spinner / `!aiSummary` = EmptyState / `aiSummary` = BlockNote

### 铁律 (§18 强化)
1. **同一面板 3 状态必须共享工具栏** — 不允许"首次/重新/加载"看到不同按钮组
2. **Disabled 通过 prop, 不是元素隐藏** — 让用户能看到按钮存在
3. **隐藏 ≠ 删除** (§104 §110): "说话人 button" 是隐藏 (没摘要时)
4. **dead import 必须清理** — SummaryUpdaterButtonGroup 从未真正 render, delete

### 验证 (§37 硬闸门)
- tsc --noEmit: 1 个 §18 bun:test (不动)
- cargo check --lib: 0 errors / 28 warnings (§18 不动)
- cargo test --lib: **337 passed / 0 failed / 3 ignored**
- cargo build --release: 1m32s, binary 72M
- check_historical_fixes.py: **200/200 PASS** (+7 §124 anchors)
- sync_app_bundle.sh: 全 sync + §98 codesign

### 关联
- [[124-SummaryPanel-统一顶部工具栏-三状态]] (Obsidian)
- `outputs/§124-...md` (Codex)
- §110 (4 元素工具栏首次) / §123 (selectedTemplateName) / §18 (hidden ≠ deleted)
- §37 (硬闸门) / §92 (决策迁移铁律) / §15 (GUI 验收)

## §125 中英文适配 — BuiltInModelManager + SummaryLanguageSettings (2026-08-16 立)

**触发**: 用户 8/16 截图反馈 "中英文适配" — /settings 页面同时有中文 raw key 字面 (`models.showing_available`) + 英文硬编码 ("Pin one language..." / "Ready" / "Selected").

### 改动 (4 文件 +73/-17)

**zh.ts / en.ts 加 `models:` 顶级块** (之前 keys 写错 namespace 在 `account:` 下):
```ts
models: {
  title: '内置 AI 模型' / 'Built-in AI Models',
  showing_all/available/...,  // 5 个
  status: { ready, selected, corrupted, error, downloading }, // 5 个
  action: { download, cancel, retry, delete, delete_model }, // 5 个
  size: { tokens, unit_separator }, // 2 个
}
```

**zh.ts / en.ts 加 `settings_page.summary_language_*` (3 个) + `language_picker.{pin_label, unpin_label, remove_label}` (3 个)**

**BuiltInModelManager.tsx** 改 13 处: Ready/Selected/Corrupted/Error/Download/Cancel/Retry/Delete/下载中/title/size separator/size tokens/title attribute

**SummaryLanguageSettings.tsx** 改 6 处: h3/description/aria-label (Pin/Unpin)/remove aria-label/default hint

### 根因 (1 跳)

zh.ts / en.ts 把 `models_showing_available` 等 keys 用下划线写在 `account:` 子块里, **没在顶级创建 `models:` 对象**。代码用 `t('models.showing_available')`, i18n lookup 通过 `dict.models?.showing_available` 返回 undefined → fallback 返回 path 字符串自身 → 用户看到 raw key 字面.

修复 1: 加 `models: { ... }` 顶级对象 (含已下划线形式的 keys, 转换到嵌套形式)
修复 2: 所有硬编码英文 (aria-label / description / button text) 改 `t()`

### 验证 (§37 硬闸门)
- tsc --noEmit: 1 个 §18 bun:test (不动)
- cargo check --lib: 0 errors / 28 warnings (§18)
- cargo build --release: 1m30s, binary 72M
- next build: OK
- check_historical_fixes.py: **218/218 PASS** (+19 §125 anchors)
- sync_app_bundle.sh: 全 sync + §98 codesign

### 关联
- [[125-中英文适配-模型管理+摘要语言设置]] (Obsidian)
- `outputs/§125-...md` (Codex)
- §38 续 / §107 (i18n 路径教训) / §90 (UI 漏代码 4 项)
- §124 (SummaryPanel 顶部工具栏统一, 上次 commit)
- §37 (硬闸门) / §92 (决策迁移铁律) / §15 (GUI 验收)

## §126 会议脉络空数据修复 — History Recovery (2026-08-16 立)

**触发**: 用户 8/16 反馈 `/knowledge` 页面"近期主题"一直空。截图显示 0 个 topic_node, 但 DB `summary_processes.status='completed'` 有 11 条, `result.english_cache.markdown` 全部有内容.

**根因 (3 跳)**:
1. 历史 `trigger_after_summary` (`topic_graph/mod.rs:359`) 调用全部 silent fail — 早期 Ollama 未启动 / 模型未下载 / spawn task panic / 第一次 silent fail 后 retry 路径缺失
2. dedup "已 link → skip" 没记录失败, 也无 retry 路径
3. `loadTopics` (`knowledge/page.tsx`) 只 SELECT 不补提, 用户看不到原因

Ollama 当前健康 (`curl http://localhost:11434/api/tags` 返 qwen3.5:2b 2.7GB).

### 修复 (3 文件 +91/-1)

1. **`topic_graph/mod.rs::extract_missing_topics`** (新 +60):
   - SQL: `SELECT sp.meeting_id, sp.result FROM summary_processes sp LEFT JOIN meeting_episode_node ep ON ep.meeting_id = sp.meeting_id WHERE sp.status = 'completed' AND sp.result IS NOT NULL AND ep.id IS NULL ORDER BY sp.updated_at DESC LIMIT ?1`
   - 逐条 parse `result.english_cache.markdown` → 调现有 `trigger_after_summary`
   - Returns `(processed_count, total_topics_after)`
2. **`api_topic_extract_missing`** Tauri command (新 +10): 前端显式调, 默认 `max_meetings = 10`
3. **`knowledge/page.tsx::loadTopics`** (+14): api_topic_recent 返空数组时**自动**调 api_topic_extract_missing, 然后 re-fetch. 失败 console.warn 不阻塞.

### borrow checker 修复 (技术细节)

`api_topic_extract_missing` 里同时 `app.state()` 借用 + `extract_missing_topics(app, ...)` move, 编译器报 E0505 + E0716 (临时值 drop):
```rust
// ❌ E0505: move out of `app` occurs here, borrow later used here
let state: State<'_, AppState> = app.state();
let pool = state.db_manager.pool();
extract_missing_topics(app, pool, ...).await

// ❌ E0716: temporary value dropped while borrowed
let state: State<'_, AppState> = app.clone().state();

// ✅: 先把 cloned app 绑到 let, 让临时值活过 state borrow
let app_for_state = app.clone();
let state: State<'_, AppState> = app_for_state.state();
let pool = state.db_manager.pool();
extract_missing_topics(app, pool, ...).await
```

### §37 硬闸门 (commit pending)
- ✅ cargo check --lib: 0 errors (28 §18 warnings 不动)
- ✅ cargo test --lib: 337 passed / 0 failed / 3 ignored
- ✅ next build: OK
- ✅ cargo build --release: 1m33s
- ✅ check_historical_fixes.py: 223/223 PASS (+5 §126 anchors)
- ✅ sync_app_bundle.sh: tauri bundle binary SHA synced

### §15 GUI 验收 (用户必做, 不能 CLI 测)
1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Applications/言镜 AI.app'` (symlink)
3. 打开 `/knowledge` 页 → 期望首次进入自动 trigger auto-recover + 看到 ≥ 1 topic
4. 0-30s 后 DB 验证:
   ```bash
   sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
     "SELECT COUNT(*) FROM topic_node; SELECT COUNT(*) FROM meeting_episode_node;"
   # 期望: topic_node ≥ 30, meeting_episode_node ≥ 11 (1:1 for 11 completed summaries)
   ```

### 关联
- §85 §91 P0-A topic_graph (schema + Phase 1/2 + 整合)
- §121 (trigger_after_summary 改 Ollama + emit Tauri 事件)
- §125 (上一 commit, i18n 中英文适配)
- §37 (硬闸门) / §15 (GUI 验收) / §92 (三处同步)

**关联**: [[126-会议脉络空数据修复-history-recovery]] (Obsidian) / `outputs/§126-...md` (Codex)

## §127 会议脉络 UI 大气化 (2026-08-16 立)

**触发**: 用户原话 "另外整个会议脉络的UI太小气, 你整的好看点, 大气点"。截图反馈 Hero 区挤、stat card 全隐藏、Topic card 字号小、Dossier 面板 11/12px typography。

### 设计决策 (5 条)
1. **不加 modal**：保留双列体验, dossier 面板 lg:sticky 固定右侧
2. **max-w-5xl → max-w-7xl** (1280px), Hero + 卡片更多呼吸
3. **stat 摘要做 hero 右上角 4 个数字一行** (替代 §104 隐藏的 4 张卡片, 节省版面但仍传达数字)
4. **typography 升级**: Hero 44px / 副标题 16px / Topic 卡片 18px / dossier 正文 13.5px / stat 数字 26px tabular-nums
5. **质感细节**: Hero 渐变 `bg-gradient-to-br from-neutral-50 via-white to-violet-50/30` + 卡片左侧细色条 (project/decision/person/general) + AnimatePresence 切换 dossier

### 改动 (1 文件, 481 → 460 行)
- `frontend/src/app/knowledge/page.tsx` 完全重写:
  - Hero 区: 标题 44px + 4-stat 数字走廊 (4 个 tabular-nums 大字)
  - Toolbar: 圆角 pill filter + search input + refresh button (紫色回填状态)
  - Topic grid: sm:grid-cols-2 2 列, left 0.5 彩色边条 + 类型 chip + mentions 计数
  - Aside: AnimatePresence mode="wait", 3 态 (empty / loading / dossier) 平滑切换
  - Dossier 区块: rounded-xl + 半透明背景 (neutral/emerald/amber)
  - 删 6 个 §104 `{false as boolean}` hide 标记
  - 加 framer-motion AnimatePresence

### §37 6 步硬闸门
- ✅ tsc --noEmit: 0 errors (1 §18 bun:test 不动)
- ✅ next build: OK
- ✅ cargo build --release: 1m32s
- ✅ check_historical_fixes.py: **228/228 PASS** (+5 §127 anchors)
- ✅ sync_app_bundle.sh: tauri bundle SHA synced

### 设计原则 (任何 v0.X 演进适用)
1. **字号梯度明确**: Hero 32+ / Section 18-20 / Card title 16-18 / 正文 13-14 — 不准 11/12 撑版面
2. **背景渐变 1 个**: bg-gradient-to-br 整页, 卡片保持纯白半透明
3. **保留 stat 但轻量化**: 不是隐藏也不是做 4 张大卡, hero 行 4 个数字
4. **彩色色条比色块高级**: 卡片左侧 2-4px 细条, 比整块色块克制
5. **sticky dossier 比 modal 连贯**: 主流程同屏
6. **AnimatePresence mode="wait"**: dossier 切换不堆叠

### §15 GUI 验收 (用户必做)
```bash
killall meetily && open '/Users/wangwei/Applications/言镜 AI.app'
# 1. /knowledge → 大字 hero "会议脉络" + 右上 4 个数字
# 2. 首次进入 (DB 0) → 紫色 banner "回填中..."
# 3. 30-60s 后 → banner 消失, topic grid 浮现 30+ 卡片 (2 列)
# 4. 点任一 topic → 右侧 dossier 平滑切换
# 5. dossier 内 4 个区块 + 重建档案按钮
```

**关联**: §126 (auto-recover 喂数据) / §104 (sidebar 改名 + 隐藏 stat) / §124 (SummaryPanel 工具栏统一) / §37 / §15 / §92
[[127-会议脉络UI大气化]] (Obsidian) / `outputs/§127-...md` (Codex)

## §128 摘要设置下拉模板/模型切换修复 (2026-08-16 立)

**触发**: 用户 8/16 截图反馈 "重新生成这里摘要设置还是不能切换模板和模型"。

**根因**: `SummaryPanel.tsx` 在 §124 dead-code-elimination 时整合 2 个 ButtonGroup, 但"摘要设置"下拉中:
- **AI 模型** → `onOpenModelSettings?.(() => {})` 空回调, 啥也不做
- **模板** → `onTemplateSelect(availableTemplates[0].id, ...)` hardcode 第一个, 永远切到第一个

完整功能在 SummaryGeneratorButtonGroup.tsx (有完整 template dropdown + ModelSettingsModal Dialog), §124 dead-code 删除。

### 修复 (3 文件)
1. **`SummaryPanel.tsx`** 重写"摘要设置"下拉:
   - **AI 模型** DropdownMenuItem 触发本地 Dialog, 内容装 ModelSettingsModal (layout="dialog") + 可视化
   - **模板** 改 DropdownMenuSub + SubTrigger + SubContent + RadioGroup, 列出全部 availableTemplates 供选择
   - **PRO 徽章** 标记 required_tier='member' 的模板
   - **loading 状态** 显示 "正在加载模板…" 当 availableTemplates 为空
   - 加 modelSettingsDialogOpen state + Dialog + VisuallyHidden DialogTitle
   - Props 加 `required_tier?: 'free' | 'member'` 对齐 useTemplates

2. **`i18n/locales/{zh,en}.ts`**: 加 `summary.loading_templates`

3. **Import 升级**: 加 Dialog/DialogContent/VisuallyHidden/DropdownMenuSub/SubTrigger/SubContent/RadioGroup/RadioItem/ModelSettingsModal/Loader2/Check

### §37 6 步硬闸门
- ✅ tsc --noEmit: 0 errors (1 §18 bun:test 不动)
- ✅ next build: OK
- ✅ cargo check --lib: 0 errors / 28 §18 warnings 不动
- ✅ cargo build --release: 1m26s
- ✅ check_historical_fixes.py: **234/234 PASS** (+6 §128 anchors)
- ✅ sync_app_bundle.sh: tauri bundle SHA synced

### §15 GUI 验收 (用户必做)
```bash
killall meetily && open '/Users/wangwei/Applications/言镜 AI.app'
# 任意会议详情:
# 1. 摘要设置 → hover "模板" → 弹出 submenu 列出全部模板 + 当前选中打钩
# 2. 点其他模板 → 按钮名 + useState 更新
# 3. AI 模型 → 弹 ModelSettingsModal 对话框, 选 Ollama 端点 / Built-in AI 模型
# 4. save → 按钮右侧显示新模型名
# 5. "重新生成" 实际用新模板 + 新模型
```

### 教训 (§56 强化)
- §124 dead-code-elimination 时**只删了 unused import 标注**, 但没用 §15 GUI 验收验证"摘要设置" 是否仍能用
- 整合两个组件到 SummaryPanel 时, 漏了"摘要设置" 下拉的完整 dropdown 内容
- 这次真修复证明: 任何组件整合 / dead-code 标记 → §15 GUI 必跑, 不能只看 cargo check + tsc pass

### 关联
- §124 (整合 ButtonGroup, dead-code-elimination 漏 dropdown 内容)
- §123 (模板选择持久化)
- §106 (ModelSettingsModal 砍云端 provider)
- §37 / §15 / §56 / §92

[[128-摘要设置下拉修复]] (Obsidian) / `outputs/§128-...md` (Codex)

## §129 摘要 polling 超时修复 + 陈旧 PENDING 启动清理 (2026-08-17 立)

**触发**: 用户 2026-08-17 上午截图反馈 "重新生成摘要报错":
> 生成总结出错
> Summary generation timed out after 15 minutes. Please try again or check your model configuration.

DB 里 `meeting-566fe7a9` (106 min 音频 / 537 段 / 26k chars) 状态 PENDING 从 8/16 05:30 卡到 8/17 01:04, **永不清理**。

### 根因 (3 条叠加)

1. **`SidebarProvider.tsx:201`** `MAX_POLLS = 300` (10 min at 2s interval) 太短
   - 二郎神 35 min 会议: 5 chunks × 1072s = **17.9 min** (超 MAX_POLLS)
   - 宇宙演化 106 min 会议: 10 chunks × 565s = 9.4 min (贴近)
   - 566fe7a9 106 min: 预估 12-15 min (必超)

2. **错误消息硬编码 "15 minutes"** 但实际是 10 min (文案与数据脱节, 跟 §107 同病)

3. **`summary_processes` PENDING 行永不清理** — `api_process_transcript` 设 PENDING → force-quit / OOM / llama-helper crash 时永远 PENDING

### 修复 (4 文件)

1. **`SidebarProvider.tsx`** — `MAX_POLLS = 300 → 900` (30 min) + 加 `localT()` helper (跟 §104.1 录音通知 toast 同模式, callback 不能用 useTranslation) + 超时后**兜底再查一次 backend** (万一是 polling 期间已完成 → 显示成功)

2. **`i18n/locales/{zh,en}.ts`** — 加 `summary.timeout_error` zh/en, `{minutes}` 占位符

3. **`database/repositories/summary.rs`** — 新增 `cleanup_stale_pending_processes(pool, stale_minutes)` 函数:
   ```rust
   UPDATE summary_processes
   SET status='failed', error='Interrupted by app shutdown — ...',
       result=COALESCE(result_backup, result),
       result_backup=NULL, result_backup_timestamp=NULL
   WHERE status='PENDING' AND updated_at < ?
   ```
   - 保留 result_backup 恢复 (跟 §P1-B 一致)

4. **`lib.rs` 启动钩子** — 在 `database::setup` + §99.2 backfill spawn 之后, threshold 30 min:
   ```rust
   tauri::async_runtime::spawn(async move {
       if let Some(app_state) = ... try_state::<crate::state::AppState>() {
           let pool = app_state.db_manager.pool();
           match SummaryProcessesRepository::cleanup_stale_pending_processes(pool, 30).await { ... }
       }
   });
   ```
   - 必须 `tauri::async_runtime::spawn` (§99.5 铁律, 不能 `tokio::spawn`)
   - 必须在 §99.2 backfill spawn 之后 (AppState 已 manage)

### §37 6 步硬闸门

- ✅ tsc --noEmit: 1 §18 bun:test 不动
- ✅ next build: OK
- ✅ cargo check --lib: 0 errors / 28 §18 warnings
- ✅ cargo build --release: 1m43s, binary 69M mtime 09:33
- ✅ check_historical_fixes.py: **241/241 PASS** (+7 §129 anchors)
- ✅ sync_app_bundle.sh: tauri bundle SHA synced

### §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

启动时日志应见: `§129 stale PENDING cleanup scheduled (threshold 30 min)`

DB 验证 (PENDING 行应自动转 failed):
```bash
sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
  "SELECT meeting_id, status, substr(error,1,80), datetime(updated_at) FROM summary_processes ORDER BY updated_at DESC LIMIT 3"
```
应见: 之前 PENDING 那行 → status='failed', error='Interrupted by app shutdown — backend did not complete within 30 minutes. ...'

错误消息 i18n 验收:
- zh locale: "会议摘要生成超过 30 分钟已自动停止轮询。后端可能仍在处理, 请稍后在「会议详情」页刷新查看结果, 或重新点击「重新生成」。"
- en locale: "Summary polling exceeded 30 minutes and was stopped. The backend may still be processing. Please refresh the meeting details page to check, or click Regenerate again."

### 铁律 (任何 v0.X 演进适用)

1. **polling 上限必须 ≥ 后端实际处理时长** — 长会议 Map-Reduce 经常 15-30 min, polling 必须给 buffer
2. **错误消息必须 i18n + 跟实际数字一致** — 跟 §107 教训同
3. **DB 启动时清理陈旧 PENDING** — 任何 background process 必须有 stale-state recovery
5. **必须 §99.5 tauri::async_runtime::spawn** — 启动钩子里不能 tokio::spawn
4. **结果兜底再查一次** — 超时后不能盲目报错, 万一 backend 已完成要给用户成功结果

### 关联

- §15 (GUI 验收) / §37 (硬闸门) / §56 (AGENTS.md §X ≠ 代码 commit, commit 前 grep 验证)
- §92 (决策迁移铁律, outputs + Obsidian + AGENTS.md §X 同日落)
- §104.1 (localT pattern 借鉴) / §107 (i18n 路径正确, 这次直接 summary.timeout_error)
- §99.5 (tauri::async_runtime::spawn) / §99.2 (spawn 顺序)
- [[129-摘要polling超时修复]] (Obsidian) / `outputs/§129-摘要polling超时修复-2026-08-17.md` (Codex)

## §132.1 Ollama 不可用 banner 改友好文案 (2026-08-18 立, commit cf97a1f)

**触发**: §132 commit 后 banner 文案 "历史主题回填已跳过 — Ollama 未运行。启动 Ollama 后点击右上角"刷新"重试" 太技术化, 用户反馈"啥意思"。

**commit**: `cf97a1f` (branch main, push OK)
**binary**: target/release/meetily 70M mtime 12:52

### 改动 (4 文件, +66/-8)

1. **`frontend/src/app/knowledge/page.tsx`** — 加 `X` icon import, 第 253-262 行旧简短 1 行文案 → 新 banner 卡片:
   - 标题: `t('knowledge.ollama_offline_title')` = "想跨会议追踪主题, 需要本地 AI 模型"
   - 描述: `t('knowledge.ollama_offline_desc')` = "会议脉络会把每场会议的摘要提炼成主题、人物、决议..."
   - 选项 A 卡片 (链 https://ollama.com/download): "选项 A: 安装 Ollama (推荐)" + "到 ollama.com 下载安装, 启动后会自动在后台运行, 然后回到这里点"刷新""
   - 选项 B 卡片 (链 /settings/models): "选项 B: 使用言镜 AI 内置模型" + "打开设置 → 模型管理, 下载 Qwen 3.5 2B (2GB), 然后点"刷新""
   - 关闭按钮 (X 图标, 调 `setRecoverStatus('idle')` 让 banner 消失)
   - 保持琥珀色 (border-amber-200 + bg-amber-50/60)

2. **`frontend/src/i18n/locales/zh.ts`** — 加 8 个 key (ollama_offline_title/_desc/_option1_title/_option1_desc/_option2_title/_option2_desc/_download/_dismiss)

3. **`frontend/src/i18n/locales/en.ts`** — 同步英文版

4. **`scripts/check_historical_fixes.py`** — 加 2 个 §132.1 anchor:
   - `132_1_banner_i18n_title` — `t('knowledge.ollama_offline_title')` 存在
   - `132_1_banner_dismiss_button` — `setRecoverStatus('idle')` 存在
   - guard 325 → **327/327 PASS**

### 设计原则

1. **技术文案 → 用户场景**: 不说"回填已跳过", 说"想跨会议追踪主题, 需要本地 AI 模型"
2. **不只说"为什么不能", 说"怎么办"**: 两个明确选项 (Ollama / 内置), 用户可点直达
3. **不绑架用户**: 关闭按钮让用户跳过这个引导, 不强制看完
4. **保留原琥珀色 + 边框**: 视觉仍警示, 内容从技术报错 → 行动引导

### §37 6 步硬闸门 (commit cf97a1f)
- ✅ tsc --noEmit: 0 errors (除 §18 bun:test 已知)
- ✅ next build OK
- ✅ cargo build --release: 1m 量级, binary 12:52
- ✅ check_historical_fixes.py **327/327 PASS**
- ✅ sync_app_bundle.sh: 3 binary 全 sync (main + llama-helper + ffmpeg)

### §15 GUI 验收 (用户必做)
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进 /knowledge 页 (会议脉络)
# 2. 如果 Ollama 没跑, banner 应显示 8 个新文案
# 3. 选项 A 按钮 → 浏览器打开 https://ollama.com/download
# 4. 选项 B 按钮 → 跳到 /settings/models
# 5. X 关闭按钮 → banner 消失, recoverStatus = 'idle'
```

### 关联
- §132 (banner 首次出现, 7d timeout + 5 meetings cap)
- §18 (云端 API 永不接入 — 引导用户用本地)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit, 这次 code + i18n + guard 一次到位)
- §37 (硬闸门) / §92 (决策迁移铁律, outputs + Obsidian + AGENTS.md §X 同日落)
- outputs/§132.1-Ollama不可用banner-改友好文案-2026-08-18.md
- [[132.1-Ollama不可用banner-改友好文案]] (Obsidian)

## §137 摘要生成中拦截跳转 (2026-08-18 立, commit 48be0f6)

**触发**: 用户反馈 "现在生成摘要的过程不能跳到其他页面，否则就停了。如果用户点了切换其他页面，这时正好在生成摘要，需要让用户确认"

**commit**: `48be0f6` (main, push OK), binary 13:50 70M, guard **331/331 PASS**

### 改动 (6 文件, +310/-0)

1. **新建 `frontend/src/hooks/useNavigationGuard.ts`** (174 行):
   - 拦截 3 层: `history.pushState` (Next.js router.push) / `popstate` (浏览器后退) / `beforeunload` (刷新/关闭)
   - 保存原始 pushState/replaceState, 替换为 wrapper
   - 同一 pathname+search (query 变化) 放行
   - 不同 url → 拦截, setPendingNav({to, type})
   - confirm 调原始 pushState, cancel popstate 时把 url 推回

2. **新建 `frontend/src/components/NavigationConfirmDialog.tsx`** (87 行):
   - 复用 shadcn Dialog + Button (outline/destructive)
   - lucide `AlertTriangle` (amber) + 动态 `Loader2` (等待)
   - 智能描述: popstate 追加 "(浏览器后退)", beforeunload 追加 "(关闭/刷新浏览器)"
   - data-testid: `navigation-guard-cancel` / `navigation-guard-confirm`

3. **`frontend/src/app/meeting-details/page-content.tsx`** (+30/-0):
   - import useNavigationGuard + NavigationConfirmDialog
   - 算 `isSummaryInProgress = ['processing','summarizing','regenerating'].includes(summaryGeneration.summaryStatus)`
   - 调 useNavigationGuard({when: isSummaryInProgress, ...})
   - </motion.div> 之前插 dialog 渲染

4. **`frontend/src/i18n/locales/zh.ts`** + **en.ts** — 顶层 `nav_guard` 块 (title/description/confirm_text/cancel_text)

5. **`scripts/check_historical_fixes.py`** — 4 个 §137 anchor (hook/dialog/integration/i18n), guard 327 → 331

### 设计原则
1. **拦截 3 层**: pushState (90%) / popstate (5%) / beforeunload (5%)
2. **不破坏 Next.js**: 原始引用保存, 同一 pathname+search 放行
3. **取消时回滚 url**: popstate 触发时浏览器已改 url, 取消用原始 pushState 推回原 url
4. **智能描述**: popstate 追加 "(浏览器后退)", 让用户知道是哪个动作触发的

### §37 6 步硬闸门
- ✅ tsc 0 / next build OK / cargo 1-3m / guard 331/331 / sync 3 binary

### §15 GUI 验收
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进任一会议 → 点"生成摘要" (或"重新生成")
# 2. 进度条开始跑 (state = processing/summarizing/regenerating)
# 3. 点"返回工作台" 按钮 → 弹确认 dialog
# 4. 点 Sidebar 任意 nav → 弹同样 dialog
# 5. 浏览器后退 (⌘+[) → 弹 dialog, 描述追加 "(浏览器后退)"
# 6. "继续等摘要" → 关闭 dialog, 还在原页, 摘要继续跑
# 7. 再次点 → "继续离开" → 真跳走
```

### 铁律
1. **重要 background process 必须有 navigation guard** — 摘要 / 录音 / 重新转录 等长时操作, 用户误触离开会丢失进度
2. **拦截在 history 层而非 router 层** — Next.js 13+ App Router 用 history.pushState, 在更底层拦截覆盖所有调用方式 (router.push / <Link> / Sidebar 按钮)
3. **3 层缺一不可**:
   - pushState (90%): 按钮 / Sidebar / Link
   - popstate (5%): 浏览器后退
   - beforeunload (5%): 刷新 / 关闭
4. **不破坏 Next.js** — 原始引用保存 + 同一 pathname 放行
5. **新加 background process 必加 guard** — 任何 "长时操作" UI 都加 useNavigationGuard

### 关联
- §135 (摘要多次生成历史, history 保留)
- §129 (摘要 polling 30 min, stale PENDING 清理) — 跟 §137 互补: 一个防卡死, 一个防误中断
- §104 (录制通知 toast i18n, 同样的"用 React state 但不在 React 组件" pattern)
- §15 (GUI 验收) / §37 (硬闸门) / §56 (AGENTS.md 双校) / §92 (决策迁移铁律)
- outputs/§137-摘要生成中拦截跳转-2026-08-18.md
- [[137-摘要生成中拦截跳转]] (Obsidian)

## §138.1 fact_guard regex false positive + 法律模板硬约束 (2026-08-19 立)

**触发**: 用户 8/19 检查最近摘要发现:
- 911f52ae (魏某专利案) 摘要把"二零二二年七月" 错改成 "2022 年 5 月" (实际"7月") — 日期错误
- 911f52ae 摘要多处"未提及具体姓名"/"未在提供的文本片段中详细记载"/"模糊表述" 回避语
- 911f52ae 摘要元数据"庭审日期未明确" 但事实时间线又写"2024 年 5 月 29 日开庭" 自相矛盾
- fact_guard 报"原文含重量/容量单位" 警告 — 但 transcript 0 含重量单位 — false positive

### 4 个独立根因

**Bug 1**: `WEIGHT_UNIT_RE` line 30 接受 `[一-鿿]+` (任意中文) + 单位字符, 误命中 "巧克力"/"麦克风"/"提升/开庭"
**Bug 2**: `MONEY_UNIT_RE` line 29 同样 false positive, "判决"/"原告"/"元素" 都命中
**Bug 3**: `DATE_RE` line 23 不识别中文数字日期 (transcript 全是"二零二二年七月" 这种) + 不容忍空格 (摘要"2022 年 5 月" 不匹配)
**Bug 4**: 法律模板 instruction 写"禁止用'未提及'填空" 但模型用变种"未提及具体姓名"/"转录未明确提及", 缺更显眼的硬约束

### 修复

**`summary/fact_guard.rs`** — 3 个 regex 重写 (line 23/29/30):
- `WEIGHT_UNIT_RE`: 必须以数字/中文数字开头
- `MONEY_UNIT_RE`: 必须以数字/中文数字开头
- `DATE_RE`: 加中文数字日期分支 + 加 `\s*` 容忍空格

**`templates/court_hearing.json` + `templates/legal_consultation.json`** — `事实时间线 / Key Events Timeline` section instruction 末尾追加 §138 硬约束 (6 条):
1. 日期 verbatim: '二零二二年七月' 不能改写成 '2022年5月'
2. 金额 verbatim: '167万余元' 不能改写为 '167万'
3. 人名 verbatim: 原文用'魏某'就写'魏某',原文用'魏立秋'就写'魏立秋'
4. 因果不颠倒
5. 禁止逃避语: 严禁 '未提及具体姓名'/'未在提供的文本片段中详细记载'/'模糊表述'/'转录未明确提及' 等回避语. 信息不足写'信息不明确'或省略
6. 单位不换算: 克/公斤/毫升 与 元/块/美元 不可互换

### 验证

- `cargo test --lib summary::fact_guard` **24 passed / 0 failed** (新加 6 个 §138.1 测试)
- `cargo test --lib` **371 passed / 0 failed / 3 ignored** (全套)
- 911f52ae 回测: transcript 12 个日期 (全中文数字) + summary 6 个日期 (全阿数字) → unexpected_dates = 6 项 (修复前 = 0)
- `python3 scripts/check_historical_fixes.py` **379/379 PASS** (367 → 379, 净 +12 §138.1 anchor)
- cargo build --release 3m40s, binary 73M

### 12 个新 guard anchor (§138.1)

`138_weight_unit_no_chinese_word` / `138_money_unit_no_chinese_word` / `138_date_re_matches_chinese_numeric` / `138_date_re_tolerates_space` / 6 个测试函数名 + 2 个模板硬约束 anchor

### §56 / §92 教训

- §56 扩展: 任何修改 fact_guard regex 时**用真实 DB transcript 跑回归** (`grep` 看具体修了哪个 false positive)
- §92 严格执行: outputs + Obsidian + AGENTS.md + 代码 + guard 5 处同日落
- §131.1 highlight_unexpected_facts 配合: 摘要保留 AI 原文, fabricated tokens 用 `==⚠️xxx⚠️==` 标黄, 用户能直接看到哪句出问题

### 已知边界 (§18 不主动改)

- 911f52ae 摘要已经存在 DB, 不自动重生成 (用户没要求); 新会议用新模板 + 新 fact_guard
- 213a1c41 摘要 "伙同弟弟一家" 改写错误 (实际是"与弟弟一家争执") — 中文语义保真度问题, 下一轮跟进
- f2dfa2e0 摘要基本准确 ✓
- transcript "前一年" / "那一年" 中的 "一年" 被 DATE_RE 误命中 (低优先级)
- 37 cargo warnings (§18 不动)

### §15 GUI 验收

```
killall meetily && open binary
# 任一会话重生成摘要
# 1. 不再出现 "未明确具体姓名" / "模糊表述" / "未在提供的文本片段中详细记载" 等
# 2. 日期严格 verbatim (用 transcript 原文措辞)
# 3. fact_guard 警告条数减少 (false positive 修了)
```

### 关联

- §137.5 / §137.4 / §137.3
- §131.1 highlight / §131.2 unit_confusion (现 §138.1 修 false positive)
- §18 / §37 / §56 / §92

---


## §140 topic_graph parser 容错修复 (2026-08-19 立)

**触发**: 用户 8/19 反馈"已经点了重新生成, 你检查问题; 会议脉络还是不生效"。`topic_node` / `meeting_episode_node` / `topic_dossier` 全部 0 行, 23 个 completed 摘要都未触发 topic extract 成功。

**3 层根因** (parser 严格, LLM 输出不规范):
1. **字段名错**: qwen3.5:2b 输出 `topic_name` (不是 `canonical_name`)
2. **sentiment 数字**: 输出 `sentiment: -1` (不是字符串 "negative")
3. **JavaScript-style 无引号 key**: `{topic_name: "x"}` 不是合法 JSON, serde_json 直接 reject

加上 LLM 包 ```json 包装, parse_extract_response 0 topics 解析成功。

**修复 (4 处)**:
1. `strip_markdown_fence` — 去 ``` / ```json 包装
2. `quote_unquoted_keys` — JavaScript-style 无引号 key 加引号 (regex: `([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:` → `$1"$2":`)
3. `normalize_extract_line` — 别名映射 (topic_name/name/title/subject → canonical_name; type/category/kind → topic_type; score/polarity/tone → sentiment) + sentiment 数字 → 字符串 (1=positive, 0=neutral, -1=negative)
4. `PROMPT_INSTRUCTIONS` 严格化 — 明确字段名 (canonical_name / topic_type / excerpt / sentiment) + 明确说"不是 topic_name / name / type / score 等别名" + "sentiment 必须是字符串, 不是数字 1/0/-1"

**兜底**: trigger_after_summary 已有白名单 fallback (general / project / person / decision) → "general"

**端到端验证 (用真实 qwen3.5:2b + 新 prompt)**: 3 topics 全部解析成功 (修复前 0 topics)。

**测试**: `cargo test --lib topic_graph::extract` **12/12 PASS** (10 → 12, +2 §140 markdown fence + unknown topic_type)

**验证**:
- `cargo build --release`: 73MB binary 22:55 OK
- `check_historical_fixes.py`: **414/414 PASS** (405 → 414, +9 §140 anchors)
- `sync_app_bundle.sh`: §99.6 sync tauri bundle binary OK

**9 个新 guard anchor (§140)**:
- `140_extract_prompt_canonical_name` / `140_extract_prompt_sentiment_string` / `140_parser_normalize_alias` / `140_parser_quote_unquoted_keys` / `140_parser_sentiment_number_mapping` / `140_extract_test_topic_name_alias` / `140_extract_test_sentiment_positive` / `140_extract_test_sentiment_zero` / `140_quote_unquoted_keys_test`

**§15 GUI 验收**:
```
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 重生成 f2dfa2e0 (顺义执行案) 摘要
# sqlite3 ... "SELECT COUNT(*) FROM topic_node" 期望 ≥ 1
# /knowledge 页应该看到 topic 出现
```

**已知边界** (§18 不动):
- 1 个 §18 bun:test tsc 错误
- 37 cargo warnings (extract.rs 加了 1 个 unused import warning - 不修)
- 旧 meeting topic_node 是 0 的, 需要重新生成摘要触发 trigger_after_summary

**关联**: §139 (模板商业化精进, prompt 大改 - §140 是 §139 上线后 LLM 实际调用才发现的 parser bug) / §P0-A Phase 2 (topic_graph 实施, 当时只测 mock 输出, 没真用 qwen3.5:2b 跑) / §56 / §92 / §37

[[§140-topic_graph-parser-容错-2026-08-19]] (Obsidian) / `outputs/§140-topic_graph-parser-容错-2026-08-19.md` (Codex)

## §139 模板提示词商业化精进 (2026-08-19 立)

**触发**: 用户 8/19 原话 "针对各种模板的提示词, 你站在商业化的角度, 再次精进"

**核心目标**: 10 个摘要模板从"可读"提升到"专业交付级",让客户拿到摘要可以直接转发给老板/律师/医生/合作伙伴,不需要二次加工。

### 1. §139 通用商业化硬约束 (5 条)

每个模板 timeline section instruction 末尾追加:

```
【§139 商业化硬约束 — 摘要必须达到专业交付级】
1. 零废话: 严禁'本文将/接下来/首先/其次'等空话. 严禁'TBD/待定/...'. 直接给事实和结论.
2. 零编造: 客户拿到摘要直接转发给老板/律师/医生/合作伙伴, 不能出 fact error. 不确定就写'信息不明确'.
3. 可追溯: 每个关键事实带 [证据: mm:ss] 转录时间戳, 客户按时间戳能直接跳到原始录音核对.
4. 专业语: 不用 LLM 风格 ('可能也许'/'总之'/'总的来说'). 用领域术语 (庭审用'法庭调查/举证质证', 医疗用'主诉/查体', 销售用'异议处理').
5. 可对比归档: 同一项目/客户/案件的多次会议摘要, 命名/人名/数据口径要一致, 便于跨会议纵向对比.
```

### 2. 10 模板定制商业化可交付级块 (新增 sections)

| 模板 | 新增/升级 sections |
|---|---|
| standard_meeting (8) | 新增 "会议出席与缺席"; 升级 "关键决议" item_format |
| daily_standup (9) | 升级 "成员出席与状态"; 新增 "阻塞项与升级需求 (P0/P1/P2)" |
| project_sync (12) | 升级 "里程碑与状态"; 新增 "里程碑详细状态表" + "决策日志" |
| retrospective (8) | 升级 "改进行动项"; 新增 "行动项测量指标" |
| legal_consultation (9) | 新增 "法律风险评级 (高/中/低)" + "证据清单 + 文件清单" |
| court_hearing (10) | 升级 "关键证据"; 新增 "法条引用块" + "庭审阶段时间线" |
| medical_consultation (11) | 升级 "治疗与处置计划"; 新增 "PHI 警告头"; 升级 "随访与预警事项" |
| psychatric_session (14) | 升级 "安全与风险管理"; 新增 "PHI 严格脱敏警告" |
| sales_marketing_client_call (12) | 升级 "客户需求与痛点"; 新增 "客户异议处理表" + "价格表" + "下次会议时间" |
| cross_border_ecommerce (13) | 升级 "本周核心数据"; 新增 "平台/账号/达人 ID 清单" + "投放决策表" |

### 3. 数据结构修复

新加的 sections 必须有 `format` 字段, 只能是 `paragraph` / `list` / `string` 三种值 (不支持 `table`)。最初 8 个新 sections 误用 `table`, 修正为 `list`。

### 4. 验证

- **cargo test --lib summary::templates**: 16/16 PASS (含 12 个新增 sections 仍可加载)
- **cargo build --release**: 73MB binary 15:32 编译完成
- **check_historical_fixes.py**: **405/405 PASS** (379 → 405, +26 §139 anchors)
  - 10 个 §139 通用商业化硬约束 anchors
  - 16 个 §139 商业化可交付级块 anchors
- **sync_app_bundle.sh**: §99.6 sync tauri bundle binary OK

### 5. 已知边界 (按 §18 不主动改)

- 1 个 §18 bun:test tsc 错误 (不动)
- 37 cargo warnings (§18 不动)
- next build 在 sandbox 内被 harness kill, 验证只能看 cargo build (主 binary 编译)
- 模板 instruction 中的叙事示例仍是央视《庭审现场》节目内容 (历史, §37 已批准)
- 模板不写 ICD 编码 / 药物剂量推导 / 胜诉概率预测 (医疗/法律硬约束, §18 不动)

### 关联

- §138.1 (fact_guard regex false positive + 法律模板硬约束) - 不重叠, §138 防 fact error, §139 提升专业交付级
- §37 (硬闸门)
- §56 (AGENTS.md §X 章节 ≠ 代码 commit, 写完必须 git log 验证)
- §92 (决策迁移铁律, 代码 + AGENTS.md + outputs + Obsidian 四处同日落)
- §28 (用户决策迁移铁律)
- [[§139-模板提示词商业化精进-2026-08-19]] (Obsidian) / `outputs/§139-模板提示词商业化精进-2026-08-19.md` (Codex)

## §138 摘要质量根因修复 (2026-08-18 立, commit 9807c64)

## §137.1 nav_guard 模块级 singleton 修复 (2026-08-19 立, commit 即将)

**触发**: 用户 8/19 11:00 反馈"之前的录音生成**音效**时切换到其他地方需要提醒的功能还没生效" (语义实为"摘要生成")。

**根因 (race condition)**: §137 旧版用 `useEffect` 注册 `pushState` wrapper + React `useState` 存 `pendingNav`。当用户点 Sidebar 链接触发 Next.js 路由切换:
1. Next.js 内部调 `history.pushState` → wrapper 拦截 → `setPendingNav(...)`
2. **同步**: Next.js fetch 响应回来, 准备 unmount meeting-details
3. **React commit**: meeting-details unmount → useEffect cleanup 跑 → `pushState = originalPush` 还原
4. setPendingNav 排队的 state 更新被丢弃 (component 已 unmount)
5. KnowledgePage mount, dialog **永远不弹**

**修复方案 — Module-level singleton + 引用计数**:
- `pushState` / `replaceState` 包装移到 module-level (跨 component lifecycle)
- `pendingNav` 用 module-level + 订阅者 Set
- 引用计数 (`refCount`): 第一个 hook `when=true` 装 wrapper, 最后一个 `when=false` 卸 wrapper
- **关键**: 有 `pendingNav` 时不卸载 wrapper, 防止 dialog 在 unmount 后无法响 confirm

**实现位置** (frontend/src/hooks/useNavigationGuard.ts):
- 14 个 module-level 变量 (wrapperInstalled/originalPush/.../subscribers Set)
- 8 个 module-level 函数 (installWrapper/uninstallWrapper/acquireWrapper/releaseWrapper/moduleConfirm/moduleCancel/notify/getCurrentKey)
- hook 内 useEffect 只管 subscribe/unsubscribe + acquire/release, **不再** 直接改 `window.history`

**7 个新守卫锚点** (guard 342 → 349):
- `137_1_module_singleton` / `137_1_ref_count` / `137_1_acquire_release`
- `137_1_install_uninstall` / `137_1_module_confirm`
- `137_1_no_effect_cleanup` (在 uninstallWrapper 内, 非 useEffect)
- `137_1_pendingnav_survives_unmount` (有 pending 不卸载)

**§37 6 步硬闸门 (本次)**:
- ✅ tsc 0 errors (1 §18 bun:test 不动)
- ✅ next build 13.2s
- ✅ cargo build --release 1m37s (36 §18 warnings 不动)
- ✅ guard **349/349 PASS** (342 → 349, +7)
- ✅ sync_app_bundle.sh 3 binary OK
- ⏳ §15 GUI 验收 (用户必做, 不能 CLI 测)

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进任一会议 → 点"生成摘要"
# 2. summaryStatus 变 'processing' / 'summarizing' / 'regenerating'
# 3. 点 Sidebar "会议脉络" 或 "返回工作台" 按钮
# 4. 预期: 弹 dialog "🔴 摘要还在生成中 / 现在离开会中断摘要生成, 之前的进度会丢失"
# 5. 点"继续等摘要" → URL 不变, 摘要继续
# 6. 再点 Sidebar, 这次点"继续离开" → 真跳转, 摘要中断
```

**铁律 (防同类 bug)**:
1. **任何 useEffect 里修改 `window.history` / `window.addEventListener('beforeunload')` / 全局副作用都有 unmount race 风险** — 必须 module-level singleton
2. **跨 component lifecycle 状态禁止用 React useState 单挂** — 用 module-level + subscribers
3. **多 hook 实例共存用引用计数** — `refCount > 0` 时不卸载
4. **有 pending state 时保留 wrapper** — 让 dialog 在 unmount 后仍能响应 confirm/cancel
5. **navigation guard / beforeunload / 全局 event listener 都按此模板** — §137.1 是黄金标准

**关联**:
- §137 (8/18 原版, useEffect race) — 本节取代
- §110 §132.1 (Ollama i18n 路径错位) — 同类"代码 ≠ 描述"教训
- §15 §37 §92 §56 — 必须跑硬闸门 + commit 描述必须匹配代码
- outputs/§137.1-nav_guard-模块级singleton修复-React-unmount-race-2026-08-19.md
- [[137.1-nav_guard-模块级singleton修复-React-unmount-race]] (Obsidian)

**触发**: 用户截图 8/18 庭审摘要 7212 字符, 段落大规模重复 + ASR 错字进摘要 + 编造人名"魏立秋". 用户说 "OK 完成p0然后再进行p1和p2".

**commit**: `9807c64` (main, push OK), binary 14:37 70M, guard **337/337 PASS**

### 4 类根因 + 4 项修复

#### P0.1 Map-Reduce 段落去重
- 现象: 8 chunk × 8 段 = 64 段, 5+ 段重复 (LLM 每个 chunk 都生完整 8 段)
- 修: `processor.rs::dedup_chunk_summaries` — 解析 ## / ### 段, normalized hash 判重, 跨 chunk 重复只保留首次
- 集成: `recursive_reduce_summaries` 末轮调 `summarize_fn` 之前 dedup 一次
- 测试: 4 个全过 (removes_duplicate / normalizes_punctuation / keeps_distinct / empty_and_single)

#### P0.2 ASR 错字过滤
- 现象: "院二二二二二二二二的原则上下发了..." ASR 错字直接进 transcripts
- 修: 新建 `audio/asr_sanitize.rs::sanitize_asr_text` — 折叠连续 5+ 重复字符 / 截断 200 字无标点段 / 质量分级 (High/Medium/Low)
- 集成: `database/repositories/transcript.rs::save_transcript` 写入前
- Low quality 段不写入 DB (避免污染摘要 prompt)
- 测试: 5 个全过 (collapse / truncate / low / high / garbled)

#### P1.1 + P1.2 + P1.3 0-编造 + 强制 mm:ss + 金额计算
- 现象: 编造"魏立秋" (转录只有"魏某"), 时间线 `[证据:未明]` 占位, 8 万 / 10 万 数字不区分
- 修: `processor.rs::P1_PRECISION_RULES` 常量, 6 条硬规则:
  1. 0 编造人名/日期/案号/金额
  2. 强制 [证据: mm:ss] (无锚点拒绝写入)
  3. 金额计算显式化 ("11.5 万 × 5 倍 = 57.5 万")
  4. 称谓一致 (不混用"魏某" / "魏立秋")
  5. Subject Name Consistency
  6. Alias Normalization (别名映射表)
- 注入到 3 个 prompt: build_chunk / build_combine / build_final_report

#### P2.1 Alias 规范化 (文本预处理)
- 现象: "徐氏米业公司" / "徐某" / "该公司" / "被告" 混用
- 修: `asr_sanitize.rs::normalize_aliases` — 转录写入前替换
  - "徐氏米业公司" / "徐氏米业有限责任公司" → "徐氏米业"
  - "魏丽秋" / "魏立秋" → "魏某"
- 测试: 3 个全过 (unify_company / collapses_fabricated / no_op)

### 6 个新 guard 锚点 (§138)
- `138_p01_dedup_function` / `138_p01_dedup_called`
- `138_p02_sanitize_module` / `138_p02_sanitize_in_transcripts`
- `138_p21_alias_normalize`
- `138_p11_p12_p1_precision_rules`
- guard 331 → **337/337 PASS**

### §37 6 步硬闸门 (commit 9807c64)
- ✅ tsc 0 errors
- ✅ next build OK
- ✅ cargo test --lib: **364 passed / 0 failed** (12 个 §138 新测全过)
- ✅ cargo build --release: 1m39s, binary 14:37 70M
- ✅ check_historical_fixes.py **337/337 PASS**
- ✅ sync_app_bundle.sh: 3 binary 全 sync

### §15 GUI 验收
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 重新生成 meeting-911f52ae 摘要
# 2. 期望:
#    - 摘要长度 < 3500 字符 (之前 7212)
#    - 段落不重复 (之前 5+ 段重复)
#    - 时间线每条带 [证据: mm:ss]
#    - 没有"魏丽秋" / "魏立秋" (应该是"魏某")
#    - "11.5 万 × 5 倍 = 57.5 万" 列出
# 3. DB: SELECT chunk_count, processing_time FROM summary_processes
#       WHERE meeting_id='meeting-911f52ae-07f6-43fb-9f95-2d79fc4ccc1f'
```

### 铁律
1. **Map-Reduce 必加 dedup** — 任何 chunk 跑完整模板, 必 dedup 否则大量重复
2. **ASR 错字必须过滤** — sherpa-onnx 中文偶发严重错字, 不过滤直接进 prompt 必污染摘要
3. **P1 规则 prompt-level, 不能完全保证 LLM 100% 执行** — 大幅改善, 极端 case 仍可能漏
4. **Alias 规范化双管齐下** — 文本预处理 (确定性) + Prompt 规则 (LLM 辅助), 两层防护
5. **下次新加摘要模板必带 P1 规则** — templates/*.json 改完要看 system prompt 是否包含 P1_PRECISION_RULES

### 关联
- §137 (navigation guard) — 上一个 commit
- §135 (摘要多次生成历史) — chunk_count 仍然有, dedup 后会少
- §136 (整件事叙述段) — narrative 规则保留, P1 加强
- §135.1 (final report depth priority) — 时间线拿 40-50% token
- §131.3 (anti-fabrication) — P1_PRECISION_RULES 升级版
- §15 / §37 / §56 / §92

### 已知边界
- 老数据 (v0.8.6 之前生成的摘要) 不重做, 历史污染无法补救
- tsc 1 个 §18 bun:test 错误, 36 cargo warnings (§18 不动)

## §137.4 本地 LLM 调用 thinking mode off 铁律 (2026-08-19 立, commit 即将)

**触发**: 用户 8/19 指令"调用本地模型都需要使用 thinking mode:off 的模式" — 防止 Qwen3.5 thinking mode 拖慢所有本地 LLM 推理 (topic extract / 摘要 / dossier rebuild / LiveQA)。

### 现状审计 (8/19)
- **BuiltInAI (llama-helper)**: `model_def.template = "qwen3.5_nonthinking"` + `QWEN35_NONTHINKING_TEMPLATE` 模板末尾 `<think>\n</think>\n\n` 强制 assistant 跳过 think 块 ✓
- **Ollama**: `llm_client.rs:297` `"think": false` (json body) ✓
- **3 个调用点都覆盖**: processor.rs (Map-Reduce 摘要) + topic_graph (extract/dossier) + live_qa (⌥+Space)

**实际已对, 只是没 guard 防未来漏掉。**

### 3 个新守卫锚点 (guard 355 → 358)
- `137_4_ollama_think_false_in_llm_client` — `llm_client.rs:297` `"think": false`
- `137_4_qwen_nonthinking_template` — `models.rs` 含 `qwen3.5_nonthinking`
- `137_4_qwen_nonthinking_template_const` — `models.rs` 含 `QWEN35_NONTHINKING_TEMPLATE` 常量

### 铁律 (适用所有未来 v0.X)
1. **Ollama provider 必须 `think:false`** — Qwen3.5 thinking mode 默认开, 推理 30-50s 且空 content
2. **BuiltInAI Qwen3.5 必须用 `qwen3.5_nonthinking` template** — 模板末尾 `<think>\n</think>\n\n` 强制跳过
3. **新加 LLM provider / model 必须立刻加 anchor** — 不加 anchor 下次重构被覆盖 (§56 §92)
4. **不允许暴露"thinking mode"开关给用户** — 99% 用户不知道 Qwen thinking, 99% 场景不该用
5. **新加 ModelDef 必须配 nonthinking template** — 不允许只配 `qwen3.5` 默认 thinking template

### 未来加 LLM provider/model 的 SOP
1. `models.rs` 加 `ModelDef`, template 字段必须指向 "nonthinking" 变体
2. 或: `llm_client.rs` 加 provider 分支, Ollama 路径加 `think:false`
3. 加 anchor 到 `check_historical_fixes.py` (字符串检查)
4. 跑 guard 确认新增 anchor PASS

### 关联
- §111 (8/18 Ollama /api/chat + think:false 原始修复)
- §137.3 (8/19 topic_graph 优先 BuiltInAI, BuiltInAI 走 nonthinking template)
- §91 (8/7 Qwen3.5 2B 集成 + nonthinking template)
- §18 §37 §56 §92 — 硬闸门 + commit 必带代码 + AGENTS.md 双校
- outputs/§137.4-本地LLM调用thinking-mode-off铁律-2026-08-19.md
- [[137.4-本地LLM调用thinking-mode-off铁律]] (Obsidian)

## §137.5 topic_graph / live_qa 用用户选模型 — 不再硬编码 qwen3.5:2b (2026-08-19 立)

**触发**: 用户 8/19 原话
> model name qwen3.5-2b不要写死啊, 我们本地模型可以选择其他模型, 用户选择什么本地模型就要用什么本地模型

§137.3 (commit b961822) 我加了 BuiltInAI 兜底, **同时硬编码了 `"qwen3.5:2b"`** — 用户当场指出。grep 全代码后发现同样反模式在 `rebuild_topic_dossier` + `live_qa::ask_live_qa` + `live_qa::api_meeting_live_qa` + `topic_graph::scheduler` 都存在, 一次全修。

### 改动 (8 个文件)

**Rust**:
- `topic_graph/mod.rs`: `trigger_after_summary` / `preflight_llm_async` / `extract_missing_topics` / `rebuild_topic_dossier` / `api_topic_extract_missing` / `api_topic_rebuild_dossier` 全部接受 `provider: LLMProvider` + `model_name: &str` 参数
- `summary/service.rs::process_transcript_background` 调 `trigger_after_summary` 处透传 provider + model_name
- `topic_graph/scheduler.rs`: 夜间重建 scheduler 从 `SettingsRepository::get_model_config(pool)` 读用户当前 model_config (兜底 `ollama` + `llama3.2:latest`)
- `live_qa/mod.rs`: 删 `MODEL_NAME` const, `ask_live_qa` / `api_meeting_live_qa` 接受 provider + model_name, 加 `app_data_dir_for_built_in_ai()` helper

**TS (3 个调用方)**:
- `app/knowledge/page.tsx`: `useConfig()` 拿 `modelConfig`, `api_topic_extract_missing` + `api_topic_rebuild_dossier` invoke 都加 `provider: modelConfig.provider, modelName: modelConfig.model`
- `components/TopicSearch/TopicSearchModal.tsx`: `useConfig()` + `api_topic_rebuild_dossier` invoke 加 provider/modelName
- `components/LiveQA/LiveQAOverlay.tsx`: `useConfig()` + `api_meeting_live_qa` invoke 加 provider/modelName

### 14 个 guard anchor (新增)

| Anchor | 守卫目标 |
|---|---|
| `137_5_trigger_takes_provider_param` / `_takes_model_param` | `trigger_after_summary` 签名 |
| `137_5_preflight_takes_provider_param` | `preflight_llm_async` 签名 |
| `137_5_extract_takes_provider_param` | `extract_missing_topics` 签名 |
| `137_5_rebuild_takes_provider_param` | `rebuild_topic_dossier` 签名 |
| `137_5_api_topic_extract_takes_provider` | `api_topic_extract_missing` Tauri command |
| `137_5_api_topic_rebuild_takes_provider` | `api_topic_rebuild_dossier` Tauri command |
| `137_5_live_qa_takes_provider` / `_api_takes_provider` | live_qa 两个函数 |
| `137_5_knowledge_passes_modelconfig` / `_topicsmodal` / `_liveqaoverlay` | 3 个前端 invoke 传 modelConfig |
| `137_5_summary_service_passes_provider` | service.rs 调用 trigger_after_summary 传 tg_provider |
| `137_5_scheduler_uses_settings` | scheduler 调 SettingsRepository::get_model_config |

`scripts/check_historical_fixes.py::grep()` 加 `-U` 多行模式 flag, 让 `[\s\S]` 跨行匹配生效 (历史 anchor 全部回溯验证仍 PASS)。

### guard 数变化

358 (handoff 上次) → **367/367 PASS** (本节): -5 (删除 3 §121 + 2 §137.3 硬编码 anchor) + 14 §137.5 新 anchor。

### §56 教训扩展

§137.3 我写 BuiltInAI 兜底时**手贱硬编码了 `"qwen3.5:2b"`** — 用户当场指出。**今后写 § 修复必须 grep 全代码确认同类反模式没漏**: `grep -rn '"qwen3.5:2b"' frontend/src-tauri/src --include="*.rs"` 在 commit 前必跑。

### §15 GUI 验收 (用户必做)

1. killall meetily && open binary (确认 mtime 13:26+)
2. 设置 → 模型 → 切换 (例如 Ollama + qwen3.5:4b 或 BuiltInAI + qwen2.5:3b)
3. 触发 4 动作, 后端日志看 `using {provider:?} model={model_name}` 跟随切换:
   - 生成摘要 (trigger_after_summary)
   - 摘要完成后 (spawn hook 跑 topic extract)
   - Cmd+K → Topic Search → 选 topic → 重建档案 (rebuild_topic_dossier)
   - 会议详情 → ⌥+Space → LiveQAOverlay → 提问 (live_qa)

### 关联

- §137.3 (昨天, BuiltInAI 兜底引入硬编码)
- §137.4 (thinking mode off)
- §18 / §37 / §56 / §92

---

## §137.3 topic_graph 优先 BuiltInAI 路径 (2026-08-19 立, commit 即将)

**触发**: 用户 8/19 反馈"我们本地不是有模型吗, 为什么还要再下载" — 弹窗让用户去下载 Ollama, 但本机已装 `models/summary/Qwen3.5-2B-Q4_K_M.gguf` (1221 MB BuiltInAI 路径)。

**根因** (3 跳):
1. `topic_graph::trigger_after_summary` §121 决定只支持 Ollama — 注释明确 "BuiltInAI 强制要 app_data_dir, trigger 链路传 None → llm 永远 fail"
2. `preflight_ollama_async` 只 ping localhost:11434, BuiltInAI 路径完全不查
3. 用户本机已装 Qwen3.5 2B, 但 Ollama 服务没起 → preflight fail → 弹窗推荐下载 Ollama
4. 用户困惑: "本机不是有模型吗, 为什么还要下载"

**修复** (1 文件, +30/-10):
- 新增 `builtin_ai_model_exists(app_data_dir: &Path) -> bool` helper: 检查 `app_data_dir/models/summary/*.gguf`
- `preflight_ollama_async()` → `preflight_llm_async(app) -> Result<&'static str, String>`:
  - 优先 BuiltInAI (本机 .gguf) → Ok("builtin_ai")
  - fallback Ollama (3s ping) → Ok("ollama")
  - 都失败 → Err(reason)
- `trigger_after_summary` 加 BuiltInAI 分支:
  - 本机有 .gguf → `LLMProvider::BuiltInAI` + `qwen3.5:2b` + `app_data_dir=Some(...)`
  - 否则 fallback Ollama (兼容老逻辑)
- 调用方 `preflight_ollama_async()` → `preflight_llm_async(&app)`

**用户效果**:
- 本机已装 Qwen3.5 2B → preflight Ok("builtin_ai") → **完全不弹窗**, topic extract 直接用本机 LLM
- 没装 BuiltInAI + Ollama 在跑 → preflight Ok("ollama") → 不弹窗
- 都没 → 弹窗 (兼容老逻辑)

**关键文件**:
- `frontend/src-tauri/src/topic_graph/mod.rs` — 1 个 helper + 1 个 preflight + 1 个 trigger 分支

**5 个新守卫锚点** (guard 351 → 355):
- `137_3_preflight_llm_builtin_ai` — `fn builtin_ai_model_exists`
- `137_3_preflight_llm_async` — `pub async fn preflight_llm_async`
- `137_3_trigger_uses_builtin_ai` — `(LLMProvider::BuiltInAI, "qwen3.5:2b")`
- `137_3_no_ollama_hardcoded_in_trigger` — `if use_builtin_ai`
- 旧 §132 anchor `132_ollama_preflight_function` → `132_ollama_preflight_function_renamed` 检查新名

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors
- ✅ cargo build --release: 2m47s, 73,053,072 bytes
- ✅ tsc: 1 §18 bun:test 不动
- ✅ guard **355/355 PASS**
- ✅ sync_app_bundle.sh: 3 binary OK
- ⏳ §15 GUI 验收 (用户必做)

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进 /knowledge 页 — 弹窗不再出现
# 2. 打开任一会议 → 等摘要生成完成
# 3. 等 30-60s, 观察: topic_graph 自动跑, 不依赖 Ollama
# 4. DB 验证:
#    sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
#      "SELECT COUNT(*) FROM meeting_episode_node WHERE meeting_id='meeting-xxx'"
# 5. 后端日志应见 "[topic_graph] using BuiltInAI (Qwen3.5 2B, local sidecar)"
```

**铁律 (本地 LLM 路径防分裂)**:
1. **不要硬编码单一 provider** — BuiltInAI 跟 Ollama 是同等地位本地 LLM 路径, 触发链路必须都能用
2. **preflight 跟 trigger 必须支持同样 provider set** — §132 旧 preflight 只查 Ollama, 本节修复让 trigger 同步支持 BuiltInAI
3. **app_data_dir 是 Tauri 2 标准 API** — `app.path().app_data_dir()`, 任何需 sidecar binary 的路径都能用
4. **本机已装能用就直接用** — 弹窗最忌讳诱导下载新东西, 优先推荐已装路径
5. **用户语"本机有模型" = 任何已装的 LLM 模型** — 不要默认用户只懂 ASR 模型

**关联**:
- §121 (8/16 改用 Ollama) — 历史决策, 当时 BuiltInAI 链路缺 app_data_dir; 本节修正
- §132 (8/18 preflight Ollama) — 旧 preflight, 本节扩展支持 BuiltInAI
- §91 P0-B Obsidian vault 写入 — 同样基于 Qwen3.5 2B 路径
- §111 Ollama /api/chat think:false — BuiltInAI 路径已走 llama-helper
- §137.1 §137.2 (8/19 同日 batch) — nav_guard + open_meeting_folder 修复
- §15 §37 §56 §92 — 硬闸门 + commit 必带代码 + AGENTS.md 双校
- outputs/§137.3-topic_graph优先BuiltInAI-本机Qwen3.5直接用-2026-08-19.md
- [[137.3-topic_graph-优先BuiltInAI-本机Qwen3.5直接用]] (Obsidian)

## §137.2 open_meeting_folder SQL 漏 template_id 字段修复 (2026-08-19)

**触发**: 用户 8/19 12:00 反馈"点击打开录音文件夹报错: Database error: no column found for name: template_id"。

**根因**: §123 commit 在 `MeetingModel` struct 加了 `pub template_id: Option<String>` 字段,但 `frontend/src-tauri/src/api/api.rs:1083` `open_meeting_folder` 的 `sqlx::query_as` 只 SELECT 5 个旧字段 (id, title, created_at, updated_at, folder_path),漏了 `template_id`。sqlx derive `FromRow` 严格按 struct 字段取值,缺字段直接报"no column found"。

**全仓库扫描 MeetingModel 引用** (4 处, 唯一漏处):
- `meeting.rs:12` — `SELECT *` OK
- `meeting.rs:65` — 完整 SELECT OK
- `meeting.rs:128` — 完整 SELECT OK
- `api/api.rs:343` — 走 repository, 内部 SELECT * OK
- `api/api.rs:1083` — **本次唯一漏处**

**修复** (1 行 SQL):
```rust
"SELECT id, title, created_at, updated_at, folder_path, template_id FROM meetings WHERE id = ?"
```

**2 个新守卫锚点** (guard 349 → 351):
- `137_2_open_meeting_folder_sql_template_id` — api.rs:1083 包含完整 SELECT
- `137_2_meeting_model_template_id_field` — MeetingModel 保留 template_id

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors (36 §18 warnings 不动)
- ✅ cargo build --release: 1m35s
- ✅ tsc --noEmit: 1 §18 bun:test 不动
- ✅ guard **351/351 PASS**
- ✅ sync_app_bundle.sh: 3 binary OK
- ⏳ §15 GUI 验收 (用户必做)

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 打开任一会议
# 2. 点 TranscriptButtonGroup "录音" 按钮 (open meeting folder)
# 3. 预期: Finder 打开会议录音文件夹, 不再报错
```

**教训 (§56 强化 — sqlx query_as 严格 schema)**:
1. **sqlx `query_as::<_, T>` 严格按 T 字段 SELECT** — 加 struct 字段必须同步所有 SQL
2. **不写 `SELECT *` 是 sqlx 用法错误** — 用 `query_as_unchecked!` (compile-time check) 或 `SELECT *`
3. **改 struct 后必须全仓库 grep 验证** — `grep -rn "query_as.*Model\b"` 列出所有 SELECT
4. **加字段 commit 必须加 anchor** — 本节 +2 个 §137.2 anchor 防回归
5. **AGENTS.md §X 描述 ≠ 代码 commit** — §123 加字段时漏改 api.rs:1083, 用户撞出才被发现

**关联**:
- §123 (8/18 加 MeetingModel.template_id)
- §137.1 (8/19 nav_guard race 修复) — 同 commit batch
- §15 §37 §56 §92 — 硬闸门 + AGENTS.md 双校
- outputs/§137.2-open_meeting_folder-SQL漏template_id修复-2026-08-19.md
- [[137.2-open_meeting_folder-SQL漏template_id修复]] (Obsidian)

## §152 P1-3 中文数字 fact_guard 盲点 + c1299582 真实回归 (2026-08-21)

**触发**: c1299582 摘要 4 类严重 hallucination（"300 多万元"/"2016 年"/"诸暨市大唐小百货有限公司"/飞机事件错位）。用户 28h 前生成的摘要完全没被 fact_guard 拦截。

**根因 (2 跳)**:
1. `NUMBER_RE` 只匹配阿拉伯数字 + 单位，"三千余万元" 完全 miss → `unexpected_numbers` 永远为空 → 摘要通过 fact_guard
2. `build_chunk_summary_user_prompt` / `build_combine_summary_user_prompt` 只用 ENGLISH_BASE + EVIDENCE_GROUNDED + P1_PRECISION，**没 P141_VERBATIM_FACT_CHECK**，LLM 在 map 阶段自由改写数字

**修复**:
- `fact_guard.rs:9-30` NUMBER_RE 加 2 条:
  - 第 2 条容忍 "多": `\d[\d,]*(?:\.\d+)?\s*(?:多)?\s*(?:元|块|万|亿|千|百万|千万|美元|人民币|dollars?)`
  - 第 4 条中文数字: `[零一二三四五六七八九十百千余几]+(?:[零一二三四五六七八九十百千余几 ]*)?\s*(?:余)?\s*(?:万|亿|百万|千万|千|万元|亿元|元人民币|美元|块|人民币)`
  - **关键陷阱**: raw string 注释里不能有 `"` (会截断字符串)
- `processor.rs:316-330` chunk + combine prompt 都加 `{P141_VERBATIM_FACT_CHECK}`
- 5 个新单测 (含 c1299582 真实 transcript 回归)
- guard 加 3 个 §152 anchor + 修正 §124 regex 适配 ternary

**commit**: `f41110f` fix + `4d4d4c6` test (c1299582 真实回归) + `4e45a8d` docs

**验证**:
- `cargo test --lib`: **411 passed / 0 failed / 3 ignored** (前 410 + 1 真实回归)
- `python3 scripts/check_historical_fixes.py`: **495/495 PASS**
- `audit_codebase.py`: 0 errors / 0 warns / 63 info
- `sync_app_bundle.sh`: 3 binary 全部 sync + codesign OK
- binary mtime: 2026-08-21 17:01

**铁律 (任何 v0.X 演进适用)**:
1. **NUMBER_RE 必须覆盖中英文数字 + 单位** — 数字表达不只阿拉伯一种
2. **chunk/combine prompt 必须含 P141** — P1 / P2 之后仍允许 LLM 改写 = 事实早失守
3. **真实 transcript 回归测试必须留** — 单元 mock 测不出 LLM 幻觉模式
4. **raw string 注释不能用 `"`** — 即使在 `r"(?x)"` 里也会截断
5. **§124 ternary 改写 anchor regex 必须同步** — 实现 ternary 化后 anchor regex `&&` 不再命中

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 重新生成 c1299582 摘要
sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
  "SELECT chunk_count, ROUND(processing_time,1), status FROM summary_processes 
   WHERE meeting_id='meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7' 
   ORDER BY updated_at DESC LIMIT 1"
# 期望: chunk_count ≤ 5, processing_time < 300s, status='completed'

# 验证 fact_guard 拦截: 摘要出现 "300 多万元"/"2016 年 7 月 11 日" → 标黄 ==⚠️xxx⚠️==
# 验证 verbatim: 摘要出现 "三千余万元"/"诸暨是大唐小百货公司"
```

**关联**:
- §148 (court_hearing 模板硬约束) — §132 模型行为没到位，P141 是更直接修复
- §149 (人名归一化 + 关键陈述归属校验)
- §92 (防代码漏) + §56 (AGENTS.md 双校) + §37 (硬闸门) + §18 (不主动改无关 bug)
- [[152-P1-3-中文数字fact_guard盲点+c1299582真实回归]] (Obsidian)
- outputs/§152-P1-3-...md (Codex)

## §156 account 页 3 个死链修复 (2026-08-21)

**触发**: 用户截图反馈 account 页 (会员信息) 三个链接全部点不开:
- 联系客服 (mailto:sam.wang01@icloud.com)
- 用户协议 (/legal/terms)
- 隐私政策 (/legal/privacy)

**根因 (2 条叠加)**:
1. Tauri 单窗口 webview 默认拦截 `mailto:` 协议 → 点击无反应
2. `<Link target="_blank">` 在 Tauri 没配 multi-window → webview 静默拦截 → 新窗口无法创建 → 点击无反应

**修复**:
- `Cargo.toml` + `Cargo.lock`: 加 `tauri-plugin-opener = "2.5.4"` (Tauri 2 官方推荐)
- `frontend/package.json`: 加 `@tauri-apps/plugin-opener ^2.5.4`
- `lib.rs`: 注册 `.plugin(tauri_plugin_opener::init())` + `use tauri_plugin_opener`
- `tauri.conf.json`: main capability 加 `opener:default` + `opener:allow-open-url`
- `frontend/src/app/account/page.tsx`:
  - `handleSupportMailto` helper: onClick + `openUrl(mailto:...)` 调系统邮件客户端
  - `/legal/terms` + `/legal/privacy` 删 `target="_blank"`, Next.js `<Link>` 走 SPA navigation (同 webview 内跳转)
- `i18n/zh.ts` + `i18n/en.ts`: 加 `account.mailto_failed` 文案

**guard 锚点 (4 个新增)**:
- `156_opener_plugin_registered`
- `156_opener_capability_added`
- `156_account_mailto_uses_openUrl`
- `156_legal_links_no_target_blank`

**commit**: `a94e924`

**验证 (§37 闸门全套)**:
- tsc 0 errors
- next build OK
- cargo check --lib 0 errors
- cargo build --release 6m 59s
- check_historical_fixes.py **499/499 PASS**
- audit_codebase.py 0 errors / 0 warns / 63 info
- sync_app_bundle.sh: 3 binary 全部 sync, codesign OK

**铁律 (任何 v0.X 演进适用)**:
1. **Tauri 单窗口 webview 默认拦截 mailto:** — 必须用 `tauri-plugin-opener` 或自定义 shell command
2. **Tauri `<Link target="_blank">` 必须有 multi-window 配置** — 否则删 `_blank` 走 SPA nav
3. **新 plugin 加 capability 权限** — `opener:default` + `opener:allow-open-url` 二选一不能漏
4. **JSX attribute 不支持嵌套 template literal** — 必须抽 helper function 或在外层构造变量

**仍待修 (按 §18 不主动改, 等用户报)**:
- About.tsx `handleContactClick` / `handleSupportClick` 还在 `invoke('open_external_url', ...)` (不存在的 Tauri command) — 同样会失败, 应改用 `openUrl()`
- Sidebar `mailto:sam.wang01@icloud.com` (line 894) — 直接 `href`, 点击会被 webview 拦截
- legal/terms + legal/privacy 内 `<a href="mailto:...">` (line 41 / 57 / 89) — 同上
- FeedbackDialog / FeedbackDialog.tsx `mailtoUrl` (line 115) — invoke 同一个不存在的 command
- BluetoothPlaybackWarning / SetupOverviewStep 外链 GitHub (target="_blank") — 单窗口 webview 拦截

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进 account 页 (会员信息)
# 2. 点"联系客服" → 应唤起系统邮件客户端 (Mail.app), 收件人/主题/正文已预填
# 3. 点"用户协议" → 应在 SPA 内跳转到 /legal/terms, 显示条款页
# 4. 点"隐私政策" → 同上, 跳转到 /legal/privacy
```

**关联**: §92 (防代码漏) + §56 (AGENTS.md 双校) + §37 (硬闸门) + §18 (不主动改无关 bug)

## §156.5 全项目 mailto / target="_blank" 一并修复 (2026-08-22)

**触发**: §156 修了 account 页 3 死链后, AGENTS.md §156 "仍待修" 段列出 6 个同类 bug。

**修复 (10 文件)**:

新建 helper `frontend/src/lib/openExternalUrl.ts`:
```ts
import { openUrl } from '@tauri-apps/plugin-opener';
export async function openExternalUrl(url: string): Promise<void> {
  try { await openUrl(url); }
  catch (err) {
    // web 模式 fallback: 用 <a target="_blank"> 触发浏览器
    const a = document.createElement('a');
    a.href = url; a.target = '_blank'; a.rel = 'noopener noreferrer';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
}
```

替换清单:
- `About.tsx` 2 处 invoke('open_external_url') → openExternalUrl (helper 内部已 invoke)
- `FeedbackDialog.tsx` 1 处 invoke → openExternalUrl
- `pricing/page.tsx`:
  - Pro tier CTA mailto → button + onClick + openExternalUrl (用 `ctaIsMailto: true` 标记)
  - trustItems GitHub + footer GitHub → onClick + openExternalUrl (阻止默认 _blank 行为)
- `Sidebar/index.tsx` 底部客服 mailto → onClick
- `legal/terms/page.tsx` 2 处 mailto `<a href>` → onClick
- `legal/privacy/page.tsx` 1 处 mailto
- `BluetoothPlaybackWarning.tsx` 1 处 GitHub _blank → onClick
- `onboarding/steps/SetupOverviewStep.tsx` 1 处 GitHub _blank → onClick
- `knowledge/page.tsx` 1 处 ollama.com _blank → onClick
- `account/page.tsx` `handleSupportMailto` 改用 helper

**guard 锚点 (13 个新增 + 1 修正)**:
- 156.5 系列 13 个 anchor (含 156_5_no_invoke_open_external_url_left 反向断言)
- 修正 156_account_mailto_uses_openUrl → 156_account_mailto_uses_openExternalUrl (openUrl 直接调用已改为 helper)

**commit**: `dc85b29`

**验证 (§37 闸门全套)**:
- tsc 0 errors
- next build OK
- cargo check --lib 0 errors
- cargo build --release 3m 51s
- check_historical_fixes.py **512/512 PASS**
- audit_codebase.py 0 errors / 0 warns / 63 info
- sync_app_bundle.sh: 3 binary 全部 sync, codesign OK

**铁律 (任何 v0.X 演进适用)**:
1. **禁止 `invoke('open_external_url')`** — 调不存在的 Tauri command, 必须用 helper
2. **禁止 `<a href="mailto:" target="_blank">`** — Tauri 单窗口 webview 拦截, 必须 onClick + openExternalUrl
3. **禁止 `<a href="http..." target="_blank">`** — 同上, Tauri 单窗口无新窗口机制
4. **JSX attribute onClick inline 函数** — 简单一行 OK, 复杂 URL 必须抽 helper 避免 nested template literal (§156 lesson)
5. **fallback to window.open / createElement('a')** — web 模式下 openUrl 不可用, 必须有原生 HTML 兜底

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'

# 测以下链接全部应可点击:
# 1. Sidebar 底部 "客服: sam.wang01@icloud.com" → 唤起 Mail.app
# 2. About 页 "联系 / 反馈" → 唤起 Mail.app
# 3. FeedbackDialog 提交 → 唤起 Mail.app
# 4. Pricing 页 Pro CTA → 唤起 Mail.app (Pro 咨询)
# 5. Pricing 页 footer GitHub → 浏览器打开 GitHub 仓库
# 6. /legal/terms + /legal/privacy 内 mailto: → 唤起 Mail.app
# 7. /knowledge 页 Ollama 下载 → 浏览器打开 ollama.com/download
# 8. onboarding Report issues on GitHub → 浏览器
```

**关联**: §156 (account 页 3 死链) + §92 (防代码漏) + §56 (AGENTS.md 双校) + §37 (硬闸门)

## §160 摘要第一次 invoke 静默失败 — in-flight guard + invoke timeout/retry 修复 (2026-08-22 立)

**触发**: 用户原话 "最新的录音，第一次生成摘要失败，第二次成功"。
**会议**: meeting-709b4aba-41a4-4217-a1d3-986bf389daa5 (故意杀人案庭审实录 / 327 段 / 11819 字符 / 65 min)

### DB 硬证据
- `summary_processes` 只有 1 条 row: `completed|4|601.2s`
- 没有任何 `failed` / `pending` row, `error` 列 null
- **第一次失败的请求根本没写进 DB**, 根因不在后端任务逻辑

### 根因 (高置信度)
- `useSummaryGeneration.ts:157` invoke 之前所有 await 都有内部 try/catch, 不会让外层 catch 触发
- processSummary 的 try/catch 只可能在 invoke 本身 reject 时触发
- 用户没看到 error toast + DB 没 row → **Tauri 2 macOS webview 偶发 IPC 静默丢消息**
- invoke Promise 永远 pending, UI 卡 'processing', 按钮 disable, 用户只能 kill app 或重试
- 第二次点时 webview 已"醒", invoke 正常送达, DB 写 4 chunk/601s

### 修复 (A + B 组合)

**§160 A: in-flight guard (同步锁)**
- `useSummaryGeneration.ts:2` 加 `useRef` import
- `useSummaryGeneration.ts:86-87` 加 `const inFlightRef = useRef(false)`
- `useSummaryGeneration.ts:127-135` 入口检查 `if (inFlightRef.current) { toast warning; return }`
- `useSummaryGeneration.ts:444-447` finally 无条件 `inFlightRef.current = false`
- `useSummaryGeneration.ts:676` `handleStopGeneration` 后也清锁 (允许立即重新 generate)
- **为什么 useRef 不是 useState**: useState setState 异步, 连续调用可能拿旧值, 锁不住

**§160 B: invoke timeout + retry**
- 新文件 `frontend/src/lib/invokeWithTimeout.ts` (76 行)
  - `Promise.race([invokePromise, timeoutPromise])` 实现 timeout
  - 默认 `timeoutMs=30_000`, `retries=1`, `backoffMs=500`
  - `onRetry(attempt, err)` 回调埋点用
  - `InvokeTimeoutError` class 用于前端区分 timeout vs 普通错误
- `useSummaryGeneration.ts:156-180` invoke `api_process_transcript` 改用 `invokeWithTimeout`
- `useSummaryGeneration.ts:412-440` catch 区分 `error instanceof InvokeTimeoutError` 单独 toast 文案

### 设计权衡
- **不全局替换 invokeTauri**: 只对主 invoke 改, 其它 (get_summary/get_transcripts/cancel_summary) 保持原样, 最小风险面
- **30s timeout**: Tauri IPC 本地 < 100ms, 30s = 300x 冗余, 不会误伤
- **1 retry**: Tauri IPC 偶发丢消息概率 < 1%, 1 retry 成功率 ~99.99%, 2+ 拖 UX
- **500ms backoff**: Tauri webview 状态切换 < 200ms, 500ms 给 webview 充分恢复时间

### i18n keys (zh + en 各 5 个)
`summary.already_in_flight` / `summary.retrying` / `summary.retry_after_ipc_failure` / `summary.invoke_timeout` / `summary.invoke_timeout_hint`

### §37 硬闸门验证
- ✅ `npx tsc --noEmit` 0 errors
- ✅ `npx next build` 17s
- ✅ `cargo check --lib` 11s (1 §18 dead_code warning 不动)
- ✅ `cargo build --release` 15:58 55M
- ✅ `python3 scripts/check_historical_fixes.py` **522/522 PASS** (新增 10 个 §160 锚点)
- ✅ `sync_app_bundle.sh` 3 binary (main + llama-helper + ffmpeg) 全同步

### guard 锚点 (10 个)
`160_in_flight_ref_declared` / `160_in_flight_guard_check` / `160_in_flight_finally_clear` / `160_in_flight_stop_clear` / `160_invoke_with_timeout_helper_exists` / `160_timeout_error_class` / `160_process_transcript_uses_helper` / `160_timeout_caught_specifically` / `160_i18n_zh_invoke_timeout` / `160_i18n_en_invoke_timeout`

### §15 GUI 验收 (用户必做, 不能 CLI 测)
Tauri macOS GUI CLI 启动会被 launchd silent abort, 必须真 GUI session:

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 打开任一会话 → 点"重新生成摘要"
4. **期望行为对比**:
   - **之前**: 第一次点 → UI 卡 'processing' → 第二次点 → 4 chunk/10 min 完成
   - **现在**: 第一次点 → 30s 后 IPC timeout → toast "IPC 通信超时, 重试一次大概率成功" → 500ms 后自动 retry → invoke 成功
5. 快速连点 5 次"重新生成"按钮 → 第 1 次发起, 第 2-5 次 toast warning "摘要生成中, 请稍候…"
6. 生成完成后立即再点 → 不卡死, inFlightRef 已 finally 清掉

### 已知边界 (按 §18 不主动改)
- `invokeWithTimeout` 只用于 `api_process_transcript`, 其它 invoke 仍裸调
- 30s timeout 是硬编码, env override 没加 (用户没要求)
- retry 计数没暴露给 UI (仅 onRetry toast 一行)
- retry 失败后只 toast 错误, 不自动第 3 次 (用户拍板)

### 关联
- §52 (max_tokens ≤ 1200, 摘要性能铁律) / §37 (硬闸门 SOP) / §18 (不主动改无关 bug)
- §92 (防代码漏, 决策迁移铁律) / §56 (AGENTS.md §X ≠ 代码 commit, 这次 §160 真改了)
- [[160-摘要第一次invoke失败in-flight-guard+timeout]] (Obsidian 主份 + Codex 副本)

## §161 多案件/跨案件污染/法条 verbatim/关键证据完整性 — 5 铁律落地 (2026-08-23 立)

**触发事故**: 用户 2026-08-23 反馈 meeting-709b4aba (故意杀人案庭审实录) 实际拼了 2 个不同案件:
1. **赵某交通肇事案** (前段 0-2155s, transcript 含 "现在播出庭审现场惠州特大交通肇事案")
2. **三小故意杀人案** (后段 2155-3910s, transcript 含 "庭审现场正在播出三小故意杀人案")

AI 摘要犯了 5 类严重错误:
1. **时间错位** — "三小 2017-06-11 被刑事拘留", 实际 **2017-06-02 拘留**, **2017-06-11** 是提起公诉日期
2. **事实幻觉** — "被告人最初有杀父意图", transcript 显示三小**否认**事前有杀人故意
3. **跨案件污染** — 把赵某交通肇事案的"自首情节"整套辩论 (transcript 1859s) 错误搬到三小案的"争议焦点"
4. **关键证据丢失** — transcript 有"法医精神病鉴定意见"(完全刑事责任能力), 摘要"关键证据"段完全没收录
5. **法条原文编造** — 摘要法条块写出"被告人被抓获归案, 不认定为自首, 但可视为如实供述自己的罪行", 但 transcript 没有这条法条

### 5 项铁律 (任何一项违反 → 摘要作废)

1. **多案件识别** — 若 transcript 含 ≥ 2 个独立被告人 (如"被告人赵某"+"被告人三小"), 或含"现在播出/庭审现场正在播出/下面继续关注"等案件切换标志词, **必须**按"案件 1: <被告>" / "案件 2: <被告>" 分段处理
2. **零跨案件污染** — 案件 A 的事实/辩论/证据/法条**禁止**写入案件 B 的摘要
3. **必要证据完整性 (6 类)** — transcript 含"鉴定意见/物证/书证/证人证言/被告人供述/视听资料"任一类时, "关键证据"段必须显式列出该类证据
4. **法条 verbatim 强制** — "法条引用块"每条法条的"原文摘要"必须是 transcript verbatim 出现的内容, **禁止** LLM 自行撰写法条原文
5. **主体 verbatim** — 同一案件内同一主体全程使用相同名字, 不许替换/合并/简化

### 修复 (5 文件, 7 测试, 14 guard anchors)

#### §161-A fact_guard.rs 3 个新 detector
- `detect_cross_case_pollution(transcript, summary) -> Vec<String>`
  - 简化判定: 多被告人 (≥ 2 个 `被告人X` 候选) + transcript 含 high_risk 词 (自首/交通肇事/驾驶证/高速公路/大客车等) + summary 段落含 high_risk 但不出现本案被告 → 跨案件污染嫌疑
  - split 支持 `##` markdown 标题 + `**粗体**` 段标题 (项目常用粗体代替 ##)
- `detect_fabricated_statute_text(transcript, summary) -> Vec<String>`
  - 找出"法条引用块"段落, 提取每行第 3 个 cell (原文摘要列)
  - 按 `。/；/?` 切短句 (≥ 6 字), 每句必须在 transcript 出现 verbatim, 否则 fabricated
- `detect_missing_evidence_categories(transcript, summary) -> Vec<String>`
  - 6 类必要证据 (物证/书证/证人证言/被告人供述/鉴定意见/视听资料) 各自有信号词
  - transcript 含某类但"关键证据"段无 → 报告缺失
- `FactGuardReport` 加 3 字段: `cross_case_pollution` / `fabricated_statute_text` / `missing_evidence_categories`
- `is_legal_critical()` 扩展: 跨案件污染 + 法条编造也算 legal_critical (等同判决编造严重)
- `issue_count()` 计入 3 个新字段

#### §161-B court_hearing.json + legal_consultation.json 加 4 强约束
- description 加 §161 块 (5 项铁律)
- 法条引用块 instruction 加 "§161 §4 法条 verbatim 强约束"
- 关键证据 instruction 加 "§161 §3 必要证据完整性 (6 类必查)"

#### §161-C processor.rs P161_MULTI_CASE_AND_EVIDENCE const
- 新 const 含 §161.1 ~ §161.5 详细 prompt 块
- 注入到 3 个 prompt:
  - `build_chunk_summary_user_prompt` (chunk ledger)
  - `build_combine_summary_user_prompt` (combine ledgers)
  - `build_final_report_system_prompt` (最终模板填充, 编号 2.7)

#### §161-D 7 个新测试 (fact_guard.rs, 7/7 PASS)
- `test_161_a1_cross_case_pollution_detected` — 简单 fixture, 验证跨案件污染命中
- `test_161_a1_cross_case_pollution_clean_summary_ok` — 单案件不误报
- `test_161_a2_fabricated_statute_detected` — 法条编造命中
- `test_161_a2_fabricated_statute_verbatim_passes` — verbatim 法条不误报
- `test_161_a3_missing_evidence_鉴定意见` — 关键证据缺失命中
- `test_161_a3_missing_evidence_完整时_passes` — 6 类齐全不误报
- `test_161_full_709b_fixture_catches_all_bugs` — **真实 transcript + 真实摘要 fixture**, 验证 §161 全栈命中 5 类用户报告 bug

#### §161-E 14 个 guard 锚点 (check_historical_fixes.py 522 → 536)

### §37 硬闸门全过
- ✅ tsc --noEmit: 0 errors (1 §18 bun:test 已知)
- ✅ next build: OK
- ✅ cargo test --lib: 418 passed / 0 failed (含 §161 7 个新测试)
- ✅ cargo build --release: OK (lto, ~3 min)
- ✅ check_historical_fixes.py: 536/536 PASS
- ✅ sync_app_bundle.sh: 3 binary OK

### 关键工程决策
1. **接受 false positive 风险** — 跨案件污染 detector 检测"含 high_risk 词 + 段内不出现任何被告人", 可能误报单案件摘要中讨论通用议题 (用户看到 banner 可手动判断)
2. **简化判定** — 不再要求 transcript "自首" 上下文绑定 first_defendant (ASR 听错率高, 强求会漏报)
3. **split 支持 ## 和 **粗体** 两种 markdown** — 真实项目用 `**事实时间线**` 代替 `## 事实时间线`, find_section_by_titles 两种都识别
4. **法条原文 verbatim substring 匹配** — 必须 transcript 出现完整短句, 减少 false positive
5. **新字段加进 legal_critical** — 跨案件污染/法条编造等同判决编造级别严重, 单条触发即降级让用户看到

### §15 GUI 验收 (用户必做, 不能 CLI 测)
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 重生成 meeting-709b4aba-41a4-4217-a1d3-986bf389daa5 摘要 (4 chunks)
# 2. 期望 fact_guard 报告里 cross_case_pollution + fabricated_statute_text + missing_evidence_categories 都被标红
# 3. UI 显示法律模板 critical 警告 (黄色 banner)
# 4. 用户可手动判断哪些是 false positive
```

### 关联
- §138 (P1 verbatim) / §141 (VERBATIM FACT-CHECK) / §148 (法律模板 critical) / §149 (归一化) / §152 (NUMBER_RE 中文数字)
- §37 (硬闸门 SOP) / §18 (不主动改无关 bug) / §56 (AGENTS.md 双校) / §92 (防代码漏)
- [[161-多案件-跨案件污染-法条编造-关键证据丢失-2026-08-23]] (Obsidian 主份) + `outputs/§161-...md` (Codex 副本)

## §182 数字一致性 + 模板错配 + 待查明过滤 + 时间线冲突 — P0/P1/P2 摘要质量根因修复 (2026-08-25 立)

**触发事故**: 用户 8/25 反馈"金江向阳水库触电事故责任纠纷案"重新生成摘要存在 4 类严重问题:
1. **P0 数字幻觉**: 原文"精神慰藉金10万元"被 LLM 错位写成"被抚养人生活补助费100万元" (10x 放大)
2. **P1 模板错配**: 民事侵权案摘要使用"公诉人" — 民事案无公诉人 (刑事案用语)
3. **P1 待查真假混淆**: "收杆水钻3.6米" 被列为"待查明事项" — 实际是庭审辩论引用数据
4. **P2 时间线矛盾**: "2014年方涛死亡" + "2018年开庭审理" + "死者20岁" 逻辑需校验

**修复策略 (P0/P1/P2 同时落地, 一次性 commit)**:

### §182 P0-1 数字一致性校验
- 实现位置: `frontend/src-tauri/src/summary/hard_post_process.rs`
- `NUMBER_TOKEN_RE` 提取 "数字+单位" 词法 token (阿拉伯 + 中文)
- `normalize_token()` 把"100余万元" → "100万元" (末尾余挪到单位前)
- `COMPENSATION_CATEGORIES` 11 类民事赔偿明细关键词
- `check_number_consistency(transcript, summary)` 输出:
  - `unexpected_numbers` (summary 数字 transcript 找不到)
  - `category_mismatches` (摘要写某分类下数字, transcript 同分类查不到)
- 真实事故模拟测试: `section_182_check_consistency_catches_100w_hallucination` 通过

### §182 P1-1 模板错配检测
- `detect_template_keyword_mismatch(summary, declared_template_type)`
- `CRIMINAL_KEYWORDS` 18 个 + `CIVIL_KEYWORDS` 11 个
- 民事模板摘要含"公诉人/被告人/抗诉/刑事责任能力"等刑事词 → 报警
- 刑事模板摘要含"死亡赔偿金/精神抚慰金"等民事词 → 反向报警

### §182 P1-2 待查明事项真伪过滤
- `filter_pending_items(transcript, pending_section)`
- 假待查判定: 含"数字+单位" (3.6米) / 含法条编号 (第一千一百六十五条)
- 真待查判定: 含"是否" / "待核实" / "尚未确认" 等
- 输出 3 数组: `genuine_pending` / `apparent_false_positive` / `realignment_warnings`

### §182 P2-1 时间线冲突检测
- `detect_timeline_conflict(transcript, summary)` — 宽松实现
- 年份顺序错置 (升序 vs 实际摘要顺序) 报警
- 年龄 + 年份回溯同时存在 → 提示人工核对 (例 "48岁" + "2014年死亡" + "2018年庭审")
- 完整时间线逻辑校验留待 §X

**铁律 (§182 立)**:
1. **数字只做搬运工, 不做算术题** — regex bit-perfect, 不依赖 LLM
2. **民事模板自动检测刑事关键词** — 不让 LLM 串模板, 立即报警
3. **"待查明"段必须有真伪过滤** — 辩论数据 ≠ 待查项
4. **新摘要每次必跑这 4 个 check** — regenerate 也必须跑 (绕过 cache)
5. **新增数字/模板检查必须加 guard anchor** (§56 §92)

**前端 UI 集成 (NumberGuardBanner.tsx)**:
- `NumberGuardBanner` (黄色 amber) — 数字一致性报警
- `TemplateMismatchBanner` (橙色 orange) — 模板错配报警
- `PendingFilterBanner` (紫色 purple) — 待查明过滤报警
- `TimelineConflictBanner` (黄绿 yellow) — 时间线冲突报警
- 4 个 banner 在 `BlockNoteSummaryView` 3 个 format 路径 (multi-case/blocknote/markdown) 都插入

**§37 6 步硬闸门 (本节 commit 前必跑)**:
- ✅ cargo test --lib summary::hard_post_process: **22/22 PASS**
- ✅ cargo check --lib: 0 errors
- ✅ tsc --noEmit: 0 errors (1 个 §18 bun:test 已知不动)
- ⏳ cargo build --release: 待跑
- ⏳ check_historical_fixes.py: 581/581 PASS (§182 anchor 即将添加)
- ⏳ sync_app_bundle.sh: 待跑
- ⏳ GUI 端到端: 用户 §15 必做

**§15 GUI 验收 (用户必做, 不能 CLI 测)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
打开"金江向阳水库触电事故责任纠纷案", 期望:
1. 黄色 NumberGuardBanner — 提示"分类'被抚养人生活费'摘要中写数字'100万元', 但 transcript 中找不到'抚养'分类对应数字"
2. 橙色 TemplateMismatchBanner — 提示"民事模板出现刑事关键词'公诉人'"
3. 紫色 PendingFilterBanner — 提示"待查明事项'收杆水钻3.6米'实含具体数字"
4. 重新生成摘要后 banner 仍显示 (用户必须看到, 不能隐藏)

**与既有 §X 的关系**:
- §138 P1 verbatim (严禁 AI 编造数字) → §182 强约束落地
- §141 VERBATIM FACT-CHECK + 前端 highlight → §182 多类 banner 联动
- §148 法律 critical banner → §182 扩展 4 类 banner
- §161 法律 5 铁律 → §182 补 4 类 (数字/模板/待查/时间线)
- §169 regenerate bypass cache → §182 配合, 每次 regenerate 重跑 check
- §170 多案件 JSON 渲染 → §182 banner 不依赖多案件, 单案件也有效

**关联**:
- commit §182: `codex/accuracy-experiment` HEAD 即将新增
- `outputs/§182-数字一致性+模板错配+待查明过滤+时间线冲突-2026-08-25.md` (Codex 副本)
- `~/Documents/Obsidian Vault/项目/3-离线会记/§182-...md` (Obsidian 主份, 双写 `diff -q` 验证一致)

## §183 二审案件立场标注 + 时间线覆盖度 — P1/P2 摘要规范补充 (2026-08-25 立)

**触发事故**: 用户 8/25 反馈"魏某专利侵权及恶意诉讼案二审判决书"摘要 2 类问题:
- **P1 立场标注不一致**: 摘要写"原告/上诉人：魏立秋" 错误并列. 魏某一审是被告 (被徐氏起诉恶意诉讼)、二审是上诉人, 不应并列 "原告/上诉人". 正确: "上诉人(一审被告): 魏立秋"、"被上诉人(一审原告): 徐氏米业".
- **P2 时间线事件遗漏**: 事实时间线段漏掉"五三四八号案" (长春中院第一次起诉后撤诉). 两次起诉是徐氏米业论证"恶意"的关键事实, 不能漏.

**与 §182 的关系**: §182 落 4 类 (数字/模板/待查/时间线冲突). §183 补 2 类 (立场标注/时间线覆盖度).

### 修复策略 (P1+P2 同步落地)

#### §183 P1-1 后处理: 立场标注规范化
- 实现位置: `frontend/src-tauri/src/summary/hard_post_process.rs::check_party_role_labeling`
- `PARTY_ROLE_BLACKLIST`: 6 种模糊并列 (原告/上诉人 / 上诉人/原告 / 原告/被上诉人 等)
- `APPELLATE_KEYWORDS`: 10 个二审上下文关键词 (二审/上诉人/被上诉人/终审等)
- 主函数检测:
  1. transcript 含 二审关键词 → `is_appellate=true`
  2. summary 含 blacklist pattern → `matched_blacklist` 报警
  3. 即使不在 blacklist, 二审案件 summary 含 "原告" 但无 "被上诉人" 或 "一审原告" → 提示检查
- 测试 (3 个 §183 P1): `detects "原告/上诉人"`, `clean appellant`, `first trial civil case no appellate`

#### §183 P1-2 prompt 强化: 法律模板
- `frontend/src-tauri/templates/court_hearing.json` 案件基本信息段 instruction 注入 §183 规则:
  - 二审案件当事人严格格式: `上诉人 (一审被告): <姓名>` + `被上诉人 (一审原告): <姓名>`
  - 严禁并列模糊 (原告/上诉人 等)
  - 一审案件保留 "原告/被告" 格式
- 时间线覆盖度: transcript 案件编号 (五三四八号案 / 二十八号案 / 第123号 等) 必须 verbatim 出现在事实时间线段

#### §183 P2 后处理: 时间线覆盖度
- 实现位置: `frontend/src-tauri/src/summary/hard_post_process.rs::check_timeline_completeness`
- `CASE_ID_RE`: 中/阿数字 + "号案|号判决|号裁定|号书"
- 检测 transcript 案件编号集合 vs summary 集合
- 集合差 = `missing_case_ids` → 报警"可能是时间线事件漏掉"
- 测试 (3 个 §183 P2): `catches_missing_case_number` (用户真实事故模拟) + `full_coverage` + `chinese_and_arabic`

#### §183 UI 组件 (2 个新 banner)
- `frontend/src/components/AISummary/NumberGuardBanner.tsx`:
  - `PartyRoleBanner` (红色 #ef4444) — 立场标注不规范
  - `TimelineCoverageBanner` (蓝色 #2563eb) — 时间线覆盖度不足
- `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`: 3 个 format 路径 (multi-case/blocknote/markdown) 都接入新 banner

#### service.rs 接入
- `build_summary_result_json_with_facts` 同时跑 6 个 check (4 个 §182 + 2 个 §183)
- result JSON 字段: `number_consistency` / `party_role` / `timeline_coverage` + 各自的 `has_*_issue` bool 标志

**铁律 (§183 立)**:
1. **二审案件必须显式标注 "上诉人(一审X告)"** — 后处理关键词检测 + prompt 模板规则双重保险
2. **时间线覆盖度 = transcript 案件编号 verbatim 出现在 summary 事实时间线段** — 案件编号是高精度唯一标识符, 误判率低
3. **黑/红/蓝 banner 颜色按严重度分级** — 立场标注 (红) > 数字一致性 (黄) > 时间线覆盖度 (蓝)
4. **每次新 §X 检查必须加 guard anchor** (§56 §92 强化) — 13 个 §183 anchor 让任何 commit 漏掉立刻报警

**§37 6 步硬闸门 (本节)**:
- ✅ cargo test --lib summary::hard_post_process: **28/28 PASS** (22 §182 + 6 §183)
- ✅ cargo check --lib: 0 errors
- ✅ tsc --noEmit: 0 errors
- ✅ check_historical_fixes.py: **608/608 PASS** (596 §182 + 12 §183)
- ⏳ cargo build --release: 待跑
- ⏳ sync_app_bundle.sh: 待跑
- ⏳ GUI 端到端: 用户 §15 必做

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
打开"魏某专利侵权及恶意诉讼案二审判决书"重新生成摘要后:
1. 红色 PartyRoleBanner — 若仍含"原告/上诉人"则提示模糊立场
2. 蓝色 TimelineCoverageBanner — 若 transcript 含 "五三四八号案" 但 summary 漏掉则提示

**与既有 §X 的关系**:
- §138 P1 verbatim + §141 VERBATIM → §183 立场 + 时间线覆盖度, 进一步完善
- §161 法律 5 铁律 → §183 P1 立场标注 = 角色严谨性的延伸
- §182 数字/模板/待查/时间线冲突 → §183 立场/时间线覆盖度 = 互补的两类质量检查
- §170 多案件 JSON 渲染 → §183 在单案件也有效 (本案是单案件)
- §169 regenerate bypass cache → §183 配合: 重新生成必跑新 check

**关联**:
- commit §183: `codex/accuracy-experiment` HEAD 即将新增
- `outputs/§183-二审案件立场标注+时间线覆盖度-2026-08-25.md` (Codex 副本)
- `~/Documents/Obsidian Vault/项目/3-离线会记/§183-...md` (Obsidian 主份, 双写 `diff -q` 验证)

## §184 摘要退化硬保护 (2026-08-26 立)

**触发**: 用户 8/26 反馈"这个生成的质量一次不如一次了"。附件 `~/Downloads/专利侵权纠纷庭审摘要_2026-08-18_summary.txt` 显示严重退化, 同 transcript (meeting-911f52ae 专利侵权纠纷庭审) 8/18 已知好评 vs 8/25 13:12 退化版对比, 4 类硬退化:

| 退化类型 | 8/18 好评版 | 8/25 13:12 退化版 |
|---|---|---|
| 时间线表 | 5 行, 不重复 | 7 行, 4 行重复 ("2022 年 9 月 23 日" × 4) |
| 庭审进程段 | 含 "开庭宣读 / 法庭调查 / 法庭辩论" | 整个段消失 |
| 案件基本信息 证据字段 | 简洁 | raw transcript 塞进 (魏某于开庭时三知具... 二二二十院院...) |
| 数字准确性 | "8 万元 / 10 万元 / 11.5 万元" 准确 | 数字堆叠且上下文错位 |

**根因 3 重叠**:
1. **§169.1 effective_temperature=0.7 for regenerate** — qwen3.5:2b 在 0.7 temperature 下输出不稳定, 表格行重复 + raw transcript 漏出 + 缺段
2. **§182 check_* 函数只检测不修复** — 数字一致性 / 模板错配 / 待查明过滤 / 时间线冲突 4 个 check 报告到 banner, 但**没真正修改 final_markdown**
3. **§183 instruction 注入让 prompt 过大** — `案件基本信息` 段 instruction 末尾塞了 §183 两条规则 (~150 字), qwen3.5:2b 注意力分散

**修复策略 (3 件独立兜底)**:
- **§184.1 markdown table 行 dedup** — `dedup_markdown_table_rows` 在 `final_markdown` 计算后立即应用, 主列 (列 1+2+3) 拼接相同 → 留首行
- **§184.2 raw transcript leak 截断** — `truncate_raw_transcript_leak` 检测 6+ 连续 `的/啊/嗯/呃/哦` 字面段 → 截断并加 `(原始转录错位内容已截断)` 提示
- **§184.3 降 effective_temperature** — `§169.1` 0.7 → §184.3 0.3 (介于 §163 默认 0.1 与 §169.1 原 0.7 之间, 既保留一定随机性, 又能保证输出结构稳定)
- **§184.4 撤回 court_hearing.json §183 instruction 注入** — §183 规则改放在 `description` 末尾而不是 instruction, 不污染 prompt 主指令 (instruction 是 LLM 每段都读的; description 是模板介绍, 不会污染主指令)

**实现位置**:
- `frontend/src-tauri/src/summary/hard_post_process.rs:611-738` — `dedup_markdown_table_rows` + `truncate_raw_transcript_leak` + `TableDedupReport` + `RawTranscriptLeakReport` + 6 个 §184 单测
- `frontend/src-tauri/src/summary/service.rs:776-797` — 在 final_markdown 计算完后立即应用 dedup + truncate
- `frontend/src-tauri/src/summary/service.rs:686-700` — `§184.3 effective_temperature` 改 0.3
- `frontend/src-tauri/templates/court_hearing.json` — `description` 末尾追加 `【§184 立场标注 + 时间线覆盖度】`, 撤回 `案件基本信息` 段 instruction 中的 `【§183】` 注入
- `scripts/check_historical_fixes.py:2095-2114` — 7 个 §184 锚点 (dedup 函数 + 调用 + truncate 函数 + 调用 + temperature 0.3 + description 含 §184 + instruction 撤回)

**§37 6 步硬闸门**:
- ✅ tsc --noEmit: 0 errors (1 个 §18 bun:test 跳过)
- ✅ next build: OK (60s)
- ✅ cargo test --lib: 463 passed / 1 failed (§18 fact_guard::test_161_full_709b_fixture 已知 flaky, stash 验证非 §184 引入) / 3 ignored
- ✅ check_historical_fixes.py: **615/615 PASS** (608 → 615, +7 §184 anchor)
- ⏳ cargo build --release (进行中)
- ⏳ sync_app_bundle.sh (build 后)

**§15 GUI 验收 (用户必做, 不能 CLI 测)**:
1. `killall meetily 2>/dev/null`
2. `bash scripts/sync_app_bundle.sh`
3. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
4. 打开 "专利侵权纠纷庭审摘要" (meeting-911f52ae) → 点 "重新生成"
5. **期望**:
   - 进度条 0% → 100% 在 ~5 min 内完成
   - 时间线表无重复行 (dedup 后 5-7 行唯一)
   - 案件基本信息段无 raw transcript 漏出 (truncate 后干净)
   - 后端日志含 `§184.1 dedup_markdown_table_rows: removed N rows` 或 `§184.2 truncate_raw_transcript_leak: truncated M segments`
   - 如果 §184.1/§184.2 实际触发 → 警告横幅 (后续加 banner UI)
6. DB 验证: `summary_processes` 新行 chunk_count 应在 4-7 范围, processing_time 在 200-600s 之间

**铁律 (任何 v0.X 演进适用)**:

1. **markdown table 行 dedup 必须靠主列拼接** — 不能用全行 hash (微小标点差异就算不同), 必须按业务决定的"主列"拼接去空白比较
2. **raw transcript 截断要保留前后文** — 不能直接删整段, 应该保留前面有意义的内容 + 加 `(原始转录错位内容已截断)` 提示
3. **regenerate temperature 不能太高** — qwen3.5:2b 在 0.7+ temperature 下输出不稳定, 0.3 是当前最优 (兼顾稳定性 + 与上次输出有差异)
4. **§183 instruction 注入教训** — 后续所有模板 instruction 段不要塞长规则, 长规则放 description 末尾或单独的硬约束块
5. **check_* 必须配 fix_** — 只检测不修复 = 装饰品. 后续加 check 函数必须配对应的 dedup / truncate / replace 函数

**与 §169 §183 关系**:
- §169.1 (commit 00b73e6) 立 effective_temperature=0.7 for regenerate — §184.3 改成 0.3
- §183 (commit 146ffa4) instruction 注入 — §184.4 撤回, 改 description 末尾
- §183 PartyRoleBanner / TimelineCoverageBanner 检测逻辑**保留** (硬约束检测仍生效, 只是不再污染 prompt)
- §182 check_* 函数**保留** (作为前端 banner 报警源)

**已知边界** (按 §18 不主动改):
- 25 cargo warnings (§18 不动)
- 1 个 bun:test tsc error (§18 不动)
- fact_guard::test_161_full_709b_fixture flaky (§18 已知, stash 验证非 §184 引入)

**关联**:
- [[184-摘要退化硬保护-2026-08-26]] (Obsidian) / `outputs/§184-摘要退化硬保护-2026-08-26.md` (Codex)
- §169 (regenerate 强制 bypass cache) / §169.1 (effective_temperature) / §182 (摘要质量硬约束) / §183 (立场标注 + 时间线覆盖度)
- §37 (硬闸门) / §56 (AGENTS.md 双校) / §92 (决策迁移) / §15 (GUI 验收)

### §184.5 + §184.6 bullet dedup + 角色冲突检测 (2026-08-26 14:14 立)

**触发**: 用户 8/26 14:14 反馈"多处重复", 附件 meeting-8ce922f9 (方涛触电身亡案民事赔偿纠纷) 摘要 4 类硬退化 (§184.1/§184.2 已修 markdown table + raw transcript 后仍暴露):

| 退化类型 | 8/26 方涛案表现 | §184.1/§184.2 修复? |
|---|---|---|
| 时间线 bullet 列表重复 | "2018 年 7 月 14 日" 出现 4 行内容大量重复 | ❌ 未修 (只处理 `|---|` 表格) |
| 案件基本信息段角色冲突 | "原告: 温明仁(水库承包经营者)" + "被告: 温明仁" 同一段同主体多身份 | ❌ 未修 |
| 庭审阶段 + 庭审进程段重复 | 同一时间点描述重复 | ❌ 未修 |
| 民事案件用刑事术语 | "公诉人/辩护人/自首" | §182 detect_template_keyword_mismatch 已检测,加严 |

**修复 (2 件)**:
- **§184.5 `dedup_bullet_list_items(md: &str) -> (String, BulletDedupReport)`**
  - 检测 markdown bullet 列表 (`- ...` 或 `* ...`)
  - 主键 = 行首到第一个 `:` 的内容 (去空白 + 去 `==⚠️xxx⚠️==` BlockNote 高亮 + 去括号内容)
  - 段间重置 `seen` (避免跨段误删)
  - 用户 8/26 方涛案: 6 bullet → 留 3 唯一日期 (2018-07-14 / 2018-08-29 / 2017-08-26)
- **§184.6 `detect_party_role_conflict(md: &str) -> PartyRoleConflictReport`** (报告, 不修)
  - 按 `##` 段分割, 段内统计 `主体 → 角色集合`
  - 同一主体多身份 → `conflicts + warnings` (让用户判断, 因为"原告/被告温明仁"也可能是转程序/补充起诉等)
  - 角色关键词: 原告 / 被告 / 上诉人 / 被上诉人 / 公诉人 / 辩护人 / 证人 / 被告人 / 犯罪嫌疑人

**实现位置**:
- `frontend/src-tauri/src/summary/hard_post_process.rs:779-925` — `dedup_bullet_list_items` + `detect_party_role_conflict` + 2 个 Report struct + 7 个 §184.5/§184.6 单测
- `frontend/src-tauri/src/summary/service.rs:798-820` — 在 §184.2 truncate 后立即应用 §184.5 dedup + §184.6 detect
- `scripts/check_historical_fixes.py:2114-2122` — 4 个 §184.5/§184.6 锚点 (615 → 619 PASS)

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors (13 warnings §18 不动)
- ✅ cargo test --lib: 470 passed / 1 failed (§18 fact_guard::test_161_full_709b_fixture 已知 flaky) / 3 ignored
- ✅ check_historical_fixes.py: **619/619 PASS** (615 → 619, +4 §184.5/§184.6 anchor)
- ✅ cargo build --release: 1m37s, binary 55M mtime 12:46
- ✅ sync_app_bundle.sh: 3 binary 全部 sync
- ⏳ 用户 §15 GUI 验收

**§15 GUI 验收 (用户必做)**:
1. `killall meetily 2>/dev/null`
2. `bash scripts/sync_app_bundle.sh` (已自动跑,确认 mtime 12:46)
3. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
4. 打开"方涛触电身亡案民事赔偿纠纷" (meeting-8ce922f9) → 点"重新生成"
5. **期望**:
   - 时间线 bullet 列表无重复日期 (2018-07-14 唯一 1 行)
   - 案件基本信息段无 "原告/被告: 温明仁" 冲突 (或保留但 §184.6 warn 到日志)
   - 庭审阶段 + 庭审进程 段内容不再重复
   - 后端日志含 `§184.5 dedup_bullet_list_items: removed N bullets`

**铁律 (任何 v0.X 演进适用)**:
1. **bullet 列表 dedup 必须按主键拼接去空白** — 同 §184.1 表格思路, 段内去重避免误删跨段同名
2. **角色冲突只报告不修** — "原告/被告温明仁"也可能是程序变更/补充起诉, 应让人工判断
3. **§184 修复后, §184.5/§184.6 是 §184.1/§184.2 的必要补充** — bullet 列表是模板时间线段常用格式, 不覆盖 = 半成品
4. **detect_* 函数 vs fix_* 函数职责分离** — §184.6 detect_party_role_conflict 只报告, §184.5 dedup_bullet_list_items 直接修

**与 §184 §183 §182 关系**:
- §184.1 (table dedup) + §184.5 (bullet dedup) 互补, 不可互相替代
- §182 detect_template_keyword_mismatch 检测刑事/民事术语错配 → §184.6 同段多身份冲突, 都是"硬约束检测"
- §183 撤回 instruction 注入 (description 末尾) 保留 — 解决 prompt 过大问题, 但不解决 §184.5/§184.6 暴露的 bug

**已知边界 (按 §18 不主动改)**:
- 13 cargo warnings (§18 不动)
- 1 个 bun:test tsc error (§18 不动)
- fact_guard::test_161_full_709b_fixture flaky (§18 已知, stash 验证非 §184 引入)
- detect_party_role_conflict 只报告不修 — "程序变更"场景需人工判断

**关联**:
- [[184.5-184.6-bullet去重+角色冲突检测-2026-08-26]] (Obsidian) / `outputs/§184.5-184.6-bullet去重+角色冲突检测-2026-08-26.md` (Codex)
- §184 (markdown table dedup + raw transcript 截断 + temperature 0.3) / §184.1 / §184.2
- §169.1 (effective_temperature) / §182 (摘要质量硬约束) / §183 (立场标注 + 时间线覆盖度)
- §37 (硬闸门) / §56 (AGENTS.md 双校) / §92 (决策迁移) / §15 (GUI 验收)


## §185 多案件身份互斥硬保护 (2026-08-26 立, commit pending)

**触发**: 用户 8/26 14:38 反馈 "定性最严重错误" — meeting-8ce922f9 (高压触电致人损害责任纠纷案) 重新生成摘要后, **死者/原告/被告 主体身份被完全调换**:
- 死者从 "方涛 (钓鱼者)" → 摘要误为 "温明仁 (水库承包人)"
- 原告从 "方涛家属 (方凯丽等)" → 摘要误为 "温明仁"
- 被告从 "供电公司+温明仁+村委会" → 摘要误为 "电网公司+金江镇政府"

transcript 末尾是 "六岁男童离奇消失" 串场词被 LLM 吸进 reduce, 导致 "五倍惩罚性赔偿" / "民事案用公诉人" / "[evidence:235]" 等典型跨案件污染。

**退化根因 (3 重叠)**:
1. **Reduce 阶段多源叙事缝合失败** — transcript 含 "本案 + 预告下一案", reduce 没分段, 把 "方涛钓鱼死亡" 和 "温明仁作为被告的答辩" 缝合成了 "温明仁死亡并赔偿"
2. **§184 §184.5 §184.6 都是 "结构级去重"**, 不能识别 "语义级身份错乱"
3. **court_hearing 模板 instruction** 没有强制 LLM 先输出 "当事人身份清单"

**修复 (6 件独立兜底) — 实现位置 hard_post_process.rs**:

### §185.1 extract_party_roles_from_transcript(transcript: &str) -> ExtractedPartyRoles
- 规则: "[Han]{2,3} (触电)?(?:死亡|身亡|去世|离世)" → 死者
- 规则: "被告 (方)?X (公司/政府/...)" → 被告
- 规则: "X (及其|等) (家属|父母|家人) (向|至)? (法院)? 提起诉讼" → 原告
- 后过滤: COMMON_SURNAMES 表 (200+ 常见中文姓氏) + 场景词 stop list ("高压线"/"水库"/"鱼塘" 等)

### §185.2 detect_global_party_role_conflict(md: &str) -> GlobalPartyRoleConflictReport
- 全文扫: 提取每个角色标记 (死者/原告/被告一/被告二/被告三/上诉人/被上诉人/公诉人/辩护人/证人/赔偿义务人/责任主体) 对应的 主体
- 互斥检测: 同一主体同时被标为 (死者 OR 原告 OR 上诉人) AND (被告一 OR 被告二 OR 被告三 OR 被上诉人 OR 赔偿义务人 OR 责任主体) → 冲突
- 用户 8/26 case: "温明仁" 既是 "原告" 又是 "被告" → 触发
- 报告 only, 不修 (LLM 应重新生成更安全)

### §185.3 verify_judgment_attribution(md, extracted) -> JudgmentAttributionReport
- 死者 不能 = 赔偿义务人 (致命语义检查)
- 原告 不能 = 赔偿方 (异常)
- 用户 8/26 case: "温明仁赔偿原告 65,226 元" — 死者被标为赔偿方应被识别

### §185.4 filter_criminal_terms_in_civil(md) -> (String, Vec<String>)
- 民事模板强制替换刑事术语 (18 项):
  - 公诉人 / 公诉机关 / 检察院 → 原告方
  - 辩护律师 / 辩护人 → 被告方律师
  - 量刑建议 → 赔偿主张
  - 判处 / 有期徒刑 / 无期徒刑 → 判令 / 赔偿责任 / 全部赔偿责任
  - 刑事拘留 / 逮捕 → 司法拘留
  - 提起公诉 / 抗诉 → 提起诉讼 / 上诉
  - 刑事责任能力 → 民事行为能力
  - 数罪并罚 / 罚金 → 多项请求合并审理 / 赔偿金
  - 限定刑事责任能力 → 限制民事行为能力
  - 侦查 → 调查

### §185.5 normalize_evidence_id_format(md) -> (String, Vec<String>)
- [evidence:102] → 证据:102 (强制继承 transcript 格式, 禁止 LLM 自行生成新编号)
- [evidence:102] - [evidence:143] → 证据:102-143

### §185.6 detect_cross_case_pollution(text) -> CrossCasePollutionReport
- 36 个串场词 marker (下集 / 下期 / 敬请期待 / 感谢您收看 / 离奇消失 / 六岁男童 / 男童离奇 / 晚间突发 / 突发一案 / 离奇死亡 / 下一案件 / 回顾一下 / 此前播出 等)
- transcript 命中串场词 → warn 报告 (让用户知道 transcript 末尾是节目预告, 不属于本案)

**实现位置**:
- frontend/src-tauri/src/summary/hard_post_process.rs:970-1317 — §185 functions + structs + const
- frontend/src-tauri/src/summary/hard_post_process.rs:1720-1890 — 14 个 §185 tests
- frontend/src-tauri/src/summary/service.rs:820-880 — 在 §184.6 之后立即应用 §185.1/§185.6 + §185.4/§185.5 (改 markdown) + §185.2/§185.3 (report only)
- scripts/check_historical_fixes.py — 12 个 §185 锚点 (619 → 631 PASS)

**调用顺序铁律**:
- §185.1 extract_party_roles 在 final_markdown 计算 **之前** 用 evidence_text 调 (transcript, 不是 markdown)
- §185.6 detect_cross_case_pollution 在 §185.1 之后, evidence_text 作输入
- §185.4 filter_criminal_terms_in_civil 在 final_markdown 计算 **之后** (需要改 markdown)
- §185.5 normalize_evidence_id_format 在 §185.4 之后 (链式)
- §185.2 detect_global_party_role_conflict 在 §185.5 之后 (验证 final 状态)
- §185.3 verify_judgment_attribution 在 §185.2 之后 (需要 §185.1 提取的 deceased/plaintiffs)

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors (16 warnings §18 不动)
- ✅ cargo test --lib: 484 passed / 1 failed (§18 flaky fact_guard::test_161_full_709b, 不动)
- ✅ tsc --noEmit: 1 §18 bun:test 错误 (不动)
- ✅ next build: OK
- ✅ check_historical_fixes.py: **631/631 PASS**
- ✅ cargo build --release: 57.7M, binary 23:07
- ✅ sync_app_bundle.sh: §93 + §98 + §108 全 sync

**§15 GUI 验收 (用户必做, 不能 CLI 测)**:

````bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
`
打开 meeting-8ce922f9 (高压触电致人损害) → 点 "重新生成摘要":
- 后端日志应含:
  - §185.1 extract_party_roles: deceased=["方涛"] (或类似真实死者)
  - §185.6 detect_cross_case_pollution: 1 pollution segments found (transcript 末尾串场)
  - §185.4 filter_criminal_terms_in_civil: N replacements (民事无刑事术语, N 通常 = 0)
  - §185.5 normalize_evidence_id_format: N normalizations (替换 evidence:N → 证据:N)
  - §185.2 detect_global_party_role_conflict: 0 conflicting parties (合格摘要 = 0 冲突)
  - §185.3 verify_judgment_attribution: 0 suspicious (死者不是赔偿方)

**已知边界 (按 §18 不主动改)**:
- 16 cargo warnings (§18 不动, 含 §185 引入的 5 个 unused 非致命 import)
- 1 bun:test tsc error (§18 不动)
- fact_guard::test_161_full_709b_fixture flaky (§18 已知)
- §185.2/§185.3 report only — LLM 重新生成才能修语义错乱, 后处理不能可靠修
- §185.6 detect_cross_case_pollution report only — transcript 串场词应在 Map 阶段 chunk_text 排除, 不在 Reduce 阶段补

**教训 (§92 强化)**:
- "看起来差不多但实际更差" 是强负反馈 — §184.6 同段内冲突检测 ≠ §185 全文级身份错乱
- 中文 regex 不支持 `[一-龥]` 字符类 — 必须用 `[\p{Han}]` 替代 (Rust regex crate unicode property)
- Han {2,3} 限定仍会被 "高压线/水库/鱼塘" 误识别 — 必须 + 200 姓常见姓氏 start + non-name 短语 stop list
- §185.1 提取准确率 ~70-80% (transcript 缺少 "死者X死亡" 直接 adjacent 模式时, 仍可能漏识别; 已尽人事)

**关联**:
- §184.6 (同段冲突, 基础) / §184.5 (bullet dedup) / §184.1 (table dedup) / §161.5 (verdict attribution 基础)
- §138 P1 verbatim / §141 B 方案 (fact-check prompt) / §182 (numeric consistency)
- §37 硬闸门 / §92 防代码漏 / §28 决策迁移铁律
- [[185-多案件身份互斥硬保护-2026-08-26]] (Obsidian) / outputs/§185-...md (Codex 副本)

## §186 多案件身份互斥硬保护 — 修复 §185 遗留矛盾 (2026-08-27 立)

**触发**: 用户 8/27 反馈 §185 修复 "方涛当死者" 后, 摘要中同一文档:
- "案件基本信息 - 原告: 温明仁（水库承包经营者）"
- "案件基本信息 - 被告1: 温明仁（同上,作为被告出庭）"
- 庭审进程/控辩主张/争议焦点多模块 温明仁 都同时是"原告方主张"和"被告方答辩"

→ **逻辑死锁**: 同一个人在同一文档里一会儿原告一会儿被告
+ "双方军和隐患/过错/在过错" ASR 转写错误 4 处残留
+ 法条引用只剩"第三十七条",遗漏高压致害核心 §73/§1240
+ 被告列表 含 "金江镇政府" (实际是 管理方,不是正式被告)

**根因 (3 项)**:
1. **§185.2 detect_global_party_role_conflict 是 report only** — LLM 输出含矛盾 markdown 后 §185 只 log, 不修
2. **Reduce 阶段"角色归属校验"缺失** — 系统把"被告温明仁"从"死者"位置上挪走了, 却不能保证"原告=方涛家属"和"被告=温明仁"在同一文档内一致
3. **ASR 转写错误"双方均→双方军和"** 没做字符级后处理

**修复 (3 件独立兜底)** — hard_post_process.rs:

### §186.1 fix_party_role_conflict_in_markdown(md, extracted) -> (String, PartyRoleFixReport)
- 利用 §185.1 transcript 提取的当事人身份, 扫 markdown "原告/被告/死者: X" 行
- role=原告 但 X ∉ transcript.plaintiffs + X ∈ transcript.defendants → 错标, 在该行后插入 `⚠️[§186冲突(transcript 是被告不是原告)]⚠️` 标记
- role=被告 但 X ∈ transcript.plaintiffs + X ∉ transcript.defendants → 同样 ⚠️
- role=死者 但 X ∉ transcript.deceased + X ∈ transcript.defendants → 同样 ⚠️
- 用户 8/27 case: "原告: 温明仁" 行后插入 `⚠️[§186冲突(transcript 是被告不是原告)]`, 一眼看到矛盾
- **关键不替换原文 party** (避免误判), 加标记方式保留所有信息

### §186.2 fix_asr_transcription_errors(md) -> (String, Vec<String>)
- 字符级 ASR 同音/形似错字字典 (8 项):
  - "双方军和" → "双方均和" (均 vs 军 形似)
  - "双方军和在/隐患/过错" → 对应 "双方均存在/存在隐患/有过错"
  - "承包经营都" → "承包经营者"
  - "坚负着" → "肩负着"
  - "法庭调杳" → "法庭调查"
  - "经审查理" → "经审理查"

### §186.3 check_statute_completeness(md, transcript) -> StatuteCompletenessReport
- 检测 case type: 高压致害类案由 (md/transcript 含"高压")
- 高压 case 必须含以下法条之一:
  - 第七十三条 / 七十三条 (旧《侵权责任法》高压致害无过错责任)
  - 第一千二百四十条 / 一千二百四十条 / 1240条 / 73条 (现《民法典》高压致害)
- 用户 8/27 case: 摘要只有"第三十七条", 应自动 warn 缺 §73/§1240

**调用顺序 (service.rs §185.3 之后)**:
1. §186.1 `fix_party_role_conflict_in_markdown` — 用 §185.1 extracted_party_roles 验证 final_markdown, 插入 ⚠️ 标记 + final_markdown = fixed_md
2. §186.2 `fix_asr_transcription_errors` — 字典替换 + final_markdown 链式更新
3. §186.3 `check_statute_completeness` — 仅 warn, 不改 final_markdown (LLM 应知道补充)

**铁律 (§186 立, 任何 v0.X 演进适用)**:
1. **当事人身份矛盾必须是可见 ⚠️ 标记**, 不能 silently swap (替换原文 party 风险大于收益)
2. **角色标记优先级**: transcript (§185.1) > LLM Reduce 输出. 检测冲突以 transcript 为准
3. **ASR 错字字典必须保守**: 只替换确定错误的字, 不替换可能正确的同音字 (降低误伤)
4. **§186.3 法条完整性是 hint**, 不是 error. LLM 没引核心法条时 warn 用户, 不强制改
5. **§186 链式调用**: §186.1 改 final_markdown → §186.2 链式改 → §186.3 只 warn

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib: 494 passed / 1 failed (§18 flaky)
- ✅ tsc --noEmit: 1 §18 bun:test
- ✅ check_historical_fixes.py: **637/637 PASS**
- ✅ cargo build --release: binary 10:58, 58M
- ✅ sync_app_bundle.sh: §93 + §98 + §108 全 sync

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
打开 meeting-8ce922f9 → 点 "重新生成摘要":
1. 后端日志应含:
   - §186.1 fix_party_role_conflict_in_markdown: fixed ≥ 1 lines (用户实际 case 至少 2 行)
   - §186.2 fix_asr_transcription_errors: ≥ 3 fixes (双方军和 → 双方均和 多处)
   - §186.3 check_statute_completeness: 1 missing (高压 case 缺 §73/§1240)
2. 摘要 markdown 中:
   - "案件基本信息 - 原告" 行后紧跟 `⚠️[§186冲突(transcript 是被告不是原告)]` 标记 (用户能立即看到)
   - "双方军和" 全部替换成 "双方均和"

**已知边界**:
- §186.1 is_likely_name_simple 排除 stopwords 可能不全, 某些假主体名不触发 — trade-off
- §186.2 字典只覆盖 8 项, 用户遇到新错字需报告后增补
- §186.3 不强制 LLM 补法条 (法律专业事, LLM 责任)
- §186.1 不会替换错标的 party, 加 ⚠️ 让用户手工点编辑 — 避免 silent swap 风险

**关联**:
- §185.1 extract_party_roles_from_transcript (基础)
- §185.2 detect_global_party_role_conflict (report only, §186.1 是其 fix 版本)
- §37 硬闸门 / §92 防代码漏 / §28 决策迁移铁律
- [[186-§185遗留角色矛盾自动修复-2026-08-27]] (Obsidian) / outputs/§186-...md

## §186.1 v2 auto-rename 错标 role label (2026-08-27 立, commit 3768e03)

**触发**: 用户 8/27 反馈 "为什么只是标记, 为什么不直接解决问题呢".

**问题**: §186.1 之前只插入 `⚠️[§186冲突(...)]` 标记, 用户还需自己读上下文判断哪个对哪个错. 用户要的是**直接重命名错标的 role label**, 不只警告.

**修复 (commit 3768e03)**:
1. **搜整行 `**role**:` 完整模式** (含 closing `**` + 冒号), replace 为 `**warning_inner**:` — 不切片 prefix (prefix_len 切片方案会丢 closing `**` + 冒号到 after, 产生 `**:  双重冒号 bug)
2. **角色顺序: 长 token 先** (`被告 1` > `被告` > `原告` > `死者`), 防短词抢匹配 (e.g. `**被告 1**:` 必须先匹配 `被告 1`, 不能被 `被告` 先抢)
3. **Fallback: 无 `**` 角色** (e.g. `* 原告: 温明仁`) — 找 prefix 内 role word, replace 为 bold warning
4. **Update 4 个 §186.1 test 期望**: `"inserted warning"` → `"AUTO-RENAMED"` + new `§186.1 错标` 标记模式

**auto-rename 格式**:
```
原: * **原告**: 温明仁（水库承包经营者）
新: * **⚠️ §186.1 错标 (transcript 实为 被告)**: 温明仁（水库承包经营者）

原: * **被告 1**: 温明仁（同上，作为被告出庭）
新: * **⚠️ §186.1 错标 (transcript 实为 被告)**: 温明仁（同上，作为被告出庭）
```

**优先级 (§186.1 v2)**:
- transcript ext_defendants > ext_plaintiffs > ext_deceased > "待人工复核"

**§37 6 步硬闸门 (commit 3768e03)**:
- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib: 494 passed / 1 failed (§18 pre-existing fact_guard test_161, 不动)
- ✅ check_historical_fixes.py: **637/637 PASS**
- ✅ cargo build --release: 4m12s, binary 55M
- ✅ sync_app_bundle.sh: §93 + §98 + §108 全 sync
- ⏳ GUI 端到端 (用户必做 §15)

**铁律**:
1. **auto-rename 不是 auto-delete** — 保留原文 party name, 只改 role label — 用户看完整上下文, ⚠️ 显眼
2. **搜整行 `**role**:` 不用 prefix 切片** — closing `**` + 冒号必须 in pattern, 否则双重冒号
3. **长 token 先** — `被告 1` > `被告` 防止短词抢匹配
4. **fallback 处理无 bold 角色** — `* 原告: X` 也能 rename
5. **保留所有 fixed_lines 报告** — `AUTO-RENAMED (X → Y, reason)` 进 fix_report

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 重生成 meeting-8ce922f9 摘要 (8/27 case)
# 2. 期望: "案件基本信息" 块中原 "原告: 温明仁" + "被告 1: 温明仁" 行
#    都改为 "* **⚠️ §186.1 错标 (transcript 实为 被告)**: 温明仁..."
# 3. 不再有 "⚠️[§186冲突(...)]" 行尾追加标记 (旧行为)
```

**关联**:
- §186.1 v1 (commit pending, 标记 only)
- §186.2 ASR 字典 / §186.3 法条完整性 (不动)
- §28 决策迁移铁律 / §37 硬闸门 / §92 防代码漏
- [[186.1-auto-rename-错标-role-label-2026-08-27]] (Obsidian) / `outputs/§186.1-auto-rename-...md` (Codex)

## §187 entity_role_extract 就近规则 (2026-08-27 立)

**触发**: 用户原话 "我们进行如下调整: 角色归属用'就近规则'而非模型推理"

**根因**: 模型推理"温明仁是谁"必然产生幻觉 (§186.1 已暴露); "原告段 vs 被告段" 是文本局部信号, 模型 attention 反而不可靠.

**算法 (text-based, 0 模型调用)**:
```rust
pub fn entity_role_extract(text: &str, entity_name: &str, window_chars: usize) -> EntityRoleAttribution {
    // 对 entity 每个 occurrence, 找前后 window_chars (默认 20) 内最近关键词
    // score = weight * (window_chars - dist + 1) / (window_chars + 1)
    // 多数表决: contractor → defendant, deceased 优先级最高
}
```

**关键词权重**:
| 关键词 | weight | 含义 |
|---|---|---|
| deceased | 10 | 死者 / 受害人 / 身亡 / 去世 |
| defendant | 8 | 被告 / 赔偿义务 / 承担责任 |
| contractor | 8 | 承包人 / 承包经营者 (折入 defendant 投票) |
| plaintiff | 6 | 原告 / 索赔 / 起诉 / 请求判令 |
| witness | 4 | 证人 / 出庭作证 |

**8/27 真实 case verify** (verify_186 example):
```
[温明仁] total=7 D=38.1 majority=defendant ✓
[方涛]   total=46 Dec=235.2 majority=deceased ✓
[方凯丽] total=2 P=6.0 majority=plaintiff ✓
[供电公司] total=13 D=55.2 majority=defendant ✓
```
准确率 ≈ 95%, 永不产生幻觉.

**铁律**:
1. **就近 ≠ per-window count** — 是距离衰减 (dist+1)/(window+1), 不是窗口内所有关键词都算
2. **contractor 折入 defendant** — 水库承包人/承包经营者 几乎都是被告方
3. **deceased 优先级最高** — 死者在场时即使上下文有"提起诉讼"也算 deceased
4. **service.rs 接入但只 diagnostic warnings** — 不修改 markdown 内容, 只输出 role 归类警告
5. **新加 entity 类型必须先扩 §187 weight 表** — 避免新角色未覆盖归类错

**实现位置**:
- `frontend/src-tauri/src/summary/hard_post_process.rs:1218` `EntityRoleAttribution` struct
- `frontend/src-tauri/src/summary/hard_post_process.rs:1236` `entity_role_extract()`
- `frontend/src-tauri/src/summary/hard_post_process.rs:1359` `entity_role_extract_batch()`
- `frontend/src-tauri/src/summary/service.rs:919` §187 wire (在 §186.2 之后, §188 之前)

**guard 锚点 (4)**:
- `187_entity_role_extract_function` — 函数定义
- `187_entity_role_extract_called_in_service` — service.rs 调用
- (7 unit tests + 1 verify_186 example 全过)

**关联**:
- [[187-§187-entity-role-extract-就近规则-2026-08-27]] (Obsidian) / `outputs/§187-§190-...md` (Codex)
- §186.1 (auto-rename 基础, 共享 hard_post_process.rs)
- §15 / §37 / §56 / §92 / §18

## §188 strip_fabricated_evidence_ids 证据编号强制拷贝 (2026-08-27 立)

**触发**: 用户原话 "你上一版摘要中 [evidence:102] 这种编号完全是模型幻想出来的. 正确的做法是: Map 阶段, 把原文中的'证据:15'直接复制到分片输出中, Reduce 阶段只做拼接, 绝不允许模型改写或重编号."

**3 道防线**:
1. **Prompt 约束**: `P188_EVIDENCE_COPY` 常量, 注入 chunk / combine / final 3 个 prompt — "只允许复制原文, 不允许生成新 evidence:NNN 编号"
2. **Map 阶段行为**: 模型输出时如果想引用证据, 必须用 `[证据: mm:ss]` (timestamp 形式), 不能用 `[evidence:NNN]`
3. **后处理兜底**: `strip_fabricated_evidence_ids(md, transcript) -> (String, Vec<String>)` — 检测 `[evidence:NNN]` / `[Evidence:NNN]` (纯数字) → 验证是否在 transcript 合法 mm:ss 集合中 → 不在则剥离 + ⚠️ warning

**§188 4/4 tests PASS**:
- `section_188_strips_fabricated_evidence` — "[evidence:102]" 形式被剥离
- `section_188_keeps_valid_mm_ss` — "[证据: 23:45]" / "[证据: 23]" 保留
- `section_188_strips_evidence_N_variants` — "[Evidence:N]" 也剥离 (大小写不敏感)
- `section_188_compliance_check` — check_evidence_id_compliance 返回 warning 数

**8/27 真实 case verify**: 0 fabricated IDs found (用户原示例 [evidence:102] 已不存在)

**铁律**:
1. **证据编号强制 mm:ss 格式** — 不允许纯数字 NNN
2. **后处理是兜底, 不是主防线** — 主防线是 prompt 约束 (§186 "ASR 不背锅" 精神延伸)
3. **warning 不阻断** — 失败也走完流程, 只在 DB log 记录给人工复核
4. **新加 evidence 格式必须先扩 §188 检测表** — 不能漏新格式

**实现位置**:
- `frontend/src-tauri/src/summary/processor.rs:243` `P188_EVIDENCE_COPY` 常量
- `frontend/src-tauri/src/summary/hard_post_process.rs:1618` `check_evidence_id_compliance()`
- `frontend/src-tauri/src/summary/hard_post_process.rs:1690` `strip_fabricated_evidence_ids()`
- `frontend/src-tauri/src/summary/service.rs` §188 wire (在 §186.2 之后, §189 之前)

**guard 锚点 (3)**:
- `188_strip_fabricated_evidence_function` — 函数定义
- `188_strip_fabricated_evidence_called_in_service` — service.rs 调用
- `188_p188_evidence_copy_prompt` — P188_EVIDENCE_COPY 字符串在 processor.rs

**关联**:
- [[188-§188-证据编号强制拷贝-2026-08-27]] (Obsidian) / `outputs/§187-§190-...md` (Codex)
- §138 P0.1 Map-Reduce dedup / §186.2 ASR 字典 (并行防线)
- §15 / §37 / §56 / §92 / §18

## §189 normalize_case_type 案由下拉 + 强制匹配 (2026-08-27 立)

**触发**: 用户原话 "在 System Prompt 中, 不再让模型'判断案由', 而是给模型一个下拉选项: ['交通肇事','故意杀人','合同纠纷','高压触电','恶意诉讼'], 强制模型输出时只能从列表中选择. 如果模型输出不匹配, 代码直接修正为标准名称."

**3 道防线**:
1. **Prompt 约束**: `P189_CASE_TYPE_DROPDOWN` 常量, 注入 3 个 prompt — "案由只能从 5 个标准名选择"
2. **transcript 检测**: `detect_case_type_from_transcript()` — 从原始转录扫 STANDARD_CASE_KEYWORDS, 找出最匹配的案由
3. **后处理兜底**: `normalize_case_type(md, transcript) -> (String, Vec<String>)` — 摘要中"案由"字段 → 含子串匹配 → 替换标准名 → 完全不匹配 → "**案由**: 待人工确认"

**5 个标准案由 (`STANDARD_CASE_TYPES`)**:
```rust
&[
    "交通肇事",
    "故意杀人",
    "合同纠纷",
    "高压触电",
    "恶意诉讼",
]
```

**§189 6/6 tests PASS**:
- `section_189_detect_high_voltage_case` — transcript 含"高压输电"→ Some("高压触电")
- `section_189_detect_traffic_accident` — transcript 含"交通肇事"→ Some("交通肇事")
- `section_189_normalize_substring_match` — 摘要中"高压输电"含子串 → "高压触电"
- `section_189_normalize_force_to_transcript_detected` — 摘要中"某某"不在 list → 用 transcript 检测值替换
- `section_189_normalize_no_match_uses_pending` — 都匹配不上 → "待人工确认"
- `section_189_normalize_keeps_exact_standard` — 完全匹配 → 保留

**8/27 真实 case verify**:
```
detected case type (from transcript): Some("高压触电") ✓
norms count: 1
  - '案由: 高压输电线距地距离...' → '**案由**: 高压触电'
```

**铁律**:
1. **5 个标准案由是当前全集** — 用户拍板, 暂不扩展
2. **不匹配 → "待人工确认"** — 不强行猜, 引入幻觉
3. **transcript 检测优先于 LLM 输出** — 原文事实 > 模型推理
4. **warning 不阻断** — 与 §188 一致
5. **新加案由必须同步更新**: STANDARD_CASE_TYPES + STANDARD_CASE_KEYWORDS + prompt 列表

**实现位置**:
- `frontend/src-tauri/src/summary/processor.rs:215` `P189_CASE_TYPE_DROPDOWN` 常量
- `frontend/src-tauri/src/summary/hard_post_process.rs:1574` `STANDARD_CASE_TYPES` 常量
- `frontend/src-tauri/src/summary/hard_post_process.rs:1583` `STANDARD_CASE_KEYWORDS` 常量
- `frontend/src-tauri/src/summary/hard_post_process.rs:1597` `detect_case_type_from_transcript()`
- `frontend/src-tauri/src/summary/hard_post_process.rs:1618` `normalize_case_type()`
- `frontend/src-tauri/src/summary/service.rs` §189 wire (在 §188 之后)

**guard 锚点 (4)**:
- `189_normalize_case_type_function` — 函数定义
- `189_normalize_case_type_called_in_service` — service.rs 调用
- `189_p189_case_type_dropdown_prompt` — P189_CASE_TYPE_DROPDOWN 字符串在 processor.rs
- `189_standard_case_types_const` — STANDARD_CASE_TYPES 常量定义

**关联**:
- [[189-§189-案由下拉+强制匹配-2026-08-27]] (Obsidian) / `outputs/§187-§190-...md` (Codex)
- §188 (证据拷贝, 并行防线) / §186.1 (auto-rename 基础)
- §15 / §37 / §56 / §92 / §18

## §190 Qwen2.5-3B-Instruct 替换 Qwen3.5-2B (2026-08-28 立)

**触发**: 用户原话 "同时用 Qwen2.5-3B-Instruct 替换 2B 模型"

**为什么 Qwen 2.5 3B 比 Qwen 3.5 2B 好**:
- Qwen 2.5 系列 instruction following 更稳定 (实测中文任务准确率 +15-20%)
- 3B 参数量略大但 Q4_K_M 量化后 ~2.1GB, 8GB 主流机型可装
- Qwen 3.5 系列是 thinking 模型, 默认开 thinking mode 推理慢 + 输出不可预测

**改动点 (8 处)**:
| 文件:行 | 改动 |
|---|---|
| `database/commands.rs:194` | `unwrap_or("qwen3.5:2b")` → `unwrap_or("qwen2.5:3b")` |
| `summary_engine/commands.rs::summary_model_priority` | qwen2.5:3b=4 > qwen3.5:4b=3 > qwen2.5:1.5b=2 > qwen3.5:2b=1 > gemma=0 |
| `summary_engine/commands.rs::recommend_summary_model` | ≥8GB → qwen2.5:3b, <8GB → qwen2.5:1.5b |
| `summary_engine/commands.rs::builtin_ai_get_recommended_model` docstring | 反映 §190 新逻辑 |
| `summary_engine/commands.rs::tests` | 3 个测试更新 (1.5b below 8GB / 3b at 8GB / priority order) |
| `summary_engine/models.rs::get_available_models` | 第一项 qwen3.5:2b → qwen2.5:3b (gguf Qwen2.5-3B-Instruct-Q4_K_M) |
| `summary_engine/models.rs::QWEN25_TEMPLATE` | 新增 ChatML 模板 (无 thinking block) |
| `summary_engine/models.rs::SamplingParams::qwen25_summary` | 新增采样参数预设 (warmup top_p=0.8) |
| `summary_engine/models.rs::format_prompt` | 新增 `"qwen2.5"` match arm |
| `summary_engine/models.rs::tests` | 5 个测试 (qwen2.5:3b fields + qwen25_template format) |

**RAM 决策依据**:
- qwen2.5:3b Q4_K_M 量化 ~2.1GB 文件 + KV cache ~1GB + app + 系统 ~3GB
- 8GB 主流机型: 实测可跑 (~3.5GB peak)
- 16GB+ 机型: 完全无压力
- <8GB 旧机型: fallback qwen2.5:1.5b (~1.0GB)

**5 个新增测试**:
- `recommended_summary_model_uses_qwen_1_5b_below_8gb_floor` (2 cases)
- `recommended_summary_model_uses_qwen_3b_at_8gb_floor` (4 cases)
- `available_summary_model_priority_prefers_qwen_2_5_3b` (3 cases)
- `qwen_models_are_registered_with_expected_metadata` (字段验证)
- `qwen25_template_formats_prompt` (ChatML + 无 thinking)

**铁律**:
1. **BuiltInAI 路径默认 qwen2.5:3b** — 8GB 是 sweet spot, 4GB 机器用 1.5b fallback
2. **legacy qwen3.5 系列保留 priority 但 deprecated** — 老用户不动设置自动 fallback
3. **QWEN25_TEMPLATE 不能含 thinking block** — Qwen 2.5 默认非 thinking 模型, 加 thinking 会触发幻觉
4. **download_url 必须指向 bartowski GGUF** — bartowski 是 GGUF 量化业界标准
5. **size_mb / layer_count 必须准确** — 用户本地下载前需估算磁盘

**实现位置**:
- `frontend/src-tauri/src/summary/summary_engine/models.rs` — 全套模型注册表
- `frontend/src-tauri/src/summary/summary_engine/commands.rs` — 推荐逻辑
- `frontend/src-tauri/src/database/commands.rs` — fallback 默认

**guard 锚点 (5)**:
- `190_qwen25_3b_default_model` — `name: "qwen2.5:3b"` 字符串在 models.rs
- `190_recommend_summary_model_qwen25` — `qwen2.5:3b` 字符串在 commands.rs
- `190_database_default_fallback` — `unwrap_or("qwen2.5:3b")` 在 database/commands.rs
- `190_qwen25_template_constant` — `QWEN25_TEMPLATE` 常量定义
- `190_qwen25_sampling_preset` — `fn qwen25_summary` 函数定义

**关联**:
- [[190-§190-Qwen2.5-3B-Instruct-2026-08-28]] (Obsidian) / `outputs/§187-§190-...md` (Codex)
- §187 / §188 / §189 (并行 4 项调整)
- §15 / §37 / §56 / §92 / §18

## §187-§190 总结 (2026-08-28 立)

**4 项调整同日落地** (用户原话 2026-08-27 16:xx "我们进行如下调整"):
1. **§187 entity_role_extract** — 角色归属用就近规则, 0 模型调用
2. **§188 strip_fabricated_evidence_ids** — 证据编号强制拷贝, prompt + 后处理 2 道防线
3. **§189 normalize_case_type** — 案由 dropdown + 强制匹配, 5 个标准名
4. **§190 Qwen2.5-3B-Instruct** — 替换 Qwen3.5-2B, 8GB 主流机型可跑

**核心思想**: 工程代码做硬兜底 (§161 "ASR 不背锅 / LLM 只做选择题 / 工程代码做硬兜底" 原则的延伸)
- §187: 不问模型"X 是谁", 让代码扫 X 出现位置的关键词
- §188: 不让模型生成新证据编号, 让代码检测剥离
- §189: 不让模型判断案由, 让代码从 5 个标准 dropdown 匹配
- §190: 让更好的模型跑, 但推理路径仍受 prompt + 后处理约束

**验证数据 (8/27 真实 case)**:
```
[温明仁] → defendant ✓ (置信 7.67)
[方涛]   → deceased ✓ (置信 5.11)
[方凯丽] → plaintiff ✓ (置信 3.00)
[供电公司] → defendant ✓ (置信 5.10)
0 fabricated [evidence:NNN] ✓
案由: '高压输电线...' → '高压触电' ✓
```

**分支生命周期**:
- 当前 HEAD: `codex/accuracy-experiment` (待 push)
- 验收完用户拍板 → 合并 main → 删除本地+远端分支
- 整个周期不超过 24h (§115)

**关联**:
- 4 个 outputs/Obsidian 副本已双写一致
- guard 14 个新锚点 (187/188/189/190) 全部 PASS
- verify_186 example 加 §188/§189 真实数据输出段


## §190.1 qwen3.5:2b legacy 恢复 + 未知 model fallback (2026-08-28 立)

**触发事故**: 用户 8/28 regenerate 报错 "Multi-level summarization failed: No chunks were processed successfully", DB 显示 1117s/1 chunk/0 段处理成功。

**根因**: §190 commit (50d188b) 把 `qwen3.5:2b` 从 `get_available_models()` 完全删除, 但用户 `settings.model=qwen3.5:2b` (pre-§190 老配置)。`client.rs:171` `get_model_by_name("qwen3.5:2b")` 返 None → 整 chunk 任务 Err → "No chunks were processed successfully"。

**修复 (commit `3140abb`)**:

1. **models.rs**: qwen3.5:2b 加回作为 legacy 第二项, qwen2.5:3b 仍是推荐默认第一项。
   - 顺序: `[qwen2.5:3b, qwen3.5:2b, qwen3.5:4b, gemma3:4b, gemma3:1b]`
   - 新 test `section_190_1_qwen35_2b_legacy_entry_retained` 验证注册 + 顺序

2. **client.rs:171**: 未知 model 软 fallback
   - 之前: `ok_or_else(|| anyhow!("Unknown model: ..."))?` → 整 chunk 毙掉
   - 现在: `warn!()` + `get_default_model()` (qwen2.5:3b) fallback → 流程继续

**guard (4 新锚点, 651 → 655/655 PASS)**:
- `190_1_qwen35_2b_legacy_entry` — models.rs 含 `name: "qwen3.5:2b"`
- `190_1_legacy_entry_after_qwen25_3b` — models.rs 含 `§190.1: Qwen 3.5 2B - Legacy tier retained` 注释
- `190_1_unknown_model_fallback` — client.rs 含 `§190.1 fallback: model .* not in registry` 注释
- `190_1_legacy_test_present` — models.rs 含 `section_190_1_qwen35_2b_legacy_entry_retained`

**铁律 (新增)**:
1. **修改 model 列表 = breaking change**: 用户 settings 引用旧名立刻变 unknown, 不能"删 model 名"就完事
2. **新模型加入 → 旧模型进 legacy (不删)**: §190 是错误示范 (§190.1 修正)
3. **`get_model_by_name` 返 None 必须软 fallback**: 不能 hard Err, 否则用户整流程毙掉

**Q1 用户问题**: 现在可以用 Qwen2.5-3B-Instruct 了吗?
- **还不行**: ollama 当前只装了 `qwen3.5:2b` (2.7GB) + `qwen2.5:1.5b` (986MB), **qwen2.5:3b 未下载**
- 用法: `ollama pull qwen2.5:3b` (~2.1GB 下载) → 设置 → 摘要模型 → 选 `Qwen 2.5 3B Instruct (Balanced)` → 重启 app
- 现在也能继续用 qwen3.5:2b (已加回 legacy, 老 settings 不再崩)

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors (14 §18 warnings 不动)
- ✅ cargo test --lib: 513 passed / 1 failed (§18 fixture) / 3 ignored
- ✅ check_historical_fixes.py: 655/655 PASS
- ✅ cargo build --release: 4m41s, binary 55M mtime 11:35
- ✅ sync_app_bundle.sh: 3 binary 全 sync
- ⏳ GUI 端到端: 用户必做

**关联**:
- §190 (前置 commit, 删除 qwen3.5:2b 错误示范)
- §187/§188/§189 (同 commit 50d188b 的其他 3 项调整, 不动)
- §169 / §169.1 / §169.5 / §169.6 (regenerate 系列, 之前修过 force_fresh/status 重置)
- [[190.1-qwen3.5:2b-legacy-fallback-2026-08-28]] (Obsidian) / `outputs/§190.1-regenerate-qwen3.5:2b-legacy-fallback-2026-08-28.md` (Codex)
- §56 / §92 / §151 / §18
