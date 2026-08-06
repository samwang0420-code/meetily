// §P0-B Obsidian 模板渲染 (Phase 1: 纯函数, 不写盘, 不连 DB)
//
// 设计目标:
// - 100% local, 0 网络
// - 输出 UTF-8 干净 YAML frontmatter + Markdown body
// - 三个区块: Summary (来自 summary_processes) / Minutes / Transcript
// - [[wikilink]] 关联同标题/同 topic 的旧会议 (Phase 2 接 P0-A 后可用)
// - 中文友好: slug 用 pinyin 库 OR 直接 unicode-safe transliteration
//
// Phase 1 只做模板渲染, Phase 2 才接 DB + 写盘.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Per-user export settings (DB row 1:1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub user_id: i64,
    pub enabled: bool,
    pub vault_path: String,
    pub subdir: String,
    pub template_id: String,
    pub last_exported_meeting_id: Option<String>,
    pub last_exported_at: Option<String>,
    pub last_export_status: Option<String>,
    pub last_export_error: Option<String>,
}

impl Settings {
    pub fn default_for_user(user_id: i64) -> Self {
        Self {
            user_id,
            enabled: false,
            vault_path: "~/Documents/Obsidian Vault".to_string(),
            subdir: "会议".to_string(),
            template_id: "default".to_string(),
            last_exported_meeting_id: None,
            last_exported_at: None,
            last_export_status: None,
            last_export_error: None,
        }
    }
}

/// 模板输入 (纯数据, 无 DB 依赖 — 容易单测)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateVars {
    pub meeting_id: String,
    pub title: String,
    pub created_at: String,        // ISO 8601
    pub duration_minutes: i64,
    pub transcript_count: i64,
    pub audio_total_seconds: f64,
    pub asr_provider: String,
    pub asr_model: String,
    pub summary: Option<String>,     // markdown content
    pub minutes: Option<String>,     // markdown content
    pub transcript: Option<String>,  // markdown content
    pub related_links: Vec<String>,  // [[wikilink]] strings
}

/// 渲染结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedDoc {
    pub frontmatter: String,
    pub body: String,
    pub full_markdown: String,
    pub filename: String,
}

const DEFAULT_TEMPLATE: &str = "default";

/// 主入口: 根据 template_id 渲染一个 meeting 的 .md
pub fn render_meeting_doc(vars: &TemplateVars, template_id: &str) -> RenderedDoc {
    let body = match template_id {
        DEFAULT_TEMPLATE => render_default_template(vars),
        other => {
            // 未知 template fallback 到 default, 不报错 (Phase 2 加 warning log)
            let _ = other;
            render_default_template(vars)
        }
    };
    let frontmatter = render_frontmatter(vars);
    let full_markdown = format!("---\n{frontmatter}---\n\n{body}");
    let filename = build_filename(vars);
    RenderedDoc {
        frontmatter,
        body,
        full_markdown,
        filename,
    }
}

fn render_frontmatter(vars: &TemplateVars) -> String {
    // YAML 不支持多行字符串内的特殊字符需要 quoting — 我们用双引号包裹, 转义内部双引号和反斜杠
    let title_escaped = escape_yaml_string(&vars.title);
    let summary_escaped = vars
        .summary
        .as_deref()
        .map(escape_yaml_string)
        .unwrap_or_default();
    let mut fm = String::new();
    fm.push_str(&format!("created: \"{}\"\n", vars.created_at));
    fm.push_str(&format!("meeting_id: \"{}\"\n", vars.meeting_id));
    fm.push_str(&format!("title: \"{title_escaped}\"\n"));
    fm.push_str(&format!("duration_minutes: {}\n", vars.duration_minutes));
    fm.push_str(&format!("transcript_count: {}\n", vars.transcript_count));
    fm.push_str(&format!("asr_provider: \"{}\"\n", vars.asr_provider));
    fm.push_str(&format!("asr_model: \"{}\"\n", vars.asr_model));
    fm.push_str("tags:\n  - meeting\n  - 言镜AI\n");
    fm.push_str(&format!("summary_preview: \"{summary_escaped}\"\n"));
    fm
}

fn render_default_template(vars: &TemplateVars) -> String {
    let mut s = String::new();
    // 标题
    s.push_str(&format!("# {}\n\n", vars.title));
    // 元信息行
    s.push_str(&format!(
        "> 📅 {} · ⏱ {} 分钟 · 🎤 {} 段 · ASR `{} / {}`\n\n",
        vars.created_at, vars.duration_minutes, vars.transcript_count, vars.asr_provider, vars.asr_model
    ));

    // Summary 段
    if let Some(summary) = vars.summary.as_deref() {
        s.push_str("## 📋 摘要\n\n");
        s.push_str(summary.trim());
        s.push_str("\n\n");
    }

    // Minutes 段 (如果 Minutes 与 Summary 不同 — Phase 1 简化为相同内容)
    if let Some(minutes) = vars.minutes.as_deref() {
        if vars.summary.as_deref() != Some(minutes) {
            s.push_str("## 📝 会议纪要\n\n");
            s.push_str(minutes.trim());
            s.push_str("\n\n");
        }
    }

    // Transcript 折叠
    if let Some(transcript) = vars.transcript.as_deref() {
        s.push_str(&format!("## 🎤 完整转录 ({} 段)\n\n", vars.transcript_count));
        s.push_str("<details>\n<summary>点击展开转录</summary>\n\n");
        s.push_str(transcript.trim());
        s.push_str("\n\n</details>\n\n");
    }

    // Related 段
    if !vars.related_links.is_empty() {
        s.push_str("## 🔗 关联会议\n\n");
        for link in &vars.related_links {
            s.push_str(&format!("- {link}\n"));
        }
        s.push('\n');
    }

    s
}

