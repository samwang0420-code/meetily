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
}
