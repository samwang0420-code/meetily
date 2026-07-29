//! 离线会记 v0.7.0+: 兑换激活码速率限制 (P1 安全加固 #1)
//!
//! 防 DoS / DB 刷: 单 user 在滑动窗口 (默认 60s) 内最多 N 次 (默认 5) redeem_attempt 尝试.
//! 内存计数, 重启清零, 可接受 (攻击者重启 app 才能清计数 == 失去原有 session, 无意义).
//!
//! 设计取舍:
//! - 内存而非 DB: 速率限制是「防 DoS」不是「业务逻辑」, 重启清零可接受.
//! - 按 user_id 限: 用户共享 machine 不会互相影响.
//! - 滑动窗口 (Vec<Instant>): 简单 + 精确, 1 分钟 5 次不会因为边界效应放过 burst.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 单用户兑换尝试的最大次数 (60s 内)
pub const MAX_ATTEMPTS_PER_WINDOW: usize = 5;
/// 滑动窗口长度
pub const WINDOW: Duration = Duration::from_secs(60);

/// 全局计数器: user_id -> [Instant; ...] (按时间倒序追加, 旧的在前面)
static ATTEMPTS: once_cell::sync::Lazy<Mutex<HashMap<i64, Vec<Instant>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// 清理过期时间戳 (避免 map 无限增长)
fn prune(user_attempts: &mut Vec<Instant>, now: Instant) {
    user_attempts.retain(|&t| now.duration_since(t) < WINDOW);
}

/// 检查是否允许尝试; 如果允许, 记录这次尝试 (返回 Ok); 否则返回 Err
///
/// 调用时机: redeem 函数最前面 (先于格式校验)
pub fn check_and_record(user_id: i64) -> Result<(), RateLimited> {
    let now = Instant::now();
    let mut map = ATTEMPTS.lock().unwrap_or_else(|p| p.into_inner());
    let entry = map.entry(user_id).or_default();
    prune(entry, now);
    if entry.len() >= MAX_ATTEMPTS_PER_WINDOW {
        // 算最早一次还有多久可以重试
        let earliest = entry.iter().min().copied().unwrap_or(now);
        let retry_after = WINDOW.saturating_sub(now.duration_since(earliest));
        return Err(RateLimited {
            retry_after_secs: retry_after.as_secs(),
        });
    }
    entry.push(now);
    Ok(())
}

/// 速率限制触发时返回
#[derive(Debug, Clone)]
pub struct RateLimited {
    pub retry_after_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    /// 注意: tests 在同一进程跑, 共享 static, 测试间不要冲突
    const TEST_USER: i64 = 99_999;

    #[test]
    fn allows_up_to_max_then_blocks() {
        // 5 次连续应全部 OK
        for i in 0..MAX_ATTEMPTS_PER_WINDOW {
            let r = check_and_record(TEST_USER);
            assert!(r.is_ok(), "attempt #{} should pass, got {:?}", i + 1, r);
        }
        // 第 6 次应被拦
        let r = check_and_record(TEST_USER);
        assert!(r.is_err(), "6th attempt should be blocked");
        let err = r.unwrap_err();
        assert!(
            err.retry_after_secs <= 60,
            "retry_after must be within window: {}",
            err.retry_after_secs
        );
    }

    #[test]
    fn window_slides_correctly() {
        // 假设一个全新的 user, 先灌满 5 次
        let user: i64 = 88_888;
        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert!(check_and_record(user).is_ok());
        }
        // 第 6 次被拒
        assert!(check_and_record(user).is_err());

        // 不真等 60s (太慢), 改用手动清
        let mut map = ATTEMPTS.lock().unwrap();
        map.remove(&user);
        // 清掉后立刻可用
        drop(map);
        assert!(check_and_record(user).is_ok());
    }

    #[test]
    fn per_user_isolation() {
        // 用户 A 满, 用户 B 不受影响
        let user_a: i64 = 77_777;
        let user_b: i64 = 77_778;
        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert!(check_and_record(user_a).is_ok());
        }
        assert!(check_and_record(user_a).is_err(), "A should be blocked");
        assert!(check_and_record(user_b).is_ok(), "B should NOT be blocked");
    }
}
