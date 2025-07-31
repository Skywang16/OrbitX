//! 上下文收集器模块
//!
//! 负责捕获和解析命令执行的输出结果，提取有用的上下文信息

use crate::completion::types::{
    CommandExecutionContext, CommandOutput, OutputDataType, OutputEntity, 
    ParsedOutputData, EntityType
};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// 上下文收集器
pub struct ContextCollector {
    /// 输出解析器映射
    parsers: HashMap<String, Box<dyn OutputParser + Send + Sync>>,
    
    /// 收集的上下文数据
    contexts: Arc<RwLock<Vec<CommandExecutionContext>>>,
    
    /// 最大保存的上下文数量
    max_contexts: usize,
}

/// 输出解析器特征
pub trait OutputParser {
    /// 解析器名称
    fn name(&self) -> &'static str;
    
    /// 检查是否可以解析该命令的输出
    fn can_parse(&self, command: &str) -> bool;
    
    /// 解析命令输出
    fn parse(&self, command: &str, output: &str) -> AppResult<ParsedOutputData>;
    
    /// 解析器优先级
    fn priority(&self) -> i32 {
        0
    }
}

impl ContextCollector {
    /// 创建新的上下文收集器
    pub fn new(max_contexts: usize) -> Self {
        let mut collector = Self {
            parsers: HashMap::new(),
            contexts: Arc::new(RwLock::new(Vec::new())),
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
    ) -> AppResult<()> {
        // 创建命令输出
        let output = CommandOutput::new(stdout.clone(), stderr.clone());
        
        // 尝试解析输出
        let parsed_output = self.parse_output(&command, &stdout)?;
        let output_with_parsed = output.with_parsed_data(parsed_output);
        
        // 创建命令执行上下文
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
    fn parse_output(&self, command: &str, output: &str) -> AppResult<ParsedOutputData> {
        // 查找合适的解析器
        let mut suitable_parsers: Vec<_> = self.parsers
            .values()
            .filter(|parser| parser.can_parse(command))
            .collect();
            
        // 按优先级排序
        suitable_parsers.sort_by_key(|parser| std::cmp::Reverse(parser.priority()));
        
        // 尝试使用第一个合适的解析器
        if let Some(parser) = suitable_parsers.first() {
            parser.parse(command, output)
        } else {
            // 没有合适的解析器，返回基本的解析数据
            Ok(ParsedOutputData::new(OutputDataType::Unknown))
        }
    }
    
    /// 添加上下文到存储
    fn add_context(&self, context: CommandExecutionContext) -> AppResult<()> {
        let mut contexts = self.contexts
            .write()
            .map_err(|_| anyhow!("获取上下文写锁失败"))?;
            
        contexts.push(context);
        
        // 限制上下文数量
        if contexts.len() > self.max_contexts {
            contexts.remove(0);
        }
        
        Ok(())
    }
    
    /// 获取所有上下文
    pub fn get_contexts(&self) -> AppResult<Vec<CommandExecutionContext>> {
        let contexts = self.contexts
            .read()
            .map_err(|_| anyhow!("获取上下文读锁失败"))?;
            
        Ok(contexts.clone())
    }
    
    /// 获取最近的上下文
    pub fn get_recent_contexts(&self, count: usize) -> AppResult<Vec<CommandExecutionContext>> {
        let contexts = self.contexts
            .read()
            .map_err(|_| anyhow!("获取上下文读锁失败"))?;
            
        Ok(contexts
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect())
    }
    
    /// 根据命令搜索上下文
    pub fn search_contexts_by_command(&self, command: &str) -> AppResult<Vec<CommandExecutionContext>> {
        let contexts = self.contexts
            .read()
            .map_err(|_| anyhow!("获取上下文读锁失败"))?;
            
        Ok(contexts
            .iter()
            .filter(|ctx| ctx.command == command)
            .cloned()
            .collect())
    }
    
    /// 清空上下文
    pub fn clear_contexts(&self) -> AppResult<()> {
        let mut contexts = self.contexts
            .write()
            .map_err(|_| anyhow!("获取上下文写锁失败"))?;
            
        contexts.clear();
        Ok(())
    }
}

impl Default for ContextCollector {
    fn default() -> Self {
        Self::new(1000) // 默认保存1000个上下文
    }
}
