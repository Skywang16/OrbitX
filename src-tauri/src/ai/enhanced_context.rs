use crate::ai::types::Message;
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::AppResult;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tiktoken_rs::{cl100k_base, CoreBPE};
use tracing::{debug, info};

// ============= 配置层 =============

/// 简化的上下文管理配置
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// 最大token数量
    pub max_tokens: usize,
    /// 压缩触发阈值(0.0-1.0)
    pub compress_threshold: f32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 120000,       // 适当的token上限
            compress_threshold: 0.70, // 70%触发压缩
        }
    }
}

// ============= 简化的缓存层 =============

/// 简单的缓存项
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 简化的缓存管理器
pub struct SimpleCache {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl SimpleCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 简单的缓存获取
    pub fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.lock().ok()?;
        cache.get(key).map(|entry| entry.content.clone())
    }

    /// 简单的缓存设置
    pub fn set(&self, key: String, content: String) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(
                key,
                CacheEntry {
                    content,
                    created_at: Utc::now(),
                },
            );
        }
    }

    /// 清理过期缓存
    pub fn cleanup_expired(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            let now = Utc::now();
            cache.retain(|_, entry| {
                now.signed_duration_since(entry.created_at).num_seconds() < 3600
                // 1小时过期
            });
        }
    }
}

/// 简化的缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
}

// ============= 管理层 =============

/// 简化的上下文管理器
pub struct ContextManager {
    config: ContextConfig,
    cache: SimpleCache,
    tokenizer: Arc<CoreBPE>,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Self {
        let tokenizer = Arc::new(cl100k_base().expect("failed to init cl100k_base tokenizer"));
        Self {
            cache: SimpleCache::new(),
            tokenizer,
            config,
        }
    }

    /// 构建智能上下文 - 主要API
    pub async fn build_context(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        up_to_msg_id: Option<i64>,
    ) -> AppResult<ContextResult> {
        info!("构建智能上下文: conv={}, up_to={:?}", conv_id, up_to_msg_id);

        // 1. 获取原始消息
        let raw_msgs = self.fetch_messages(repos, conv_id, up_to_msg_id).await?;
        if raw_msgs.is_empty() {
            debug!("消息列表为空");
            return Ok(ContextResult {
                messages: Vec::new(),
                original_count: 0,
                token_count: 0,
                compressed: false,
            });
        }

        debug!("获取到原始消息: {} 条", raw_msgs.len());

        let token_count = self.estimate_tokens(&raw_msgs);
        let original_count = raw_msgs.len();

        // 2. 统一的压缩逻辑
        let processed_msgs = if token_count as f32
            > self.config.max_tokens as f32 * self.config.compress_threshold
        {
            info!(
                "触发压缩: tokens={}/{} ({}%), 消息数={}",
                token_count,
                self.config.max_tokens,
                (token_count as f32 / self.config.max_tokens as f32 * 100.0) as u32,
                original_count
            );

            // 使用30%保留策略，但确保最少保留8条消息
            let keep_ratio = 0.3; // 保留30%
            let min_keep = 8; // 最少保留8条消息
            let keep_count = (raw_msgs.len() as f32 * keep_ratio)
                .max(min_keep as f32)
                .min(raw_msgs.len() as f32) as usize;

            let compress_from = raw_msgs.len().saturating_sub(keep_count);

            debug!("保留最后{}条消息，压缩前{}条", keep_count, compress_from);

            if compress_from > 0 {
                // 生成摘要并替换早期消息
                self.compress_with_summary(repos, conv_id, &raw_msgs, compress_from)
                    .await?
            } else {
                debug!("无需压缩：消息数量太少");
                raw_msgs
            }
        } else {
            debug!(
                "无需压缩: tokens={}/{} ({}%)",
                token_count,
                self.config.max_tokens,
                (token_count as f32 / self.config.max_tokens as f32 * 100.0) as u32
            );
            raw_msgs
        };

        let final_token_count = self.estimate_tokens(&processed_msgs);

        debug!(
            "上下文构建完成: {} -> {} 条消息, tokens: {} -> {}",
            original_count,
            processed_msgs.len(),
            token_count,
            final_token_count
        );

        Ok(ContextResult {
            messages: processed_msgs,
            original_count,
            token_count: final_token_count,
            compressed: token_count as f32
                > self.config.max_tokens as f32 * self.config.compress_threshold,
        })
    }

