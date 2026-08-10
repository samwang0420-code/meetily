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
