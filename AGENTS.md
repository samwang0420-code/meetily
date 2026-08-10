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
