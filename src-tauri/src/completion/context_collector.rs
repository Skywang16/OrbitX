//! 上下文收集器

use crate::completion::error::{ContextCollectorError, ContextCollectorResult};
use crate::completion::types::{
    CommandExecutionContext, CommandOutput, EntityType, OutputDataType, OutputEntity,
    ParsedOutputData,
};
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ContextCollector {
    parsers: HashMap<String, Box<dyn OutputParser + Send + Sync>>,

    contexts: Arc<Mutex<VecDeque<CommandExecutionContext>>>,

    max_contexts: usize,
}

pub trait OutputParser {
    fn name(&self) -> &'static str;

    fn can_parse(&self, command: &str) -> bool;

    fn parse(&self, command: &str, output: &str) -> ContextCollectorResult<ParsedOutputData>;
    
    fn priority(&self) -> i32 {
        0
    }
}

impl ContextCollector {
    /// 创建新的上下文收集器
    pub fn new(max_contexts: usize) -> Self {
        let mut collector = Self {
            parsers: HashMap::new(),
            contexts: Arc::new(Mutex::new(VecDeque::new())),
            max_contexts,
        };

        // 注册默认解析器
        collector.register_default_parsers();
        collector
    }

    /// 注册默认解析器
    fn register_default_parsers(&mut self) {
        self.register_parser("lsof", Box::new(LsofParser::new()));
        self.register_parser("ps", Box::new(PsParser::new()));
        self.register_parser("netstat", Box::new(NetstatParser::new()));
        self.register_parser("ls", Box::new(LsParser::new()));
        self.register_parser("git", Box::new(GitParser::new()));
        self.register_parser("top", Box::new(TopParser::new()));
        self.register_parser("htop", Box::new(HtopParser::new()));
    }
    
    /// 注册输出解析器
    pub fn register_parser(&mut self, command: &str, parser: Box<dyn OutputParser + Send + Sync>) {
        self.parsers.insert(command.to_string(), parser);
    }

    /// 收集命令执行上下文
    pub fn collect_context(
        &self,
        command: String,
        args: Vec<String>,
        working_directory: String,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
        duration: Option<u64>,
    ) -> ContextCollectorResult<()> {
        // 尝试解析输出
        let parsed_output = self.parse_output(&command, &stdout)?;
        let output = CommandOutput::new(stdout, stderr);
        let output_with_parsed = output.with_parsed_data(parsed_output);

        let mut context = CommandExecutionContext::new(command, args, working_directory);
        context = context.with_output(output_with_parsed);

        if let Some(code) = exit_code {
            context = context.with_exit_code(code);
        }

        if let Some(dur) = duration {
            context = context.with_duration(dur);
        }

        // 保存上下文
        self.add_context(context)?;

        Ok(())
    }

    /// 解析命令输出
    fn parse_output(&self, command: &str, output: &str) -> ContextCollectorResult<ParsedOutputData> {
        let mut best: Option<&(dyn OutputParser + Send + Sync)> = None;
        let mut best_priority = i32::MIN;

        for parser in self.parsers.values() {
            if !parser.can_parse(command) {
                continue;
            }
            let priority = parser.priority();
            if priority > best_priority {
                best_priority = priority;
                best = Some(parser.as_ref());
            }
        }

        match best {
            Some(parser) => parser.parse(command, output),
            None => Ok(ParsedOutputData::new(OutputDataType::Unknown)),
        }
    }

    /// 添加上下文到存储
    fn add_context(&self, context: CommandExecutionContext) -> ContextCollectorResult<()> {
        let mut contexts = self.lock_contexts()?;
        contexts.push_back(context);
        while contexts.len() > self.max_contexts {
            contexts.pop_front();
        }
        Ok(())
    }

    fn lock_contexts(&self) -> ContextCollectorResult<MutexGuard<'_, VecDeque<CommandExecutionContext>>> {
        self.contexts.lock().map_err(|_| ContextCollectorError::MutexPoisoned {
            resource: "contexts",
        })
    }

    /// 获取所有上下文
    pub fn get_contexts(&self) -> ContextCollectorResult<Vec<CommandExecutionContext>> {
        let contexts = self.lock_contexts()?;
        Ok(contexts.iter().cloned().collect())
    }

    /// 获取最近的上下文
    pub fn get_recent_contexts(&self, count: usize) -> ContextCollectorResult<Vec<CommandExecutionContext>> {
        let contexts = self.lock_contexts()?;
        Ok(contexts.iter().rev().take(count).cloned().collect())
    }

    /// 根据命令搜索上下文
    pub fn search_contexts_by_command(
        &self,
        command: &str,
    ) -> ContextCollectorResult<Vec<CommandExecutionContext>> {
        let contexts = self.lock_contexts()?;
        Ok(contexts.iter().filter(|ctx| ctx.command == command).cloned().collect())
    }

    /// 清空上下文
    pub fn clear_contexts(&self) -> ContextCollectorResult<()> {
        let mut contexts = self.lock_contexts()?;
        contexts.clear();
        Ok(())
    }
}

impl Default for ContextCollector {
    fn default() -> Self {
        Self::new(1000) // 默认保存1000个上下文
    }
}
