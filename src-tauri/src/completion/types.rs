//! 补全功能相关的类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 补全项类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionType {
    /// 文件路径
    File,
    /// 目录路径
    Directory,
    /// 可执行命令
    Command,
    /// 历史命令
    History,
    /// 环境变量
    Environment,
    /// 别名
    Alias,
    /// 函数
    Function,
    /// 命令选项
    Option,
    /// 子命令
    Subcommand,
    /// 选项值
    Value,
}

/// 补全项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    /// 补全文本
    pub text: String,

    /// 显示文本（可能包含额外信息）
    pub display_text: Option<String>,

    /// 补全类型 (前端期望字段名为 kind)
    #[serde(rename = "kind")]
    pub completion_type: String,

    /// 描述信息
    pub description: Option<String>,

    /// 匹配分数（用于排序）
    pub score: f64,

    /// 补全来源 (前端需要的字段)
    pub source: Option<String>,

    /// 是否为精确匹配 (前端不使用，跳过序列化)
    #[serde(skip)]
    pub exact_match: bool,

    /// 额外元数据 (前端不使用，跳过序列化)
    #[serde(skip)]
    pub metadata: HashMap<String, String>,
}

impl CompletionItem {
    /// 创建新的补全项
    pub fn new(text: impl Into<String>, completion_type: CompletionType) -> Self {
        Self {
            text: text.into(),
            display_text: None,
            completion_type: completion_type.to_string(),
            description: None,
            score: 0.0,
            source: None,
            exact_match: false,
            metadata: HashMap::new(),
        }
    }

    /// 设置显示文本
    pub fn with_display_text(mut self, display_text: impl Into<String>) -> Self {
        self.display_text = Some(display_text.into());
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置分数
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    /// 设置来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// 设置为精确匹配
    pub fn with_exact_match(mut self, exact: bool) -> Self {
        self.exact_match = exact;
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl PartialOrd for CompletionItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CompletionItem {}

impl Ord for CompletionItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 按分数降序排序，分数相同则按文本字母序
        other.score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.text.cmp(&other.text))
    }
}

impl fmt::Display for CompletionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::File => "file",
                Self::Directory => "directory",
                Self::Command => "command",
                Self::History => "history",
                Self::Environment => "environment",
                Self::Alias => "alias",
                Self::Function => "function",
                Self::Option => "option",
                Self::Subcommand => "subcommand",
                Self::Value => "value",
            }
        )
    }
}

/// 补全上下文
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// 当前输入的完整命令行
    pub input: String,

    /// 光标位置
    pub cursor_position: usize,

    /// 当前工作目录
    pub working_directory: PathBuf,

    /// 当前正在补全的词
    pub current_word: String,

    /// 当前词的开始位置
    pub word_start: usize,

    /// 命令行解析结果
    pub parsed_command: Option<ParsedCommand>,
}

impl CompletionContext {
    /// 创建新的补全上下文
    pub fn new(input: String, cursor_position: usize, working_directory: PathBuf) -> Self {
        let (current_word, word_start) = Self::extract_current_word(&input, cursor_position);

        Self {
            input,
            cursor_position,
            working_directory,
            current_word,
            word_start,
            parsed_command: None,
        }
    }

    /// 提取当前正在编辑的词
    fn extract_current_word(input: &str, cursor_position: usize) -> (String, usize) {
        let chars: Vec<char> = input.chars().collect();
        let cursor_pos = cursor_position.min(chars.len());

        // 向前查找词的开始
        let mut start = cursor_pos;
        while start > 0 {
            let ch = chars[start - 1];
            if ch.is_whitespace() || ch == '|' || ch == '&' || ch == ';' {
                break;
            }
            start -= 1;
        }

        // 向后查找词的结束
        let mut end = cursor_pos;
        while end < chars.len() {
            let ch = chars[end];
            if ch.is_whitespace() || ch == '|' || ch == '&' || ch == ';' {
                break;
            }
            end += 1;
        }

        let word: String = chars[start..end].iter().collect();
        (word, start)
    }
}

