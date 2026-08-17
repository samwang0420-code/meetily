/// Embedded default templates using compile-time inclusion
///
/// These templates are bundled into the binary and serve as fallbacks
/// when custom templates are not available.

/// Daily standup template for engineering/product teams
pub const DAILY_STANDUP: &str = include_str!("../../../templates/daily_standup.json");

/// Standard meeting notes template
pub const STANDARD_MEETING: &str = include_str!("../../../templates/standard_meeting.json");

/// Industry-specific templates shipped with the application.
pub const LEGAL_CONSULTATION: &str = include_str!("../../../templates/legal_consultation.json");
pub const MEDICAL_CONSULTATION: &str = include_str!("../../../templates/medical_consultation.json");
/// §131.2 庭审纪要模板 — 法院庭审专版, 不与 legal_consultation 律师咨询混用
pub const COURT_HEARING: &str = include_str!("../../../templates/court_hearing.json");

/// Registry of all built-in templates
///
/// Maps template identifiers to their embedded JSON content
pub fn get_builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("daily_standup", DAILY_STANDUP),
        ("standard_meeting", STANDARD_MEETING),
        ("legal_consultation", LEGAL_CONSULTATION),
        ("medical_consultation", MEDICAL_CONSULTATION),
        ("court_hearing", COURT_HEARING),
    ]
}

/// Get a built-in template by identifier
///
/// # Arguments
/// * `id` - Template identifier (e.g., "daily_standup", "standard_meeting")
///
/// # Returns
/// The template JSON content if found, None otherwise
pub fn get_builtin_template(id: &str) -> Option<&'static str> {
    match id {
        "daily_standup" => Some(DAILY_STANDUP),
        "standard_meeting" => Some(STANDARD_MEETING),
        "legal_consultation" => Some(LEGAL_CONSULTATION),
        "medical_consultation" => Some(MEDICAL_CONSULTATION),
        "court_hearing" => Some(COURT_HEARING),
        _ => None,
    }
}

/// List all built-in template identifiers
pub fn list_builtin_template_ids() -> Vec<&'static str> {
    vec![
        "daily_standup",
        "standard_meeting",
        "legal_consultation",
        "medical_consultation",
        "court_hearing",
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
        assert!(get_builtin_template("daily_standup").is_some());
        assert!(get_builtin_template("standard_meeting").is_some());
        assert!(get_builtin_template("legal_consultation").is_some());
        assert!(get_builtin_template("medical_consultation").is_some());
        assert!(get_builtin_template("court_hearing").is_some());
        assert!(get_builtin_template("nonexistent").is_none());
    }

    // §131.2 庭审模板必须有 6 段, 且明确禁止推测判决结果
    #[test]
    fn test_court_hearing_template_has_required_sections() {
        let content = get_builtin_template("court_hearing").expect("court_hearing template");
        let template: crate::summary::templates::Template = serde_json::from_str(content).expect("parse");
        let titles: Vec<&str> = template.sections.iter().map(|s| s.title.as_str()).collect();
        assert!(titles.contains(&"案件基本信息"), "missing 案件基本信息: {titles:?}");
        assert!(titles.contains(&"庭审进程"), "missing 庭审进程: {titles:?}");
        assert!(titles.contains(&"控辩主张"), "missing 控辩主张: {titles:?}");
        assert!(titles.contains(&"关键证据"), "missing 关键证据: {titles:?}");
        assert!(titles.contains(&"争议焦点"), "missing 争议焦点: {titles:?}");
        assert!(titles.contains(&"待查明事项"), "missing 待查明事项: {titles:?}");
        // 关键证据段必须明确禁止混淆单位
        let evidence_section = template.sections.iter().find(|s| s.title == "关键证据").expect("关键证据 section");
        assert!(
            evidence_section.instruction.contains("克") && evidence_section.instruction.contains("元"),
            "关键证据 instruction 必须明确禁止克/元混淆: {}",
            evidence_section.instruction
        );
    }

    // §131.2 legal_consultation 不应再被用于庭审 (用户之前误用导致 9.29千 幻读)
    #[test]
    fn test_legal_consultation_excludes_court_content() {
        let content = get_builtin_template("legal_consultation").expect("legal_consultation");
        let template: crate::summary::templates::Template = serde_json::from_str(content).expect("parse");
        // description 必须明确写"非庭审", 防止用户误用
        assert!(
            template.description.contains("非庭审"),
            "legal_consultation description 必须明确写'非庭审'以防误用, got: {}",
            template.description
        );
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
}