    /// 构建简化的prompt
    pub async fn build_prompt(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        current_msg: &str,
        up_to_msg_id: Option<i64>,
        current_working_directory: Option<&str>,
    ) -> AppResult<String> {
        debug!(
            "构建prompt: conv_id={}, up_to_msg_id={:?}, current_msg_len={}",
            conv_id,
            up_to_msg_id,
            current_msg.len()
        );

        // 1. 获取上下文消息
        let ctx = self.build_context(repos, conv_id, up_to_msg_id).await?;

        // 2. 构建简单的prompt
        let mut parts = Vec::new();

        // 添加前置提示词
        if let Ok(Some(prefix)) = repos.ai_models().get_user_prefix_prompt().await {
            if !prefix.trim().is_empty() {
                parts.push(format!("【前置提示】\n{}\n", prefix));
            }
        }

        // 添加环境信息
        if let Some(cwd) = current_working_directory {
            if !cwd.trim().is_empty() {
                parts.push(format!("【当前环境】\n工作目录: {}\n", cwd));
            }
        }

        // 添加对话历史
        if !ctx.messages.is_empty() {
            let history = ctx
                .messages
                .iter()
                .map(|m| self.format_message(m))
                .collect::<Vec<_>>()
                .join("\n");

            let compression_info = if ctx.compressed {
                format!("，已压缩至{}条", ctx.messages.len())
            } else {
                String::new()
            };

            parts.push(format!(
                "【对话历史】(共{}条消息{})\n{}\n",
                ctx.original_count, compression_info, history
            ));
        }

        // 添加当前问题
        parts.push(format!("【当前问题】\n{}", current_msg));

        Ok(parts.join("\n"))
    }

    // ============= 私有方法 =============

