//! Completion runtime globals.
//!
//! 顶层设计：Completion 相关的“全局单例”只保留一个入口，避免到处散落 `OnceLock`。

use crate::completion::metadata::CommandRegistry;
use crate::completion::output_analyzer::OutputAnalyzer;
use crate::completion::smart_extractor::SmartExtractor;
use std::sync::{Arc, OnceLock};

pub struct CompletionRuntime {
    output_analyzer: Arc<OutputAnalyzer>,
    extractor: SmartExtractor,
    registry: CommandRegistry,
}

static GLOBAL_COMPLETION: OnceLock<CompletionRuntime> = OnceLock::new();

impl CompletionRuntime {
    pub fn global() -> &'static CompletionRuntime {
        GLOBAL_COMPLETION.get_or_init(|| CompletionRuntime {
            output_analyzer: Arc::new(OutputAnalyzer::new()),
            extractor: SmartExtractor::new(),
            registry: {
                let mut registry = CommandRegistry::new();
                registry.load_builtin_commands();
                registry
            },
        })
    }

    pub fn output_analyzer(&self) -> &Arc<OutputAnalyzer> {
        &self.output_analyzer
    }

    pub fn extractor(&self) -> &SmartExtractor {
        &self.extractor
    }

    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }
}
