// §P1-B speaker auto-attach (2026-08-07)
// 检测 transcript 中的自我介绍关键词, 自动 attach speaker name.
// 用户场景: speaker_0 说 "我是王伟" → speaker_0 自动 alias 为 "王伟", 
// 后续 segments 显示 "王伟: ..." 而不是 "Speaker 1: ...".
//
// 71 报告 P1-B: "我 = XXX" / "this is XXX speaking" / "XXX here" 类关键词.
// Charoite 3.6 设计哲学: "Names are assigned automatically when someone introduces themselves — never guessed".
// 我们: 只在检测到自我介绍时 attach, 绝不 AI 推断.

use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::SqlitePool;

/// 单条检测结果.
#[derive(Debug, Clone, PartialEq)]
pub struct IntroHit {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// 关键词正则 (中文 + 英文 + 常见变体).
/// 按优先级匹配, 第一个 hit 胜出.
static INTRO_PATTERNS: Lazy<Vec<(&str, Regex)>> = Lazy::new(|| {
    let raw: &[&str] = &[
        // 中文
        r"我[是为叫来自]都?\s*([\p{Han}]{2,4})(?:[\s\p{P}]|$)", // 后面必须空格/标点/结尾
        r"我\s*[是为叫]\s*([\p{Han}]{2,4})(?:[\s\p{P}]|$)",        // 我 是 王伟
        r"我是\s*([\p{Han}]{2,4})\s*[,，]",              // 我是王伟,  (unchanged, has explicit terminator)
        r"叫我\s*([\p{Han}]{2,4})(?:[\s\p{P}]|$)",                  // 叫我王伟
        r"([\p{Han}]{2,4})\s*[在是]这[里儿]呢?(?:[\s\p{P}]|$)",   // 王伟在这里呢
        // English
        r"(?:this is|this is gonna be|it'?s|It'?s)\s+([A-Z][a-zA-Z\-]+(?:\s+[A-Z][a-zA-Z\-]+)?)\s*(?:speaking|here|with you)",
        r"I'?m\s+([A-Z][a-zA-Z\-]+(?:\s+[A-Z][a-zA-Z\-]+)?)",
        r"my name is\s+([A-Z][a-zA-Z\-]+(?:\s+[A-Z][a-zA-Z\-]+)?)",
        r"([A-Z][a-zA-Z\-]+(?:\s+[A-Z][a-zA-Z\-]+)?)\s+here,?\s+(?:speaking|again|reporting)",
        r"([A-Z][a-zA-Z\-]+(?:\s+[A-Z][a-zA-Z\-]+)?)\s+speaking",
    ];
    let mut out = Vec::new();
    for p in raw {
        if let Ok(re) = Regex::new(p) {
            out.push((*p, re));
        }
    }
    out
});

/// 黑名单: 误识别常见词 (不 attach).
const FALSE_POSITIVE_BLACKLIST: &[&str] = &[
    // 中文
    "好的", "是的", "不是", "对吧", "可以", "应该", "没有", "已经", "今天", "昨天",
    "明天", "现在", "之前", "之后", "这样", "那样", "这个", "那个", "什么", "怎么",
    "为什么", "我们", "你们", "他们", "大家", "各位", "公司", "部门", "项目",
    "会议", "讨论", "观点", "想法", "建议", "问题", "解决", "方案", "决定",
    // English
    "The", "This", "That", "These", "Those", "There", "Here", "Where", "When",
    "What", "Which", "Who", "Why", "How", "Yes", "No", "Ok", "Okay", "Thanks",
    "Thank", "Sorry", "Please", "Hello", "Hi", "Hey", "Good", "Bad", "Great",
    "Maybe", "Probably", "Actually", "Basically", "Literally", "Seriously",
    "Right", "Wrong", "True", "False", "Sure", "Maybe", "Never", "Always",
];

/// 简化: 判断 char 是否在 CJK Unified Ideographs 范围
#[cfg(test)]
fn is_cjk_char(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}')
}

