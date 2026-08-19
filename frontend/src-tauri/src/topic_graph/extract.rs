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

const PROMPT_INSTRUCTIONS: &str = "请仔细阅读以下会议摘要, 提取 3-8 个反复出现或被讨论的核心主题, 用于建立跨会议知识图谱.\n\n要求 (严格按字段名输出):\n1. 每个 topic 1 行 JSON 对象, 不要解释, 不要 markdown 代码块, 不要 ``` ```\n2. 字段名必须为英文: \"canonical_name\" / \"topic_type\" / \"excerpt\" / \"sentiment\" (不是 topic_name / name / type / score 等别名)\n3. canonical_name 用最简短的中文短语 (3-15 字), 例: API限流 / Q3OKR / 张伟招聘\n4. topic_type 必须是字符串: \"general\" / \"project\" / \"person\" / \"decision\" 之一\n5. excerpt 直接引用摘要中相关原句 (<= 50 字), 不要改写\n6. sentiment 必须是**字符串**: \"positive\" / \"negative\" / \"neutral\" (不是数字 1/0/-1)\n\n格式: 逐行 JSON, 每行一个对象, 字段顺序不要求, 严格按上面字段名.\n\n会议摘要:\n\n";

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
/// §140: 宽容解析 — LLM (尤其 qwen3.5:2b) 经常字段名错 (topic_name vs canonical_name) 或
///       sentiment 返数字 (-1/0/1) 而非字符串. 我们容错: 别名映射 + 数字映射.
pub fn parse_extract_response(response: &str) -> Vec<ExtractedTopic> {
    use serde_json::Value;
    // §140: 预处理 — 去 markdown 包装 ```json ... ```
    let stripped = strip_markdown_fence(response);
    let mut out = Vec::new();
    for line in stripped.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            continue;
        }
        // 步骤 1: 容错 — alias 字段重命名 (topic_name/name -> canonical_name, type -> topic_type, score -> sentiment)
        // 步骤 2: 容错 — sentiment 数字 (-1/0/1) -> 字符串 (negative/neutral/positive)
        let normalized = normalize_extract_line(trimmed);
        if let Ok(topic) = serde_json::from_str::<ExtractedTopic>(&normalized) {
            out.push(topic);
        }
    }
    out
}

fn normalize_extract_line(line: &str) -> String {
    use serde_json::Value;
    // §140: 预处理 — qwen3.5:2b 经常输出 JavaScript-style 无引号 key {topic_name: "x", ...}
    //       这种不是合法 JSON, 必须先 regex 加引号. 只在 line 是 { 开头 + 含未引号 key 时触发.
    let prequoted = quote_unquoted_keys(line);
    let Ok(mut v) = serde_json::from_str::<Value>(&prequoted) else {
        return prequoted;
    };
    let obj = match v.as_object_mut() {
        Some(m) => m,
        None => return line.to_string(),
    };
    // 字段重命名: topic_name / name / title -> canonical_name
    for src in ["topic_name", "name", "title", "subject"] {
        if let Some(val) = obj.remove(src) {
            obj.entry("canonical_name".to_string()).or_insert(val);
        }
    }
    // 字段重命名: type / category -> topic_type
    for src in ["type", "category", "kind"] {
        if let Some(val) = obj.remove(src) {
            obj.entry("topic_type".to_string()).or_insert(val);
        }
    }
    // 字段重命名: score / polarity -> sentiment
    for src in ["score", "polarity", "tone"] {
        if let Some(val) = obj.remove(src) {
            obj.entry("sentiment".to_string()).or_insert(val);
        }
    }
    // sentiment 数字 -> 字符串
    if let Some(s) = obj.get("sentiment").cloned() {
        let mapped = match s {
            Value::Number(n) => {
                let f = n.as_f64().unwrap_or(0.0);
                if f > 0.3 {
                    "positive"
                } else if f < -0.3 {
                    "negative"
                } else {
                    "neutral"
                }
            }
            Value::String(s) => {
                let s_lower = s.to_lowercase();
                if s_lower == "1" || s_lower == "positive" || s_lower == "pos" || s_lower == "good" {
                    "positive"
                } else if s_lower == "-1" || s_lower == "negative" || s_lower == "neg" || s_lower == "bad" {
                    "negative"
                } else {
                    "neutral"
                }
            }
            _ => "neutral",
        };
        obj.insert("sentiment".to_string(), Value::String(mapped.to_string()));
    }
    v.to_string()
}

