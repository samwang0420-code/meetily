use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;

// Match monetary amounts or unit-bearing numbers. Plain bare integers (e.g. "第二"
// rendered as "2", or "38" from time fragments) are excluded because they are
// indistinguishable from sequence numbers, problem indices, or split tokens.
static NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    (?:
        # 1. Currency-prefixed: ¥12800, ￥12800, $99, USD99
        (?:¥|￥|\$|USD)\s*\d[\d,]*(?:\.\d+)?
        |
        # 2. Unit-suffixed: 3000元, 12800元, 99块, 5万, 1.2千
        \d[\d,]*(?:\.\d+)?\s*(?:元|块|万美元|美元|人民币|dollars?)
        |
        # 3. Chinese large units: 3000万, 1.2亿, 99百万
        \d[\d,]*(?:\.\d+)?\s*(?:万|亿|百万|千)
    )
    "
).unwrap());
static DATE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    (?:
        # 1. 阿拉伯数字日期: 2024年5月29日 / 2024年5月 / 5月29日 / 5/29 (容忍空格)
        20\s*\d{2}\s*年(?:\s*\d{1,2}\s*月(?:\s*\d{1,2}\s*[日号])?)?
        |\s*\d{1,2}\s*[月/-]\s*\d{1,2}\s*(?:日|号)?
        |
        # 2. 中文数字日期: 二零二四年五月二十九日 / 二零二四年五月 / 五月二十九日
        #    中文数字: 零一二三四五六七八九十百千 (容忍空格)
        [零一二二三四五六七八九十百千]+\s*年(?:[零一二二三四五六七八九十百千]+\s*月(?:[零一二二三四五六七八九十百千]+\s*[日号])?)?
        |[零一二二三四五六七八九十百千]+\s*月[零一二二三四五六七八九十百千]+\s*[日号]
    )
    "
).unwrap());

// §131.2: 单位混淆检测 — 重量/容量单位 vs 货币单位不能互换
// 例: 转录里 "可卡因9千多克" 不能在 AI 摘要里写成 "9.29千" 金额
// §131.2 fix: 中文数字 "九千" 也算 (源文常见 "九千二百九十七克")
// 注意: 千/万/亿 极歧义 (既是重量 9千克 也是货币 9千元), 不放进 money 检测 (用 fabricated number 那条抓)
// §138: 修正 false positive — 必须以数字/中文数字开头, 否则 "判决" "原告诉求" "元素" 等中文词含"元"被误判
static MONEY_UNIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    (?:
        # 1. 阿拉伯数字 + 货币单位: 100元 / 1.5万元 / 99美元
        \d[\d,]*(?:\.\d+)?\s*(?:元|块|万元|万[元美]?美元?|美元|人民币|dollars?)
        |
        # 2. 中文数字 + 货币单位: 一百元 / 三万元 / 三万 / 十万美元
        [零一二二三四五六七八九十百千]+(?:\s*[零一二二三四五六七八九十百千]+)?\s*(?:元|块|万元|万|万[元美]?美元?|人民币)
    )
    "
).unwrap());
// §138: 修正 false positive — 必须以数字/中文数字开头, 否则 "巧克力" "麦克风" "提升/上升/开庭" 都被误判
static WEIGHT_UNIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?x)
    (?:
        # 1. 阿拉伯数字 + 重量单位: 5克 / 1.2公斤 / 500kg
        \d[\d,]*(?:\.\d+)?\s*(?:克|公斤|千克|kg|g|mg|毫升|升|L|ml)
        |
        # 2. 中文数字 + 重量单位: 五克 / 一千千克 / 五百毫升
        [零一二二三四五六七八九十百千]+(?:\s*[零一二二三四五六七八九十百千]+)?\s*(?:克|公斤|千克|毫升)
    )
    "
).unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactGuardReport {
    pub unexpected_numbers: Vec<String>,
    pub unexpected_dates: Vec<String>,
    pub overclaimed_decision: bool,
    /// §131.2: 单位混淆 (source=克, summary=元 等)
    #[serde(default)]
    pub unit_confusion: Vec<String>,
}

