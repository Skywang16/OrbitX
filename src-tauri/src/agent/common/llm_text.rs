pub fn extract_text_from_llm_message(message: &crate::llm::anthropic_types::Message) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            crate::llm::anthropic_types::ContentBlock::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