/// 解析后的命令
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// 命令名称
    pub command: String,

    /// 参数列表
    pub args: Vec<String>,

    /// 当前正在补全的参数位置
    pub current_arg_index: usize,

    /// 是否在补全命令名称
    pub completing_command: bool,
}

/// 补全请求
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
    /// 输入文本
    pub input: String,

    /// 光标位置
    pub cursor_position: usize,

    /// 工作目录
    pub working_directory: String,

    /// 最大返回结果数
    pub max_results: Option<usize>,
}

/// 补全响应
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    /// 补全项列表
    pub items: Vec<CompletionItem>,

    /// 替换的开始位置
    pub replace_start: usize,

    /// 替换的结束位置
    pub replace_end: usize,

    /// 是否有更多结果
    pub has_more: bool,
}

/// 命令执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionContext {
    /// 命令文本
    pub command: String,

    /// 命令参数
    pub args: Vec<String>,

    /// 执行时间戳
    pub timestamp: u64,

    /// 工作目录
    pub working_directory: String,

    /// 命令输出
    pub output: Option<CommandOutput>,

    /// 退出码
    pub exit_code: Option<i32>,

    /// 执行持续时间（毫秒）
    pub duration: Option<u64>,

    /// 环境变量（仅保存关键的）
    pub environment: HashMap<String, String>,
}

/// 命令输出
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput {
    /// 标准输出
    pub stdout: String,

    /// 标准错误输出
    pub stderr: String,

    /// 解析出的结构化数据
    pub parsed_data: Option<ParsedOutputData>,
}

/// 解析的输出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedOutputData {
    /// 数据类型
    pub data_type: OutputDataType,

    /// 提取的实体
    pub entities: Vec<OutputEntity>,

    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 输出数据类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OutputDataType {
    /// 进程列表（如 ps, lsof 输出）
    ProcessList,

    /// 文件列表（如 ls 输出）
    FileList,

    /// 网络信息（如 netstat 输出）
    NetworkInfo,

    /// 系统信息（如 top, htop 输出）
    SystemInfo,

    /// Git 信息
    GitInfo,

    /// 包管理器信息
    PackageInfo,

    /// 服务信息
    ServiceInfo,

    /// 未知类型
    Unknown,
}

/// 输出实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEntity {
    /// 实体类型
    pub entity_type: EntityType,

    /// 实体值
    pub value: String,

    /// 实体描述
    pub description: Option<String>,

    /// 相关属性
    pub attributes: HashMap<String, String>,

    /// 置信度分数
    pub confidence: f64,
}

/// 实体类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    /// 进程ID
    ProcessId,

    /// 端口号
    Port,

    /// 文件路径
    FilePath,

    /// 目录路径
    DirectoryPath,

    /// IP地址
    IpAddress,

    /// 用户名
    Username,

    /// 服务名
    ServiceName,

    /// 包名
    PackageName,

    /// Git分支
    GitBranch,

    /// Git提交哈希
    GitCommit,

    /// 环境变量
    EnvironmentVariable,

    /// 其他
    Other,
}

/// 上下文会话
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSession {
    /// 会话ID
    pub session_id: String,

    /// 会话开始时间
    pub start_time: u64,

    /// 最后活动时间
    pub last_activity: u64,

    /// 命令执行历史
    pub command_history: Vec<CommandExecutionContext>,

    /// 会话状态
    pub state: SessionState,

    /// 工作目录历史
    pub directory_history: Vec<String>,
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    /// 活跃状态
    Active,

    /// 空闲状态
    Idle,

    /// 已结束
    Ended,
}

