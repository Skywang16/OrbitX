use std::path::Path;
use tokio::fs;

use crate::agent::agents::frontmatter::split_frontmatter;
use crate::agent::error::AgentResult;

use super::types::ValidationResult;

/// 技能验证器 - 确保技能符合 Agent Skills 标准
pub struct SkillValidator;

impl SkillValidator {
    /// 验证技能目录是否符合 Agent Skills 标准
    pub async fn validate(skill_dir: &Path) -> AgentResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. 检查是否是目录
        if !skill_dir.is_dir() {
            errors.push(format!(
                "Skill path is not a directory: {}",
                skill_dir.display()
            ));
            return Ok(ValidationResult {
                valid: false,
                errors,
                warnings,
            });
        }

        // 2. 检查 SKILL.md 是否存在
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            errors.push("Missing SKILL.md file".to_string());
            return Ok(ValidationResult {
                valid: false,
                errors,
                warnings,
            });
        }

        // 3. 验证 SKILL.md 格式
        let content = fs::read_to_string(&skill_md).await?;
        Self::validate_skill_md(&content, &mut errors, &mut warnings);

        // 4. 检查子目录 (可选但推荐)
        Self::check_optional_directories(skill_dir, &mut warnings).await;

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// 验证 SKILL.md 内容格式
    fn validate_skill_md(content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        let (frontmatter, body) = split_frontmatter(content);

        // 检查是否有 frontmatter
        if frontmatter.is_none() {
            errors.push("Missing frontmatter in SKILL.md".to_string());
            return;
        }

        let frontmatter = frontmatter.unwrap();

        // 解析 frontmatter
        let parsed = crate::agent::agents::frontmatter::parse_frontmatter(frontmatter);

        // 检查必需字段
        if !parsed.fields.contains_key("name") {
            errors.push("Missing required field 'name' in frontmatter".to_string());
        }

        if !parsed.fields.contains_key("description") {
            errors.push("Missing required field 'description' in frontmatter".to_string());
        }

        // 检查推荐字段
        if !parsed.fields.contains_key("license") {
            warnings.push("Missing recommended field 'license' in frontmatter".to_string());
        }

        // 检查主体内容
        if body.trim().is_empty() {
            warnings.push("SKILL.md body is empty".to_string());
        }

        // 检查主体内容长度 (太短可能不够详细)
        if body.trim().len() < 50 {
            warnings.push("SKILL.md body is very short (< 50 chars)".to_string());
        }
    }

    /// 检查可选目录
    async fn check_optional_directories(skill_dir: &Path, warnings: &mut Vec<String>) {
        let optional_dirs = ["scripts", "references", "assets"];

        for dir_name in &optional_dirs {
            let dir = skill_dir.join(dir_name);
            if dir.exists() && dir.is_dir() {
                // 检查目录是否为空
                if let Ok(mut entries) = fs::read_dir(&dir).await {
                    if entries.next_entry().await.ok().flatten().is_none() {
                        warnings.push(format!("Directory '{dir_name}' exists but is empty"));
                    }
                }
            }
        }
    }

    /// 快速检查 (仅检查结构,不读取文件内容)
    pub async fn quick_check(skill_dir: &Path) -> bool {
        skill_dir.is_dir() && skill_dir.join("SKILL.md").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_valid_skill() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("valid-skill");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = r#"---
name: valid-skill
description: A valid test skill
license: MIT
---

# Valid Skill

This is a valid skill with proper structure and content.
"#;

        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_missing_skill_md() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("invalid-skill");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Missing SKILL.md")));
    }

    #[tokio::test]
    async fn test_missing_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("no-frontmatter");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = "# No Frontmatter\n\nThis skill has no frontmatter.";
        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Missing frontmatter")));
    }

    #[tokio::test]
    async fn test_missing_required_fields() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("incomplete");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let skill_md = r#"---
name: incomplete-skill
---

# Incomplete Skill

Missing description field.
"#;

        std_fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();

        let result = SkillValidator::validate(&skill_dir).await.unwrap();

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Missing required field 'description'")));
    }

    #[tokio::test]
    async fn test_quick_check() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("quick-test");
        std_fs::create_dir_all(&skill_dir).unwrap();
        std_fs::write(skill_dir.join("SKILL.md"), "test").unwrap();

        assert!(SkillValidator::quick_check(&skill_dir).await);
    }
}
