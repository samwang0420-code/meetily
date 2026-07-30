/// Embedded default templates using compile-time inclusion
///
/// These templates are bundled into the binary and serve as fallbacks
/// when custom templates are not available.

/// Standard general meeting template (Chinese)
pub const STANDARD_MEETING: &str = include_str!("../../../templates/standard_meeting.json");

/// Industry-specific templates shipped with the application.
pub const LEGAL_CONSULTATION: &str = include_str!("../../../templates/legal_consultation.json");
pub const MEDICAL_CONSULTATION: &str = include_str!("../../../templates/medical_consultation.json");

/// Registry of all built-in templates
///
/// Maps template identifiers to their embedded JSON content
pub fn get_builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("standard_meeting", STANDARD_MEETING),
        ("legal_consultation", LEGAL_CONSULTATION),
        ("medical_consultation", MEDICAL_CONSULTATION),
    ]
}

/// Get a built-in template by identifier
///
/// # Arguments
/// * `id` - Template identifier (e.g., "standard_meeting", "legal_consultation")
///
/// # Returns
/// The template JSON content if found, None otherwise
pub fn get_builtin_template(id: &str) -> Option<&'static str> {
    match id {
        "standard_meeting" => Some(STANDARD_MEETING),
        "legal_consultation" => Some(LEGAL_CONSULTATION),
        "medical_consultation" => Some(MEDICAL_CONSULTATION),
        _ => None,
    }
}

/// List all built-in template identifiers
pub fn list_builtin_template_ids() -> Vec<&'static str> {
    vec![
        "standard_meeting",
        "legal_consultation",
        "medical_consultation",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates_valid_json() {
        for (id, content) in get_builtin_templates() {
            let result = serde_json::from_str::<serde_json::Value>(content);
            assert!(
                result.is_ok(),
                "Built-in template '{}' contains invalid JSON: {:?}",
                id,
                result.err()
            );
        }
    }

    #[test]
    fn test_get_builtin_template() {
        assert!(get_builtin_template("standard_meeting").is_some());
        assert!(get_builtin_template("legal_consultation").is_some());
        assert!(get_builtin_template("medical_consultation").is_some());
        assert!(get_builtin_template("nonexistent").is_none());
    }

    #[test]
    fn test_all_builtin_templates_validate() {
        for (id, content) in get_builtin_templates() {
            let template = serde_json::from_str::<crate::summary::templates::Template>(content)
                .unwrap_or_else(|error| panic!("template '{}' failed to parse: {}", id, error));
            template
                .validate()
                .unwrap_or_else(|error| panic!("template '{}' failed validation: {}", id, error));
        }
    }

    /// v0.7.0+ §31 P1: 法律 / 医疗 模板必须 required_tier=member.
    #[test]
    fn test_legal_medical_templates_are_member_only() {
        use crate::summary::templates::TemplateTier;
        let legal_json = get_builtin_template("legal_consultation").expect("legal_consultation exists");
        let legal: crate::summary::templates::Template =
            serde_json::from_str(legal_json).expect("legal parses");
        assert_eq!(legal.required_tier, TemplateTier::Member);
        assert!(!legal.is_available_for("free"));
        assert!(legal.is_available_for("member"));

        let medical_json = get_builtin_template("medical_consultation").expect("medical_consultation exists");
        let medical: crate::summary::templates::Template =
            serde_json::from_str(medical_json).expect("medical parses");
        assert_eq!(medical.required_tier, TemplateTier::Member);
        assert!(!medical.is_available_for("free"));
        assert!(medical.is_available_for("member"));
    }

    /// v0.7.0-rc8: 防 standard_meeting 回退到含 "会议" 默认假设的旧版.
    /// 关键决议 / 行动事项 / 遗留与风险 三段必须显式要求 LLM 在单向叙事
    /// (视频 / 讲座 / 旁白等) 下写"本次无"空状态, 而非把主题梳理虚构成决议 / 任务.
    /// include_str! 在编译期内嵌, 所以缓存回退只能通过改源 + 不重建触发;
    /// 此测试就是那种重建的硬闸门.
    #[test]
    fn test_standard_meeting_has_narrative_empty_states() {
        let std_json = get_builtin_template("standard_meeting")
            .expect("standard_meeting exists");
        for phrase in ["仅列出", "本次无新决议", "本次无行动事项", "单向叙事"] {
            assert!(
                std_json.contains(phrase),
                "standard_meeting embedded template missing phrase {:?}, source may have silently reverted",
                phrase
            );
        }
    }
    /// §31 P1: standard_meeting 保持 free tier (默认免费可用).
    #[test]
    fn test_standard_meeting_remains_free() {
        use crate::summary::templates::TemplateTier;
        let std_json = get_builtin_template("standard_meeting").expect("standard_meeting exists");
        let std: crate::summary::templates::Template =
            serde_json::from_str(std_json).expect("standard parses");
        assert_eq!(std.required_tier, TemplateTier::Free);
        assert!(std.is_available_for("free"));
        assert!(std.is_available_for("member"));
    }
}
