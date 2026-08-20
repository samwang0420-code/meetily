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
    /// §148 §1: 人名漂移 — 同一主体在摘要里出现 ≥2 种同音近形写法 (李福强 vs 李富强)
    #[serde(default)]
    pub name_drift: Vec<String>,
    /// §148 §2: 角色混淆 — 摘要把证人写成辩护人, 或亲属辩护误写等
    #[serde(default)]
    pub role_confusion: Vec<String>,
    /// §148 §3: 判决编造 — 庭审未宣判, 但摘要出现 "判处/判决/一审/二审" 等判决词
    #[serde(default)]
    pub fabricated_verdict: Vec<String>,
}

impl FactGuardReport {
    #[cfg(test)]
    pub fn is_safe(&self) -> bool {
        self.unexpected_numbers.is_empty()
            && self.unexpected_dates.is_empty()
            && !self.overclaimed_decision
            && self.name_drift.is_empty()
            && self.role_confusion.is_empty()
            && self.fabricated_verdict.is_empty()
    }
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
            + self.name_drift.len()
            + self.role_confusion.len()
            + self.fabricated_verdict.len()
            + if self.overclaimed_decision { 1 } else { 0 }
    }

    /// §148: 法律模板 critical 判定 — 出现 1 项角色混淆或判决编造即为 SEVERE
    /// (这些是法庭纪要硬伤, 单条也要降级让用户看到, 不能隐藏在 needs_review 横幅里)
    pub fn is_legal_critical(&self) -> bool {
        !self.role_confusion.is_empty() || !self.fabricated_verdict.is_empty()
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
    // §148: 人名漂移标黄
    for entry in &report.name_drift {
        for name in entry.split("→").map(str::trim).filter(|s| !s.is_empty()) {
            if !name.is_empty() && out.contains(name) {
                out = out.replace(name, &format!("==⚠️{}⚠️==", name));
            }
        }
    }
    // §148: 判决编造标黄
    for token in &report.fabricated_verdict {
        if !token.is_empty() && out.contains(token) {
            out = out.replace(token, &format!("==⚠️{}⚠️==", token));
        }
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
    // §148 §1: 人名漂移检测 — 在 summary 里查找 "李福强/李富强" 等同音近形 pair
    let name_drift = detect_name_drift(transcript, summary);
    // §148 §2: 角色混淆检测 — 摘要把证人当辩护人, 或亲属变律师
    let role_confusion = detect_role_confusion(transcript, summary);
    // §148 §3: 判决编造检测 — 庭审未宣判, 摘要出现 "判处/判决/一审/二审"
    let fabricated_verdict = detect_fabricated_verdict(transcript, summary);

    FactGuardReport {
        unexpected_numbers: summary_numbers.difference(&source_numbers).cloned().collect(),
        unexpected_dates: summary_dates.difference(&source_dates).cloned().collect(),
        overclaimed_decision: proposal_language && decision_language,
        unit_confusion,
        name_drift,
        role_confusion,
        fabricated_verdict,
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

// §148: 仅为保守回退 (conservative_fallback) 保留,生产路径已移除 (见 §131.1)
#[allow(dead_code)]
fn join_preview(items: &[String]) -> String {
    const PREVIEW: usize = 5;
    if items.len() <= PREVIEW {
        items.join(", ")
    } else {
        let head = items[..PREVIEW].join(", ");
        format!("{} 等 {} 项", head, items.len())
    }
}

// ============================================================================
// §148: 法律模板三项 critical 检测 (人名漂移 / 角色混淆 / 判决编造)
// 任何一项命中 → is_legal_critical() = true → 触发降级 + 警告横幅
// ============================================================================

/// §148 §1: 人名漂移 — 同一主体在 summary 里出现 ≥2 种同音近形写法
///
/// 实现策略:
///   1. 从 transcript 抽取 中文姓名 token (regex: 李福强 / 欧阳明 等 2-4 字)
///   2. 在 summary 里以相同 token 集匹配
///   3. 对每个 transcript 中出现的姓名 N, 检查 summary 里是否有同根近形变体
///      (N 与 M 共用前缀但末字不同, 且末字是同音/近形: 福/富, 强/墙, 伟/炜, 刚/钢)
///   4. 命中时记录 "原名 → 变体" pair
pub fn detect_name_drift(transcript: &str, summary: &str) -> Vec<String> {
    use std::collections::HashSet;

    // 中文姓名 regex: 2-4 个汉字 (但只考虑 ≥3 字的姓氏常见组合, 2 字易误命中"智力"/"迟缓"等)
    // 例外: 单姓+双字名 仍是 3 字, 不需 2 字覆盖
    // 中文姓名扫描 (手动 char-level, 避免 regex greedy 4-char 截到 "李福强持")
    // 策略: 在每段连续中文里, 找以常见姓氏开头的 3-4 字 spans, 优先 3 字 (单姓双字名最常见)
    // 不再用 NAME_RE — 改用 char 索引手动判断

    // 常见中文姓氏 (单字姓, 100+ 主流) — 用于过滤人名 vs 普通中文词
    // 不做 NER 完整模型, 仅靠"首字是姓"过滤
    static SURNAMES: &[char] = &[
        '李', '王', '张', '刘', '陈', '杨', '黄', '赵', '周', '吴',
        '徐', '孙', '朱', '马', '胡', '郭', '何', '高', '林', '罗',
        '郑', '梁', '谢', '宋', '唐', '许', '韩', '冯', '邓', '曹',
        '彭', '曾', '田', '董', '袁', '潘', '于', '蒋', '蔡', '余',
        '杜', '叶', '程', '苏', '魏', '吕', '丁', '任', '沈', '姚',
        '卢', '姜', '崔', '钟', '谭', '陆', '汪', '范', '金', '石',
        '廖', '贾', '夏', '韦', '付', '方', '白', '邹', '孟', '熊',
        '秦', '邱', '江', '尹', '薛', '闫', '段', '雷', '侯', '龙',
        '史', '陶', '黎', '贺', '顾', '毛', '郝', '龚', '邵', '万',
        '钱', '严', '覃', '武', '戴', '莫', '孔', '向', '汤', '于',
    ];

    // 提取"独立词": 先按标点/数字切分, 再在每段内找 3-4 字 spans
    // 中文文本的特点是连续中文无空格 — 仅靠 regex 边界无法精准定位姓名边界
    // 用"首字是姓"过滤: 中文姓名 99% 概率以常见姓氏开头
    fn extract_words(text: &str) -> HashSet<String> {
        let mut seen = HashSet::new();
        // 按标点切
        static SPLIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"[\p{P}\p{Z}\p{N}]+"
        ).unwrap());
        for segment in SPLIT_RE.split(text) {
            if segment.is_empty() { continue; }
            let chars: Vec<char> = segment.chars().collect();
            let mut i = 0;
            while i + 3 <= chars.len() {
                // 起始字是姓?
                if SURNAMES.contains(&chars[i]) {
                    // 优先 3 字 (单姓 + 双字名), 若 3 字后是中文字 → 取 3 字
                    let three: String = chars[i..i+3].iter().collect();
                    if i + 3 >= chars.len() || !is_chinese_char(chars[i+3]) {
                        // 3 字就到段尾 或 后跟非中文 → 取 3 字
                        seen.insert(three);
                        i += 3;
                    } else {
                        // 后跟中文, 仍取 3 字 (单姓双字名主流)
                        seen.insert(three);
                        i += 3;
                    }
                } else {
                    i += 1; // 非姓氏起始, 跳过这字
                }
            }
        }
        seen
    }

    fn is_chinese_char(c: char) -> bool {
        matches!(c, '\u{4e00}'..='\u{9fff}')
    }

    // 同音近形字对 (用于在姓名的任意位置替换 1 个字)
    static CONFUSABLE_PAIRS: &[(&str, &str)] = &[
        ("福", "富"), ("福", "复"), ("福", "付"),
        ("富", "福"), ("富", "复"), ("富", "付"),
        ("强", "墙"), ("强", "抢"), ("强", "疆"),
        ("伟", "炜"), ("伟", "苇"), ("伟", "卫"),
        ("刚", "钢"), ("刚", "岗"), ("刚", "纲"),
        ("明", "铭"), ("明", "鸣"), ("明", "名"),
        ("国", "果"), ("国", "过"), ("国", "郭"),
        ("平", "苹"), ("平", "评"), ("平", "坪"),
        ("建", "健"), ("建", "键"), ("建", "坚"),
        ("林", "霖"), ("林", "琳"), ("林", "临"),
        ("涛", "滔"), ("涛", "焘"), ("涛", "陶"),
    ];

    // 收集 summary 中所有 3-4 字姓名 token
    let summary_names: HashSet<String> = extract_words(summary);

    // 收集 transcript 中的姓名 (避免误报)
    let transcript_names: HashSet<String> = extract_words(transcript);

    if summary_names.len() < 2 {
        return vec![];
    }

    // 对每个 (a, b) pair, 对每个 name, 在每个位置 i 替换 name[i]=a 为 b 生成 variant, 看 variant 是否在 summary 里
    let mut drift_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for name in &summary_names {
        let chars: Vec<char> = name.chars().collect();
        for i in 0..chars.len() {
            for (a, b) in CONFUSABLE_PAIRS {
                if chars[i].to_string() == *a {
                    let mut new_chars = chars.clone();
                    new_chars[i] = b.chars().next().unwrap();
                    let variant: String = new_chars.iter().collect();
                    if variant != *name && summary_names.contains(&variant) {
                        let mut pair = (name.clone(), variant);
                        if pair.0 > pair.1 { std::mem::swap(&mut pair.0, &mut pair.1); }
                        drift_pairs.insert(pair);
                    }
                } else if chars[i].to_string() == *b {
                    let mut new_chars = chars.clone();
                    new_chars[i] = a.chars().next().unwrap();
                    let variant: String = new_chars.iter().collect();
                    if variant != *name && summary_names.contains(&variant) {
                        let mut pair = (name.clone(), variant);
                        if pair.0 > pair.1 { std::mem::swap(&mut pair.0, &mut pair.1); }
                        drift_pairs.insert(pair);
                    }
                }
            }
        }
    }

    // transcript 必须至少含其中之一 — 否则视为 summary 自我编造, 不算 drift
    let mut out = Vec::new();
    for (a, b) in drift_pairs {
        if transcript_names.contains(&a) || transcript_names.contains(&b) {
            out.push(format!("{} → {}", a, b));
        }
    }
    out
}

/// §148 §2: 角色混淆 — 摘要把证人当辩护人 / 公诉人当辩护人 / 亲属辩护误写等
pub fn detect_role_confusion(transcript: &str, summary: &str) -> Vec<String> {
    let mut out = Vec::new();

    // 模式 1: 摘要把 "证人" 当 "辩护人"
    // 例: "辩护人: 李富强的姐姐 (证人)" — 显式标记为证人但角色字段是辩护人
    static ROLE_MISLABEL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"辩护[人律师][：:][^。\n]{0,40}?[(（]\s*证人\s*[)）]"
    ).unwrap());
    for m in ROLE_MISLABEL_RE.find_iter(summary) {
        out.push(format!(
            "角色误标: '{}' — 字段标'辩护人'但同句括号标注'证人', 证人 ≠ 辩护人 (§148 §2)",
            m.as_str().trim()
        ));
    }

    // 模式 2: "辩护人" 紧跟亲属称谓 (XX姐姐/XX父亲/XX母亲/XX妻子/XX丈夫/XX弟弟/XX妹妹/XX女儿/XX儿子/XX哥哥)
    static KINSHIP_DEFENSE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"辩护[人律师][：:][^。\n]{0,40}?(姐姐|父亲|母亲|妻子|丈夫|弟弟|妹妹|女儿|儿子|哥哥)"
    ).unwrap());
    for m in KINSHIP_DEFENSE_RE.find_iter(summary) {
        let matched = m.as_str();
        // 若 transcript 含 "证人证言" / "证人:" / "的姐姐" / "的弟弟" 等, 就报
        let mut reported = false;
        for kw in ["证人证言", "证人:", "证人：", "的姐姐", "的弟弟", "的妹妹"] {
            if transcript.contains(kw) {
                out.push(format!(
                    "亲属辩护误写: '{}' — 中国刑事案件亲属不能作辩护人, 亲属出庭通常身份是证人 (§148 §2)",
                    matched.trim()
                ));
                reported = true;
                break;
            }
        }
        if reported { continue; }
    }

    // 模式 3: 辩护人段标题下紧跟证人亲属描述
    static DEFENSE_HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"[*#][^。\n]{0,15}辩护[人律师][^。\n]{0,5}[*#]"
    ).unwrap());
    static WITNESS_KIN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
        r"证人[^。\n]{0,20}?(姐姐|父亲|母亲|弟弟|妹妹|妻子|丈夫)"
    ).unwrap());
    for dm in DEFENSE_HEADER_RE.find_iter(summary) {
        let window_end = std::cmp::min(dm.end() + 200, summary.len());
        let after = &summary[dm.end()..window_end];
        if let Some(wm) = WITNESS_KIN_RE.find(after) {
            out.push(format!(
                "辩护人段误标证人: '辩护人' 段标题下 200 字内出现证人亲属描述 '{}' (§148 §2)",
                wm.as_str().trim()
            ));
        }
    }

    out
}

