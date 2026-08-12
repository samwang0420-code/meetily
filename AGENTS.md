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
