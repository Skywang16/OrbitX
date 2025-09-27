use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// 模板引擎，支持变量替换和嵌套对象访问
pub struct TemplateEngine {
    // 可以在这里添加缓存等优化
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// 解析模板，支持 {variable} 和 {object.property} 格式
    pub fn resolve(
        &self,
        template: &str,
        context: &HashMap<String, Value>,
    ) -> Result<String, String> {
        let mut result = template.to_string();

        // 使用正则表达式查找所有 {variable} 占位符
        let re = Regex::new(r"\{([^}]+)\}").map_err(|e| e.to_string())?;

        for capture in re.captures_iter(template) {
            let full_match = &capture[0];
            let variable_path = &capture[1].trim();

            // 获取嵌套值
            let value = self.get_nested_value(context, variable_path);

            let replacement = match value {
                Some(Value::String(s)) => s,
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(other) => serde_json::to_string(&other).unwrap_or_else(|_| "null".to_string()),
                None => continue, // 保留未找到的占位符
            };

            result = result.replace(full_match, &replacement);
        }

        Ok(result)
    }

    /// 获取嵌套对象的值，支持 "object.property" 格式
    fn get_nested_value(&self, context: &HashMap<String, Value>, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        let mut current = context.get(parts[0])?;

        for part in &parts[1..] {
            current = match current {
                Value::Object(obj) => obj.get(*part)?,
                _ => return None,
            };
        }

        Some(current.clone())
    }

    /// 提取模板中的所有占位符
    pub fn extract_placeholders(&self, template: &str) -> Result<Vec<String>, String> {
        let re = Regex::new(r"\{([^}]+)\}").map_err(|e| e.to_string())?;
        let mut placeholders = Vec::new();

        for capture in re.captures_iter(template) {
            let placeholder = capture[1].trim().to_string();
            if !placeholders.contains(&placeholder) {
                placeholders.push(placeholder);
            }
        }

        Ok(placeholders)
    }

    /// 验证模板语法是否正确
    pub fn validate_template(&self, template: &str) -> bool {
        // 检查括号是否匹配
        let mut open_count = 0;
        for ch in template.chars() {
            match ch {
                '{' => open_count += 1,
                '}' => {
                    open_count -= 1;
                    if open_count < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        open_count == 0
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