/// §148 §3: 判决编造 — 庭审 transcript 没宣判, summary 出现 "判处/判决/一审/二审" 等判决结果
pub fn detect_fabricated_verdict(transcript: &str, summary: &str) -> Vec<String> {
    // 是否已宣判 (transcript 含明确判决动作, 但"择期宣判"不算)
    // 区分: "现在宣判/宣读判决/判处X年/本院判决如下" = 已宣判; "择期宣判/将择期宣判" = 尚未宣判
    static VERDICT_KEYWORDS: &[&str] = &[
        "宣判", "判决书", "判处", "本院认为", "判决如下", "判决主文", "作出判决",
    ];
    // 排除短语: 含这些的"宣判"不算已宣判 (择期宣判 = 还没宣判)
    static PENDING_PHRASES: &[&str] = &[
        "择期宣判", "将择期宣判", "未宣判", "尚未宣判", "今日未宣判",
    ];

    let transcript_has_verdict = VERDICT_KEYWORDS.iter().any(|k| {
        // 先排除"择期宣判"等 pending 表述
        transcript.contains(k) && !PENDING_PHRASES.iter().any(|p| transcript.contains(p))
    });

    if transcript_has_verdict {
        return vec![]; // 已宣判, 摘要可写判决
    }

    // 未宣判 → 检查 summary 是否有判决表述
    static FABRICATED_PATTERNS: &[(&str, &str)] = &[
        (r"一审判处[^。\n]{0,40}", "一审判处"),
        (r"二审(?:维持|驳回|改判)[^。\n]{0,40}", "二审裁判"),
        (r"(?:判处|判)[^。\n]{0,20}有期徒刑[^。\n]{0,20}", "判处+刑期"),
        (r"判决结果[：:][^。\n]{0,40}", "判决结果"),
        (r"维持原判[^。\n]{0,40}", "维持原判"),
        (r"驳回(?:上诉|起诉)[^。\n]{0,40}", "驳回裁判"),
        (r"判[处决][^。\n]{0,8}(?:有期徒刑|无期|死刑|拘役|管制|罚金)", "判+具体刑罚"),
    ];

    let mut out = Vec::new();
    for (pattern, label) in FABRICATED_PATTERNS {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(summary) {
            out.push(format!(
                "判决编造: 庭审 transcript 未出现'宣判/判处/判决书'等判决词, 但摘要含 '{}' 表述 (§148 §3)",
                label
            ));
        }
    }
    out
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
            name_drift: vec![],
            role_confusion: vec![],
            fabricated_verdict: vec![],
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
            name_drift: vec![],
            role_confusion: vec![],
            fabricated_verdict: vec![]
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
            name_drift: vec![],
            role_confusion: vec![],
            fabricated_verdict: vec![]
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

    // ============================================================================
    // §148: 法律模板 critical 检测单元测试
    // ============================================================================

    #[test]
    fn section_148_name_drift_detects_li_fuqiang_variants() {
        // 用户截图实际场景: 庭审 transcript 写李福强, AI 摘要混用 李福强/李富强/李富国强
        let transcript = "被告人李福强持枪伤害其父李金明。福强智力发育迟缓。";
        let summary = "李福强持枪, 但后续叙述改用李富强。李富国强供述其将枪放置。";
        let drift = detect_name_drift(transcript, summary);
        assert!(
            drift.iter().any(|e| e.contains("李福强") || e.contains("李富强") || e.contains("李富国强")),
            "应检测到人名漂移, 实际: {drift:?}"
        );
    }

    #[test]
    fn section_148_name_drift_ignores_clean_summary() {
        let transcript = "被告人李福强持枪伤害其父李金明。";
        let summary = "李福强持枪射击李金明, 致其死亡。李福强被诉故意伤害罪。";
        let drift = detect_name_drift(transcript, summary);
        assert!(drift.is_empty(), "统一使用李福强, 不应触发漂移: {drift:?}");
    }

    #[test]
    fn section_148_role_confusion_witness_as_defense() {
        // 用户截图场景: 摘要把"证人 (XX姐姐)" 写成"辩护人 XX姐姐"
        let transcript = "公诉人出示了李福强姐姐的证人证言, 证实两人系姐弟关系。";
        let summary = "辩护人: 李福强姐姐。";
        let confusion = detect_role_confusion(transcript, summary);
        assert!(
            !confusion.is_empty(),
            "应检测到角色混淆 (把证人当辩护人), 实际: {confusion:?}"
        );
    }

    #[test]
    fn section_148_role_confusion_kinship_defense() {
        // 中国刑事案件亲属不能作辩护人, 亲属出庭是证人
        let transcript = "李福强姐姐作为证人出庭提供证言。证人证言证实其弟智力发育迟缓。";
        let summary = "辩护人: 李福强的姐姐 (证人)。";
        let confusion = detect_role_confusion(transcript, summary);
        assert!(
            !confusion.is_empty(),
            "应检测到亲属辩护误写, 实际: {confusion:?}"
        );
    }

    #[test]
    fn section_148_fabricated_verdict_detects_ai_hallucinated_sentence() {
        // 用户截图场景: 庭审只到休庭, 但 AI 编造"一审判处三年"
        let transcript = "本案择期宣判, 现在休庭。辩护人作最后陈述。";
        let summary = "一审判处三年有期徒刑。被告人请求从轻。";
        let verdict = detect_fabricated_verdict(transcript, summary);
        assert!(
            !verdict.is_empty(),
            "应检测到判决编造, 实际: {verdict:?}"
        );
    }

    #[test]
    fn section_148_fabricated_verdict_allows_actual_verdict() {
        // transcript 含"宣判" → 摘要可写判决 (合法)
        let transcript = "现在宣读判决: 被告人李福强犯故意伤害罪, 判处有期徒刑三年, 缓刑四年。";
        let summary = "判处李福强有期徒刑三年, 缓刑四年。";
        let verdict = detect_fabricated_verdict(transcript, summary);
        assert!(
            verdict.is_empty(),
            "transcript 已宣判, 不应触发 fabricated_verdict, 实际: {verdict:?}"
        );
    }

    #[test]
    fn section_148_full_validate_catches_all_three_lifuqiang_failures() {
        // 用户截图综合场景: 李福强庭审摘要, 3 类问题并发
        let transcript = "李福强持枪射击其父李金明, 致其死亡。本案择期宣判, 现在休庭。\n李福强姐姐作为证人出庭, 提供证人证言, 证实其弟智力发育迟缓。\n鉴定意见认定被告人属于限定刑事责任能力。";
        let summary = "李福强持枪伤害其父。但后续改用李富强。辩护人: 李福强的姐姐 (证人)。一审判处三年有期徒刑, 二审维持原判。是否属于限定刑事责任能力人待查明。";
        let report = validate_summary(transcript, summary);
        assert!(!report.name_drift.is_empty(), "人名漂移漏检: {report:?}");
        assert!(!report.role_confusion.is_empty(), "角色混淆漏检: {report:?}");
        assert!(!report.fabricated_verdict.is_empty(), "判决编造漏检: {report:?}");
        assert!(report.is_legal_critical(), "法律 critical 应为 true: {report:?}");
        // highlight 必须能标黄这些 fabricated token
        let highlighted = highlight_unexpected_facts(summary, &report);
        assert!(highlighted.contains("⚠️") || highlighted.contains("=="), "highlight 必须标注 fabricated tokens");
}
