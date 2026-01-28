use dashmap::DashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;

use crate::agent::error::{AgentError, AgentResult};

use super::loader::SkillLoader;
use super::types::{SkillContent, SkillEntry, SkillMetadata};

/// 技能注册表 - 线程安全的技能存储
/// 使用 DashMap 提供高性能的并发访问
pub struct SkillRegistry {
    /// skill_name -> SkillEntry
    skills: DashMap<String, SkillEntry>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: DashMap::new(),
        }
    }

    /// 注册一个技能元数据
    pub fn register(&self, metadata: SkillMetadata) -> AgentResult<()> {
        let name = metadata.name.clone();

        // 获取文件修改时间
        let last_modified = std::fs::metadata(metadata.skill_dir.join("SKILL.md"))
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(SystemTime::now);

        let entry = SkillEntry {
            metadata,
            content: None,
            last_modified,
        };

        self.skills.insert(name.to_string(), entry);
        Ok(())
    }

    /// 获取技能元数据
    pub fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.skills.get(name).map(|entry| entry.metadata.clone())
    }

    /// 获取所有技能元数据
    pub fn list_all(&self) -> Vec<SkillMetadata> {
        self.skills
            .iter()
            .map(|entry| entry.metadata.clone())
            .collect()
    }

    /// 检查技能是否存在
    pub fn contains(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// 获取或加载技能内容
    pub async fn get_or_load_content(&self, name: &str) -> AgentResult<SkillContent> {
        // 快速路径: 检查缓存
        if let Some(entry) = self.skills.get(name) {
            if let Some(content) = &entry.content {
                return Ok(content.clone());
            }
        }

        // 慢速路径: 加载内容
        let metadata = self
            .get_metadata(name)
            .ok_or_else(|| AgentError::SkillNotFound(name.to_string()))?;

        let content = SkillLoader::load_content(&metadata).await?;

        // 更新缓存
        if let Some(mut entry) = self.skills.get_mut(name) {
            entry.content = Some(content.clone());
        }

        Ok(content)
    }

    /// 清除技能内容缓存 (保留元数据)
    pub fn clear_content_cache(&self, name: &str) {
        if let Some(mut entry) = self.skills.get_mut(name) {
            entry.content = None;
        }
    }

    /// 清空整个注册表
    pub fn clear(&self) {
        self.skills.clear();
    }

    /// 重新加载技能 (检查文件修改时间)
    pub async fn reload_if_modified(&self, name: &str) -> AgentResult<bool> {
        let entry = self
            .skills
            .get(name)
            .ok_or_else(|| AgentError::SkillNotFound(name.to_string()))?;

        let skill_md = entry.metadata.skill_dir.join("SKILL.md");
        let current_modified = fs::metadata(&skill_md)
            .await?
            .modified()
            .unwrap_or_else(|_| SystemTime::now());

        if current_modified > entry.last_modified {
            // 重新加载
            drop(entry);
            self.clear_content_cache(name);

            let skill_dir = self.get_metadata(name).unwrap().skill_dir;
            let new_metadata = SkillLoader::load_metadata(&skill_dir).await?;
            self.register(new_metadata)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 批量获取技能内容
    pub async fn get_multiple_contents(&self, names: &[String]) -> AgentResult<Vec<SkillContent>> {
        let mut contents = Vec::with_capacity(names.len());

        for name in names {
            match self.get_or_load_content(name).await {
                Ok(content) => contents.push(content),
                Err(e) => {
                    tracing::warn!("Failed to load skill '{}': {}", name, e);
                }
            }
        }

        Ok(contents)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷的 Arc-wrapped 版本
pub type SkillRegistryRef = Arc<SkillRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // 使用公共测试工具
    use crate::agent::skill::test_utils::create_test_skill;

    #[tokio::test]
    async fn test_register_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir, "test-skill").await.unwrap();

        let registry = SkillRegistry::new();
        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();

        registry.register(metadata).unwrap();

        assert!(registry.contains("test-skill"));
        assert!(registry.get_metadata("test-skill").is_some());
    }

    #[tokio::test]
    async fn test_get_or_load_content() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill(&skill_dir, "test-skill").await.unwrap();

        let registry = SkillRegistry::new();
        let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();
        registry.register(metadata).unwrap();

        // 第一次加载
        let content1 = registry.get_or_load_content("test-skill").await.unwrap();
        assert!(content1.instructions.contains("Test content"));

        // 第二次应该从缓存加载
        let content2 = registry.get_or_load_content("test-skill").await.unwrap();
        assert_eq!(content1.instructions, content2.instructions);
    }

    #[tokio::test]
    async fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new();

        for i in 1..=3 {
            let name = format!("skill-{}", i);
            let skill_dir = temp_dir.path().join(&name);
            create_test_skill(&skill_dir, &name).await.unwrap();

            let metadata = SkillLoader::load_metadata(&skill_dir).await.unwrap();
            registry.register(metadata).unwrap();
        }

        let all = registry.list_all();
        assert_eq!(all.len(), 3);
    }
}
