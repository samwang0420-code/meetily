//! §138 P0.2: ASR 错字过滤
//!
//! sherpa-onnx 中文 ASR 偶尔输出严重错字 (e.g. "啊啊啊啊啊", "院二二二二二二").
//! 之前这些错字会直接进 transcripts 表, 然后被 LLM 抄进摘要.
//! 现在在 transcript 写入前 sanitize.

/// §138 P0.2 质量分级
/// - "high": 正常, 无错字
/// - "medium": 有轻度错字 (折叠后保留)
/// - "low": 严重错字, 折叠后仍可疑 (空段 / 单字符 / 单一字符循环)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsrQuality {
    High,
    Medium,
    Low,
}

impl AsrQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            AsrQuality::High => "high",
            AsrQuality::Medium => "medium",
            AsrQuality::Low => "low",
        }
    }
}

/// §138 P0.2: ASR 错字过滤 (主入口)
///
/// 处理步骤:
/// 1. 折叠连续重复字符 (>= 5 次 → 保留 2 次)
/// 2. 截断过长的无标点段 (> 200 字无任何标点 → 截断到 200 + "...")
/// 3. 计算质量分级
///
/// 返回: (sanitized_text, was_modified, quality)
pub fn sanitize_asr_text(text: &str) -> (String, bool, AsrQuality) {
    let original = text;

    // 1) 折叠连续重复字符
    let collapsed = collapse_repeated_chars(text, 5);

    // 2) 截断过长无标点段
    let truncated = truncate_long_no_punct(&collapsed, 200);

    let was_modified = truncated != original;

    // 3) 质量分级
    let quality = if original.trim().is_empty() {
        AsrQuality::Low
    } else {
        let unique_chars: std::collections::HashSet<char> = original.chars().collect();
        let char_count = original.chars().count();
        if char_count > 30 && unique_chars.len() <= 3 {
                // 长段 (>30 字) + 几乎单一字符 (e.g. 100 字 90 个 "啊")
                AsrQuality::Low
            } else if char_count > 0 && (unique_chars.len() as f64 / char_count as f64) < 0.10 {
                // 字符多样性 < 10% (一般中文 70-80%)
                AsrQuality::Medium
        } else {
            AsrQuality::High
        }
    };

    (truncated, was_modified, quality)
}

/// §138 P0.2: 折叠连续重复字符
/// e.g. "啊啊啊啊啊" (5 次) → "啊啊" (2 次)
///       "哈哈哈哈哈哈" (6 次) → "哈哈" (2 次)
///       "哈哈哈" (3 次) → 不变 (低于阈值)
pub fn collapse_repeated_chars(text: &str, threshold: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i + 1;
        while j < chars.len() && chars[j] == chars[i] {
            j += 1;
        }
        let run_len = j - i;
        if run_len >= threshold {
            // 保留 2 个 (e.g. "啊啊啊啊啊啊" → "啊啊")
            out.push(chars[i]);
            out.push(chars[i]);
        } else {
            for k in i..j {
                out.push(chars[k]);
            }
        }
        i = j;
    }
    out
}

