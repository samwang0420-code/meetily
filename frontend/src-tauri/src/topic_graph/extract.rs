// §P0-A LLM extract 骨架 (Phase 1: 纯函数 prompt + JSON 解析, 不实际调 LLM)
//
// Phase 2 接 BuiltInAI (Qwen 3.5 2B) 后:
//   prompt = ExtractPromptBuilder::build(summary_markdown)
//   response = BuiltInAI.complete(prompt, max_tokens=800).await
//   topics: Vec<ExtractedTopic> = parse_extract_response(&response)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedTopic {
    pub canonical_name: String,
    pub topic_type: String,  // general / project / person / decision
    pub excerpt: String,
    pub sentiment: String,   // positive / negative / neutral
}

pub struct ExtractPromptBuilder;

const PROMPT_INSTRUCTIONS: &str = "请仔细阅读以下会议摘要, 提取 3-8 个反复出现或被讨论的核心主题, 用于建立跨会议知识图谱.\n\n要求:\n1. 每个 topic 1 行 JSON 对象, 不要解释, 不要 markdown 代码块\n2. canonical_name 用最简短的中文短语 (3-15 字), 例: API限流 / Q3OKR / 张伟招聘\n3. topic_type 必须是: general / project / person / decision 之一\n4. excerpt 直接引用摘要中相关原句 (<= 50 字), 不要改写\n5. sentiment 必须是: positive / negative / neutral 之一\n\n格式: 逐行 JSON, 每行一个对象.\n\n会议摘要:\n\n";

impl ExtractPromptBuilder {
    pub fn build(summary_markdown: &str) -> String {
        let mut s = String::with_capacity(PROMPT_INSTRUCTIONS.len() + summary_markdown.len() + 4);
        s.push_str(PROMPT_INSTRUCTIONS);
        s.push_str(summary_markdown);
        s.push('\n');
        s
    }
}

/// 解析 LLM 返回的逐行 JSON, 跳过空行 / 非 JSON 行 / 缺字段
pub fn parse_extract_response(response: &str) -> Vec<ExtractedTopic> {
    response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                return None;
            }
            serde_json::from_str::<ExtractedTopic>(trimmed).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_builder_contains_instructions_and_summary() {
        let prompt = ExtractPromptBuilder::build("王伟: API 限流从 100 QPS 升到 1000 QPS");
        assert!(prompt.contains("提取 3-8 个反复出现"));
        assert!(prompt.contains("API 限流从 100 QPS"));
        assert!(prompt.ends_with('\n'));
    }

    #[test]
    fn parse_extract_response_with_valid_lines() {
        let response = r#"
{"canonical_name":"API 限流","topic_type":"project","excerpt":"API 限流从 100 QPS 升到 1000 QPS","sentiment":"positive"}
{"canonical_name":"Q3 OKR","topic_type":"decision","excerpt":"Q3 OKR 同步: 增长 15%","sentiment":"neutral"}
这是解释, 跳过
{"canonical_name":"张伟招聘","topic_type":"person","excerpt":"张伟确认入职","sentiment":"positive"}
"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 3);
        assert_eq!(topics[0].canonical_name, "API 限流");
        assert_eq!(topics[0].topic_type, "project");
        assert_eq!(topics[2].sentiment, "positive");
    }

    #[test]
    fn parse_extract_response_skips_invalid_json() {
        let response = r#"
{"canonical_name":"valid","topic_type":"general","excerpt":"ok","sentiment":"neutral"}
{this is not valid json}
{"missing":"fields"}
"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].canonical_name, "valid");
    }

    #[test]
    fn parse_extract_response_empty() {
        assert_eq!(parse_extract_response("").len(), 0);
        assert_eq!(parse_extract_response("   \n  \n").len(), 0);
    }

    #[test]
    fn extracted_topic_serializes_back_to_json() {
        let t = ExtractedTopic {
            canonical_name: "测试".into(),
            topic_type: "general".into(),
            excerpt: "ex".into(),
            sentiment: "neutral".into(),
        };
        let s = serde_json::to_string(&t).unwrap();
        assert!(s.contains("\"canonical_name\":\"测试\""));
        assert!(s.contains("\"topic_type\":\"general\""));
    }
}