    /// 简化的压缩函数
    async fn compress_with_summary(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        messages: &[Message],
        compress_from: usize,
    ) -> AppResult<Vec<Message>> {
        debug!(
            "开始压缩: 总消息={}, 压缩前{}条",
            messages.len(),
            compress_from
        );

        let (to_compress, to_keep) = messages.split_at(compress_from);

        if to_compress.is_empty() {
            return Ok(messages.to_vec());
        }

        // 生成简单的摘要
        let summary = self.generate_simple_summary(to_compress);

        // 创建摘要消息
        let summary_msg = Message {
            id: None,
            conversation_id: conv_id,
            role: "system".to_string(),
            content: summary,
            steps_json: None,
            status: Some("complete".to_string()), // 使用数据库允许的status值
            duration_ms: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // 保存摘要消息到数据库
        let _summary_id = repos.conversations().save_message(&summary_msg).await?;

        // 构建新的消息列表：摘要 + 保留的消息
        let mut result = vec![summary_msg];
        result.extend_from_slice(to_keep);

        info!(
            "压缩完成: {}条 -> {}条 (摘要+{}条保留)",
            messages.len(),
            result.len(),
            to_keep.len()
        );
        Ok(result)
    }

    /// 生成智能摘要
    fn generate_simple_summary(&self, messages: &[Message]) -> String {
        let mut summary_parts = Vec::new();

        // 1. 摘要头部
        summary_parts.push("=== 对话摘要 ===".to_string());

        // 2. 统计信息
        let user_msgs = messages.iter().filter(|m| m.role == "user").count();
        let assistant_msgs = messages.iter().filter(|m| m.role == "assistant").count();
        let tool_msgs = messages.iter().filter(|m| m.steps_json.is_some()).count();

        summary_parts.push(format!(
            "压缩了{}条消息: {}条用户消息, {}条助手回复, {}条工具调用",
            messages.len(),
            user_msgs,
            assistant_msgs,
            tool_msgs
        ));

        // 3. 智能提取关键信息
        let key_points = self.extract_key_conversation_points(messages);

        if !key_points.is_empty() {
            summary_parts.push("关键信息:".to_string());
            summary_parts.extend(key_points);
        }

        // 4. 控制摘要长度，避免过长
        let mut summary = summary_parts.join("\n");
        let token_count = self.tokenizer.encode_ordinary(&summary).len();

        if token_count > 1500 {
            // 提高token限制，允许更详细的摘要
            // 如果摘要太长，进行截断
            let max_chars = (summary.chars().count() * 1500) / token_count;
            if max_chars < summary.chars().count() {
                summary = summary.chars().take(max_chars).collect();
                summary.push_str("\n... (摘要已截断)");
            }
        }

        summary_parts.push("=== 摘要结束 ===".to_string());
        summary
    }

    /// 智能提取对话关键点
    fn extract_key_conversation_points(&self, messages: &[Message]) -> Vec<String> {
        let mut key_points = Vec::new();
        let mut seen_topics = HashSet::new();

        // 优先处理最近的消息，但限制数量避免摘要过长
        for msg in messages.iter().rev().take(8) {
            match msg.role.as_str() {
                "user" => {
                    let content = self.truncate_content(&msg.content, 120);
                    // 简单去重：避免相似的用户问题重复
                    let topic_key = self.extract_topic_key(&content);
                    if !seen_topics.contains(&topic_key) {
                        key_points.push(format!("• 用户: {}", content));
                        seen_topics.insert(topic_key);
                    }
                }
                "assistant" => {
                    // 优先保留工具调用信息
                    if let Some(steps) = &msg.steps_json {
                        if let Ok(steps_value) = serde_json::from_str::<serde_json::Value>(steps) {
                            let tool_summary = self.extract_tool_summary(&steps_value);
                            if !tool_summary.is_empty() && tool_summary != "Completed" {
                                key_points.push(format!("• 工具: {}", tool_summary));
                            }
                        }
                    }

                    // 保留有意义的助手回复
                    if !msg.content.trim().is_empty()
                        && !msg.content.contains("AbortError")
                        && !msg.content.contains("我来帮你")
                    // 过滤常见的开场白
                    {
                        let content = self.truncate_content(&msg.content, 120);
                        key_points.push(format!("• 助手: {}", content));
                    }
                }
                _ => {}
            }
        }

        // 反转以保持时间顺序
        key_points.reverse();
        key_points
    }

    /// 智能截断内容
    fn truncate_content(&self, content: &str, max_len: usize) -> String {
        if content.chars().count() <= max_len {
            return content.to_string();
        }

        // 先按字符数截断到安全长度
        let safe_content: String = content.chars().take(max_len).collect();

        // 尝试在句号或换行处截断
        let truncate_at = safe_content
            .rfind('。')
            .or_else(|| safe_content.rfind('\n'))
            .or_else(|| safe_content.rfind(' '))
            .unwrap_or_else(|| {
                // 如果找不到合适的截断点，就截断到max_len-3个字符
                std::cmp::max(3, max_len.saturating_sub(3))
            });

        let truncated: String = safe_content.chars().take(truncate_at).collect();
        format!("{}...", truncated)
    }

    /// 提取话题关键词用于去重
    fn extract_topic_key(&self, content: &str) -> String {
        // 简单的话题提取：取前20个字符作为话题标识
        content.chars().take(20).collect()
    }

    async fn fetch_messages(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        up_to_msg_id: Option<i64>,
    ) -> AppResult<Vec<Message>> {
        debug!(
            "获取消息: conv_id={}, up_to_msg_id={:?}",
            conv_id, up_to_msg_id
        );

        let all = repos
            .conversations()
            .get_messages(conv_id, None, None)
            .await?;

        // 如果指定了up_to_message_id，只获取到该消息为止的历史
        let filtered_msgs = if let Some(up_to_id) = up_to_msg_id {
            all.into_iter()
                .filter(|m| {
                    if let Some(msg_id) = m.id {
                        msg_id <= up_to_id
                    } else {
                        true // 保留没有ID的消息（不应该发生，但为了安全）
                    }
                })
                .collect::<Vec<_>>()
        } else {
            all
        };

        debug!("过滤后消息数量: {}", filtered_msgs.len());

        // 查找最新摘要消息（内容以"=== 对话摘要 ==="开头的 system 消息）
        let latest_summary_idx = filtered_msgs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, m)| m.role == "system" && m.content.starts_with("=== 对话摘要 ==="))
            .map(|(i, _)| i);