/// §140: 去 markdown 包装. 兼容: ```json\n{...}\n```, ```\n{...}\n```, 以及纯文本.
fn strip_markdown_fence(response: &str) -> String {
    let mut out = response.to_string();
    out = out.replace("```json", "");
    out = out.replace("```JSON", "");
    out = out.replace("```Json", "");
    out = out.replace("```", "");
    out
}

/// §140: 把 JavaScript-style 无引号 key 加双引号 → 合法 JSON
/// 例: `{topic_name: "x",topic_type:"y"}` → `{"topic_name": "x","topic_type":"y"}`
/// 不影响已经是合法 JSON 的输入 (regex 不会匹配已经加引号的 key)
fn quote_unquoted_keys(line: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static RE: Lazy<Regex> = Lazy::new(|| {
        // 匹配 `,` 或 `{` 后跟空白 + 不含 `"` 和 `:` 的字母数字下划线串 + 空白 + `:`
        // 排除已经引号的 key: 要求前面是 `,` 或 `{`, 后面是 `:` (不是 `":"`)
        Regex::new(r#"([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:"#).unwrap()
    });
    RE.replace_all(line, r#"$1"$2":"#).to_string()
}

#[cfg(test)]
mod quote_tests {
    use super::*;
    #[test]
    fn quote_unquoted_keys_basic() {
        let r = quote_unquoted_keys(r#"{topic_name: "x",topic_type:"project",sentiment:-1}"#);
        assert!(r.contains(r#""topic_name":"#));
        assert!(r.contains(r#""topic_type":"#));
        assert!(r.contains(r#""sentiment":"#));
    }
    #[test]
    fn quote_unquoted_keys_passes_through_valid_json() {
        let r = quote_unquoted_keys(r#"{"canonical_name":"x","topic_type":"project"}"#);
        assert_eq!(r, r#"{"canonical_name":"x","topic_type":"project"}"#);
    }
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
    fn parse_extract_response_handles_topic_name_alias() {
        // §140: qwen3.5:2b 实际输出 topic_name + sentiment:-1
        let response = r#"{topic_name: "事故赔偿",topic_type:"project",excerpt:"法院判决运输公司赔偿 167 万余元",sentiment:-1}
{topic_name: "暖风行动",topic_type:"decision",excerpt:"顺义法院启动暖风行动拘留陈某",sentiment:-1}
"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 2, "应容错解析 2 个 topics (topic_name + sentiment:-1)");
        assert_eq!(topics[0].canonical_name, "事故赔偿");
        assert_eq!(topics[0].topic_type, "project");
        assert_eq!(topics[0].sentiment, "negative");  // -1 -> negative
        assert_eq!(topics[1].canonical_name, "暖风行动");
    }

    #[test]
    fn parse_extract_response_handles_sentiment_number_positive() {
        let response = r#"{"canonical_name":"OKR 完成","topic_type":"decision","excerpt":"Q3 OKR 全部完成","sentiment":1}"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].sentiment, "positive");
    }

    #[test]
    fn parse_extract_response_handles_sentiment_zero() {
        let response = r#"{"canonical_name":"日常会议","topic_type":"general","excerpt":"每周例会","sentiment":0}"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].sentiment, "neutral");
    }

    #[test]
    fn parse_extract_response_handles_markdown_fence() {
        // §140: qwen3.5:2b 实际包 ```json 包装
        let response = r#"\`\`\`json
{ "canonical_name": "国展事故", "topic_type": "general", "excerpt": "皮皮事故双目失明", "sentiment": "negative" }
\`\`\`
\`\`\`json
{ "canonical_name": "暖风行动", "topic_type": "general", "excerpt": "顺义法院拘留陈某", "sentiment": "negative" }
\`\`\`
"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 2, "应能去掉 markdown 包装 + 解析 2 个 topics");
        assert_eq!(topics[0].canonical_name, "国展事故");
        assert_eq!(topics[1].canonical_name, "暖风行动");
    }

    #[test]
    fn parse_extract_response_handles_unknown_topic_type() {
        // §140: topic_type 不在 4 个允许值时, 触发 mod.rs fallback 到 "general"
        // 我们这里只验证解析不挂; fallback 在 caller (trigger_after_summary) 处理
        let response = r#"{"canonical_name":"x","topic_type":"legal_incident","excerpt":"y","sentiment":"negative"}"#;
        let topics = parse_extract_response(response);
        assert_eq!(topics.len(), 1);
        // topic_type 原样保留, 由 caller 做白名单校验
        assert_eq!(topics[0].topic_type, "legal_incident");
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