impl CommandExecutionContext {
    /// 创建新的命令执行上下文
    pub fn new(command: String, args: Vec<String>, working_directory: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            command,
            args,
            timestamp,
            working_directory,
            output: None,
            exit_code: None,
            duration: None,
            environment: HashMap::new(),
        }
    }

    /// 设置命令输出
    pub fn with_output(mut self, output: CommandOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// 设置退出码
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }

    /// 设置执行持续时间
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self
    }

    /// 添加环境变量
    pub fn with_environment(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// 获取完整的命令行
    pub fn get_full_command(&self) -> String {
        if self.args.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.args.join(" "))
        }
    }

    /// 检查命令是否成功执行
    pub fn is_successful(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// 获取相关的实体
    pub fn get_entities(&self) -> Vec<&OutputEntity> {
        self.output
            .as_ref()
            .and_then(|output| output.parsed_data.as_ref())
            .map(|data| data.entities.iter().collect())
            .unwrap_or_default()
    }

    /// 根据类型获取实体
    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&OutputEntity> {
        self.get_entities()
            .into_iter()
            .filter(|entity| &entity.entity_type == entity_type)
            .collect()
    }
}

impl CommandOutput {
    /// 创建新的命令输出
    pub fn new(stdout: String, stderr: String) -> Self {
        Self {
            stdout,
            stderr,
            parsed_data: None,
        }
    }

    /// 设置解析数据
    pub fn with_parsed_data(mut self, parsed_data: ParsedOutputData) -> Self {
        self.parsed_data = Some(parsed_data);
        self
    }

    /// 检查是否有输出
    pub fn has_output(&self) -> bool {
        !self.stdout.is_empty() || !self.stderr.is_empty()
    }

    /// 获取所有输出文本
    pub fn get_all_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
            .trim()
            .to_string()
    }
}

impl ParsedOutputData {
    /// 创建新的解析输出数据
    pub fn new(data_type: OutputDataType) -> Self {
        Self {
            data_type,
            entities: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 添加实体
    pub fn add_entity(mut self, entity: OutputEntity) -> Self {
        self.entities.push(entity);
        self
    }

    /// 添加元数据
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// 根据类型获取实体
    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&OutputEntity> {
        self.entities
            .iter()
            .filter(|entity| &entity.entity_type == entity_type)
            .collect()
    }

    /// 获取高置信度的实体
    pub fn get_high_confidence_entities(&self, min_confidence: f64) -> Vec<&OutputEntity> {
        self.entities
            .iter()
            .filter(|entity| entity.confidence >= min_confidence)
            .collect()
    }
}

impl OutputEntity {
    /// 创建新的输出实体
    pub fn new(entity_type: EntityType, value: String, confidence: f64) -> Self {
        Self {
            entity_type,
            value,
            description: None,
            attributes: HashMap::new(),
            confidence,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 检查是否为高置信度实体
    pub fn is_high_confidence(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
}

impl ContextSession {
    /// 创建新的上下文会话
    pub fn new(session_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            session_id,
            start_time: now,
            last_activity: now,
            command_history: Vec::new(),
            state: SessionState::Active,
            directory_history: Vec::new(),
        }
    }

    /// 添加命令执行上下文
    pub fn add_command_context(&mut self, context: CommandExecutionContext) {
        self.command_history.push(context);
        self.update_last_activity();

        // 限制历史记录数量
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }
    }

    /// 更新最后活动时间
    pub fn update_last_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 获取最近的命令
    pub fn get_recent_commands(&self, count: usize) -> Vec<&CommandExecutionContext> {
        self.command_history.iter().rev().take(count).collect()
    }

    /// 根据命令名称搜索历史
    pub fn search_by_command(&self, command: &str) -> Vec<&CommandExecutionContext> {
        self.command_history
            .iter()
            .filter(|ctx| ctx.command == command)
            .collect()
    }

    /// 获取相关的实体
    pub fn get_related_entities(
        &self,
        entity_type: &EntityType,
        limit: usize,
    ) -> Vec<&OutputEntity> {
        let mut entities = Vec::new();

        for context in self.command_history.iter().rev() {
            for entity in context.get_entities_by_type(entity_type) {
                entities.push(entity);
                if entities.len() >= limit {
                    break;
                }
            }
            if entities.len() >= limit {
                break;
            }
        }

        entities
    }

    /// 检查会话是否活跃
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// 结束会话
    pub fn end_session(&mut self) {
        self.state = SessionState::Ended;
        self.update_last_activity();
    }
}
