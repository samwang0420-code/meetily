- 全站其他 toast / 错误英文还没扫完
- en.ts 实际切换没测 (默认是 zh, 自动检测 browser lang)

**学习**:
- "去重" 必须在**所有写入路径** (Rust → Frontend → DB) 都加, 单一层漏修整体仍漏
- i18n 必须用 lint/grep 强制扫描硬编码英文, 组件级 useTranslation 是可发现的, 但漏 import 是不可发现的

## 2026-07-10 18:45 · v0.6.11 DB UNIQUE INDEX 彻底拦截 transcript 重复

**项目**: 离线会记 · [[41-v0.6.11-Wave-db-unique-dedup]]

**用户反馈**: 上一轮 (#40) 修复后 18-22-37 录音仍 6 段重复 (3 段 × 2)

**根因**:
- 18-22-37 DB 6 条同 transaction 一次性 INSERT = api_save_transcript 一次性写 = 前端 transcriptsRef.current 有 6 条重复
- 上一轮前端 isDup + TIME_TOL=0.05 (chunk 1206 验证 4 处) 编译进去了但**实际没生效** (React 18 batch/时序, 未深查)
- audio_start_time 最大 335.67s 超过实际音频 185s = streaming chunk.timestamp 累计 bug (本次不修)

**修复**:
- 新 migration `20260710000000_dedup_transcripts_unique.sql`: DELETE 现存重复 + UNIQUE INDEX(meeting_id, ROUND(audio_start_time*20)/20, ROUND(audio_end_time*20)/20) 50ms 容差
- 3 个 INSERT 路径 (transcript.rs:50 + import.rs:722 + retranscription.rs:531) 全改 INSERT OR IGNORE
- 手动跑 migration apply 到当前 DB + 记录 _sqlx_migrations
- 18-22-37 DB 6 → 3 段 (手动 dedup)

**验证**: 单测 3/3 / cargo build 1m33s / strings 验证 INSERT OR IGNORE 命中 3 处 / app 启动 PID 73429 / 手动 dedup 后 18-22-37 显示 3 段

**未做**: 前端 dedup 真没生效根因 (DB 层拦截已足够, 暂不深挖) / chunk.timestamp 累计 bug / 延时 1 分钟 (待新录音复测)

**学习**: 重复问题应该在**最底层 (DB)** 兜底, 上层 dedup 可能因时序/batch 失效

---

## 2026-07-11 · meetily DB checksum 损坏导致启动崩溃(零数据丢失修复)

**现象**: 用户跑 `/Users/wangwei/Documents/meetily/target/release/meetily` 直接 panic
```
ERROR app_lib::database::manager] Database connection failed: while executing migrations:
  error occurred while decoding column 1: mismatched types;
  Rust type `alloc::vec::Vec<u8>` (as SQL type `BLOB`) is not compatible with SQL type `INTEGER`
thread 'main' panicked at frontend/src-tauri/src/lib.rs:497:14
zsh: abort
```

**根因**:
- sqlx 0.8 `_sqlx_migrations` 表 schema `checksum BLOB NOT NULL`,启动时跑 `SELECT version, checksum FROM _sqlx_migrations ORDER BY version` → `(i64, Vec<u8>)`
- DB 12 条记录里 11 条 checksum 正确 48 字节 BLOB (sha384)
- **最后一条 `20260710000000_dedup_transcripts_unique` 的 checksum 只有 1 字节,类型竟然是 INTEGER (hex `30` = ASCII '0')**,不是 BLOB
- 推测:这个 migration (7/10 加的) 在最初 apply 时,**可能用了某个老版本的 sqlx 或者手写 INSERT,导致 integrity check 失败时把空 checksum 当整数 '0' 落库**
- sqlx 0.8 严格读 BLOB 时直接报错,12 条全部 reject,启动 crash

**诊断步骤**:
1. cp 备份 `.bak.1783753681`(139264 bytes,原文件大小一致)
2. `sqlite3 .schema` 看所有 12 张表 + migrations 表 — schema 完全正常
3. `sqlite3 ... "SELECT typeof(checksum), length(checksum) FROM _sqlx_migrations"` — 11 条 blob/48,最后 1 条 integer/1
4. 读 sqlx 0.8.6 源码 `~/.cargo/.../sqlx-sqlite-0.8.6/src/migrate.rs:107` 确认就是从这一行取 checksum 失败

**修复(1 条 SQL,零数据丢失)**:
```sql
UPDATE _sqlx_migrations
SET checksum = X'E7A522C2C2AD2D28B52F3B81522BD0930C4B5AB7D8B104E30D3F0190D6108D71EECA571FEDD4579F84B708DA1FDA34F9'
WHERE version = 20260710000000;
```
- SHA-384 用 Python `hashlib.sha384(open(...,'rb').read()).digest()` 算,跟 sqlx `Migration::new` 内部算法对齐
- 验证 12/12 行 SHA-384 全部跟当前文件字节匹配 ✅

**验证**:
- `/Users/wangwei/Documents/meetily/target/release/meetily` 启动 5s+,无 ERROR/panic
- DB 数据完整:meetings=15 / transcripts=59 / users=1 全部保留
- 备份保留 `meeting_minutes.sqlite.bak.1783753681` 供回滚

**后续观察项**:
1. **为什么这个 migration 当时落库成 integer '0'?** 加一段 _sqlx_migrations 写入审计 log,看是不是 INSERT 路径漏了 BLOB 类型转换
2. **未来 sqlx 升级时**:同样的 schema mismatch 可能再次出现(major version 升级都可能改 column 类型),迁移前用 `sqlite3 ... "PRAGMA table_info(_sqlx_migrations);"` 先看现状
3. **是否提供 self-healing**:在 `meetily/src/database/manager.rs` 加一段启动 check,如果发现 checksum 类型不对/长度不对,自动用当前 SQL 文件重新写 — **本轮不做,等用户反馈**

**相关**:
- 项目: [[离线会记]]
- sqlx 0.8.6 migrate source: `~/.cargo/registry/src/.../sqlx-sqlite-0.8.6/src/migrate.rs:107` (SELECT version, checksum)
- sqlx `Migration::new` checksum 算法: `~/.cargo/registry/src/.../sqlx-core-0.8.6/src/migrate/migration.rs:8-37` (SHA-384)
- 出错位置: `frontend/src-tauri/src/lib.rs:497`