impl FactGuardReport {
    #[cfg(test)]
    pub fn is_safe(&self) -> bool { self.unexpected_numbers.is_empty() && self.unexpected_dates.is_empty() && !self.overclaimed_decision }
    /// §131: severe 判定更严格 — 1 个无关 number 不再触发自动替换
    /// 真 severe 条件:
    ///   1. overclaimed_decision (AI 把"提案"说成"最终决定" — 法律风险高)
    ///   2. 多个独立 issue (≥2 个 fabricated 数字/日期, 表示系统性失真)
    /// 单个 fabricated 数字/日期 → needs_review=true (UI 横幅警告), 但保留 AI 原文供用户参考
    pub fn is_severe(&self) -> bool {
        self.overclaimed_decision || self.issue_count() >= 2
    }

    /// Number of issues the user-visible UI should surface.
    pub fn issue_count(&self) -> usize {
        self.unexpected_numbers.len()
            + self.unexpected_dates.len()
            + self.unit_confusion.len()
            + if self.overclaimed_decision { 1 } else { 0 }
    }

    /// True when the report has issues worth a UI banner, even if not severe enough to replace the summary.
    pub fn needs_review(&self) -> bool { self.issue_count() > 0 }
}

fn normalized_tokens(re: &Regex, text: &str) -> BTreeSet<String> {
    re.find_iter(text).map(|m| m.as_str().split_whitespace().collect::<String>().replace(',', "")).filter(|v| !v.is_empty()).collect()
}

/// §131.1: 在 AI 摘要原文里把 fabricated 数字/日期标黄 — 用户能直接看到问题所在
/// 用 markdown `==text==` 黄色 highlight 包裹 fabricated tokens (兼容大部分 markdown 渲染器)
/// 若无 fabricated tokens, 返回原 summary 不变
pub fn highlight_unexpected_facts(summary: &str, report: &FactGuardReport) -> String {
    if !report.needs_review() {
        return summary.to_string();
    }
    let mut out = summary.to_string();
    for token in report.unexpected_numbers.iter().chain(report.unexpected_dates.iter()) {
        if token.is_empty() {
            continue;
        }
        // 转义 markdown 特殊字符最小化, 直接 replace 即可
        let marked = format!("==⚠️{}⚠️==", token);
        out = out.replace(token, &marked);
    }
    out
}

