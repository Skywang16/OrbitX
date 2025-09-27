use std::sync::Arc;

use tokio_stream::StreamExt;

use crate::agent::common::xml::parse_task_tree;
use crate::agent::core::context::TaskContext;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::{build_tree_plan_system_prompt, build_tree_plan_user_prompt};
use crate::agent::types::PlannedTask;
use crate::llm::service::LLMService;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMRequest, LLMStreamChunk};

pub struct TreePlanner {
    context: Arc<TaskContext>,
}

impl TreePlanner {
    pub fn new(context: Arc<TaskContext>) -> Self {
        Self { context }
    }

    pub async fn plan_tree(&self, prompt: &str) -> AgentResult<PlannedTask> {
        let system_prompt = build_tree_plan_system_prompt().await;
        let user_prompt = build_tree_plan_user_prompt(prompt);

        let messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: LLMMessageContent::Text(system_prompt),
            },
            LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text(user_prompt),
            },
        ];

        let model_id = self.get_default_model_id().await?;
        let request = LLMRequest {
            model: model_id,
            messages,
            temperature: Some(0.6),
            max_tokens: Some(4096),
            tools: None,
            tool_choice: None,
            stream: true,
        };

        let llm_service = LLMService::new(self.context.repositories());
        let mut stream = llm_service
            .call_stream(request)
            .await
            .map_err(|e| AgentError::LLMServiceError(e.to_string()))?;

        let mut stream_text = String::new();
        while let Some(chunk) = stream.next().await {
            if let Err(e) = self.context.check_aborted(true).await {
                return Err(AgentError::TaskExecutionError(e.to_string()));
            }
            match chunk {
                Ok(LLMStreamChunk::Delta { content, .. }) => {
                    if let Some(text) = content {
                        stream_text.push_str(&text);
                    }
                }
                Ok(LLMStreamChunk::Error { error }) => {
                    return Err(AgentError::LLMServiceError(error));
                }
                Ok(LLMStreamChunk::Finish { .. }) => break,
                Err(e) => return Err(AgentError::LLMServiceError(e.to_string())),
            }
        }

        let (xml_text, _) = split_root_content(&stream_text);
        let planned = parse_task_tree(&xml_text)?;
        self.context.set_planned_tree(Some(planned.clone())).await;
        Ok(planned)
    }

    async fn get_default_model_id(&self) -> AgentResult<String> {
        let models = self
            .context
            .repositories()
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .map_err(|e| AgentError::DatabaseError(e.to_string()))?;

        if let Some(first_enabled) = models.iter().find(|m| m.enabled) {
            return Ok(first_enabled.id.clone());
        }
        if let Some(any_model) = models.first() {
            return Ok(any_model.id.clone());
        }
        Err(AgentError::ConfigurationError(
            "No enabled LLM model available".into(),
        ))
    }
}

fn split_root_content(text: &str) -> (String, String) {
    let root_close = "</root>";
    if let Some(idx) = text.rfind(root_close) {
        let xml = text[..idx + root_close.len()].to_string();
        let trailing = text[idx + root_close.len()..].to_string();
        (xml, trailing)
    } else {
        (text.to_string(), String::new())
    }
}
