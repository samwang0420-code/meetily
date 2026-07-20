// Context-aware corrections for high-value product and engineering terms.
// Keep this list intentionally small: broad fuzzy replacement can corrupt
// ordinary meeting language and is harder to audit than ASR mistakes.

const TECH_CONTEXT: &[&str] = &[
    "模型", "识别", "转录", "语音", "编辑器", "桌面", "本地", "离线", "ASR", "api", "API",
];

fn has_tech_context(text: &str) -> bool {
    TECH_CONTEXT.iter().any(|term| text.contains(term))
}

/// Public entry point: L0 product corrections + L1 industry homophone. L3 is
/// disabled by default to keep ordinary meeting language untouched.
pub fn correct_industry_terms(text: &str) -> String {
    correct_industry_terms_with_known(text, &[], L3Config::default())
}

/// L0 + L1 + safe L3 fuzzy replacement. `known_terms` is the hotword vocabulary
/// the user is actively enabling. When the list is empty, L3 is bypassed.
pub fn correct_industry_terms_with_known(text: &str, known_terms: &[&str], cfg: L3Config) -> String {
    let mut corrected = text.trim().to_string();
    if corrected.is_empty() {
        return corrected;
    }

    let lower = corrected.to_ascii_lowercase();
    let technical = has_tech_context(&corrected)
        || ["sense voice", "ss voice", "paraformer", "poweraform", "block note"]
            .iter()
            .any(|alias| lower.contains(alias));

    if technical {
        let replacements = [
            ("sense voice", "SenseVoice"),
            ("ss voice", "SenseVoice"),
            ("s voice", "SenseVoice"),
            ("poweraformer", "Paraformer"),
            ("poweraform", "Paraformer"),
            ("para former", "Paraformer"),
            ("black note", "BlockNote"),
            ("block note", "BlockNote"),
            ("fun asr", "FunASR"),
            ("sherpa onnx", "sherpa-onnx"),
        ];
        for (alias, canonical) in replacements {
            corrected = replace_ascii_case_insensitive(&corrected, alias, canonical);
        }
    }

    if !known_terms.is_empty() && cfg.enabled {
        corrected = safe_hotword_replace(&corrected, known_terms, &cfg);
    }

    corrected
}

fn replace_ascii_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    let needle_lower = needle.to_ascii_lowercase();

    while !rest.is_empty() {
        let rest_lower = rest.to_ascii_lowercase();
        let Some(byte_index) = rest_lower.find(&needle_lower) else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..byte_index]);
        output.push_str(replacement);
        rest = &rest[byte_index + needle.len()..];
    }
    output
}

/// Tunable knobs for L3. All defaults are picked to keep ordinary meeting
/// language untouched and to only rescue product/feature names that the user
/// explicitly enabled.
#[derive(Debug, Clone, Copy)]
pub struct L3Config {
    pub enabled: bool,
    pub min_score: f64,
    pub min_chars: usize,
    pub max_chars: usize,
    pub max_window: usize,
    pub max_passes: usize,
}

impl Default for L3Config {
    fn default() -> Self {
        Self {
            enabled: true,
            min_score: 0.78,
            min_chars: 3,
            max_chars: 12,
            max_window: 24,
            max_passes: 8,
        }
    }
}

impl L3Config {
    /// Conservative profile for production: requires ≥ 0.82 similarity and
    /// ≤ 8 character terms. Use this when you only want to rescue the most
    /// obvious product-name typos.
    pub fn conservative() -> Self {
        Self { min_score: 0.82, min_chars: 3, max_chars: 8, ..Self::default() }
    }
}

