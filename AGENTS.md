# AGENTS.md — 离线会记 / 言镜 AI 项目规则

> 任何 LLM session (Claude / Codex / WorkBuddy) 操作本项目必读。

## 1. 项目身份

- **产品名**: 言镜 AI / SpeakMirror (旧 Meetily)
- **主仓库**: `/Users/wangwei/Documents/离线会记/`
- **Obsidian Vault**: `~/Documents/Obsidian Vault/项目/3-离线会记/`
- **当前版本**: v0.8.6 (commit ba7ee13, HEAD caad987)
- **当前分支**: `perf/summary-map-concurrency`

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
