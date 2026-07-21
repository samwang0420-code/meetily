// 离线会记 v0.6.10+: 配额 (免费版限制 / Pro 版不限)
// 商业化入口阀门. 没有配额 = 没有付费墙.
// 
// 设计:
// - 未登录用户: 一次免费试用 (1 个会议保存后强制登录)
// - 注册用户 (free): 每月 max 5 次录音, 每次转录 max 100 段
// - Pro 用户 (member): 无上限
//
// 配额按"自然月" 算 (UTC, 月初清零)

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuotaStatus {
    pub tier: String,              // "anonymous" | "free" | "member"
    pub month_meetings_used: i64,  // 这个月已用
    pub month_meetings_limit: i64, // 限额 (member=-1 表示无限)
    pub segments_per_transcript_limit: i64, // 单次转录的段数上限
    pub can_record: bool,          // 是否能开始录音
    pub reason: Option<String>,    // 不能的原因
}

pub const FREE_MONTHLY_MEETING_LIMIT: i64 = 5;
pub const FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT: i64 = 100;
pub const ANONYMOUS_FREE_RECORDINGS: i64 = 1;  // 未登录能录 1 次

/// 计算本月 (UTC) 的 meeting 计数
pub fn current_month_key() -> String {
    use chrono::Utc;
    let now = Utc::now();
    format!("{:04}-{:02}", now.format("%Y"), now.format("%m"))
}

/// 配额判定 (给定用户信息 + 本月已用次数)
pub fn compute_quota(
    tier: &str,
    month_meetings_used: i64,
) -> QuotaStatus {
    match tier {
        "member" => QuotaStatus {
            tier: "member".into(),
            month_meetings_used,
            month_meetings_limit: -1,
            segments_per_transcript_limit: -1,
            can_record: true,
            reason: None,
        },
        "anonymous" => QuotaStatus {
            tier: "anonymous".into(),
            month_meetings_used,
            month_meetings_limit: ANONYMOUS_FREE_RECORDINGS,
            segments_per_transcript_limit: FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT,
            can_record: month_meetings_used < ANONYMOUS_FREE_RECORDINGS,
            reason: if month_meetings_used >= ANONYMOUS_FREE_RECORDINGS {
                Some("试用已达上限,请注册后继续使用".into())
            } else {
                None
            },
        },
        _ => QuotaStatus {  // "free"
            tier: "free".into(),
            month_meetings_used,
            month_meetings_limit: FREE_MONTHLY_MEETING_LIMIT,
            segments_per_transcript_limit: FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT,
            can_record: month_meetings_used < FREE_MONTHLY_MEETING_LIMIT,
            reason: if month_meetings_used >= FREE_MONTHLY_MEETING_LIMIT {
                Some("本月免费额度已用完,请升级到 Pro".into())
            } else {
                None
            },
        },
    }
}


/// v0.7.x: 按 membership tier 截断 transcript segments.
/// member (Pro) 不截断 (返回 -1 = 无限制); free / anonymous 用 FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT.
pub fn truncate_segments_for_tier<T>(segments: &mut Vec<T>, tier: &str) -> i64 {
    let limit: i64 = match tier {
        "member" => i64::MAX,
        _ => FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT,
    };
    if (segments.len() as i64) > limit {
        segments.truncate(limit as usize);
        return limit;
    }
    -1  // -1 = 没截断, unlimited
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_member_keeps_all() {
        let mut v: Vec<u32> = (0..200u32).collect();
        let r = truncate_segments_for_tier(&mut v, "member");
        assert_eq!(r, -1);
        assert_eq!(v.len(), 200);
    }

    #[test]
    fn truncate_free_caps_at_100() {
        let mut v: Vec<u32> = (0..250u32).collect();
        let r = truncate_segments_for_tier(&mut v, "free");
        assert_eq!(r, 100);
        assert_eq!(v.len(), 100);
    }

    #[test]
    fn truncate_anonymous_caps_at_100() {
        let mut v: Vec<u32> = (0..105u32).collect();
        let r = truncate_segments_for_tier(&mut v, "anonymous");
        assert_eq!(r, 100);
        assert_eq!(v.len(), 100);
    }

    #[test]
    fn truncate_under_limit_noop() {
        let mut v: Vec<u32> = (0..50u32).collect();
        let r = truncate_segments_for_tier(&mut v, "free");
        assert_eq!(r, -1);
        assert_eq!(v.len(), 50);
    }

    #[test]
    fn truncate_empty() {
        let mut v: Vec<u32> = Vec::new();
        let r = truncate_segments_for_tier(&mut v, "free");
        assert_eq!(r, -1);
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn truncate_exactly_at_limit() {
        let mut v: Vec<u32> = (0..100u32).collect();
        let r = truncate_segments_for_tier(&mut v, "free");
        assert_eq!(r, -1);
        assert_eq!(v.len(), 100);
    }
}
