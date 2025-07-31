/*!
 * 提示词引擎模块
 */

use crate::ai::{AIRequest, AIRequestType};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 提示词模板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub name: String,
    pub template: String,
    pub variables: Vec<String>,
    pub description: Option<String>,
    pub version: String,
}

impl PromptTemplate {
    /// 创建新的提示词模板
    pub fn new(name: String, template: String, variables: Vec<String>) -> Self {
        Self {
            name,
            template,
            variables,
            description: None,
            version: "1.0".to_string(),
        }
    }

    /// 添加描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// 设置版本
    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    /// 验证模板变量
    pub fn validate(&self) -> AppResult<()> {
        // 检查模板中是否包含所有声明的变量
        for var in &self.variables {
            let placeholder = format!("{{{var}}}");
            if !self.template.contains(&placeholder) {
                return Err(anyhow!(
                    "AI输入验证错误: Template '{}' declares variable '{}' but doesn't use it",
                    self.name,
                    var
                ));
            }
        }
        Ok(())
    }
}

/// 提示词引擎
#[derive(Debug)]
pub struct PromptEngine {
    templates: HashMap<AIRequestType, PromptTemplate>,
    custom_templates: HashMap<String, PromptTemplate>,
    template_cache: HashMap<String, String>,
}

/// 提示词生成选项
#[derive(Debug, Clone)]
pub struct PromptOptions {
    pub include_system_info: bool,
    pub max_history_length: usize,
    pub include_environment: bool,
    pub custom_variables: HashMap<String, String>,
    pub user_prefix_prompt: Option<String>,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            include_system_info: true,
            max_history_length: 10,
            include_environment: false,
            custom_variables: HashMap::new(),
            user_prefix_prompt: None,
        }
    }
}