/// True for any character in the CJK Unified Ideographs blocks plus the
/// common kana / hangul ranges we see in transcribed meetings.
fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    (0x3400..=0x4DBF).contains(&cp)    // CJK Ext A
        || (0x4E00..=0x9FFF).contains(&cp) // CJK Unified
        || (0xF900..=0xFAFF).contains(&cp) // CJK Compat
        || (0x20000..=0x2A6DF).contains(&cp) // CJK Ext B (rare names/terms)
        || (0x2A700..=0x2EBEF).contains(&cp) // CJK Ext C/D/E
        || (0x3040..=0x309F).contains(&cp) // Hiragana
        || (0x30A0..=0x30FF).contains(&cp) // Katakana
        || (0xAC00..=0xD7AF).contains(&cp) // Hangul syllables
        || (0x3000..=0x303F).contains(&cp) // CJK symbols/punctuation
        || (0xFF00..=0xFFEF).contains(&cp) // Halfwidth/fullwidth forms
}

/// Damerau-Levenshtein distance, iterative, bounded.
fn damerau_levenshtein_char(a: &[char], b: &[char]) -> usize {
    let n = a.len();
    let m = b.len();
    if n == 0 { return m; }
    if m == 0 { return n; }
    let mut prev2: Vec<usize> = (0..=m).collect();
    let mut prev1: Vec<usize> = (0..=m).collect();
    let mut row: Vec<usize> = (0..=m).collect();
    for i in 1..=n {
        row[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let mut v = prev1[j] + 1;
            if row[j - 1] + 1 < v { v = row[j - 1] + 1; }
            if prev1[j - 1] + cost < v { v = prev1[j - 1] + cost; }
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                let t = prev2[j - 2] + cost;
                if t < v { v = t; }
            }
            row[j] = v;
        }
        std::mem::swap(&mut prev2, &mut prev1);
        std::mem::swap(&mut prev1, &mut row);
    }
    prev1[m]
}

/// L3 protected fuzzy replacement. Operates only when:
///  * the hotword vocabulary is non-empty (caller-supplied signal),
///  * candidate length is in `[cfg.min_chars, cfg.max_chars]`,
///  * similarity >= `cfg.min_score` (Damerau-Levenshtein),
///  * source slice is bounded by CJK/non-CJK or punctuation boundaries,
///  * replacement loop is bounded by `cfg.max_passes` to guarantee termination.
/// Build a stable `&'static [&'static str]` list of effective hotword terms
/// for a given domain. Empty slice means L3 stays disabled.
pub fn runtime_hotword_terms() -> Vec<&'static str> {
    crate::audio::hotwords_globals::current_custom_with_product_terms()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| Box::leak(s.to_string().into_boxed_str()) as &'static str)
        .collect()
}

pub fn safe_hotword_replace(text: &str, known_terms: &[&str], cfg: &L3Config) -> String {
    let mut current = text.to_string();
    for _ in 0..cfg.max_passes {
        let next = safe_hotword_replace_once(&current, known_terms, cfg);
        if next == current { return current; }
        current = next;
    }
    current
}