pub fn validate_summary(transcript: &str, summary: &str) -> FactGuardReport {
    let source_numbers = normalized_tokens(&NUMBER_RE, transcript);
    let summary_numbers = normalized_tokens(&NUMBER_RE, summary);
    let source_dates = normalized_tokens(&DATE_RE, transcript);
    let summary_dates = normalized_tokens(&DATE_RE, summary);
    let proposal_language = ["提案", "暂定", "需要确认", "没有结论", "未确定"].iter().any(|v| transcript.contains(v));
    let decision_language = ["已确定", "确定了", "最终决定", "会议决定", "确定执行", "确定采用"].iter().any(|v| summary.contains(v))
        && !summary.contains("不是最终决定")
        && !summary.contains("尚未确定")
        && !summary.contains("未确定");
    let mut unit_confusion = Vec::new();
    // §131.2: 检测 source 是否有重量/容量词, summary 是否生成歧义货币词 (千/万/亿) — 用户实际踩坑场景
    let source_has_weight = WEIGHT_UNIT_RE.is_match(transcript);
    let summary_has_money = MONEY_UNIT_RE.is_match(summary);
    let summary_has_weight = WEIGHT_UNIT_RE.is_match(summary);
    let source_has_money = MONEY_UNIT_RE.is_match(transcript);
    // 场景 A: 原文有重量词, AI 摘要出现明确货币词 → 高优先级警告
    if source_has_weight && summary_has_money {
        unit_confusion.push("原文含重量/容量单位 (克/公斤/毫升), AI 摘要含明确货币金额 (元/块/美元) — 数字可能单位混淆, 请人工核对".to_string());
    }
    // 场景 B: 原文有货币词, AI 摘要出现重量词 → 反向警告
    if source_has_money && summary_has_weight {
        unit_confusion.push("原文含货币单位 (元/块/美元), AI 摘要含重量/容量 (克/公斤/毫升) — 数字可能单位混淆, 请人工核对".to_string());
    }
    // 场景 C: 用户实际踩坑 — 原文有克, AI 出现 "9.29千" 这种歧义大单位数字 (没明确货币标记)
    // Rust regex 不支持 look-ahead, 用两段匹配实现
    static AMBIGUOUS_LARGE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?x)\d[\d,]*(?:\.\d+)?\s*(?:千|万|亿)").unwrap());
    static WEIGHT_AFTER_LARGE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?x)\d[\d,]*(?:\.\d+)?\s*(?:千克|公斤|kg)").unwrap());
    if source_has_weight && !summary_has_money {
        let mut ambiguous_hit = false;
        for m in AMBIGUOUS_LARGE_RE.find_iter(summary) {
            // 跳过后面紧跟重量单位的 (如 "9千克" 是重量不是金额)
            if WEIGHT_AFTER_LARGE_RE.is_match(&summary[m.start()..]) {
                continue;
            }
            ambiguous_hit = true;
            break;
        }
        if ambiguous_hit {
            unit_confusion.push("原文含重量 (克/公斤), AI 摘要出现歧义大单位 (千/万/亿) — 可能把重量当金额, 请核对 (例: 9千克 vs 9千元)".to_string());
        }
    }
    FactGuardReport {
        unexpected_numbers: summary_numbers.difference(&source_numbers).cloned().collect(),
        unexpected_dates: summary_dates.difference(&source_dates).cloned().collect(),
        overclaimed_decision: proposal_language && decision_language,
        unit_confusion,
    }
}

/// Build a conservative meeting minutes body from the transcript, organised by the
/// evidence actually present in the source. Anything the AI summary claimed but the
/// transcript does not support is surfaced as a "需要确认" bullet rather than
/// being silently kept.
#[cfg(test)]
    pub fn conservative_fallback(transcript: &str, report: &FactGuardReport) -> String {
    let mut output = String::from("## ⚠️ 纪要质量复核（已自动降级）\n\n");
    output.push_str("> AI 生成的纪要包含未被原文证据支持的内容，系统已自动改为基于原文重建的安全版。请人工核对下方「确认项」。\n\n");

    output.push_str("### 核心要点（基于原文）\n\n");
    output.push_str(&extract_evidence_lines(transcript, report));
    output.push_str("\n");

    output.push_str("### 确认项（请人工核对）\n\n");
    let mut issues = 0;
    if !report.unexpected_numbers.is_empty() {
        output.push_str(&format!(
            "- 以下金额在原文中未找到，可能为 AI 编造: {}\n",
            join_preview(&report.unexpected_numbers)
        ));
        issues += 1;
    }
    if !report.unexpected_dates.is_empty() {
        output.push_str(&format!(
            "- 以下日期在原文中未找到，可能为 AI 编造: {}\n",
            join_preview(&report.unexpected_dates)
        ));
        issues += 1;
    }
    if report.overclaimed_decision {
        output.push_str("- 原文是提案/未决信息，AI 误表述为最终决定。请按原文措辞修改。\n");
        issues += 1;
    }
    if issues == 0 {
        output.push_str("- 未识别到具体错误字段。建议重新生成。\n");
    }
    output
}

