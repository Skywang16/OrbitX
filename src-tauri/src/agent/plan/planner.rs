use std::sync::Arc;

use anyhow::{bail, Context};
use tokio_stream::StreamExt;

use crate::agent::common::xml::parse_task_detail;
use crate::agent::core::context::TaskContext;
use crate::agent::error::AgentResult;
use crate::agent::prompt::{build_plan_system_prompt, build_plan_user_prompt};
use crate::agent::types::{Agent, Context as PromptContext, Task, TaskDetail};
use crate::llm::service::LLMService;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMRequest, LLMStreamChunk};

pub struct Planner {
    context: Arc<TaskContext>,
}

impl Planner {
    pub fn new(context: Arc<TaskContext>) -> Self {
        Self { context }
    }

    pub async fn plan(&self, task_prompt: &str, save_history: bool) -> AgentResult<TaskDetail> {
        let (system_prompt, user_prompt) = self.build_prompts(task_prompt).await?;
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

        self.execute_plan(task_prompt, messages, save_history).await
    }

    pub async fn replan(&self, task_prompt: &str, save_history: bool) -> AgentResult<TaskDetail> {
        let mut messages = Vec::new();
        let (prev_request, prev_result) = self
            .context
            .with_chain(|chain| (chain.plan_request.clone(), chain.plan_result.clone()))
            .await;
        if let Some(prev_request) = prev_request {
            messages.extend(prev_request.messages.clone());
        }
        if let Some(prev_result) = prev_result {
            messages.push(LLMMessage {
                role: "assistant".to_string(),
                content: LLMMessageContent::Text(prev_result),
            });
        }

        if messages.is_empty() {
            return self.plan(task_prompt, save_history).await;
        }

        messages.push(LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text(task_prompt.to_string()),
        });

        self.execute_plan(task_prompt, messages, save_history).await
    }

    async fn execute_plan(
        &self,
        task_prompt: &str,
        messages: Vec<LLMMessage>,
        save_history: bool,
    ) -> AgentResult<TaskDetail> {
        let model_id = self.get_default_model_id().await?;
        let request = LLMRequest {
            model: model_id,
            messages: messages.clone(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            tools: None,
            tool_choice: None,
            stream: true,
        };

        let llm_service = LLMService::new(self.context.repositories());
        let mut stream = llm_service
            .call_stream(request.clone())
            .await
            .context("failed to start LLM planning stream")?;

        let mut stream_text = String::new();
        while let Some(chunk) = stream.next().await {
            self.context
                .check_aborted(true)
                .await
                .context("task planning aborted")?;
            match chunk {
                Ok(LLMStreamChunk::Delta { content, .. }) => {
                    if let Some(text) = content {
                        stream_text.push_str(&text);
                    }
                }
                Ok(LLMStreamChunk::Error { error }) => bail!("LLM stream error: {error}"),
                Ok(LLMStreamChunk::Finish { .. }) => break,
                Err(e) => bail!("LLM stream error: {e}"),
            }
        }

        let (xml_text, _) = split_root_content(&stream_text);
        let task_detail = parse_task_detail(&self.context.task_id, &xml_text, false)?;

        if save_history {
            let request_snapshot = request.clone();
            let result_snapshot = stream_text.clone();
            let prompt_snapshot = task_prompt.to_string();
            self.context
                .with_chain_mut(move |chain| {
                    chain.plan_request = Some(request_snapshot);
                    chain.plan_result = Some(result_snapshot);
                    chain.set_task_prompt(prompt_snapshot);
                })
                .await;
        }

        self.context
            .set_task_detail(Some(task_detail.clone()))
            .await;

        Ok(task_detail)
    }

    async fn build_prompts(&self, task_prompt: &str) -> AgentResult<(String, String)> {
        let agent_info = Agent {
            name: "OrbitX Agent".to_string(),
            description: "An AI coding assistant for OrbitX".to_string(),
            capabilities: vec![],
            tools: Vec::new(),
        };

        let task_stub = Task {
            id: self.context.task_id.clone(),
            conversation_id: self.context.conversation_id,
            user_prompt: self.context.user_prompt.clone(),
            xml: None,
            status: crate::agent::types::TaskStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let system_prompt = build_plan_system_prompt(
            agent_info,
            Some(task_stub),
            Some(PromptContext::default()),
            Vec::new(),
        )
        .await
        .context("failed to build plan system prompt")?;

        let user_prompt = build_plan_user_prompt(task_prompt)?;

        Ok((system_prompt, user_prompt))
    }

    async fn get_default_model_id(&self) -> AgentResult<String> {
        let models = self
            .context
            .repositories()
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .context("failed to load LLM models")?;

        if let Some(first_enabled) = models.iter().find(|m| m.enabled) {
            return Ok(first_enabled.id.clone());
        }
        if let Some(any_model) = models.first() {
            return Ok(any_model.id.clone());
        }
        bail!("No enabled LLM model available")
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