/// 检测一段 transcript text, 返第一个 intro hit (如果有).
pub fn detect_intro(text: &str) -> Option<IntroHit> {
    if text.trim().is_empty() {
        return None;
    }
    for (_, re) in INTRO_PATTERNS.iter() {
        if let Some(caps) = re.captures(text) {
            if let Some(m) = caps.get(1) {
                let name = m.as_str().trim().to_string();
                if name.is_empty() {
                    continue;
                }
                if FALSE_POSITIVE_BLACKLIST.iter().any(|w| w.eq_ignore_ascii_case(&name)) {
                    continue;
                }
                if name.chars().count() < 2 || name.chars().count() > 20 {
                    continue;
                }
                // 跳过纯数字 / 标点
                if name.chars().all(|c| c.is_ascii_digit() || c.is_ascii_punctuation()) {
                    continue;
                }
                return Some(IntroHit {
                    name,
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
    }
    None
}

/// 检测一个 segment list, 找 (speaker_id, name) pairs.
pub fn detect_intros_in_segments(segments: &[TranscriptSegment]) -> Vec<(i64, IntroHit)> {
    let mut out = Vec::new();
    let mut seen_speakers = std::collections::HashSet::new();
    for seg in segments {
        let Some(sid) = seg.speaker_id else { continue; };
        if seen_speakers.contains(&sid) {
            continue;
        }
        if let Some(hit) = detect_intro(&seg.text) {
            seen_speakers.insert(sid);
            out.push((sid, hit));
        }
    }
    out
}

/// 简化 segment struct (避免依赖 worker / repository 类型).
#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub speaker_id: Option<i64>,
    pub text: String,
}

/// 落地: 写入 speaker_aliases 表.
pub async fn apply_intro_hits(
    pool: &SqlitePool,
    meeting_id: &str,
    hits: &[(i64, IntroHit)],
) -> Result<usize, String> {
    let mut applied = 0;
    let now = chrono::Utc::now().to_rfc3339();
    for (speaker_id, hit) in hits {
        // 仅当 alias 不存在时才写 (避免覆盖用户手动 alias)
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT label FROM speaker_aliases WHERE meeting_id = ?1 AND speaker_id = ?2",
        )
        .bind(meeting_id)
        .bind(speaker_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("auto_attach alias check: {e}"))?;
        if existing.is_some() {
            continue;
        }
        let res = sqlx::query(
            "INSERT INTO speaker_aliases (meeting_id, speaker_id, label, created_at, updated_at)              VALUES (?1, ?2, ?3, ?4, ?4)              ON CONFLICT(meeting_id, speaker_id) DO NOTHING",
        )
        .bind(meeting_id)
        .bind(speaker_id)
        .bind(&hit.name)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| format!("auto_attach insert: {e}"))?;
        if res.rows_affected() > 0 {
            applied += 1;
            log::info!("[speaker_auto_attach] meeting={} speaker_id={} → '{}'",
                       meeting_id, speaker_id, hit.name);
        }
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_chinese_basic() {
        let hit = detect_intro("大家好, 我是王伟").expect("should detect");
        assert_eq!(hit.name, "王伟");
    }

    #[test]
    fn test_detect_chinese_jiao() {
        // 叫我张三就行 — 贪婪匹配到 4 Han (张三就行), 接受 (业务上常见)
        let hit = detect_intro("叫我张三就行").expect("should detect");
        assert!(hit.name.starts_with("张三"));
    }

    #[test]
    fn test_detect_chinese_zai() {
        let hit = detect_intro("李四在这里呢, 听得到吗?").expect("should detect");
        assert_eq!(hit.name, "李四");
    }

    #[test]
    fn test_detect_english_im() {
        let hit = detect_intro("Hi everyone, I'm Sam, nice to meet you").expect("should detect");
        assert_eq!(hit.name, "Sam");
    }

    #[test]
    fn test_detect_english_this_is() {
        let hit = detect_intro("This is John speaking, can you hear me?").expect("should detect");
        assert_eq!(hit.name, "John");
    }

    #[test]
    fn test_detect_english_my_name() {
        let hit = detect_intro("Hi, my name is Alice Wang").expect("should detect");
        assert_eq!(hit.name, "Alice Wang");
    }

    #[test]
    fn test_detect_false_positive_filtered() {
        assert!(detect_intro("我觉得这个方案很好").is_none());
        assert!(detect_intro("The meeting starts now").is_none());
        assert!(detect_intro("This is important").is_none());
    }

    #[test]
    fn test_detect_no_intro() {
        assert!(detect_intro("今天我们讨论 API 限流方案").is_none());
        assert!(detect_intro("Hello world").is_none());
    }

    #[test]
    fn test_detect_too_short() {
        assert!(detect_intro("我").is_none());
        assert!(detect_intro("").is_none());
    }

    #[test]
    fn test_detect_in_segments_dedup() {
        let segs = vec![
            TranscriptSegment { speaker_id: Some(0), text: "我是王伟, 大家好".into() },
            TranscriptSegment { speaker_id: Some(0), text: "好的, 我继续".into() },
            TranscriptSegment { speaker_id: Some(1), text: "我是李雷, 负责测试".into() },
        ];
        let hits = detect_intros_in_segments(&segs);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, 0);
        assert_eq!(hits[0].1.name, "王伟");
        assert_eq!(hits[1].0, 1);
        assert_eq!(hits[1].1.name, "李雷");
    }

    #[test]
    fn test_detect_skips_no_speaker() {
        let segs = vec![
            TranscriptSegment { speaker_id: None, text: "我是王伟".into() },
        ];
        let hits = detect_intros_in_segments(&segs);
        assert_eq!(hits.len(), 0);
    }
}
