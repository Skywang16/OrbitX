//! Prompt builder - 组装完整的 system prompt

use chrono::Utc;
use std::collections::HashMap;

use super::loader::{BuiltinPrompts, PromptLoader};

/// System prompt 的各个部分
#[derive(Debug, Clone, Default)]
pub struct SystemPromptParts {
    /// Agent 特定的提示词（从 agents/*.md 加载，去掉 frontmatter）
    pub agent_prompt: Option<String>,
    /// 基础规则
    pub rules: Option<String>,
    /// 工作方法论
    pub methodology: Option<String>,
    /// 环境信息
    pub env_info: Option<String>,
    /// 运行时 reminder
    pub reminder: Option<String>,
    /// 用户自定义指令（CLAUDE.md / project rules）
    pub custom_instructions: Option<String>,
    /// 工具描述
    pub tools_description: Option<String>,
}

/// Prompt 构建器
pub struct PromptBuilder {
    loader: PromptLoader,
}

impl PromptBuilder {
    pub fn new(workspace_path: Option<String>) -> Self {
        Self {
            loader: PromptLoader::new(workspace_path),
        }
    }

    /// 构建完整的 system prompt
    pub async fn build_system_prompt(&mut self, parts: SystemPromptParts) -> String {
        let mut sections = Vec::new();

        // 1. Agent 特定提示词
        if let Some(agent_prompt) = parts.agent_prompt {
            let body = strip_frontmatter(&agent_prompt);
            if !body.trim().is_empty() {
                sections.push(body);
            }
        }

        // 2. 基础规则
        if let Some(rules) = parts.rules {
            sections.push(rules);
        } else {
            sections.push(BuiltinPrompts::rules().to_string());
        }

        // 3. 工作方法论
        if let Some(methodology) = parts.methodology {
            sections.push(methodology);
        } else {
            sections.push(BuiltinPrompts::methodology().to_string());
        }

        // 4. 环境信息
        if let Some(env_info) = parts.env_info {
            sections.push(env_info);
        }

        // 5. 工具描述
        if let Some(tools) = parts.tools_description {
            sections.push(format!("# Available Tools\n\n{}", tools));
        }

        // 6. 用户自定义指令
        if let Some(custom) = parts.custom_instructions {
            if !custom.trim().is_empty() {
                sections.push(format!("# Project Instructions\n\n{}", custom.trim()));
            }
        }

        // 7. 运行时 reminder（放最后，优先级最高）
        if let Some(reminder) = parts.reminder {
            sections.push(format!(
                "<system-reminder>\n{}\n</system-reminder>",
                reminder.trim()
            ));
        }

        sections.join("\n\n").trim().to_string()
    }

    /// 构建环境信息
    pub fn build_env_info(
        &self,
        working_directory: Option<&str>,
        file_list_preview: Option<&str>,
    ) -> String {
        let wd = working_directory.unwrap_or("(none)");
        let platform = std::env::consts::OS;
        let date = Utc::now().format("%Y-%m-%d").to_string();

        let mut env = format!(
            "Here is useful information about the environment you are running in:\n\n<env>\nWorking directory: {}\nPlatform: {}\nToday's date: {}\n</env>",
            wd, platform, date
        );

        if let Some(files) = file_list_preview {
            if !files.trim().is_empty() {
                env.push_str("\n\n");
                env.push_str(files);
            }
        }

        env
    }

    /// 获取 agent 提示词
    pub async fn get_agent_prompt(&mut self, agent_type: &str) -> Option<String> {
        self.loader.load("agents", agent_type).await
    }

    /// 获取 reminder
    pub async fn get_reminder(&mut self, name: &str) -> Option<String> {
        self.loader.load("reminders", name).await
    }

    /// 渲染模板变量
    pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    /// 获取 loop warning reminder 并填充变量
    pub fn get_loop_warning(&self, count: usize, tools: &str) -> String {
        let template = BuiltinPrompts::reminder_loop_warning();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), count.to_string());
        vars.insert("tools".to_string(), tools.to_string());
        Self::render_template(template, &vars)
    }

    /// 获取 duplicate tools reminder 并填充变量
    pub fn get_duplicate_tools_warning(&self, count: usize) -> String {
        let template = BuiltinPrompts::reminder_duplicate_tools();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), count.to_string());
        Self::render_template(template, &vars)
    }
}

/// 去掉 frontmatter，只返回正文
fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    // 找第二个 ---
    if let Some(end_idx) = trimmed[3..].find("---") {
        let body_start = 3 + end_idx + 3;
        return trimmed[body_start..].trim().to_string();
    }

    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let content = r#"---
name: test
description: Test agent
---

This is the body."#;

        let body = strip_frontmatter(content);
        assert_eq!(body, "This is the body.");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let content = "Just plain content";
        let body = strip_frontmatter(content);
        assert_eq!(body, "Just plain content");
    }

    #[test]
    fn test_render_template() {
        let template = "Hello {{name}}, you have {{count}} messages.";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("count".to_string(), "5".to_string());

        let result = PromptBuilder::render_template(template, &vars);
        assert_eq!(result, "Hello Alice, you have 5 messages.");
    }
}
