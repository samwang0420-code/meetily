-- §P1-B15 (audit 2026-08-23): quota_increments table for idempotency.
-- A row exists iff this (user_id, meeting_id) was counted in this month.
-- Re-incrementing the same meeting_id is a no-op (idempotency key).
CREATE TABLE IF NOT EXISTS quota_increments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    meeting_id TEXT NOT NULL,
    month_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, meeting_id, month_key)
);
CREATE INDEX IF NOT EXISTS idx_quota_increments_user_month
    ON quota_increments (user_id, month_key);