impl Default for PromptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            templates: HashMap::new(),
            custom_templates: HashMap::new(),
            template_cache: HashMap::new(),
        };

        engine.load_default_templates();
        engine
    }

    /// 生成提示词
    pub fn generate_prompt(&self, request: &AIRequest) -> AppResult<String> {
        let template = self.templates.get(&request.request_type).ok_or_else(|| {
            anyhow!(
                "AI输入验证错误: No template found for request type: {:?}",
                request.request_type
            )
        })?;

        let mut prompt = template.template.clone();

        // 替换基本变量
        prompt = prompt.replace("{content}", &request.content);

        // 替换上下文变量
        if let Some(context) = &request.context {
            if let Some(working_dir) = &context.working_directory {
                prompt = prompt.replace("{working_directory}", working_dir);
            }

            if let Some(history) = &context.command_history {
                let history_str = history.join("\n");
                prompt = prompt.replace("{command_history}", &history_str);
            }

            if let Some(env) = &context.environment {
                let env_str = env
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                prompt = prompt.replace("{environment}", &env_str);
            }
        }

        Ok(prompt)
    }

    /// 加载默认模板
    fn load_default_templates(&mut self) {
        // 聊天模板
        self.templates.insert(
            AIRequestType::Chat,
            PromptTemplate::new(
                "chat".to_string(),
                r#"
你是一个专业的终端助手，专门为终端模拟器用户提供智能协助。你具备以下能力：

🔧 **核心技能**：
- 命令行操作指导和优化建议
- Shell脚本编写和调试
- 系统管理和故障排除
- 开发工具使用指导
- 文件系统操作和权限管理

📍 **当前环境**：
- 工作目录：{working_directory}
- 最近命令：{command_history}
- 环境信息：{environment}

💬 **用户问题**：{content}

📋 **回复原则**：
1. 提供准确、实用的命令和解决方案
2. 解释命令的作用和潜在风险
3. 根据用户技能水平调整回复复杂度
4. 优先推荐安全、高效的方法
5. 必要时提供相关文档链接

请基于上述信息提供专业、有用的回复。
"#
                .to_string(),
                vec![
                    "content".to_string(),
                    "working_directory".to_string(),
                    "command_history".to_string(),
                    "environment".to_string(),
                ],
            )
            .with_description("为终端用户提供专业的命令行协助和技术支持".to_string()),
        );

        // 命令解释模板
        self.templates.insert(
            AIRequestType::Explanation,
            PromptTemplate::new(
                "explanation".to_string(),
                r#"
作为终端专家，请详细解释以下命令：

🔍 **待解释命令**：{content}
📂 **执行环境**：{working_directory}

📚 **请提供结构化解释**：

**基本信息**：
- 命令的主要功能和用途
- 适用的操作系统和Shell环境

**详细分解**：
- 逐个解释命令的各个部分
- 参数和选项的含义
- 管道和重定向的作用

**安全评估**：
- 潜在的安全风险（如果有）
- 对系统的影响程度
- 需要的权限级别

**最佳实践**：
- 使用建议和注意事项
- 常见错误和避免方法
- 更安全或高效的替代方案

请以JSON格式回复：
{
  "explanation": "命令的详细解释",
  "breakdown": [
    {"part": "命令部分", "description": "功能说明", "type": "command|option|argument"}
  ],
  "risks": [
    {"level": "low|medium|high", "description": "风险描述", "mitigation": "缓解措施"}
  ],
  "alternatives": [
    {"command": "替代命令", "description": "说明", "advantage": "优势"}
  ],
  "best_practices": ["最佳实践建议1", "最佳实践建议2"]
}
"#
                .to_string(),
                vec!["content".to_string(), "working_directory".to_string()],
            )
            .with_description("为终端用户提供专业的命令解释和安全分析".to_string()),
        );

        // 错误分析模板
        self.templates.insert(
            AIRequestType::ErrorAnalysis,
            PromptTemplate::new(
                "error_analysis".to_string(),
                r#"
作为终端故障诊断专家，请分析以下命令执行错误：

🚨 **错误情况**：
- 执行命令：{content}
- 错误输出：{error_output}
- 工作目录：{working_directory}
- 环境信息：{environment}

🔍 **诊断要求**：

**错误识别**：
- 准确识别错误类型和严重程度
- 分析错误消息的关键信息
- 确定是语法错误、权限问题还是环境问题

**根因分析**：
- 深入分析可能的根本原因
- 考虑环境配置、依赖关系、权限等因素
- 识别常见的陷阱和误区

**解决方案**：
- 提供多种可行的解决方案
- 按优先级和难易程度排序
- 包含具体的修复命令和步骤

**预防措施**：
- 建议避免类似错误的方法
- 推荐相关的检查和验证步骤

请以JSON格式回复：
{
  "analysis": "错误的详细分析和影响",
  "error_type": "syntax|permission|environment|dependency|other",
  "severity": "low|medium|high|critical",
  "possible_causes": [
    {"cause": "原因描述", "likelihood": "high|medium|low"}
  ],
  "solutions": [
    {
      "description": "解决方案描述",
      "commands": ["修复命令1", "修复命令2"],
      "priority": "high|medium|low",
      "difficulty": "easy|medium|hard",
      "explanation": "为什么这个方案有效"
    }
  ],
  "prevention": ["预防措施1", "预防措施2"],
  "related_docs": [
    {"title": "文档标题", "url": "文档链接", "relevance": "high|medium|low"}
  ]
}
"#
                .to_string(),
                vec![
                    "content".to_string(),
                    "error_output".to_string(),
                    "working_directory".to_string(),
                    "environment".to_string(),
                ],
            )
            .with_description("为终端用户提供专业的错误诊断和解决方案".to_string()),
        );
    }

    /// 生成带选项的提示词
    pub fn generate_prompt_with_options(
        &mut self,
        request: &AIRequest,
        options: &PromptOptions,
    ) -> AppResult<String> {
        // 生成缓存键
        let cache_key = format!("{:?}_{}", request.request_type, request.content);

        // 检查缓存
        if let Some(cached) = self.template_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let template = self.templates.get(&request.request_type).ok_or_else(|| {
            anyhow!(
                "AI输入验证错误: No template found for request type: {:?}",
                request.request_type
            )
        })?;

        let mut prompt = template.template.clone();

        // 替换基本变量
        prompt = prompt.replace("{content}", &request.content);

        // 替换上下文变量
        if let Some(context) = &request.context {
            if let Some(working_dir) = &context.working_directory {
                prompt = prompt.replace("{working_directory}", working_dir);
            } else {
                prompt = prompt.replace("{working_directory}", "Unknown");
            }

            if let Some(history) = &context.command_history {
                let limited_history: Vec<String> = history
                    .iter()
                    .rev()
                    .take(options.max_history_length)
                    .rev()
                    .cloned()
                    .collect();
                let history_str = limited_history.join("\n");
                prompt = prompt.replace("{command_history}", &history_str);
            } else {
                prompt = prompt.replace("{command_history}", "No recent commands");
            }

            if options.include_environment {
                if let Some(env) = &context.environment {
                    let env_str = env
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    prompt = prompt.replace("{environment}", &env_str);
                } else {
                    prompt = prompt.replace("{environment}", "No environment info");
                }
            } else {
                prompt = prompt.replace("{environment}", "");
            }

            // 处理错误输出（用于错误分析）
            if let Some(last_output) = &context.last_output {
                prompt = prompt.replace("{error_output}", last_output);
            } else {
                prompt = prompt.replace("{error_output}", "No error output available");
            }
        }

        // 替换自定义变量
        for (key, value) in &options.custom_variables {
            let placeholder = format!("{{{key}}}");
            prompt = prompt.replace(&placeholder, value);
        }

        // 添加用户前置提示词
        if let Some(user_prefix) = &options.user_prefix_prompt {
            if !user_prefix.trim().is_empty() {
                prompt = format!("{}\n\n{}", user_prefix.trim(), prompt);
            }
        }

        // 缓存结果
        self.template_cache.insert(cache_key, prompt.clone());

        Ok(prompt)
    }

    /// 添加自定义模板
    pub fn add_template(
        &mut self,
        request_type: AIRequestType,
        template: PromptTemplate,
    ) -> AppResult<()> {
        template.validate()?;
        self.templates.insert(request_type, template);
        Ok(())
    }

    /// 添加命名的自定义模板
    pub fn add_custom_template(&mut self, name: String, template: PromptTemplate) -> AppResult<()> {
        template.validate()?;
        self.custom_templates.insert(name, template);
        Ok(())
    }

    /// 获取模板
    pub fn get_template(&self, request_type: &AIRequestType) -> Option<&PromptTemplate> {
        self.templates.get(request_type)
    }

    /// 获取自定义模板
    pub fn get_custom_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.custom_templates.get(name)
    }

    /// 列出所有模板
    pub fn list_templates(&self) -> Vec<(&AIRequestType, &PromptTemplate)> {
        self.templates.iter().collect()
    }

    /// 列出所有自定义模板
    pub fn list_custom_templates(&self) -> Vec<(&String, &PromptTemplate)> {
        self.custom_templates.iter().collect()
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.template_cache.clear();
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.template_cache.len(), self.template_cache.capacity())
    }
}
