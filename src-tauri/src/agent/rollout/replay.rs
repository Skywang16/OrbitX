use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use tokio::io::{AsyncBufReadExt, BufReader};

use super::types::{ReplayedThread, RolloutItem, RolloutLine};
use crate::agent::types::{Block, Message};

pub async fn replay_rollout(path: &Path) -> io::Result<ReplayedThread> {
    let file = tokio::fs::File::open(path).await?;
    let mut lines = BufReader::new(file).lines();
    let mut state = ReplayState::default();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let decoded: RolloutLine = serde_json::from_str(&line)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        state.apply(decoded.item);
    }
    Ok(state.finish())
}

#[derive(Default)]
struct ReplayState {
    meta: Option<super::types::ThreadMeta>,
    messages: BTreeMap<i64, Message>,
    order: Vec<i64>,
}

impl ReplayState {
    fn apply(&mut self, item: RolloutItem) {
        match item {
            RolloutItem::ThreadMeta(meta) => {
                self.meta = Some(meta);
            }
            RolloutItem::UserMessageCreated { message, .. }
            | RolloutItem::AssistantMessageCreated { message, .. } => {
                if !self.messages.contains_key(&message.id) {
                    self.order.push(message.id);
                }
                self.messages.insert(message.id, message);
            }
            RolloutItem::BlockAppended {
                message_id, block, ..
            } => {
                if let Some(message) = self.messages.get_mut(&message_id) {
                    if message.blocks.last() == Some(&block) {
                        return;
                    }
                    if let Some(block_id) = block_id_of(&block) {
                        if message
                            .blocks
                            .iter()
                            .any(|existing| block_id_of(existing) == Some(block_id))
                        {
                            return;
                        }
                    }
                    message.blocks.push(block);
                }
            }
            RolloutItem::BlockUpdated {
                message_id,
                block_id,
                block,
                ..
            } => {
                if let Some(message) = self.messages.get_mut(&message_id) {
                    if let Some(existing) = message
                        .blocks
                        .iter_mut()
                        .find(|candidate| block_id_of(candidate) == Some(block_id.as_str()))
                    {
                        *existing = block;
                    } else {
                        message.blocks.push(block);
                    }
                }
            }
            RolloutItem::MessageFinished {
                message_id,
                status,
                finished_at,
                duration_ms,
                token_usage,
                ..
            } => {
                if let Some(message) = self.messages.get_mut(&message_id) {
                    message.status = status;
                    message.finished_at = Some(finished_at);
                    message.duration_ms = Some(duration_ms);
                    message.token_usage = token_usage;
                }
            }
            RolloutItem::SpawnAgent { .. }
            | RolloutItem::AgentRunCreated { .. }
            | RolloutItem::AgentRunCompleted { .. }
            | RolloutItem::AgentRunCancelled { .. }
            | RolloutItem::AgentRunError { .. }
            | RolloutItem::AgentInput { .. }
            | RolloutItem::AgentWaiting { .. }
            | RolloutItem::AgentResumed { .. }
            | RolloutItem::AgentClosed { .. } => {}
        }
    }

    fn finish(self) -> ReplayedThread {
        let mut messages = Vec::with_capacity(self.order.len());
        for id in self.order {
            if let Some(message) = self.messages.get(&id) {
                messages.push(message.clone());
            }
        }
        ReplayedThread {
            meta: self.meta,
            messages,
        }
    }
}

fn block_id_of(block: &Block) -> Option<&str> {
    match block {
        Block::Thinking(value) => Some(value.id.as_str()),
        Block::Text(value) => Some(value.id.as_str()),
        Block::Tool(value) => Some(value.id.as_str()),
        _ => None,
    }
}
