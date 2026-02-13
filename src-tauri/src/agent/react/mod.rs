// ReAct strategy utilities for Agent module

pub mod loop_detector;
pub mod orchestrator;
pub mod runtime;
pub mod types;

pub use loop_detector::LoopDetector;
pub use orchestrator::ReactOrchestrator;
pub use runtime::ReactRuntime;
pub use types::*;

use once_cell::sync::Lazy;
use regex::Regex;

// Compile regex patterns once at startup - STRICT format enforcement
static THINKING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<thinking>(.*?)</thinking>").unwrap());

static ANSWER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<answer>(.*?)</answer>").unwrap());

/// Parse the agent's thinking segment from a raw LLM text response.
///
/// **STRICT**: Only accepts `<thinking>...</thinking>` format.
/// If the LLM doesn't follow the format, it's the LLM's problem, not ours.
pub fn parse_thinking(text: &str) -> Option<String> {
    THINKING_RE.captures(text).and_then(|captures| {
        captures.get(1).and_then(|thinking| {
            let thinking_text = thinking.as_str().trim();
            if !thinking_text.is_empty() {
                Some(thinking_text.to_string())
            } else {
                None
            }
        })
    })
}

/// Parse the agent's final answer segment from a raw LLM text response.
///
/// **STRICT**: Only accepts `<answer>...</answer>` format.
/// If the LLM doesn't follow the format, it's the LLM's problem, not ours.
pub fn parse_final_answer(text: &str) -> Option<String> {
    ANSWER_RE.captures(text).and_then(|captures| {
        captures.get(1).and_then(|answer| {
            let answer_text = answer.as_str().trim();
            if !answer_text.is_empty() {
                Some(answer_text.to_string())
            } else {
                None
            }
        })
    })
}
