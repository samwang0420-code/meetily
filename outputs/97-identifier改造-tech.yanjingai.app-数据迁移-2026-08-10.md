# §97 identifier 改造 — `cn.lixianhuiji.app` → `tech.yanjingai.app` + 数据迁移 (2026-08-10)

**Commit**: `276906e` on `perf/summary-map-concurrency`

## 触发

§94 全面代码审计 P1 待办 — Bundle identifier 与言镜 AI 品牌名对齐。用户机器现状（已确认）：
- `~/Library/Application Support/cn.lixianhuiji.app/` 4.5G — 含 db + models + decode_cache
- `~/Library/Application Support/tech.yanjingai.app/` 4.0G — 缺 db.sqlite + recording_preferences.json
- **关键**：用户当前 binary 写新目录，但 db.sqlite 在旧目录，录音会找不到

## 决策（已拍板，§65 + §94 共识）

1. identifier `cn.lixianhuiji.app` → `tech.yanjingai.app`
2. 数据迁移 **§65 A 方案**：首次启动自动复制旧目录，旧目录保留不删
3. **不改** `/tmp/lixianhuiji_diar`（IPC 共享临时目录，7 处 Rust+Python 同步，改风险大于收益）
4. **保留向后兼容 env var**：`LIXIANHUIJI_DIAR_DB_PATH`（旧），新加 `YANJINGAI_DIAR_DB_PATH`（优先）
5. **保留** §65 兼容协议：`lixianhuiji.session` / `lixianhuiji.last_email` / `lixianhuiji:toggle-recording` / `lixianhuiji:search-query` / DB 表名 / migration 文件名

## 改动（13 文件，172 + / 24 -）

### 核心：config.rs + lib.rs
- `config.rs` 加常量 + 改函数：
  ```rust
  pub const APP_BUNDLE_ID: &str = "tech.yanjingai.app";
  pub const APP_BUNDLE_ID_LEGACY: &str = "cn.lixianhuiji.app";
  ```
  `dirs_root_app_data()` 8 处 `cn.lixianhuiji.app` → `APP_BUNDLE_ID`
- `lib.rs` 加 `pub fn migrate_legacy_app_data() -> anyhow::Result<()>`
  - 检测：新路径 `tech.yanjingai.app/meeting_minutes.sqlite` 不存在 + 旧路径存在
  - 复制：只复制 `meeting_minutes.sqlite` + `.sqlite-shm` + `.sqlite-wal`（**不复制 4.5G decode_cache/models**，避免磁盘爆）
  - 旧目录不动
  - 失败只 `warn!`，不阻塞启动
  - `setup()` 早期调一次
- `lib.rs:888` panic hook `data_dir().join("cn.lixianhuiji.app")` → `data_dir().join(APP_BUNDLE_ID)`

### tauri.conf.json
- `identifier: cn.lixianhuiji.app` → `tech.yanjingai.app`

### Python 3 文件
- `scripts/sherpa_asr.py:38-41` `_diar_db_path()` 加 `YANJINGAI_DIAR_DB_PATH` 优先
- `scripts/sherpa_asr.py:170-172` `MODELS_ROOT` 改 `tech.yanjingai.app`
- `scripts/diar.py:43-46` `_MODELS_CANDIDATES[0]` 改 `tech.yanjingai.app`（保留旧路径作 fallback）
- `scripts/diar_download.py:67-71` `_MODELS_ROOT_CANDIDATES` 改 + 保留旧路径

**Python 路径优先级**：
```python
os.environ.get("YANJINGAI_DIAR_DB_PATH")
or os.environ.get("LIXIANHUIJI_DIAR_DB_PATH")  # 向后兼容
or os.path.expanduser("~/Library/Application Support/tech.yanjingai.app/...")
```

### Rust inline Python（2 处）
- `api/diar_pickup_loop.rs:147` Python 内联 `_diar_db_path()` 同上
- `api/api.rs:1453` 同上

### UI 显示路径（3 文件）
- `app/legal/privacy/page.tsx:59` — 3 处 `cn.lixianhuiji.app` 改 `tech.yanjingai.app`（macOS/Windows/Linux 全路径）
- `components/TranscriptSettings.tsx:205` — UI 显示路径
- `hooks/useRecordingStart.ts:54` — 注释路径

### 注释路径
- `summary/processor.rs:1186` 测试注释路径改

### 守卫
- `scripts/check_historical_fixes.py` +10 §97 锚点（guard 87 → **97/97 PASS**）

## §97 立铁律（AGENTS.md 同日落）

新决策 — identifier 改造完成后，AGENTS.md §97 立：

1. **identifier 改动必须配 migrate 函数** — 不能只改 `tauri.conf.json`
2. **migrate 只能 COPY，不能 DELETE/MOVE** — 旧目录保留观察期
3. **migrate 必须 best-effort** — 失败 warn 不阻塞
4. **Python 路径优先 env var，fallback hardcode** — env var 名跟 identifier 对齐（`YANJINGAI_*` 优先 `LIXIANHUIJI_*`）
5. **每次 commit 前 §37 硬闸门 + AGENTS.md §92 三处同步**

## §37 硬闸门（全过）

- ✅ `cargo check --lib`: 0 errors（27 §18 warnings 不动）
- ✅ `cargo test --lib`: **327 passed / 0 failed / 3 ignored**
- ✅ `npx tsc --noEmit`: 0 errors
- ✅ `python3 scripts/audit_codebase.py`: 0 errors / 1 warn / 60 info
- ✅ `python3 scripts/check_historical_fixes.py`: **97/97 PASS**（+10 §97 锚点）

## 已知边界

- `tech.yanjingai.app/` 已有 4.0G（decode_cache + models + 部分 sqlite），迁移只补 db.sqlite + sqlite-shm + sqlite-wal
- 不复制 4.5G 旧目录全部，避免磁盘爆（§21 铁律：持续监控 `df -h /`）
- `/tmp/lixianhuiji_diar` 保留 — IPC 共享临时目录，改风险大于收益
- 旧 `cn.lixianhuiji.app/` 数据保留 30 天观察期，之后用户可手动删

## §15 GUI 验收（用户必做）

```bash
killall meetily
open /Users/wangwei/Documents/离线会记/target/release/言镜\ AI.app
```

1. 启动后 logs 应见 `§97 migrate_legacy_app_data: copied=[...] skipped=[...]`
2. DB 验证：`sqlite3 ~/Library/Application\ Support/tech.yanjingai.app/meeting_minutes.sqlite "SELECT COUNT(*) FROM meetings"` ≥ 用户历史会议数
3. 录 30s 新会议 → `SELECT COUNT(*) FROM transcripts ORDER BY id DESC LIMIT 1` ≥ 1
4. 旧目录 `cn.lixianhuiji.app/meeting_minutes.sqlite` 仍在（不删）
5. §96 python 探测应见 `[sherpa] §96 python3 selected (probe OK)`

## 关联

- [[65-言镜AI品牌改名与Bundle数据迁移]] (Obsidian 原 §65 决策)
- [[94-全面代码审计-代码漏系统性问题-2026-08-07]] (§94 P1 待办)
- [[92-代码漏根因审计-AGENTS.md与代码脱节-2026-08-07]] (§92 三处同步铁律)
- [[37-v0.7-→-v0.8-分支迁移-SOP]] (§37 硬闸门)