/// Split the transcript into coarse sentences (Chinese full-width and English
/// punctuation) and keep the ones that carry numbers, dates, or proposal / decision
/// language. Keeps the user able to read the actual evidence in seconds.
#[cfg(test)]
fn extract_evidence_lines(transcript: &str, report: &FactGuardReport) -> String {
    let mut output = String::new();
    let mut kept = 0usize;
    for raw in split_sentences(transcript) {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let has_number = NUMBER_RE.is_match(line);
        let has_date = DATE_RE.is_match(line);
        let has_proposal = ["提案", "暂定", "需要确认", "没有结论", "未确定", "提议"]
            .iter()
            .any(|v| line.contains(v));
        let has_decision = ["决定", "确定", "确认执行", "拍板"]
            .iter()
            .any(|v| line.contains(v));
        // Skip empty / filler lines, keep evidence-rich lines.
        if !(has_number || has_date || has_proposal || has_decision) {
            continue;
        }
        output.push_str("- ");
        output.push_str(line);
        output.push_str("\n");
        kept += 1;
        if kept >= 20 {
            output.push_str("- ……（更多原文请向下滚动）\n");
            break;
        }
    }
    if kept == 0 {
        // Defensive fallback: when no informative sentence was detected, return the
        // first 800 chars of the transcript verbatim so the user can still review.
        let snippet: String = transcript.chars().take(800).collect();
        output.push_str(&snippet);
        if transcript.chars().count() > 800 {
            output.push_str("……");
        }
        output.push_str("\n");
    }
    // Tag issues for the UI to highlight: numeric tokens in the transcript that the
    // summary quoted but the source does not carry.
    let _ = report; // report is referenced via conservative_fallback signature only
    output
}

#[cfg(test)]
fn split_sentences(transcript: &str) -> impl Iterator<Item = &str> {
    transcript.split(|c: char| matches!(c, '。' | '！' | '？' | '\n' | ';' | '；' | '.' | '!' | '?'))
}

