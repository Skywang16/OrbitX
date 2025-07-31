/*!
 * AI相关的数据类型定义
 */

use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use uuid::Uuid;

// ===== AI提供商类型 =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AIProvider {
    OpenAI,
    Claude,
    Local,
    Custom,
}

// ===== AI模型配置 =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    pub name: String,
    pub provider: AIProvider,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub is_default: Option<bool>,
    pub options: Option<AIModelOptions>,
}

impl AIModelConfig {
    /// 创建新的AI模型配置
    pub fn new(
        name: String,
        provider: AIProvider,
        api_url: String,
        api_key: String,
        model: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            provider,
            api_url,
            api_key,
            model,
            is_default: Some(false),
            options: None,
        }
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Model ID cannot be empty".to_string());
        }
        if self.name.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        if self.api_url.is_empty() {
            return Err("API URL cannot be empty".to_string());
        }
        if self.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        // 验证URL格式
        if !self.api_url.starts_with("http://") && !self.api_url.starts_with("https://") {
            return Err("API URL must start with http:// or https://".to_string());
        }

        Ok(())
    }

    /// 检查是否为默认模型
    pub fn is_default(&self) -> bool {
        self.is_default.unwrap_or(false)
    }

    /// 设置为默认模型
    pub fn set_default(&mut self, is_default: bool) {
        self.is_default = Some(is_default);
    }

    /// 获取超时设置
    pub fn timeout(&self) -> u64 {
        self.options
            .as_ref()
            .and_then(|opts| opts.timeout)
            .unwrap_or(30) // 默认30秒
    }

    /// 获取最大token数
    pub fn max_tokens(&self) -> u32 {
        self.options
            .as_ref()
            .and_then(|opts| opts.max_tokens)
            .unwrap_or(4096) // 默认4096 tokens
    }

    /// 获取温度参数
    pub fn temperature(&self) -> f32 {
        self.options
            .as_ref()
            .and_then(|opts| opts.temperature)
            .unwrap_or(0.7) // 默认0.7
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AIModelOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub timeout: Option<u64>,
    pub custom_config: Option<String>, // JSON字符串形式的自定义配置
}

// ===== AI请求和响应类型 =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AIRequestType {
    Chat,
    Explanation,
    ErrorAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRequest {
    pub request_type: AIRequestType,
    pub content: String,
    pub context: Option<AIContext>,
    pub options: Option<AIRequestOptions>,
}

impl AIRequest {
    /// 创建新的AI请求
    pub fn new(request_type: AIRequestType, content: String) -> Self {
        Self {
            request_type,
            content,
            context: None,
            options: None,
        }
    }

    /// 创建聊天请求
    pub fn chat(content: String) -> Self {
        Self::new(AIRequestType::Chat, content)
    }

    /// 创建解释请求
    pub fn explanation(content: String) -> Self {
        Self::new(AIRequestType::Explanation, content)
    }

    /// 创建错误分析请求
    pub fn error_analysis(content: String) -> Self {
        Self::new(AIRequestType::ErrorAnalysis, content)
    }

    /// 添加上下文
    pub fn with_context(mut self, context: AIContext) -> Self {
        self.context = Some(context);
        self
    }

