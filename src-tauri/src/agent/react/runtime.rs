use chrono::Utc;
use uuid::Uuid;

use crate::agent::tools::ToolResult;

use super::types::{
    FinishReason, FinishReasonOrTerminal, ReactAction, ReactIteration, ReactObservation,
    ReactPhase, ReactRuntimeConfig, ReactRuntimeSnapshot, ReactThought,
};

#[derive(Debug, Clone)]
pub struct ReactRuntime {
    config: ReactRuntimeConfig,
    iterations: Vec<ReactIteration>,
    consecutive_errors: u32,
    idle_rounds: u32,
    final_response: Option<String>,
    stop_reason: Option<FinishReasonOrTerminal>,
    aborted: bool,
}

impl ReactRuntime {
    pub fn new(config: ReactRuntimeConfig) -> Self {
        Self {
            config,
            iterations: Vec::new(),
            consecutive_errors: 0,
            idle_rounds: 0,
            final_response: None,
            stop_reason: None,
            aborted: false,
        }
    }

    pub fn start_iteration(&mut self) -> usize {
        let index = self.iterations.len();
        let iteration = ReactIteration {
            id: Uuid::new_v4(),
            index,
            started_at: Utc::now().timestamp_millis(),
            status: ReactPhase::Reasoning,
            thought: None,
            action: None,
            observation: None,
            response: None,
            finish_reason: None,
            error_message: None,
        };
        self.iterations.push(iteration);
        index
    }

    pub fn record_thought(
        &mut self,
        iteration_index: usize,
        raw: String,
        normalized: String,
    ) -> ReactThought {
        let thought = ReactThought {
            id: Uuid::new_v4(),
            iteration: iteration_index,
            raw,
            normalized,
            created_at: Utc::now().timestamp_millis(),
        };
        if let Some(iteration) = self.iterations.get_mut(iteration_index) {
            iteration.thought = Some(thought.clone());
            iteration.status = ReactPhase::Reasoning;
        }
        thought
    }

    pub fn record_action(
        &mut self,
        iteration_index: usize,
        tool_name: String,
        arguments: serde_json::Value,
    ) -> ReactAction {
        let action = ReactAction {
            id: Uuid::new_v4(),
            iteration: iteration_index,
            tool_name,
            arguments,
            issued_at: Utc::now().timestamp_millis(),
        };
        if let Some(iteration) = self.iterations.get_mut(iteration_index) {
            iteration.action = Some(action.clone());
            iteration.status = ReactPhase::Action;
        }
        self.idle_rounds = 0;
        action
    }

    pub fn record_observation(
        &mut self,
        iteration_index: usize,
        tool_name: String,
        outcome: ToolResult,
    ) -> ReactObservation {
        let observation = ReactObservation {
            id: Uuid::new_v4(),
            iteration: iteration_index,
            tool_name,
            outcome,
            observed_at: Utc::now().timestamp_millis(),
        };
        if let Some(iteration) = self.iterations.get_mut(iteration_index) {
            iteration.observation = Some(observation.clone());
            iteration.status = ReactPhase::Observation;
        }
        observation
    }

    pub fn complete_iteration(
        &mut self,
        iteration_index: usize,
        response: Option<String>,
        finish_reason: Option<FinishReason>,
    ) {
        if let Some(iteration) = self.iterations.get_mut(iteration_index) {
            iteration.response = response.clone();
            iteration.finish_reason = finish_reason.clone();
            iteration.status = ReactPhase::Completion;
        }
        if let Some(resp) = response {
            self.final_response = Some(resp);
        }
        if let Some(reason) = finish_reason {
            self.stop_reason = Some(reason.into());
        }
        self.idle_rounds = 0;
        self.consecutive_errors = 0;
    }

    pub fn fail_iteration(&mut self, iteration_index: usize, error_message: String) {
        if let Some(iteration) = self.iterations.get_mut(iteration_index) {
            iteration.error_message = Some(error_message);
            iteration.status = ReactPhase::Failed;
        }
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);
    }

    pub fn mark_idle_round(&mut self) {
        self.idle_rounds = self.idle_rounds.saturating_add(1);
    }

    pub fn reset_error_counter(&mut self) {
        self.consecutive_errors = 0;
    }

    pub fn reset_idle_rounds(&mut self) {
        self.idle_rounds = 0;
    }

    pub fn set_stop_reason(&mut self, reason: FinishReasonOrTerminal) {
        self.stop_reason = Some(reason);
    }

    pub fn register_final_response(&mut self, response: String) {
        self.final_response = Some(response);
    }

    pub fn mark_abort(&mut self) {
        self.aborted = true;
        self.stop_reason = Some(FinishReasonOrTerminal::Abort);
    }

    pub fn get_snapshot(&self) -> ReactRuntimeSnapshot {
        ReactRuntimeSnapshot {
            iterations: self.iterations.clone(),
            final_response: self.final_response.clone(),
            stop_reason: self.stop_reason.clone(),
            aborted: self.aborted,
        }
    }

    pub fn should_halt(&self) -> bool {
        if self.iterations.len() as u32 >= self.config.max_iterations {
            return true;
        }
        if self.consecutive_errors >= self.config.max_consecutive_errors {
            return true;
        }
        if self.idle_rounds >= self.config.max_idle_rounds {
            return true;
        }
        false
    }

    pub fn config(&self) -> &ReactRuntimeConfig {
        &self.config
    }
}
