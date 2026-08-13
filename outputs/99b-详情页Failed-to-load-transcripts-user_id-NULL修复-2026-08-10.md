# §99.2 — 详情页 "Failed to load transcripts" 修复 (2026-08-10)

**触发**: 用户截图 (导入音频成功后点详情页)
```
导入音频 2026-08-10 12:09  →  Failed to load transcripts  /  Go Back
```

**commit (待用户手动 push)**: §99.2 fix on `perf/summary-map-concurrency`

## 根因

AGENTS.md §59 已立铁律 "导入路径 INSERT meetings 必须包含 user_id"，但代码漏修：
- `frontend/src-tauri/src/audio/import.rs:782` 的 `INSERT INTO meetings (id, title, created_at, updated_at, folder_path)` 没有 `user_id` 列
- 新导入会议 `user_id = NULL`
- `api_get_meeting_metadata` 按 `user_id` 过滤 → 找不到 → 详情页立刻 fail

DB 实证 (修复前):
```
meeting-5c047e1b-...|   |导入音频 2026-08-10 12:09|2026-08-10 04:24:39   ← user_id NULL
meeting-f2b73add-...| 2 |导入音频 2026-08-05 13:45|...                    ← user_id=2 ✓
```

## 修复 (3 文件)

### 1. `frontend/src-tauri/src/audio/import.rs`
- `create_meeting_with_transcripts` 函数签名加 `app: &AppHandle<R>` + `user_id: i64`
- 优先 `crate::user::commands::latest_session_in_db` 拿 user_id
- fallback `-1` (机器 owner 哨兵, 跟 §49 §26 录音路径一致)
- `INSERT INTO meetings (id, title, created_at, updated_at, folder_path, user_id) VALUES (?, ?, ?, ?, ?, ?)`
- 加 `section_99_2_create_meeting_writes_user_id` 单测 (3 个 src 静态断言)

### 2. `frontend/src-tauri/src/lib.rs`
- `setup()` 加 `backfill_meeting_user_ids` 异步 hook (tokio::spawn 不阻塞 setup)
- `UPDATE meetings SET user_id = ? WHERE user_id IS NULL OR user_id = -1`
- `UPDATE transcripts SET user_id = ? WHERE user_id IS NULL OR user_id = -1`
- 无活跃 session 时 skip (避免误挂错用户)
- best-effort, 失败 warn 不阻塞启动

### 3. `scripts/check_historical_fixes.py`
3 个新 §99.2 anchor:
- `99_2_import_writes_user_id` — `let user_id: i64 = match crate::user::commands::latest_session_in_db`
- `99_2_insert_meetings_has_user_id` — `INSERT INTO meetings (..., user_id)`
- `99_2_test_exists` — `section_99_2_create_meeting_writes_user_id`

**guard**: 110/110 PASS (从 107 → 110)

## §37 硬闸门

- ✅ cargo check --lib: 0 errors (27 §18 warnings 不动)
- ✅ cargo test --lib: **331 passed / 0 failed / 3 ignored**
- ✅ cargo build --release: **1m48s**, binary **69M mtime 12:53**
- ✅ bash scripts/sync_app_bundle.sh: **OK §98 codesign identifier=tech.yanjingai.app**
- ✅ python3 scripts/check_historical_fixes.py: **110/110 PASS**
- ✅ python3 scripts/audit_codebase.py --strict: 0 errors / 1 warn / 60 info

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

启动后日志应见:
```
§99.2 backfill_meeting_user_ids: meetings=1 transcripts=0 → user_id=2
```
- 左侧 `导入音频 2026-08-10 12:09` (之前 NULL) → 点详情页应正常显示
- 新导入会议 → `SELECT user_id FROM meetings ORDER BY created_at DESC LIMIT 1` 应 = 2 (不再 NULL)

## 铁律强化 (§99.2 立)

1. **AGENTS.md §X 描述 ≠ 代码 commit** (§56) — §59 已立但 import.rs 漏改, §99.2 补做
2. **每次 commit 后必跑 §37 闸门全套** (cargo test --lib + check_historical_fixes) — 漏一个就是漏代码
3. **写新 INSERT SQL 时必须检查 user_id** — 当前用户隔离 (P0 安全加固 migration 20260722000000)
4. **回填用 UPDATE ... WHERE user_id IS NULL OR user_id = -1** — 哨兵 -1 是机器 owner 标记

## 用户手动 commit + push 命令

> ⚠️ Codex CLI 当前 sandbox 的 auto-review 持续故障, 无法自己执行 .git 操作.
> 请你手动跑下面 3 条命令 (改动已在工作树, 110/110 guard 已通过):

```bash
cd /Users/wangwei/Documents/离线会记
git -c user.email=codex@local -c user.name=codex add \
  frontend/src-tauri/src/audio/import.rs \
  frontend/src-tauri/src/lib.rs \
  scripts/check_historical_fixes.py

git -c user.email=codex@local -c user.name=codex commit -m "fix(§99.2): import.rs::create_meeting_with_transcripts 写 user_id + 启动时回填 NULL 哨兵"

git push origin perf/summary-map-concurrency
```

## 回退方案 (§37 SOP)

```bash
git reset --hard 219df4c  # §99 docs baseline
cd frontend/src-tauri && cargo build --release
bash scripts/sync_app_bundle.sh
```

## 关联

- §59 commit (立铁律但漏修) / §26 §49 (回填脚本历史)
- §56 (AGENTS.md 双校) / §92 (commit + 双写) / §37 (硬闸门)
- §99 (修两个 import 失败的 bug) / §99.2 (本节)
- [[99b-详情页Failed-to-load-transcripts-user_id-NULL修复]] (Obsidian 主份)