/// 构造文件名: `YYYY-MM-DD-{slug}.md`
/// slug: title 中前 30 个可打印字符, 非 [A-Za-z0-9\u4e00-\u9fff_-] 替换为 -
pub fn build_filename(vars: &TemplateVars) -> String {
    let date = vars.created_at.get(..10).unwrap_or("1970-01-01");
    let slug = slugify(&vars.title, 30);
    let slug = if slug.is_empty() { vars.meeting_id.get(..8).unwrap_or("meeting").to_string() } else { slug };
    format!("{date}-{slug}.md")
}

pub fn slugify(input: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(max_len.min(input.len()));
    let mut count = 0;
    for ch in input.chars() {
        if count >= max_len {
            break;
        }
        let is_safe = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
            || ('\u{4e00}'..='\u{9fff}').contains(&ch);
        if is_safe {
            out.push(ch);
            count += 1;
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
            count += 1;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// 展开 ~ 为 home dir
pub fn expand_home(path: &str) -> std::path::PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(stripped);
        }
    } else if path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).to_path_buf();
        }
    }
    Path::new(path).to_path_buf()
}

fn escape_yaml_string(s: &str) -> String {
    // YAML double-quoted string 规则: \ 和 " 必须转义
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod markdown_tests {
    use super::*;
    fn sample_vars() -> TemplateVars {
        TemplateVars {
            meeting_id: "meeting-8bffd804-b773-4165-a166-9fa2549f9d9e".into(),
            title: "周会复盘: Q3 目标 + OKR 对齐".into(),
            created_at: "2026-08-06T15:30:00+08:00".into(),
            duration_minutes: 77,
            transcript_count: 234,
            audio_total_seconds: 4620.5,
            asr_provider: "sherpa_funasr_nano".into(),
            asr_model: "funasr-nano-zh".into(),
            summary: Some("## 关键决议\n- Q3 目标上调 15%\n- 新增 2 个招聘名额".into()),
            minutes: None,
            transcript: Some("- [00:00:05] 王伟: 大家好, 今天的周会开始...".into()),
            related_links: vec!["[[2026-07-30-周会复盘]]".into()],
        }
    }

    #[test]
    fn test_render_meeting_doc_default() {
        let vars = sample_vars();
        let doc = render_meeting_doc(&vars, "default");
        assert!(doc.frontmatter.contains("meeting_id:"));
        assert!(doc.frontmatter.contains("\"meeting-8bffd804-b773-4165-a166-9fa2549f9d9e\""));
        assert!(doc.body.contains("# 周会复盘"));
        assert!(doc.body.contains("## 📋 摘要"));
        assert!(doc.body.contains("Q3 目标上调 15%"));
        assert!(doc.body.contains("<details>"));
        assert!(doc.body.contains("- [[2026-07-30-周会复盘]]"));
        assert_eq!(doc.filename, "2026-08-06-周会复盘-Q3-目标-OKR-对齐.md");
    }

    #[test]
    fn test_render_unknown_template_falls_back() {
        let vars = sample_vars();
        let doc = render_meeting_doc(&vars, "nonexistent");
        assert!(doc.body.contains("# 周会复盘")); // fallback works
    }

    #[test]
    fn test_slugify_chinese_safe() {
        assert_eq!(slugify("周会复盘: Q3 目标 + OKR 对齐", 30), "周会复盘-Q3-目标-OKR-对齐");
        assert_eq!(slugify("", 30), "");
        assert_eq!(slugify("!!!@@@###", 30), "");
        assert_eq!(slugify("hello_world-2026", 30), "hello_world-2026");
        // 中文 + 英文 + 数字混合
        assert_eq!(slugify("v0.8.5 release 笔记", 30), "v0-8-5-release-笔记");
    }

    #[test]
    fn test_build_filename_fallback_to_meeting_id() {
        let mut vars = sample_vars();
        vars.title = "???@@@".into();
        let doc = render_meeting_doc(&vars, "default");
        assert_eq!(doc.filename, "2026-08-06-meeting-.md");
    }

    #[test]
    fn test_expand_home() {
        let p = expand_home("~/Documents/test");
        // HOME 应当展开
        if let Some(home) = std::env::var_os("HOME") {
            let expected = Path::new(&home).join("Documents/test");
            assert_eq!(p, expected);
        }
        // 绝对路径原样
        let p2 = expand_home("/tmp/foo");
        assert_eq!(p2, Path::new("/tmp/foo"));
    }

    #[test]
    fn test_yaml_escape() {
        let vars = TemplateVars {
            meeting_id: "id-1".into(),
            title: "标题含 \"双引号\" 与 \\反斜杠".into(),
            created_at: "2026-08-06T15:30:00+08:00".into(),
            ..Default::default()
        };
        let doc = render_meeting_doc(&vars, "default");
        assert!(doc.frontmatter.contains("title: \"标题含 \\\"双引号\\\" 与 \\\\反斜杠\""));
    }

    #[test]
    fn test_minutes_omitted_when_equal_to_summary() {
        // 行为: minutes == summary 时不重复渲染 minutes 段
        let mut vars = sample_vars();
        vars.minutes = vars.summary.clone();
        let doc = render_meeting_doc(&vars, "default");
        // 应只有 1 个 "## 📋 摘要" 段
        assert_eq!(doc.body.matches("## 📋 摘要").count(), 1);
        assert!(!doc.body.contains("## 📝 会议纪要"));
    }
}
