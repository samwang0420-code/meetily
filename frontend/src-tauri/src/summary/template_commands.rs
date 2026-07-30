use crate::summary::templates;
use serde::{Deserialize, Serialize};
use tauri::Runtime;
use tracing::{info, warn};

/// Template metadata for UI display
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Template identifier (e.g., "standard_meeting", "standard_meeting")
    pub id: String,

    /// Display name for the template
    pub name: String,

    /// Brief description of the template's purpose
    pub description: String,

    /// v0.7.0+: "free" / "member" — 告诉前端此模板是否需要 Pro
    pub required_tier: String,
}

/// Detailed template structure for preview/debugging
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateDetails {
    /// Template identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: String,

    /// List of section titles in order
    pub sections: Vec<String>,
}

/// Lists all available templates
///
/// Returns templates from both built-in (embedded) and custom (user data directory) sources.
/// Templates are automatically discovered - no code changes needed to add new templates.
///
/// # Returns
/// Vector of TemplateInfo with id, name, and description for each template
#[tauri::command]
pub async fn api_list_templates<R: Runtime>(
    _app: tauri::AppHandle<R>,
    user_tier: Option<String>,
) -> Result<Vec<TemplateInfo>, String> {
    info!("api_list_templates called, user_tier={:?}", user_tier);

    let tier = user_tier.as_deref().unwrap_or("free");
    // v0.7.0+: 按 tier 过滤, free 用户看不到 member 模板
    let templates = templates::list_templates_for_tier(tier);

    let template_infos: Vec<TemplateInfo> = templates
        .into_iter()
        .map(|(id, name, description, required_tier)| TemplateInfo {
            id,
            name,
            description,
            required_tier,
        })
        .collect();

    info!("Found {} templates for tier={}", template_infos.len(), tier);

    Ok(template_infos)
}

/// Gets detailed information about a specific template
///
/// # Arguments
/// * `template_id` - Template identifier (e.g., "standard_meeting")
///
/// # Returns
/// TemplateDetails with full template structure
#[tauri::command]
pub async fn api_get_template_details<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
    user_tier: Option<String>,
) -> Result<TemplateDetails, String> {
    info!("api_get_template_details called for template_id: {}", template_id);

    // v0.7.0+: member 模板对 free 用户返 forbidden
    let template = templates::get_template(&template_id)?;
    let tier = user_tier.as_deref().unwrap_or("free");
    if !template.is_available_for(tier) {
        return Err(format!("template_requires_member: {}", template_id));
    }

    let section_titles: Vec<String> = template
        .sections
        .iter()
        .map(|section| section.title.clone())
        .collect();

    let details = TemplateDetails {
        id: template_id,
        name: template.name,
        description: template.description,
        sections: section_titles,
    };
    let _ = template.required_tier; // 未来可在 TemplateDetails 加 required_tier 字段

    info!("Retrieved template details for '{}'", details.name);

    Ok(details)
}

/// Validates a custom template JSON string
///
/// Useful for template editor UI or validation before saving custom templates
///
/// # Arguments
/// * `template_json` - Raw JSON string of the template
///
/// # Returns
/// Ok(template_name) if valid, Err(error_message) if invalid
#[tauri::command]
pub async fn api_validate_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_json: String,
) -> Result<String, String> {
    info!("api_validate_template called");

    match templates::validate_and_parse_template(&template_json) {
        Ok(template) => {
            info!("Template '{}' validated successfully", template.name);
            Ok(template.name)
        }
        Err(e) => {
            warn!("Template validation failed: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_templates() {
        // §37 + §31 P1: 验证 free tier 不看 member 模板.
        // loader.list_templates_for_tier 是 pure fn, 不需要 Runtime / app handle.
        let free = templates::list_templates_for_tier("free");
        let member = templates::list_templates_for_tier("member");
        let anon = templates::list_templates_for_tier("anonymous");

        // free / anonymous 应该看不到 legal + medical
        assert!(
            !free.iter().any(|(id, _, _, _)| id == "legal_consultation"),
            "free tier must not see legal_consultation template; got ids: {:?}",
            free.iter().map(|(id, _, _, _)| id).collect::<Vec<_>>()
        );
        assert!(
            !free.iter().any(|(id, _, _, _)| id == "medical_consultation"),
            "free tier must not see medical_consultation template; got ids: {:?}",
            free.iter().map(|(id, _, _, _)| id).collect::<Vec<_>>()
        );
        assert!(
            !anon.iter().any(|(id, _, _, _)| id == "legal_consultation"),
            "anonymous tier must not see legal_consultation template"
        );

        // member 三个模板全见
        assert!(
            member.iter().any(|(id, _, _, _)| id == "standard_meeting"),
            "member tier must see standard_meeting"
        );
        assert!(
            member.iter().any(|(id, _, _, _)| id == "legal_consultation"),
            "member tier must see legal_consultation"
        );
        assert!(
            member.iter().any(|(id, _, _, _)| id == "medical_consultation"),
            "member tier must see medical_consultation"
        );

        // required_tier 字段为 (id, name, desc, tier)
        for (id, _n, _d, tier) in member.iter() {
            if id == "standard_meeting" {
                assert_eq!(tier, "free", "standard_meeting should be free");
            } else if id == "legal_consultation" || id == "medical_consultation" {
                assert_eq!(tier, "member", "{} should be member tier", id);
            }
        }

        // 免费数量 < member 数量
        assert!(free.len() < member.len(), "free ({} templates) should be < member ({} templates)", free.len(), member.len());
    }

    #[tokio::test]
    async fn test_list_templates_falls_back_to_free() {
        // 未知 tier 应该走 free 路径 (不能因为拼错而临时获得 member 权限)
        let unknown = templates::list_templates_for_tier("pro_invalid");
        let free = templates::list_templates_for_tier("free");
        assert_eq!(
            unknown.len(),
            free.len(),
            "unknown tier should default to free behavior, got {} vs free {}",
            unknown.len(),
            free.len()
        );
        assert!(
            !unknown.iter().any(|(id, _, _, _)| id == "legal_consultation"),
            "unknown tier (effectively free) must not see legal_consultation"
        );
    }

    #[tokio::test]
    async fn test_validate_template_valid() {
        let valid_json = r#"
        {
            "name": "Test Template",
            "description": "A test template",
            "sections": [
                {
                    "title": "Summary",
                    "instruction": "Provide a summary",
                    "format": "paragraph"
                }
            ]
        }"#;

        // Mock app handle would be needed for actual testing
        // For now, test the validation logic directly
        let result = templates::validate_and_parse_template(valid_json);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_template_invalid() {
        let invalid_json = "invalid json";

        let result = templates::validate_and_parse_template(invalid_json);
        assert!(result.is_err());
    }
}
