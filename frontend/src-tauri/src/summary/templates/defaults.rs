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
pub const MEDICAL_INTERNAL_ROUND: &str = include_str!("../../../templates/medical_internal_round.json");
/// §131.2 庭审纪要模板 — 法院庭审专版, 不与 legal_consultation 律师咨询混用
pub const COURT_HEARING: &str = include_str!("../../../templates/court_hearing.json");
/// §131.4 完整注册 — 之前 JSON 写了但未 register, 用户看不到
pub const CROSS_BORDER_ECOMMERCE: &str = include_str!("../../../templates/cross_border_ecommerce.json");
pub const PROJECT_SYNC: &str = include_str!("../../../templates/project_sync.json");
pub const RETROSPECTIVE: &str = include_str!("../../../templates/retrospective.json");
pub const SALES_MARKETING_CLIENT_CALL: &str = include_str!("../../../templates/sales_marketing_client_call.json");
pub const PSYCHIATRIC_SESSION: &str = include_str!("../../../templates/psychatric_session.json");

/// Registry of all built-in templates
///
/// Maps template identifiers to their embedded JSON content
#[cfg(test)]
pub fn get_builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("daily_standup", DAILY_STANDUP),
        ("standard_meeting", STANDARD_MEETING),
        ("legal_consultation", LEGAL_CONSULTATION),
        ("medical_consultation", MEDICAL_CONSULTATION),
        ("court_hearing", COURT_HEARING),
        ("cross_border_ecommerce", CROSS_BORDER_ECOMMERCE),
        ("project_sync", PROJECT_SYNC),
        ("retrospective", RETROSPECTIVE),
        ("sales_marketing_client_call", SALES_MARKETING_CLIENT_CALL),
        ("psychiatric_session", PSYCHIATRIC_SESSION),
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
        "medical_internal_round" => Some(MEDICAL_INTERNAL_ROUND),
        "court_hearing" => Some(COURT_HEARING),
        "cross_border_ecommerce" => Some(CROSS_BORDER_ECOMMERCE),
        "project_sync" => Some(PROJECT_SYNC),
        "retrospective" => Some(RETROSPECTIVE),
        "sales_marketing_client_call" => Some(SALES_MARKETING_CLIENT_CALL),
        "psychiatric_session" => Some(PSYCHIATRIC_SESSION),
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
        "cross_border_ecommerce",
        "project_sync",
        "retrospective",
        "sales_marketing_client_call",
        "psychiatric_session",
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

    // §135.1: 全部 10 模板必须有 Key Events Timeline 段 (用户核心价值, 任何场景都有时间线)
    #[test]
    fn test_all_builtin_templates_have_key_events_timeline() {
        for (id, json) in get_builtin_templates() {
            let parsed: serde_json::Value = serde_json::from_str(json).expect("parse");
            let sections = parsed.get("sections").and_then(|s| s.as_array()).expect("sections array");
            let has_timeline = sections.iter().any(|s| {
                let title = s.get("title").and_then(|t| t.as_str()).unwrap_or("");
                title.contains("时间线") || title.to_lowercase().contains("timeline")
            });
            assert!(has_timeline, "template {id} must have a Key Events Timeline section (one of sections)");
        }
    }

    // §136: 全部 10 模板必须有"整件事叙述"段 (用户最看重的连贯叙事)
    #[test]
    fn test_all_builtin_templates_have_narrative_summary() {
        for (id, json) in get_builtin_templates() {
            let parsed: serde_json::Value = serde_json::from_str(json).expect("parse");
            let sections = parsed.get("sections").and_then(|s| s.as_array()).expect("sections array");
            let has_narrative = sections.iter().any(|s| {
                let title = s.get("title").and_then(|t| t.as_str()).unwrap_or("");
                title.contains("整件事") || title.to_lowercase().contains("narrative")
            });
            assert!(has_narrative, "template {id} must have a Narrative Summary section (整件事叙述)");
        }
    }
    
    #[test]
    fn section_167_medical_internal_round_template_loads() {
        let content = get_builtin_template("medical_internal_round").expect("medical_internal_round");
        assert!(content.contains("科室会诊纪要"), "name: {}", &content[..200]);
    }
    #[test]
    fn section_167_medical_internal_round_has_10_sections() {
        let content = get_builtin_template("medical_internal_round").expect("medical_internal_round");
        assert_eq!(content.matches("\"title\":").count(), 10, "must have 10 sections");
    }
    #[test]
    fn section_167_medical_internal_round_has_doctor_opinions_section() {
        let content = get_builtin_template("medical_internal_round").expect("medical_internal_round");
        assert!(content.contains("各医生意见汇总"), "must have 各医生意见汇总 section");
        assert!(content.contains("会诊结论与下一步"), "must have 会诊结论与下一步 section");
    }
}