        if let Some(idx) = latest_summary_idx {
            // 仅保留该摘要以及其后的消息
            let mut compacted = Vec::new();
            compacted.push(filtered_msgs[idx].clone());
            compacted.extend(filtered_msgs.into_iter().skip(idx + 1));
            debug!("使用摘要压缩，最终消息数量: {}", compacted.len());
            Ok(compacted)
        } else {
            debug!("未找到摘要，返回所有消息: {}", filtered_msgs.len());
            Ok(filtered_msgs)
        }
    }

    /// 智能token估算 - 考虑不同内容类型
    fn estimate_tokens(&self, msgs: &[Message]) -> usize {
        msgs.iter()
            .map(|msg| self.estimate_single_message_tokens(msg))
            .sum()
    }

    /// 估算单条消息的token数
    fn estimate_single_message_tokens(&self, msg: &Message) -> usize {
        // 使用真实分词器进行精确统计
        let mut tokens = self.tokenizer.encode_ordinary(&msg.content).len();
        if let Some(ref steps_json) = msg.steps_json {
            tokens += self.tokenizer.encode_ordinary(steps_json).len();
        }
        tokens += match msg.role.as_str() {
            "system" => 6,
            "assistant" => 4,
            "user" => 3,
            _ => 2,
        };
        tokens
    }

    fn format_message(&self, msg: &Message) -> String {
        if msg.role == "assistant" && msg.steps_json.is_some() {
            let steps_json = msg.steps_json.as_ref().unwrap();
            info!("🔍 原始steps_json: {}", steps_json);

            if let Ok(steps_value) = serde_json::from_str(steps_json) {
                let tool_summary = self.extract_tool_summary(&steps_value);

                // AbortError特殊处理: 只保留工具信息，不显示中断文本
                if msg.content.contains("AbortError") {
                    return format!("assistant: {}", tool_summary);
                }

                // 正常工具消息: 结合工具摘要和最终内容
                return format!("assistant: {}\n{}", tool_summary, msg.content.trim());
            }
        }

        // 默认格式化
        format!("{}: {}", msg.role, msg.content)
    }

    fn extract_tool_summary(&self, steps: &serde_json::Value) -> String {
        if let Some(array) = steps.as_array() {
            let mut segments: Vec<String> = Vec::new();

            for step in array {
                if step.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(tool_exec) = step.get("toolExecution") {
                        let tool_name = tool_exec
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let status = tool_exec
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("completed");

                        // 提取工具输入参数
                        let mut input_text = String::new();
                        if let Some(params) = tool_exec.get("params") {
                            input_text = self.format_tool_params(tool_name, params);
                            debug!("🔧 工具参数格式化: {} -> {}", tool_name, input_text);
                        }

                        // 提取工具输出文本
                        let mut output_text = String::new();
                        if let Some(result) = tool_exec.get("result") {
                            // 1) 字符串结果
                            if let Some(s) = result.as_str() {
                                output_text = s.to_string();
                            // 2) 简单对象含text字段
                            } else if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
                                output_text = text.to_string();
                            // 3) 标准对象数组内容
                            } else if let Some(contents) =
                                result.get("content").and_then(|c| c.as_array())
                            {
                                let mut pieces: Vec<String> = Vec::new();
                                for item in contents {
                                    if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                                        pieces.push(t.to_string());
                                    } else if let Some(p) =
                                        item.get("path").and_then(|p| p.as_str())
                                    {
                                        pieces.push(format!("[file] {}", p));
                                    } else if let Some(url) =
                                        item.get("url").and_then(|u| u.as_str())
                                    {
                                        pieces.push(format!("[url] {}", url));
                                    }
                                }
                                output_text = pieces.join("\n");
                            }
                        }

                        // 错误检测：文本中包含ToolError或状态为failed/error
                        let is_error = output_text.contains("ToolError:")
                            || tool_exec
                                .get("status")
                                .and_then(|s| s.as_str())
                                .map(|s| {
                                    s.eq_ignore_ascii_case("failed")
                                        || s.eq_ignore_ascii_case("error")
                                })
                                .unwrap_or(false);

                        let header = if is_error {
                            format!("{}(failed)", tool_name)
                        } else {
                            format!("{}({})", tool_name, status)
                        };

                        // 构建完整的工具信息（输入 + 输出）
                        let mut tool_info_parts = Vec::new();

                        // 添加输入参数（如果有）
                        if !input_text.trim().is_empty() {
                            tool_info_parts.push(format!("Input: {}", input_text));
                        }

                        // 添加输出结果（如果有）
                        if !output_text.trim().is_empty() {
                            tool_info_parts.push(format!("Output: {}", output_text));
                        }

                        if !tool_info_parts.is_empty() {
                            segments.push(format!("{}:\n{}", header, tool_info_parts.join("\n")));
                        } else {
                            segments.push(header);
                        }
                    }
                }
            }

            if !segments.is_empty() {
                return format!("Tools: {}", segments.join("\n\n"));
            }
        }

        "Completed".to_string()
    }

    /// 格式化工具参数为可读文本
    fn format_tool_params(&self, tool_name: &str, params: &serde_json::Value) -> String {
        match tool_name {
            // 文件操作工具
            "read_file" | "edit_file" | "create_file" | "write_file" => {
                if let Some(path) = params.get("path").and_then(|p| p.as_str()) {
                    let mut parts = vec![format!("path: {}", path)];

                    if let Some(start) = params.get("startLine").and_then(|s| s.as_u64()) {
                        parts.push(format!("startLine: {}", start));
                    }
                    if let Some(end) = params.get("endLine").and_then(|e| e.as_u64()) {
                        parts.push(format!("endLine: {}", end));
                    }
                    if let Some(content) = params.get("content").and_then(|c| c.as_str()) {
                        let preview = if content.chars().count() > 50 {
                            format!("{}...", content.chars().take(47).collect::<String>())
                        } else {
                            content.to_string()
                        };
                        parts.push(format!("content: {}", preview));
                    }

                    parts.join(", ")
                } else {
                    self.format_generic_params(params)
                }
            }
            "read_many_files" => {
                if let Some(paths) = params.get("paths").and_then(|p| p.as_array()) {
                    format!("paths: {} files", paths.len())
                } else {
                    self.format_generic_params(params)
                }
            }
            // 命令执行工具
            "shell" | "bash" | "execute" | "run_command" => {
                if let Some(command) = params.get("command").and_then(|c| c.as_str()) {
                    let cmd_preview = if command.chars().count() > 80 {
                        format!("{}...", command.chars().take(77).collect::<String>())
                    } else {
                        command.to_string()
                    };
                    format!("command: {}", cmd_preview)
                } else {
                    self.format_generic_params(params)
                }
            }
            // 网络工具
            "web_fetch" | "fetch_url" | "http_get" => {
                if let Some(url) = params.get("url").and_then(|u| u.as_str()) {
                    format!("url: {}", url)
                } else {
                    self.format_generic_params(params)
                }
            }
            // 搜索工具
            "orbit_search" | "search" | "web_search" => {
                if let Some(query) = params.get("query").and_then(|q| q.as_str()) {
                    format!("query: {}", query)
                } else {
                    self.format_generic_params(params)
                }
            }
            // 代码分析工具
            "analyze_code" | "code_review" => {
                let mut parts = Vec::new();
                if let Some(path) = params.get("path").and_then(|p| p.as_str()) {
                    parts.push(format!("path: {}", path));
                }
                if let Some(lang) = params.get("language").and_then(|l| l.as_str()) {
                    parts.push(format!("language: {}", lang));
                }
                if parts.is_empty() {
                    self.format_generic_params(params)
                } else {
                    parts.join(", ")
                }
            }
            _ => {
                // 对于未知工具，使用通用格式化
                self.format_generic_params(params)
            }
        }
    }

    /// 通用参数格式化函数
    fn format_generic_params(&self, params: &serde_json::Value) -> String {
        if let Some(obj) = params.as_object() {
            let mut parts = Vec::new();

            // 优先显示常见的重要参数
            let priority_keys = [
                "path", "command", "query", "url", "file", "content", "input",
            ];

            // 先处理优先参数
            for &key in &priority_keys {
                if let Some(value) = obj.get(key) {
                    let value_str = self.format_param_value(value);
                    parts.push(format!("{}: {}", key, value_str));
                }
            }

            // 再处理其他参数，但限制总数
            for (key, value) in obj.iter() {
                if parts.len() >= 3 {
                    break;
                } // 最多显示3个参数
                if !priority_keys.contains(&key.as_str()) {
                    let value_str = self.format_param_value(value);
                    parts.push(format!("{}: {}", key, value_str));
                }
            }

            if parts.is_empty() {
                "[no params]".to_string()
            } else {
                parts.join(", ")
            }
        } else {
            // 非对象类型的参数
            self.format_param_value(params)
        }
    }

    /// 格式化单个参数值
    fn format_param_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                if s.chars().count() > 60 {
                    format!("{}...", s.chars().take(57).collect::<String>())
                } else {
                    s.clone()
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.len() <= 3 {
                    format!(
                        "[{}]",
                        arr.iter()
                            .map(|v| self.format_param_value(v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            serde_json::Value::Object(_) => "[object]".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Null => "null".to_string(),
        }
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            total_entries: 0, // 简化版本，不统计具体数量
        }
    }

    /// 清理缓存
    pub fn cleanup_cache(&self) {
        self.cache.cleanup_expired()
    }

    /// 失效缓存（兼容性方法）
    pub fn invalidate_cache(&self, _conv_id: i64) {
        // 简化版本，不做具体操作
        info!("缓存失效请求已忽略（简化版本）");
    }
}

// ============= 结果类型 =============

/// 上下文构建结果
#[derive(Debug)]
pub struct ContextResult {
    pub messages: Vec<Message>,
    pub original_count: usize,
    pub token_count: usize,
    pub compressed: bool,
}

impl ContextResult {
    /// 转为AI上下文格式
    pub fn to_ai_context(self) -> crate::ai::types::AIContext {
        crate::ai::types::AIContext {
            chat_history: Some(self.messages),
            ..Default::default()
        }
    }
}

// ============= 工厂方法 =============

/// 创建默认上下文管理器
pub fn create_context_manager() -> ContextManager {
    ContextManager::new(ContextConfig::default())
}

/// 创建自定义配置的上下文管理器
pub fn create_context_manager_with_config(config: ContextConfig) -> ContextManager {
    ContextManager::new(config)
}