    /// 添加选项
    pub fn with_options(mut self, options: AIRequestOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// 验证请求是否有效
    pub fn validate(&self) -> Result<(), String> {
        if self.content.trim().is_empty() {
            return Err("Request content cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AIContext {
    pub working_directory: Option<String>,
    pub command_history: Option<Vec<String>>,
    pub environment: Option<HashMap<String, String>>,
    pub current_command: Option<String>,
    pub last_output: Option<String>,
    pub system_info: Option<SystemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRequestOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIResponse {
    pub content: String,
    pub response_type: AIResponseType,
    pub suggestions: Option<Vec<String>>,
    pub metadata: Option<AIResponseMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIResponseType {
    Text,
    Code,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIResponseMetadata {
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
    pub response_time: Option<u64>,
}

// ===== 聊天相关类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub message_type: ChatMessageType,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<ChatMessageMetadata>,
}

impl ChatMessage {
    /// 创建新的聊天消息
    pub fn new(message_type: ChatMessageType, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type,
            content,
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    /// 创建用户消息
    pub fn user(content: String) -> Self {
        Self::new(ChatMessageType::User, content)
    }

    /// 创建助手消息
    pub fn assistant(content: String) -> Self {
        Self::new(ChatMessageType::Assistant, content)
    }

    /// 创建系统消息
    pub fn system(content: String) -> Self {
        Self::new(ChatMessageType::System, content)
    }

    /// 添加元数据
    pub fn with_metadata(mut self, metadata: ChatMessageMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 检查是否为用户消息
    pub fn is_user(&self) -> bool {
        matches!(self.message_type, ChatMessageType::User)
    }

    /// 检查是否为助手消息
    pub fn is_assistant(&self) -> bool {
        matches!(self.message_type, ChatMessageType::Assistant)
    }

    /// 检查是否为系统消息
    pub fn is_system(&self) -> bool {
        matches!(self.message_type, ChatMessageType::System)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatMessageType {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageMetadata {
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub model_id: Option<String>,
}

impl ChatSession {
    /// 创建新的聊天会话
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            model_id: None,
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// 获取最后一条消息
    pub fn last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    /// 获取用户消息数量
    pub fn user_message_count(&self) -> usize {
        self.messages.iter().filter(|m| m.is_user()).count()
    }

    /// 获取助手消息数量
    pub fn assistant_message_count(&self) -> usize {
        self.messages.iter().filter(|m| m.is_assistant()).count()
    }

    /// 清空消息历史
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    /// 设置模型ID
    pub fn set_model_id(&mut self, model_id: String) {
        self.model_id = Some(model_id);
        self.updated_at = Utc::now();
    }

    /// 获取会话摘要（用于显示）
    pub fn summary(&self) -> String {
        if self.messages.is_empty() {
            "Empty conversation".to_string()
        } else if let Some(first_user_msg) = self.messages.iter().find(|m| m.is_user()) {
            let content = &first_user_msg.content;
            if content.len() > 50 {
                format!("{}...", &content[..47])
            } else {
                content.clone()
            }
        } else {
            "No user messages".to_string()
        }
    }
}

// ===== 命令解释类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExplanation {
    pub command: String,
    pub explanation: String,
    pub breakdown: Option<Vec<CommandPart>>,
    pub risks: Option<Vec<RiskWarning>>,
    pub alternatives: Option<Vec<CommandAlternative>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPart {
    pub part: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskWarning {
    pub level: RiskLevel,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAlternative {
    pub command: String,
    pub description: String,
    pub reason: String,
}

// ===== 错误分析类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorAnalysis {
    pub error: String,
    pub command: String,
    pub analysis: String,
    pub possible_causes: Vec<String>,
    pub solutions: Vec<ErrorSolution>,
    pub related_docs: Option<Vec<DocumentLink>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorSolution {
    pub description: String,
    pub command: Option<String>,
    pub priority: SolutionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SolutionPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLink {
    pub title: String,
    pub url: String,
}

// ===== AI设置类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AISettings {
    pub models: Vec<AIModelConfig>,
    pub default_model_id: Option<String>,
    pub features: AIFeatureSettings,
    pub performance: AIPerformanceSettings,
}

impl AISettings {
    /// 获取默认模型
    pub fn default_model(&self) -> Option<&AIModelConfig> {
        if let Some(default_id) = &self.default_model_id {
            self.models.iter().find(|m| &m.id == default_id)
        } else {
            self.models.iter().find(|m| m.is_default())
        }
    }

    /// 根据ID查找模型
    pub fn find_model(&self, id: &str) -> Option<&AIModelConfig> {
        self.models.iter().find(|m| m.id == id)
    }

    /// 根据ID查找模型（可变引用）
    pub fn find_model_mut(&mut self, id: &str) -> Option<&mut AIModelConfig> {
        self.models.iter_mut().find(|m| m.id == id)
    }

    /// 添加模型
    pub fn add_model(&mut self, model: AIModelConfig) -> Result<(), String> {
        // 验证模型配置
        model.validate()?;

        // 检查ID是否已存在
        if self.models.iter().any(|m| m.id == model.id) {
            return Err(format!("Model with ID '{}' already exists", model.id));
        }

        // 如果这是第一个模型，设置为默认
        if self.models.is_empty() {
            self.default_model_id = Some(model.id.clone());
        }

        self.models.push(model);
        Ok(())
    }

    /// 移除模型
    pub fn remove_model(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .models
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| format!("Model with ID '{id}' not found"))?;

        let removed_model = self.models.remove(index);

        // 如果移除的是默认模型，清除默认设置
        if Some(&removed_model.id) == self.default_model_id.as_ref() {
            self.default_model_id = None;
            // 如果还有其他模型，选择第一个作为默认
            if let Some(first_model) = self.models.first() {
                self.default_model_id = Some(first_model.id.clone());
            }
        }

        Ok(())
    }

    /// 设置默认模型
    pub fn set_default_model(&mut self, id: &str) -> Result<(), String> {
        if !self.models.iter().any(|m| m.id == id) {
            return Err(format!("Model with ID '{id}' not found"));
        }

        // 清除所有模型的默认标记
        for model in &mut self.models {
            model.set_default(false);
        }

        // 设置新的默认模型
        if let Some(model) = self.find_model_mut(id) {
            model.set_default(true);
        }

        self.default_model_id = Some(id.to_string());
        Ok(())
    }

    /// 验证设置
    pub fn validate(&self) -> Result<(), String> {
        // 验证所有模型配置
        for model in &self.models {
            model.validate()?;
        }

        // 验证默认模型ID是否存在
        if let Some(default_id) = &self.default_model_id {
            if !self.models.iter().any(|m| &m.id == default_id) {
                return Err("Default model ID does not exist".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct AIFeatureSettings {
    pub chat: ChatSettings,
    pub explanation: ExplanationSettings,
    pub error_analysis: ErrorAnalysisSettings,
    pub user_prefix_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSettings {
    pub enabled: bool,
    pub max_history_length: usize,
    pub auto_save_history: bool,
    pub context_window_size: usize,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_length: usize::MAX, // 无限历史长度
            auto_save_history: true,
            context_window_size: 4000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationSettings {
    pub enabled: bool,
    pub show_risks: bool,
    pub include_alternatives: bool,
}

impl Default for ExplanationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_risks: true,
            include_alternatives: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorAnalysisSettings {
    pub enabled: bool,
    pub auto_analyze: bool,
    pub show_solutions: bool,
}

impl Default for ErrorAnalysisSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_analyze: false,
            show_solutions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIPerformanceSettings {
    pub request_timeout: u64,
    pub max_concurrent_requests: usize,
    pub cache_enabled: bool,
    pub cache_ttl: u64,
}

impl Default for AIPerformanceSettings {
    fn default() -> Self {
        Self {
            request_timeout: 30,
            max_concurrent_requests: 5,
            cache_enabled: true,
            cache_ttl: 3600, // 1小时
        }
    }
}

// ===== 统计和监控类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub tokens_used: u64,
    pub cache_hit_rate: Option<f64>,
    pub model_usage: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIHealthStatus {
    pub model_id: String,
    pub status: HealthStatus,
    pub last_checked: DateTime<Utc>,
    pub response_time: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// ===== 缓存相关类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub timestamp: DateTime<Utc>,
    pub ttl: u64,
    pub hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_rate: f64,
    pub memory_usage: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

// ===== 流式响应类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub content: String,
    pub is_complete: bool,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 流式响应类型别名
pub type AIStreamResponse =
    Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::utils::error::AppError>> + Send>>;

// ===== 批量请求类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub id: String,
    pub requests: Vec<AIRequest>,
    pub options: Option<BatchRequestOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequestOptions {
    pub max_concurrent: Option<usize>,
    pub timeout_per_request: Option<u64>,
    pub fail_fast: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponse {
    pub id: String,
    pub responses: Vec<BatchItemResponse>,
    pub metadata: Option<BatchResponseMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResponse {
    pub index: usize,
    pub result: Result<AIResponse, String>,
    pub processing_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponseMetadata {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_processing_time: u64,
}

// ===== 适配器能力类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCapabilities {
    pub supports_streaming: bool,
    pub supports_batch: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub max_tokens: Option<u32>,
    pub max_batch_size: Option<usize>,
    pub supported_models: Vec<String>,
}

// ===== 健康检查类型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckRequest {
    pub include_latency: bool,
    pub include_model_info: bool,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub latency: Option<u64>,
    pub model_info: Option<ModelInfo>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub version: Option<String>,
    pub context_length: Option<u32>,
    pub capabilities: Option<AdapterCapabilities>,
}
