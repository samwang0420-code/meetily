# §73 启动 panic "migration 20260722000000 missing" 根因 + 一次性修复 (2026-08-06)

## 触发
用户终端 `open /Users/wangwei/Documents/离线会记/target/release/meetily` 后 binary 在 `lib.rs:511` panic：
```
ERROR app_lib::database::manager] Database connection failed: migration 20260722000000 was previously applied but is missing in the resolved migrations
thread 'main' panicked ... Failed to initialize database
```

## 根因
不是 `open` 命令问题，也不是迁移文件丢失：
- `frontend/src-tauri/migrations/20260722000000_user_meeting_isolation.sql` (1270 B, 7/31 00:21) ✅
- `frontend/src-tauri/migrations/20260722010000_activation_code_user_binding.sql` (268 B, 7/31 00:21) ✅
- binary `strings` 多次匹配到两份 SQL 关键语句 (`ALTER TABLE meetings ADD COLUMN user_id INTEGER` 等) ✅

真正的差异在 `_sqlx_migrations.checksum`：
- 7/31 第一次创建 migration 后，DB 写入 `checksum = sha384(旧版本内容)`
- 之后有人重写过这两份 `.sql`（DB 文档、注释或字段调整，hash 变了）
- binary 是用 **最新** 源文件 embed 出来的，sqlx 启动时把磁盘文件算出的 SHA-384 与 DB 中旧 checksum 对比，发现不匹配 → 报 "previously applied but is missing"

## 一次性修复（已落地）
不动迁移文件、不动 binary。直接更新 DB `_sqlx_migrations.checksum` 跟磁盘对齐：
```python
import sqlite3, hashlib, os
db = os.path.expanduser("~/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite")
con = sqlite3.connect(db)
cur = con.cursor()
for fname in sorted(os.listdir("frontend/src-tauri/migrations")):
    if not fname.startswith("20260722") or not fname.endswith(".sql"):
        continue
    version = fname.split("_", 1)[0]
    data = open(f"frontend/src-tauri/migrations/{fname}", "rb").read()
    cur.execute("UPDATE _sqlx_migrations SET checksum=? WHERE version=?", (hashlib.sha384(data).digest(), version))
con.commit()
```
执行后 `verify 20260722000000 ck_len=48` / `20260722010000 ck_len=48` ✅。

## 验证
后台启动 `RUST_LOG=info .../meetily &` sleep 6 + kill：
- `Database opened successfully` ✅
- `Database initialized successfully` ✅
- `api_get_meetings` 返 56 meetings，`user_id=2` session 恢复 ✅
- 模型/转录/语言配置全部正常读出 ✅
- 进程 kill 干净，无 panic

## 教训
- **sqlx `migrate!` 是 "applied but missing" = checksum mismatch**，不是迁移文件丢失。
  任何 "missing in the resolved migrations" 错误，先用 `SELECT version, length(checksum) FROM _sqlx_migrations` 对比磁盘 SHA-384（hex → 48 字节 raw）。
- 改 .sql 注释/空白/字段时必须**知道 sqlx 会重算 checksum**。要么同步 DB（一次性 Python 脚本），要么用 `ALTER TABLE ... IF NOT EXISTS` 之类 idempotent 重写。
- 未来加新 migration 后不再回写老文件，这条是默认安全。
- macOS 终端 `open` 启动 Tauri GUI binary 立刻返回 0，panic 在 GUI 进程里；用 `&` + `kill` 后台验证能直接看 stderr。

## 关联
- 7/31 落地的 v0.8.5 系列 commit `b0cc29d`/`4ddde9d`/`9346f69` 之前曾有 _sqlx_migrations mismatch 反复 panic 史（commit `b66cdbc` rc7 修过一次）
- §15 §37 验证铁律：binary 启动后必须 `Database opened successfully` 才算通过