fn safe_hotword_replace_once(text: &str, known_terms: &[&str], cfg: &L3Config) -> String {
    if known_terms.is_empty() { return text.to_string(); }
    let prepared: Vec<(&str, Vec<char>)> = known_terms.iter().map(|t: &&str| -> (&str, Vec<char>) {
        let stripped: String = t.chars().filter(|c| c.is_alphanumeric() || is_cjk(*c)).collect();
        let chars: Vec<char> = stripped.to_lowercase().chars().collect();
        // CJK 2-char terms (法院/律师/医院) are too short to fuzzy-replace safely.
        // The 短词走 L1 同音替换 (STATIC_HOMO in sherpa_asr.py), L3 仅接手 ≥ 3 字符.
        if chars.len() < cfg.min_chars || chars.len() > cfg.max_chars { return (*t, chars); }
        (*t, chars)
    })
    .filter(|(_, s)| !s.is_empty() && s.len() >= cfg.min_chars && s.len() <= cfg.max_chars)
    .collect();
    if prepared.is_empty() { return text.to_string(); }

    struct Span { start: usize, end: usize, ch: char }
    let mut normalized: Vec<Span> = Vec::new();
    let len_bytes = text.len();
    let mut i = 0usize;
    while i < len_bytes {
        let ch = match text[i..].chars().next() { Some(c) => c, None => break };
        if ch.is_alphanumeric() || is_cjk(ch) {
            normalized.push(Span { start: i, end: i + ch.len_utf8(), ch: ch.to_ascii_lowercase() });
            i += ch.len_utf8();
        } else {
            i += ch.len_utf8();
        }
    }
    if normalized.len() < cfg.min_chars { return text.to_string(); }

    // Group normalized into same-kind runs so a mixed transcript like
    // `模型使用 poweraform 推理` still finds the ASCII `poweraform` slice.
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    let mut run_start = 0usize;
    while run_start < normalized.len() {
        let k = is_cjk(normalized[run_start].ch);
        let mut run_end = run_start + 1;
        while run_end < normalized.len() && is_cjk(normalized[run_end].ch) == k {
            run_end += 1;
        }
        runs.push((run_start, run_end, k));
        run_start = run_end;
    }

    let mut best: Option<(usize, usize, &str, f64)> = None;
    for (rs, re, kind0) in runs {
        if re - rs < cfg.min_chars { continue; }
        let cap = (re - rs).min(cfg.max_window);
        for window_start in rs..re {
            let window_end_max = (window_start + cap).min(re);
            let mut window_end = window_start + cfg.min_chars;
            while window_end <= window_end_max {
                let collected: Vec<char> = normalized[window_start..window_end].iter().map(|s| s.ch).collect();
                for (term, needle_chars) in &prepared {
                    if is_cjk(needle_chars[0]) != kind0 { continue; }
                    let max_len = needle_chars.len().max(collected.len());
                    if max_len == 0 { continue; }
                    let dist = damerau_levenshtein_char(&collected, needle_chars);
                    let score = 1.0 - (dist as f64 / max_len as f64);
                    if score >= cfg.min_score && collected != *needle_chars {
                        if best.map_or(true, |(_, _, _, s)| score > s) {
                            let start_byte = normalized[window_start].start;
                            let end_byte = normalized[window_end - 1].end;
                            best = Some((start_byte, end_byte, *term, score));
                        }
                    }
                }
                window_end += 1;
            }
        }
    }
    if let Some((start, end, term, _)) = best {
        let mut rebuilt = String::with_capacity(text.len() + term.len());
        rebuilt.push_str(&text[..start]);
        rebuilt.push_str(term);
        rebuilt.push_str(&text[end..]);
        return rebuilt;
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrects_known_asr_product_aliases() {
        assert_eq!(correct_industry_terms("模型使用 ss voice 和 poweraform"), "模型使用 SenseVoice 和 Paraformer");
        assert_eq!(correct_industry_terms("编辑器使用 black note"), "编辑器使用 BlockNote");
    }

    #[test]
    fn preserves_unrelated_plain_language() {
        assert_eq!(correct_industry_terms("Please write a black note"), "Please write a black note");
    }

    #[test]
    fn l3_disabled_when_no_hotwords_loaded() {
        let known: &[&str] = &[];
        assert_eq!(safe_hotword_replace("离线会议 与会人员", known, &L3Config::default()), "离线会议 与会人员");
    }

    #[test]
    fn l3_protects_short_common_terms_from_swaps() {
        let known: &[&str] = &["BlockNote", "FunASR", "Paraformer"];
        assert_eq!(safe_hotword_replace("离线会议 准时开始", known, &L3Config::default()), "离线会议 准时开始");
    }

    #[test]
    fn l3_rescues_close_product_terms() {
        let known: &[&str] = &["BlockNote", "Paraformer"];
        assert_eq!(safe_hotword_replace("编辑器使用 block note 完成", known, &L3Config::default()), "编辑器使用 BlockNote 完成");
    }

    #[test]
    fn l3_replaces_within_safe_similarity() {
        let known: &[&str] = &["Paraformer"];
        let out = safe_hotword_replace("模型使用 paraphormer 推理", known, &L3Config::default());
        assert!(out.contains("Paraformer"), "expected Paraformer, got {out}");
    }

    #[test]
    fn l3_keeps_punctuation_boundaries_intact() {
        let known: &[&str] = &["BlockNote"];
        assert_eq!(safe_hotword_replace("这段,block note,写完", known, &L3Config::default()), "这段,BlockNote,写完");
    }

    #[test]
    fn public_entrypoint_is_default_safe() {
        let out = correct_industry_terms("离线会议 与会人员 准时开始");
        assert_eq!(out, "离线会议 与会人员 准时开始");
    }

    #[test]
    fn l3_disabled_via_config() {
        let cfg = L3Config { enabled: false, ..L3Config::default() };
        let out = correct_industry_terms_with_known("模型使用 paraphormer 推理", &["Paraformer"], cfg);
        assert!(out.contains("paraphormer"), "L3 should be disabled, got {out}");
    }

    #[test]
    fn l3_handles_3char_cjk_hotwords() {
        // 4-char CJK hotword, single-character edit (ASR misrecognition),
        // 距离 1/4=0.75 < 0.78 → 不替换 (L3 风险原则)
        let known: &[&str] = &["法院判决"];
        let out = safe_hotword_replace("原告向法院判绝 提交起诉状", known, &L3Config::default());
        assert!(!out.contains("法院判决"), "1/4 not safe enough, got {out}");
    }

    #[test]
    fn l3_skips_2char_cjk_hotwords() {
        // 2 字符 CJK hotwords 风险太高, 走 L1 同音替换而非 L3.
        let known: &[&str] = &["法院"];
        let out = safe_hotword_replace("原告向法远 提交起诉状", known, &L3Config::default());
        assert!(!out.contains("法院"), "2-char CJK must not L3-substitute, got {out}");
    }

    #[test]
    fn l3_handles_extended_cjk_blocks() {
        // U+20000 (CJK Ext B) - very rare but legal documents can hit it.
        let hotword = "\u{20000}字";
        let known: &[&str] = &[hotword];
        let out = safe_hotword_replace("提交一份萬字报告", known, &L3Config::default());
        // The hotword contains an alphanumeric-stripped CJK ext B char, so it
        // is a 2-char CJK hotword. 萬 is also 2 chars CJK and same kind, but
        // length < min_chars so should not match.
        assert!(!out.contains(hotword), "should not match (length too short): {out}");
    }

    #[test]
    fn l3_conservative_profile_skips_loose_matches() {
        // Default 0.78 would replace this; conservative 0.82 should not.
        let known: &[&str] = &["Paraformer"];
        let input = "模型使用 paraphormer 推理";
        let default_out = correct_industry_terms_with_known(input, known, L3Config::default());
        let cons_out = correct_industry_terms_with_known(input, known, L3Config::conservative());
        assert!(default_out.contains("Paraformer"), "default should match, got {default_out}");
        assert!(cons_out.contains("paraphormer"), "conservative should NOT match, got {cons_out}");
    }

    #[test]
    fn l3_terminates_under_max_passes() {
        // Worst case: every pass picks the same match. After max_passes the
        // helper must stop and return a stable string.
        let cfg = L3Config { max_passes: 3, ..L3Config::default() };
        let known: &[&str] = &["Paraformer"];
        let out = safe_hotword_replace("paraphormer", known, &cfg);
        assert!(!out.is_empty());
    }

    #[test]
    fn l3_protects_pinyin_substring_from_swaps() {
        // 离线会记 (the product name) and 离线会议 (ordinary meeting) share
        // 3 of 4 characters. L3 must NOT rewrite 离线会议 → 离线会记.
        let known: &[&str] = &["离线会记"];
        let out = safe_hotword_replace("今天的离线会议很顺利", known, &L3Config::default());
        assert!(!out.contains("离线会记"), "should not be rewritten, got {out}");
    }
}