/// §138 P0.2: 截断过长无标点段
/// 人类语音 1 句一般 < 100 字, ASR 错字串可能 500+ 字无标点
/// 截断到 max_chars + "..." 避免大段乱码进 DB
pub fn truncate_long_no_punct(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut current_segment = String::new();
    #[allow(dead_code)] // §F: 调试变量,保留以便观察
    let mut _current_segment_no_punct = true;

    for ch in text.chars() {
        if matches!(ch, '。' | '！' | '？' | '.' | '!' | '?' | ';' | '；' | ',' | '，' | '\n' | '\r') {
            // 标点, 提交当前段
            if current_segment.chars().count() > max_chars {
                // 截断
                let truncated: String = current_segment.chars().take(max_chars).collect();
                out.push_str(&truncated);
                out.push_str("...");
            } else {
                out.push_str(&current_segment);
            }
            out.push(ch);
            current_segment.clear();
            _current_segment_no_punct = true;
        } else {
            current_segment.push(ch);
        }
    }
    // 处理最后一段
    if !current_segment.is_empty() {
        if current_segment.chars().count() > max_chars {
            let truncated: String = current_segment.chars().take(max_chars).collect();
            out.push_str(&truncated);
            out.push_str("...");
        } else {
            out.push_str(&current_segment);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_138_collapse_repeated_chars_basic() {
        // 5 次 "啊" → 2 次
        assert_eq!(collapse_repeated_chars("啊啊啊啊啊", 5), "啊啊");
        // 3 次不变
        assert_eq!(collapse_repeated_chars("哈哈哈", 5), "哈哈哈");
        // 6 次 → 2 次
        assert_eq!(collapse_repeated_chars("哈哈哈哈哈哈", 5), "哈哈");
        // 混合: 正常字 + 5 次重复 + 正常字
        assert_eq!(collapse_repeated_chars("今天啊啊啊啊啊天气好", 5), "今天啊啊天气好");
    }

    #[test]
    fn section_138_truncate_long_no_punct() {
        // 短文本不动
        let s = "今天天气不错";
        assert_eq!(truncate_long_no_punct(s, 50), "今天天气不错");
        // 长无标点 → 截断
        let s = "啊".repeat(300);
        let out = truncate_long_no_punct(&s, 100);
        assert!(out.chars().count() < 110, "应截断到 ~100 字, 实际 {}", out.chars().count());
        assert!(out.ends_with("..."));
        // 有标点的不截断
        let s = format!("{}。更多内容", "啊".repeat(50));
        let out = truncate_long_no_punct(&s, 200);
        assert!(out.contains("更多内容"), "有标点的不应截断");
    }

    #[test]
    fn section_138_sanitize_asr_text_low_quality() {
        // 50 个 "啊" (low quality — 长段 + 几乎单一字符)
        let input = "啊".repeat(50);
        let (_text, modified, quality) = sanitize_asr_text(&input);
        assert_eq!(quality, AsrQuality::Low);
        assert!(modified, "应触发折叠");
    }

    #[test]
    fn section_138_sanitize_asr_text_high_quality() {
        // 正常中文
        let original = "今天我们讨论言镜 AI 的 §138 摘要质量优化方案。";
        let (text, modified, quality) = sanitize_asr_text(original);
        assert_eq!(quality, AsrQuality::High);
        assert!(!modified, "正常文本不应被改");
        assert_eq!(text, original);
    }

    #[test]
    fn section_138_sanitize_asr_text_handles_garbled_repeating() {
        // 用户截图案例: "院二二二二二二二二"
        let (text, _modified, quality) = sanitize_asr_text("院二二二二二二二二院");
        // 5+ 重复 → 折叠
        assert!(text.contains("院二院") || text.contains("院二"), "应折叠重复字符, 实际: {}", text);
        // 质量应该是 Medium (有 院 + 二 两种字, 不算 Low)
        assert!(matches!(quality, AsrQuality::Medium | AsrQuality::High));
    }
}

/// §138 P2.1: 别名规范化 (transcript 写入前替换)
///
/// 问题: 转录里"徐氏米业" / "徐某" / "该公司" / "被告" 混用, LLM 摘要时容易乱切.
/// 解决: 在转录写入 DB 之前, 把所有别名替换为 canonical 形式, 让 LLM 看到一致输入.
///
/// 设计: 保守替换. 只做明确的别名映射, 不做"补全"或"猜测" (P1 0 编造).
/// - "徐氏米业公司" / "徐氏米业有限责任公司" → "徐氏米业" (统一公司名)
/// - "该公司" / "该米业" (指代前文已提公司) → 保留 (上下文依赖, 不强改)
/// - "魏某" / "魏某方" → "魏某" (统一)
/// - "魏丽秋" / "魏立秋" (编造的全名, 之前 P1 漏的) → "魏某" (强制回退)
pub fn normalize_aliases(text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0;

    // 公司全称 → 简称
    let replacements: &[(&str, &str)] = &[
        ("徐氏米业有限责任公司", "徐氏米业"),
        ("徐氏米业股份有限公司", "徐氏米业"),
        ("徐氏米业公司", "徐氏米业"),
        ("徐某公司", "徐氏米业"),
        ("徐某方", "徐氏米业"),
        // 个人简称 → 统一
        ("魏某方", "魏某"),
        ("魏丽秋", "魏某"),
        ("魏立秋", "魏某"),
    ];

    for (from, to) in replacements {
        if out.contains(from) {
            count += out.matches(from).count();
            out = out.replace(from, to);
        }
    }

    (out, count)
}

#[cfg(test)]
mod alias_tests {
    use super::*;

    #[test]
    fn section_138_normalize_unifies_company_name() {
        // "徐氏米业公司" → "徐氏米业"
        let (out, n) = normalize_aliases("徐氏米业公司主张");
        assert_eq!(out, "徐氏米业主张");
        assert!(n >= 1);
    }

    #[test]
    fn section_138_normalize_collapses_fabricated_fullname() {
        // 编造全名 → 转录中实际称呼
        let (out, n) = normalize_aliases("魏丽秋提起诉讼, 魏丽秋不服");
        assert_eq!(out, "魏某提起诉讼, 魏某不服");
        assert_eq!(n, 2);
    }

    #[test]
    fn section_138_normalize_no_op_on_canonical() {
        let (out, n) = normalize_aliases("魏某与徐氏米业");
        assert_eq!(out, "魏某与徐氏米业");
        assert_eq!(n, 0);
    }
}
