// ReAct strategy utilities for Agent module
// Mirrors the structure of the frontend eko-core/react implementation

pub mod runtime;
pub mod types;

pub use runtime::ReactRuntime;
pub use types::*;

use regex::Regex;

/// Parse the agent's thinking segment from a raw LLM text response.
/// This mirrors the front-end eko parser semantics by supporting multiple patterns.
pub fn parse_thinking(text: &str) -> Option<String> {
    let patterns = [
        r"(?s)<thinking>(.*?)</thinking>",
        r"(?s)思考：(.*?)(?=\n\n|\n[^思考]|$)",
        r"(?s)## 思考\n(.*?)(?=\n##|\n[^#]|$)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(text) {
                if let Some(thinking) = captures.get(1) {
                    let thinking_text = thinking.as_str().trim();
                    if !thinking_text.is_empty() {
                        return Some(thinking_text.to_string());
                    }
                }
            }
        }
    }

    // If there are no explicit markers and text is short, it could be the thinking itself.
    if !text.contains("工具调用") && !text.contains("最终答案") && text.len() < 500 {
        return Some(text.trim().to_string());
    }

    None
}

/// Parse the agent's final answer segment from a raw LLM text response.
pub fn parse_final_answer(text: &str) -> Option<String> {
    let patterns = [
        r"(?s)<answer>(.*?)</answer>",
        r"(?s)最终答案：(.*?)(?=$)",
        r"(?s)## 最终答案\n(.*?)(?=\n##|$)",
        r"(?s)答案：(.*?)(?=$)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(text) {
                if let Some(answer) = captures.get(1) {
                    let answer_text = answer.as_str().trim();
                    if !answer_text.is_empty() {
                        return Some(answer_text.to_string());
                    }
                }
            }
        }
    }

    // If there is no explicit thinking tag and no tool calls mentioned, the whole text may be the answer
    if text.len() > 10 && !text.contains("<thinking>") && !text.contains("工具调用") {
        return Some(text.trim().to_string());
    }

    None
}
