use std::collections::BTreeSet;

/// Built-in high-confidence terms for the meeting/AI/SaaS domain.
/// Keep this list small and reviewable; external datasets should be converted
/// into this format offline after license and precision checks.
pub const BUILTIN_TERMS: &[&str] = &[
    "离线会记", "Meetily", "SenseVoice", "FunASR", "Paraformer", "sherpa-onnx",
    "ASR", "Ollama", "BlockNote", "Tauri", "Whisper", "Qwen", "LLM",
    "API", "SDK", "SaaS", "GPU", "CPU", "VAD", "WER", "RTF",
    "会议纪要", "热词", "重新转录", "说话人分离", "时间戳", "本地模型",
];

/// v0.7.0+: 精简到 2 个对上线内容最敏感的行业 (法律诉讼 + 医疗会诊).
/// 技术/通用词已挪到 sherpa_asr.py 的 STATIC_HOMO 通用段作为 fallback
/// (任何 pack 都会加载, 即便不选行业也有基本纠错保护).
pub const DOMAIN_TERMS: &[(&str, &[&str])] = &[
    ("legal", &["合同", "条款", "诉讼", "仲裁", "证据", "知识产权", "合规审查", "责任主体"]),
    ("medical", &["临床", "患者", "诊断", "处方", "药品", "病历", "治疗方案", "医学影像"]),
];

pub fn build_runtime_terms(custom: &str) -> String {
    let mut terms = BTreeSet::new();
    for term in BUILTIN_TERMS.iter().copied().chain(custom.split(|c: char| c == ',' || c == '\n' || c == '，')) {
        let normalized = term.trim();
        if !normalized.is_empty() && normalized.chars().count() <= 64 {
            terms.insert(normalized.to_string());
        }
    }
    terms.into_iter().collect::<Vec<_>>().join(",")
}

pub fn build_runtime_terms_for_domain(domain: Option<&str>, custom: &str) -> String {
    let domain_terms = domain
        .and_then(|name| DOMAIN_TERMS.iter().find(|(key, _)| *key == name))
        .map(|(_, terms)| terms.iter().copied().collect::<Vec<_>>().join(","))
        .unwrap_or_default();
    build_runtime_terms(&format!("{domain_terms},{custom}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_deduplicated_reviewable_vocab() {
        let terms = build_runtime_terms("Meetily, 采购合同\n采购合同，客户成功");
        assert!(terms.contains("Meetily"));
        assert!(terms.contains("采购合同"));
        assert_eq!(terms.matches("采购合同").count(), 1);
    }

    #[test]
    fn domain_lookup_is_exhaustive() {
        // 锁定 2 个行业. 任何加新行业的 PR 必须更新这里.
        let expected = ["legal", "medical"];
        let actual: Vec<&str> = DOMAIN_TERMS.iter().map(|(k, _)| *k).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn domain_lookup_unknown_returns_empty() {
        // 没在表里的 pack (例如旧用户存的 'tech' / 'cross_border') 应该返回空
        // 而不是 panic. 这是降级安全: L1 加载词表为空时不报警, L0 硬规则仍生效.
        let terms = build_runtime_terms_for_domain(Some("tech"), "");
        assert_eq!(terms, build_runtime_terms(""));
    }
}
