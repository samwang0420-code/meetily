#!/usr/bin/env python3
import sys, pathlib

def patch_file(path: pathlib.Path):
    src = path.read_text()
    orig = src

    old_from_str = '''    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "claude" => Ok(Self::Claude),
            "groq" => Ok(Self::Groq),
            "ollama" => Ok(Self::Ollama),
            "openrouter" => Ok(Self::OpenRouter),
            "builtin-ai" | "local-llama" | "localllama" => Ok(Self::BuiltInAI),
            "custom-openai" => Ok(Self::CustomOpenAI),
            _ => Err(format!("Unsupported LLM provider: {}", s)),
        }
    }'''
    new_from_str = '''    pub fn from_str(s: &str) -> Result<Self, String> {
        // 离线会记: 只接受 Ollama (本地) 与 BuiltInAI (本地内置)。
        // OpenAI/Claude/Groq/OpenRouter/CustomOpenAI 全部禁用,云端 API 不允许调用。
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Self::Ollama),
            "builtin-ai" | "local-llama" | "localllama" => Ok(Self::BuiltInAI),
            _ => Err(format!(
                "离线会记仅支持本地 LLM (Ollama / BuiltInAI),云端 provider '{}' 已禁用",
                s
            )),
        }
    }'''
    if old_from_str not in src:
        print(f"  ANCHOR 1 NOT FOUND in {path}", file=sys.stderr)
        return False
    src = src.replace(old_from_str, new_from_str)

    guard_anchor = '''    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }'''
    guard_insert = guard_anchor + '''

    // 离线会记硬守卫: 只允许 Ollama / BuiltInAI 调用 LLM
    if !matches!(provider, LLMProvider::Ollama | LLMProvider::BuiltInAI) {
        return Err("离线会记仅支持本地 LLM (Ollama / BuiltInAI),云端 provider 不可用".to_string());
    }'''
    if guard_anchor not in src:
        print(f"  ANCHOR 2 NOT FOUND in {path}", file=sys.stderr)
        return False
    src = src.replace(guard_anchor, guard_insert, 1)

    if src == orig:
        print(f"  NO CHANGE in {path}")
    else:
        path.write_text(src)
        print(f"  PATCHED  {path}")
    return True

if __name__ == "__main__":
    p = pathlib.Path(sys.argv[1])
    sys.exit(0 if patch_file(p) else 1)
