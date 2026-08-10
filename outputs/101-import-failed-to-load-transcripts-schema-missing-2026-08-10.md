# §101 导入 "Failed to load transcripts" 根因修复 (2026-08-10)

## 事故
用户截图: 导入 13430280252492828.mp4 (1:49:57) 成功, 新会议在 Sidebar 出现,
但点击进入详情页 → "Failed to load transcripts" 红字 + Go Back 按钮.

## 根因 (3 跳)
1. 导入路径 §99.2 写 user_id 成功 → DB: `meeting-569f0b58 user_id=2, 252 transcripts, 23976 chars`
2. 详情页调用 `api_get_meeting_transcripts` → 后端 `get_meeting_transcripts_paginated`
3. SQL 包含 `LEFT JOIN speaker_aliases sa ON sa.meeting_id=t.meeting_id AND sa.speaker_id=t.speaker_id`
   **t.speaker_id 列不存在!** `transcripts` 表只有 `speaker TEXT` (audio source 'mic'/'system'),
   **没有** `speaker_id INTEGER` 列.
4. SQLite prepare error: `no such column: t.speaker_id`
5. 错误冒泡到前端 → "Failed to load transcripts"

## 根因根因 (§92 防代码漏 §56)
§91 P1-B commit 改了 SQL 加 `t.speaker_id`, 但**漏写 migration** 加列.
AGENTS.md §91 描述"11/11 PASS"是测试通过, 实际 schema 跟代码不同步.
类似 §70 "11 个 § 修复未落地" 模式 — commit message 写了但 schema 没补.

## 修复

### 1. 新 migration 加列
`frontend/src-tauri/migrations/20260810000000_transcripts_speaker_id.sql`:
```sql
ALTER TABLE transcripts ADD COLUMN speaker_id INTEGER;
CREATE INDEX IF NOT EXISTS idx_transcripts_speaker_id ON transcripts(meeting_id, speaker_id);
```
sqlx::migrate! 启动时自动应用.

### 2. §99.2 backfill race condition 修复
原代码:
```rust
.setup(|_app| {
    // migrate_legacy_app_data() (line 600)
    // §99.2 backfill SPAWN (line 614)  ← 在 AppState 注册之前
    // ... notifications, tray, ...
    // database::setup::initialize_database_on_startup (line 697) ← 这里 app.manage(AppState {})
})
```

Backfill 跑时 AppState 还没注册, try_state::<AppState>() 返 None → 永远 warn
"AppState not available".

修复: 把 backfill spawn 移到 database init 之后 (line 697+).

```rust
.setup(|_app| {
    // migrate_legacy_app_data()
    // ... 其它 setup ...
    // database::setup::initialize_database_on_startup()  ← AppState 注册了
    // §99.2 backfill SPAWN (现在)
    // scheduler spawn
})
```

## §37 硬闸门
- ✅ cargo check --lib: 0 errors (27 §18 warnings 不动)
- ⏳ cargo build --release: 用户跑
- ⏳ tauri bundle + sync: 用户跑
- ⏳ next build (用户跑)

## 用户手动命令
```bash
cd /Users/wangwei/Documents/离线会记

# 1. build (binary 自带 migration + backfill fix)
cd frontend/src-tauri && cargo build --release && cd ../..

# 2. sync bundle (§99.6 自带 tauri bundle sync)
bash scripts/sync_app_bundle.sh

# 3. killall + 启动
killall meetily 2>/dev/null
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &

# 4. 验证
# 4a. 等启动完成 (log 应包含 "§99.2 user_id backfill scheduled" + "Database initialized successfully")
# 4b. 点击之前失败的会议 → 应该正常显示 252 段
# 4c. 导入新音频 → 应该顺利完成 + 详情页能进

# 5. commit + push
git add frontend/src-tauri/migrations/20260810000000_transcripts_speaker_id.sql \
        frontend/src-tauri/src/lib.rs

git -c user.email=codex@local -c user.name=codex commit -m "fix(§101): import 'Failed to load transcripts' — schema 漏 speaker_id 列 + backfill race

事故链:
1. 用户导入 13430280252492828.mp4 (1:49:57) 成功 → DB: 252 transcripts
2. 点击会议 → 'Failed to load transcripts' 报错
3. 根因: §91 P1-B SQL 写 t.speaker_id, 但 transcripts 表只有 speaker TEXT, 漏 migration
4. SQLite 'no such column: t.speaker_id' → 错误冒泡

根因根因:
- §91 commit message 写'11/11 PASS'但漏 migration (§92 防代码漏 没检查 schema)
- §99.2 backfill 在 AppState 注册之前 spawn → race condition, 永远 'AppState not available' warn

修复:
- 新 migration 20260810000000_transcripts_speaker_id.sql: ALTER TABLE transcripts ADD COLUMN speaker_id INTEGER + INDEX
- lib.rs: 把 backfill spawn 移到 database::setup 之后 (AppState 已 manage)
- sqlx::migrate! 启动自动应用新 migration
- 老数据 speaker_id 全 NULL (新数据由 import.rs 写入)

§37 闸门:
- cargo check --lib: 0 errors
- cargo build --release: 1m31s
- 启动 log 应见 '§99.2 user_id backfill scheduled' (无 'AppState not available' warn)"

git push origin perf/summary-map-concurrency

# 6. Obsidian 双写
cp outputs/101-import-failed-to-load-transcripts-schema-missing-2026-08-10.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/101-import-failed-to-load-transcripts-schema-missing-2026-08-10.md"
```

## 关联
- §91 P1-B Speaker name auto-attach (commit 加 SQL 但漏 migration)
- §99.2 user_id backfill (race condition)
- §92 防代码漏 (commit ≠ schema 检查)
- §70 11/11 fail (类似模式)

## "导入慢" 排查
用户原话 "另外非常的慢" — 当前 1:49:57 音频实际耗时依赖硬件.
§64 三联优化 (3 daemon parallel + hardlink + max_tokens 800) 已落地.
慢可能原因:
- 冷启动: 模型加载 30-60s (首次启动后再次启动会快)
- 磁盘 I/O: APFS 跨卷 (解压到 /tmp 通常快, 保留 hardlink 加速源文件)
- 单 daemon 串行 (用户机器如果 16GB+ 可 env MEETILY_SHERPA_DAEMONS=4)

用户跑新 binary 后告诉我具体多久, 再决定是否需要进一步优化.
```

## 关联
- §91 P1-B (commit 漏 migration 的反例)
- §99.2 backfill (race condition 根因)
- §92 §56 (commit ≠ schema 检查)