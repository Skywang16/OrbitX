use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::agent::agents::frontmatter::{parse_frontmatter, split_frontmatter};
use crate::agent::error::{AgentError, AgentResult};

use super::types::{SkillContent, SkillMetadata};

/// Agent Skills 标准加载器
/// 严格遵循 Agent Skills 开放标准:
/// - 技能必须是目录结构
/// - 必须包含 SKILL.md
/// - 支持 scripts/, references/, assets/ 子目录
pub struct SkillLoader;

impl SkillLoader {
    /// 从标准 Agent Skills 目录加载元数据
    /// 只读取 SKILL.md 的 frontmatter,不加载主体内容
    pub async fn load_metadata(skill_dir: &Path) -> AgentResult<SkillMetadata> {

        if !skill_dir.is_dir() {
            return Err(AgentError::InvalidSkillFormat(format!(
                "Skill path is not a directory: {}",
                skill_dir.display()
            )));
        }

        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Err(AgentError::InvalidSkillFormat(format!(
                "Missing SKILL.md in: {}",
                skill_dir.display()
            )));
        }

        let content = fs::read_to_string(&skill_md).await?;
        let (frontmatter, _) = split_frontmatter(&content);

        let frontmatter = frontmatter.ok_or_else(|| {
            AgentError::InvalidSkillFormat(format!(
                "Missing frontmatter in SKILL.md: {}",
                skill_md.display()
            ))
        })?;

        let parsed = parse_frontmatter(frontmatter);

        // 提取必需字段
        let name = parsed
            .fields
            .get("name")
            .ok_or_else(|| {
                tracing::warn!("Missing 'name' field in SKILL.md: {}", skill_md.display());
                AgentError::InvalidSkillFormat("Missing 'name' in SKILL.md frontmatter".to_string())
            })?
            .clone();

        let description = parsed
            .fields
            .get("description")
            .ok_or_else(|| {
                AgentError::InvalidSkillFormat(
                    "Missing 'description' in SKILL.md frontmatter".to_string(),
                )
            })?
            .clone();

        // 提取可选字段
        let license = parsed.fields.get("license").cloned();
        let compatibility = parsed.fields.get("compatibility").cloned();

        // 提取扩展元数据
        let mut metadata = HashMap::new();
        if let Some(meta_obj) = parsed.fields.get("metadata") {
            // 简单处理: 假设 metadata 是字符串映射
            // 实际可能需要更复杂的解析
            metadata.insert("raw".to_string(), meta_obj.clone());
        }
        for (k, v) in &parsed.fields {
            if !matches!(
                k.as_str(),
                "name" | "description" | "license" | "compatibility" | "metadata" | "allowed_tools"
            ) {
                metadata.insert(k.clone(), v.clone());
            }
        }

        // 提取 allowed_tools
        let allowed_tools = parsed.fields.get("allowed_tools").map(|s| {
            // 简单处理: 假设是逗号分隔的字符串
            s.split(',').map(|s| s.trim().to_string()).collect()
        });

        Ok(SkillMetadata {
            name: name.into(),
            description: description.into(),
            license,
            compatibility,
            metadata,
            allowed_tools,
            skill_dir: skill_dir.to_path_buf(),
        })
    }

    /// 加载完整的技能内容 (渐进式加载的第二阶段)
    pub async fn load_content(metadata: &SkillMetadata) -> AgentResult<SkillContent> {
        let skill_md = metadata.skill_dir.join("SKILL.md");
        let content = fs::read_to_string(&skill_md).await?;
        let (_, instructions) = split_frontmatter(&content);

        // 扫描子目录
        let scripts = Self::scan_directory(&metadata.skill_dir.join("scripts")).await?;
        let references = Self::scan_directory(&metadata.skill_dir.join("references")).await?;
        let assets = Self::scan_directory(&metadata.skill_dir.join("assets")).await?;

        Ok(SkillContent {
            metadata: metadata.clone(),
            instructions: instructions.trim().to_string(),
            scripts,
            references,
            assets,
        })
    }

    /// 加载引用文件的内容 (渐进式加载的第三阶段)
    pub async fn load_reference(
        skill_dir: &Path,
        reference_path: &str,
    ) -> AgentResult<String> {
        let full_path = skill_dir.join(reference_path);

        if !full_path.starts_with(skill_dir) {
            return Err(AgentError::InvalidSkillFormat(
                "Reference path escapes skill directory".to_string(),
            ));
        }

        fs::read_to_string(&full_path)
            .await
            .map_err(AgentError::from)
    }

    /// 扫描目录并返回文件列表 (相对路径)
    #[inline]
    async fn scan_directory(dir: &Path) -> AgentResult<Vec<String>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    files.push(name.to_string_lossy().to_string());
                }
            }
        }

        files.sort();
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path) -> std::io::Result<()> {
        std_fs::create_dir_all(dir)?;

        let skill_md = r#"---
name: test-skill
description: A test skill for unit testing
license: MIT
metadata:
  author: test
  version: "1.0"
---

# Test Skill

This is a test skill.
"#;

        std_fs::write(dir.join("SKILL.md"), skill_md)?;
        std_fs::create_dir_all(dir.join("scripts"))?;
        std_fs::write(dir.join("scripts").join("test.py"), "print('hello')")?;

        Ok(())
    }

    #[tokio::test]
    async fn test_load_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir).unwrap();

        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();

        assert_eq!(metadata.name.as_ref(), "test-skill");
        assert_eq!(metadata.description.as_ref(), "A test skill for unit testing");
        assert_eq!(metadata.license, Some("MIT".to_string()));
    }

    #[tokio::test]
    async fn test_load_content() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir).unwrap();

        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();
        let content = SkillLoader::load_content(&metadata).await.unwrap();

        assert!(content.instructions.contains("Test Skill"));
        assert_eq!(content.scripts, vec!["test.py"]);
    }

    #[tokio::test]
    async fn test_missing_skill_md() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("invalid-skill");
        std_fs::create_dir_all(&skill_dir).unwrap();

        let result = SkillLoader::load_metadata(&skill_dir).await;
        assert!(result.is_err());
    }
}