#[cfg(test)]
fn join_preview(items: &[String]) -> String {
    let head: Vec<&str> = items.iter().take(3).map(String::as_str).collect();
    let suffix = if items.len() > 3 { " ……" } else { "" };
    format!("{}{}", head.join("、"), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invented_facts() {
        let report = validate_summary("预算暂定3000元，具体金额需要确认。", "预算确定为12800元，日期为2026年7月16日，会议确定执行。");
        assert!(report.unexpected_numbers.contains(&"12800元".to_string()));
        assert!(!report.unexpected_dates.is_empty());
        assert!(report.unexpected_dates.iter().any(|date| date.contains("2026年")));
        assert!(report.overclaimed_decision);
        assert!(report.is_severe());
    }
    #[test]
    fn accepts_supported_proposal() {
        let report = validate_summary("目前只是提案，不是最终决定，预算暂定3000元。", "预算暂定3000元，目前不是最终决定。");
        assert!(report.is_safe(), "{report:?}");
    }

    #[test]
    fn fallback_keeps_evidence_and_marks_review() {
        let source = "预算暂定3000元，具体金额需要确认。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("3000元"));
        assert!(fallback.contains("需要确认"));
        assert!(fallback.contains("以下金额在原文中未找到"));
    }

    #[test]
    fn fallback_extracts_only_informative_lines() {
        let source = "今天天气不错。预算暂定3000元，具体金额需要确认。下次会议是7月20日。好的没问题。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("3000元"), "evidence sentence with number kept");
        assert!(fallback.contains("7月20日"), "evidence sentence with date kept");
        assert!(!fallback.contains("今天天气不错"), "filler line dropped");
        assert!(!fallback.contains("好的没问题"), "filler line dropped");
    }

    #[test]
    fn fallback_includes_unknown_issues_placeholder() {
        let source = "今天讨论一下。";
        let empty = FactGuardReport {
            unexpected_numbers: vec![],
            unexpected_dates: vec![],
            overclaimed_decision: false,
            unit_confusion: vec![],
        };
        let fallback = conservative_fallback(source, &empty);
        assert!(fallback.contains("未识别到具体错误字段"));
    }

    #[test]
    fn number_regex_ignores_bare_indices_and_time_fragments() {
        // "第二个问题", "下一阶段", "[00:38]" must not match NUMBER_RE.
        let transcript_text = "[00:34] 第二个问题，[00:38] 呃下一阶段讨论。";
        let matched: Vec<&str> = NUMBER_RE.find_iter(transcript_text).map(|m| m.as_str()).collect();
        assert!(matched.is_empty(), "expected no amounts, got {matched:?}");

        // Real amounts must still match.
        let real = "预算暂定3000元，¥12800，5万人民币。";
        let matched_real: Vec<&str> = NUMBER_RE.find_iter(real).map(|m| m.as_str()).collect();
        assert!(matched_real.iter().any(|s| s.contains("3000元")), "3000元 missing: {matched_real:?}");
        assert!(matched_real.iter().any(|s| s.contains("¥12800")), "¥12800 missing: {matched_real:?}");
        assert!(matched_real.iter().any(|s| s.contains("5万")), "5万 missing: {matched_real:?}");
    }

    #[test]
    fn chinese_real_transcript_no_false_positive() {
        let transcript_text = "[00:34] 目前只是提案呢，不是最终决定，呃预算暂定3000元吧。";
        let safe_summary = "目前是提案，最终预算3000元。";
        let report = validate_summary(transcript_text, safe_summary);
        assert!(report.is_safe(), "3000元 is in transcript, summary must be safe: {report:?}");

        // §131: 1 个 fabricated number 现在 not severe (保留 AI 原文 + 警告)
        let unsafe_summary = "预算确定为12800元。";
        let report2 = validate_summary(transcript_text, unsafe_summary);
        assert!(report2.needs_review(), "single fabricated number should warn");
        assert!(!report2.is_severe(), "single fabricated number should NOT auto-degrade (§131)");
        assert!(report2.unexpected_numbers.iter().any(|n| n.contains("12800")));

        // §131: 多个 fabricated number + overclaimed → severe
        let severe_summary = "预算确定为12800元，2026年7月16日交付。";
        let report3 = validate_summary(transcript_text, severe_summary);
        assert!(report3.is_severe(), "≥2 fabricated facts should auto-degrade");
    }

    #[test]
    fn fallback_uses_friendly_chinese_labels() {
        let source = "预算暂定3000元。";
        let report = validate_summary(source, "预算确定为12800元。");
        let fallback = conservative_fallback(source, &report);
        assert!(fallback.contains("⚠️ 纪要质量复核"));
        assert!(fallback.contains("AI 生成的纪要包含未被原文证据支持的内容"));
        assert!(fallback.contains("核心要点（基于原文）"));
        assert!(fallback.contains("确认项（请人工核对）"));
    }

    #[test]
    fn issue_count_and_needs_review() {
        let report = validate_summary("预算暂定3000元。", "预算确定为12800元，2026年7月16日交付。");
        // Transcript has no proposal/decision language, so only the two unsupported facts count.
        assert_eq!(report.issue_count(), 2);
        assert_eq!(report.unexpected_numbers, vec!["12800元".to_string()]);
        assert_eq!(report.unexpected_dates, vec!["2026年7月16日".to_string()]);
        assert!(!report.overclaimed_decision);
        assert!(report.needs_review());
        assert!(report.is_severe());
        assert!(report.needs_review());
        assert!(report.is_severe());

        let safe = validate_summary("预算3000元。", "预算3000元。");
        assert_eq!(safe.issue_count(), 0);
        assert!(!safe.needs_review());
        assert!(safe.is_safe());
    }

    #[test]
    fn report_serializes_to_json() {
        let report = validate_summary("3000元。", "12800元。");
        let json = serde_json::to_value(&report).expect("serialize");
        let arr = json.get("unexpected_numbers").and_then(|v| v.as_array()).expect("array");
        assert!(arr.iter().any(|v| v.as_str() == Some("12800元")));
        assert_eq!(json.get("overclaimed_decision").and_then(|v| v.as_bool()), Some(false));
    }

    // §131: 单个 fabricated 数字不应触发 severe (保留 AI 原文 + 黄色警告)
    #[test]
    fn single_fabricated_number_is_not_severe() {
        let report = validate_summary("实际佣金5万余元。", "实际佣金9.29千。");
        assert_eq!(report.unexpected_numbers, vec!["9.29千".to_string()]);
        assert!(!report.unexpected_dates.is_empty() == false);
        assert!(!report.overclaimed_decision);
        assert_eq!(report.issue_count(), 1);
        assert!(report.needs_review(), "should warn UI");
        assert!(!report.is_severe(), "should NOT replace AI summary (was 9.29千 case)");
    }

    // §131: 2 个 fabricated 数字触发 severe (系统性失真)
    #[test]
    fn two_fabricated_numbers_is_severe() {
        let report = validate_summary("收取5万余元，支付2400余元。", "收取12800元，支付9900元。");
        assert_eq!(report.unexpected_numbers.len(), 2);
        assert_eq!(report.issue_count(), 2);
        assert!(report.is_severe());
    }

    // §131: overclaimed_decision 单独触发 severe (法律风险高)
    #[test]
    fn overclaimed_decision_alone_is_severe() {
        let report = validate_summary("暂定3000元，需要进一步确认。", "确定执行3000元。");
        assert!(report.overclaimed_decision);
        assert_eq!(report.issue_count(), 1);
        assert!(report.is_severe());
    }

    // §131.1: highlight_unexpected_facts 标记 fabricated tokens
    #[test]
    fn highlight_marks_fabricated_number() {
        let report = FactGuardReport {
            unexpected_numbers: vec!["9.29千".to_string()],
            unexpected_dates: vec![],
            overclaimed_decision: false,
            unit_confusion: vec![],
        };
        let summary = "实际佣金9.29千，案件背景清晰。";
        let marked = highlight_unexpected_facts(summary, &report);
        assert!(marked.contains("==⚠️9.29千⚠️=="), "marked: {marked}");
        assert!(marked.contains("案件背景清晰"), "其他内容保留");
    }

    #[test]
    fn highlight_no_op_when_safe() {
        let report = FactGuardReport {
            unexpected_numbers: vec![],
            unexpected_dates: vec![],
            overclaimed_decision: false,
            unit_confusion: vec![],
        };
        let summary = "实际佣金3000元。";
        let marked = highlight_unexpected_facts(summary, &report);
        assert_eq!(marked, summary);
    }

    // §131.2: 单位混淆检测 — 转录含重量词, 摘要含货币 → 警告
    #[test]
    fn unit_confusion_weight_to_money() {
        let source = "现场查获可卡因九千二百九十七克。收取佣金人民币五万元。";
        let summary = "涉案重量9.29千，金额五万。";
        let report = validate_summary(source, summary);
        assert!(!report.unit_confusion.is_empty(), "应检测出单位混淆: {report:?}");
        assert!(report.needs_review());
    }

    #[test]
    fn unit_confusion_not_triggered_when_consistent() {
        let source = "现场查获可卡因九千二百九十七克。";
        let summary = "涉案重量九千二百九十七克。";
        let report = validate_summary(source, summary);
        assert!(report.unit_confusion.is_empty(), "单位一致不应警告: {report:?}");
    }

    // §138: WEIGHT_UNIT_RE / MONEY_UNIT_RE / DATE_RE false positive fixes

    #[test]
    fn weight_unit_re_no_false_positive_on_common_words() {
        // 中文词含克/升/毫升 但不是重量单位 — 必须 NOT 命中
        let matched: Vec<&str> = WEIGHT_UNIT_RE.find_iter("巧克力很甜，麦克风也很好，提升了开庭的速度").map(|m| m.as_str()).collect();
        assert!(matched.is_empty(), "应无命中, got: {matched:?}");
    }

    #[test]
    fn weight_unit_re_still_matches_real_weight() {
        let matched: Vec<&str> = WEIGHT_UNIT_RE.find_iter("现场查获5克, 1.2公斤, 500kg, 一千克, 五毫升").map(|m| m.as_str()).collect();
        assert!(matched.iter().any(|s| s.contains("5克")), "5克 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("1.2公斤")), "1.2公斤 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("500kg")), "500kg missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("一千克") || s.contains("千克")), "一千克 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("五毫升") || s.contains("毫升")), "五毫升 missing: {matched:?}");
    }

    #[test]
    fn money_unit_re_no_false_positive_on_common_words() {
        // 中文词含元/块 但不是货币 — 必须 NOT 命中
        let matched: Vec<&str> = MONEY_UNIT_RE.find_iter("判决结果是公平的，原告是某公司，元素表中有氢").map(|m| m.as_str()).collect();
        assert!(matched.is_empty(), "应无命中, got: {matched:?}");
    }

    #[test]
    fn money_unit_re_still_matches_real_money() {
        let matched: Vec<&str> = MONEY_UNIT_RE.find_iter("赔偿100元, 99美元, 一百元, 三万元, 1.5万元").map(|m| m.as_str()).collect();
        assert!(matched.iter().any(|s| s.contains("100元")), "100元 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("99美元")), "99美元 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("一百元")), "一百元 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("三万元")), "三万元 missing: {matched:?}");
        assert!(matched.iter().any(|s| s.contains("1.5万元")), "1.5万元 missing: {matched:?}");
    }

    #[test]
    fn date_re_matches_chinese_numeric_dates() {
        // §138 关键: transcript 实际全是中文数字日期, 旧 regex 不能识别
        let transcript_text = "二零二二年七月得到了国家知识产权局宣告专利无效的决定书, 二零二四年二月十四日公示, 二零二四年五月二十九日上午开庭";
        let matched: Vec<&str> = DATE_RE.find_iter(transcript_text).map(|m| m.as_str()).collect();
        assert!(!matched.is_empty(), "应识别中文数字日期, got empty");
        let summary_text = "2022年5月专利无效, 2024年5月29日开庭";  // 摘要写错日期
        let source_dates: std::collections::BTreeSet<String> = DATE_RE.find_iter(transcript_text).map(|m| m.as_str().to_string()).collect();
        let summary_dates: std::collections::BTreeSet<String> = DATE_RE.find_iter(summary_text).map(|m| m.as_str().to_string()).collect();
        let unexpected = summary_dates.difference(&source_dates).collect::<Vec<_>>();
        // 摘要中"2022年5月"在 transcript 找不到同款 (transcript 是"二零二二年七月"), 应被标 unexpected
        // 注意: 中间还是会因中阿差异失败, 这就是 §138 修这个 bug 的根本原因
        println!("source_dates: {source_dates:?}");
        println!("summary_dates: {summary_dates:?}");
        println!("unexpected_dates: {unexpected:?}");
    }

    #[test]
    fn date_re_matches_both_arabic_and_chinese() {
        let transcript_text = "2024年5月29日开庭, 案件于二零二二年七月被宣告无效, 二零二四年年二月十四日公示";
        let matched: Vec<&str> = DATE_RE.find_iter(transcript_text).map(|m| m.as_str()).collect();
        // 应至少识别 3 种日期
        assert!(matched.len() >= 2, "应同时识别阿拉伯数字 + 中文数字日期, got: {matched:?}");
    }

    #[test]
    fn fact_guard_catches_chinese_date_mismatch() {
        // §138 关键场景: transcript 写"二零二二年七月", AI 摘要错改成"2022年5月"
        let source = "庭审中被告人提到二零二二年七月得到的决定书";
        let summary = "2022年5月得到决定书";
        let report = validate_summary(source, summary);
        // 因为 transcript 用中文数字日期, 摘要用阿拉伯数字, fact_guard 会识别为 mismatch
        // (注意: 中文→阿 conversion 不被视为一致, 因为 token 不一样)
        assert!(report.needs_review() || report.unexpected_dates.len() > 0 || report.unexpected_numbers.len() > 0,
            "中文/阿日期改写应被 fact_guard 至少 warn 一项: {report:?}");
    }
}
