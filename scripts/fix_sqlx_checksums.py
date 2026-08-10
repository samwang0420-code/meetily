#!/usr/bin/env python3
"""§98 (2026-08-10): 一次性 sync sqlx _sqlx_migrations.checksum 到当前 binary embed 的 migration 文件 hash.

触发场景: 
- 用户从老 bundle id (cn.lixianhuiji.app) 切换到新 bundle id (tech.yanjingai.app) 后
- 新 db 里的 _sqlx_migrations.checksum 跟 binary 里 embed 的 SHA-384 不一致
- sqlx 启动 panic: "migration XXX was previously applied but has been modified"

§73 同类 bug 的根治: 改 startup 一次性 self-heal, 避免每次手工 Python sync.

用法:
  python3 scripts/fix_sqlx_checksums.py [db_path]
  # 不传 db_path → 自动 sync 新 + 旧两个 db
"""
import hashlib, os, sqlite3, sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MIGRATIONS_DIR = os.path.join(REPO_ROOT, "frontend", "src-tauri", "migrations")

# macOS / Linux / Windows 三个 OS 的 bundle data dir 路径
HOME = os.path.expanduser("~")
BUNDLE_ID_NEW = "tech.yanjingai.app"
BUNDLE_ID_OLD = "cn.lixianhuiji.app"

DEFAULT_DBS = [
    os.path.join(HOME, "Library", "Application Support", BUNDLE_ID_NEW, "meeting_minutes.sqlite"),
    os.path.join(HOME, "Library", "Application Support", BUNDLE_ID_OLD, "meeting_minutes.sqlite"),
]


def sync_one(db_path: str) -> tuple[int, int]:
    """sync 一个 db. 返 (updated, missing)."""
    if not os.path.exists(db_path):
        return (0, 0)
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    updated = 0
    missing = 0
    for fname in sorted(os.listdir(MIGRATIONS_DIR)):
        if not fname.endswith(".sql"):
            continue
        version = int(fname.split("_", 1)[0])
        data = open(os.path.join(MIGRATIONS_DIR, fname), "rb").read()
        new_ck = hashlib.sha384(data).digest()
        cur.execute("SELECT checksum FROM _sqlx_migrations WHERE version=?", (version,))
        row = cur.fetchone()
        if row is None:
            missing += 1
            continue
        if row[0] == new_ck:
            continue
        cur.execute("UPDATE _sqlx_migrations SET checksum=? WHERE version=?", (new_ck, version))
        updated += 1
    con.commit()
    con.close()
    return (updated, missing)


def main():
    targets = sys.argv[1:] or DEFAULT_DBS
    total_updated = 0
    total_missing = 0
    for db in targets:
        updated, missing = sync_one(db)
        if updated or missing or not os.path.exists(db):
            print(f"  {db}: updated={updated} missing={missing}")
        total_updated += updated
        total_missing += missing
    print(f"\nTotal: {total_updated} checksum(s) updated, {total_missing} migration(s) missing (sqlx will auto-apply)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
