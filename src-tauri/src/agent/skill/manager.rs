use std::path::Path;
use std::sync::Arc;
use tokio::fs;

use crate::agent::error::{AgentError, AgentResult};

use super::loader::SkillLoader;
use super::matcher::SkillMatcher;
use super::registry::{SkillRegistry, SkillRegistryRef};
use super::types::{SkillContent, SkillMatchingMode, SkillMetadata};

/// 技能管理器 - 渐进式披露的核心实现
///
/// 工作流程:
/// 1. `discover_skills()`: 发现阶段 - 只加载元数据
/// 2. `activate_skills()`: 激活阶段 - 加载完整内容
/// 3. `load_reference()`: 执行阶段 - 按需加载引用文件
///
/// # Examples
///
/// ```no_run
/// use orbitx::agent::skill::{SkillManager, SkillMatchingMode};
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let manager = SkillManager::new();
/// let global_skills = Path::new("~/.config/orbitx/skills");
/// let workspace = Path::new("/path/to/workspace");
///
/// // 发现技能 (全局 + 工作区)
/// manager.discover_skills(Some(global_skills), Some(workspace)).await?;
///
/// // 激活技能
/// let skills = manager.activate_skills(
///     "Use @code-review to review this PR",
///     SkillMatchingMode::Hybrid,
///     None
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct SkillManager {
    registry: SkillRegistryRef,
    matcher: Arc<SkillMatcher>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(SkillRegistry::new()),
            matcher: Arc::new(SkillMatcher::new()),
        }
    }

    pub fn with_registry(registry: SkillRegistryRef) -> Self {
        Self {
            registry,
            matcher: Arc::new(SkillMatcher::new()),
        }
    }

    /// 发现阶段: 扫描全局和工作区并加载所有技能的元数据
    ///
    /// 扫描目录优先级 (后者覆盖前者):
    /// 1. 全局: ~/.config/orbitx/skills/
    /// 2. 工作区: workspace/.orbitx/skills/
    /// 3. Claude兼容: workspace/.claude/skills/
    ///
    /// 返回所有发现的技能元数据
    pub async fn discover_skills(
        &self,
        global_skills_dir: Option<&Path>,
        workspace: Option<&Path>,
    ) -> AgentResult<Vec<SkillMetadata>> {
        // 清空旧的注册表
        self.registry.clear();

        let mut all_metadata = Vec::new();

        // 1. 扫描全局目录
        if let Some(global_dir) = global_skills_dir {
            if global_dir.exists() {
                self.scan_skills_directory(global_dir, &mut all_metadata)
                    .await?;
            }
        }

        // 2. 扫描工作区目录 (优先级更高,可覆盖全局)
        if let Some(workspace_root) = workspace {
            for skill_dir_name in &[".orbitx/skills", ".claude/skills"] {
                let skills_dir = workspace_root.join(skill_dir_name);
                if skills_dir.exists() {
                    self.scan_skills_directory(&skills_dir, &mut all_metadata)
                        .await?;
                }
            }
        }
        Ok(all_metadata)
    }

    /// 扫描指定目录下的所有 skills
    async fn scan_skills_directory(
        &self,
        skills_dir: &Path,
        all_metadata: &mut Vec<SkillMetadata>,
    ) -> AgentResult<()> {
        let mut entries = fs::read_dir(skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // 必须是目录且包含 SKILL.md
            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            // 加载元数据
            match SkillLoader::load_metadata(&path).await {
                Ok(metadata) => {
                    let skill_name = metadata.name.clone();

                    // 检查是否已存在 (工作区覆盖全局)
                    if self.registry.contains(&skill_name) {
                        self.registry.clear_content_cache(&skill_name);
                    }

                    self.registry.register(metadata.clone())?;

                    // 移除旧的同名 skill 再添加新的
                    all_metadata.retain(|m| m.name != skill_name);
                    all_metadata.push(metadata);
                }
                Err(e) => {
                    tracing::warn!("Failed to load skill from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// 激活阶段: 根据用户提示和匹配模式激活技能
    ///
    /// 参数:
    /// - user_prompt: 用户输入
    /// - mode: 匹配模式 (Explicit/Semantic/Hybrid)
    /// - explicit_skills: 显式指定的技能列表 (可选)
    ///
    /// 返回: 激活的技能完整内容
    pub async fn activate_skills(
        &self,
        user_prompt: &str,
        mode: SkillMatchingMode,
        explicit_skills: Option<&[String]>,
    ) -> AgentResult<Vec<SkillContent>> {
        let all_metadata = self.registry.list_all();

        // 根据模式选择技能
        let selected_names = match mode {
            SkillMatchingMode::Explicit => {
                // 只使用显式指定的技能
                let explicit = explicit_skills.unwrap_or(&[]);
                self.matcher.extract_explicit_mentions(user_prompt, explicit)
            }
            SkillMatchingMode::Semantic => {
                // 只使用语义匹配
                self.matcher
                    .semantic_match(user_prompt, &all_metadata, 3)
            }
            SkillMatchingMode::Hybrid => {
                // 混合模式: 先提取显式引用,再补充语义匹配
                let explicit = explicit_skills.unwrap_or(&[]);
                let mut selected = self.matcher.extract_explicit_mentions(user_prompt, explicit);

                // 如果显式引用少于3个,用语义匹配补充
                if selected.len() < 3 {
                    let semantic = self
                        .matcher
                        .semantic_match(user_prompt, &all_metadata, 3 - selected.len());
                    selected.extend(semantic);
                }

                selected
            }
        };

        // 去重
        let unique_names: Vec<String> = selected_names
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // 批量加载内容
        self.registry.get_multiple_contents(&unique_names).await
    }

    /// 执行阶段: 加载技能的引用文件
    ///
    /// 参数:
    /// - skill_name: 技能名称
    /// - reference_path: 引用文件路径 (相对于技能目录, 如 "references/api.md")
    pub async fn load_reference(
        &self,
        skill_name: &str,
        reference_path: &str,
    ) -> AgentResult<String> {
        let metadata = self
            .registry
            .get_metadata(skill_name)
            .ok_or_else(|| AgentError::SkillNotFound(skill_name.to_string()))?;

        SkillLoader::load_reference(&metadata.skill_dir, reference_path).await
    }

    /// 获取技能元数据 (不触发内容加载)
    pub fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.registry.get_metadata(name)
    }

    /// 列出所有已发现的技能
    pub fn list_all(&self) -> Vec<SkillMetadata> {
        self.registry.list_all()
    }

    /// 重新加载已修改的技能
    pub async fn reload_if_modified(&self, name: &str) -> AgentResult<bool> {
        self.registry.reload_if_modified(name).await
    }

    /// 获取底层注册表引用 (用于高级操作)
    pub fn registry(&self) -> &SkillRegistryRef {
        &self.registry
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs;
    use tempfile::TempDir;

    // 使用公共测试工具
    use crate::agent::skill::test_utils::create_test_skill;

    #[tokio::test]
    async fn test_discover_skills_workspace_only() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // 创建工作区 skills
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        let skill1_dir = orbitx_skills.join("skill-1");
        create_test_skill(&skill1_dir, "skill-1").await.unwrap();

        let manager = SkillManager::new();
        let skills = manager.discover_skills(None, Some(workspace)).await.unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_ref(), "skill-1");
    }

    #[tokio::test]
    async fn test_discover_skills_global_and_workspace() {
        let temp_dir = TempDir::new().unwrap();

        // 创建全局 skills
        let global_dir = temp_dir.path().join("global");
        std_fs::create_dir_all(&global_dir).unwrap();
        let global_skill1 = global_dir.join("skill-global");
        create_test_skill(&global_skill1, "skill-global").await.unwrap();

        // 创建工作区 skills
        let workspace = temp_dir.path().join("workspace");
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();
        let workspace_skill1 = orbitx_skills.join("skill-workspace");
        create_test_skill(&workspace_skill1, "skill-workspace").await.unwrap();

        let manager = SkillManager::new();
        let skills = manager
            .discover_skills(Some(&global_dir), Some(&workspace))
            .await
            .unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_workspace_overrides_global() {
        let temp_dir = TempDir::new().unwrap();

        // 全局和工作区都有同名 skill
        let global_dir = temp_dir.path().join("global");
        std_fs::create_dir_all(&global_dir).unwrap();
        let global_skill = global_dir.join("shared-skill");
        create_test_skill(&global_skill, "shared-skill").await.unwrap();

        let workspace = temp_dir.path().join("workspace");
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();
        let workspace_skill = orbitx_skills.join("shared-skill");
        create_test_skill(&workspace_skill, "shared-skill").await.unwrap();

        let manager = SkillManager::new();
        let skills = manager
            .discover_skills(Some(&global_dir), Some(&workspace))
            .await
            .unwrap();

        // 应该只有1个 (工作区覆盖全局)
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_ref(), "shared-skill");

        // 验证来源是工作区
        assert!(skills[0].skill_dir.starts_with(&workspace));
    }

    #[tokio::test]
    async fn test_activate_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        let skill_dir = orbitx_skills.join("pdf-processing");
        create_test_skill(&skill_dir, "pdf-processing").await.unwrap();

        let manager = SkillManager::new();
        manager.discover_skills(None, Some(workspace)).await.unwrap();

        let prompt = "Please use @pdf-processing to extract text";
        let activated = manager
            .activate_skills(prompt, SkillMatchingMode::Explicit, None)
            .await
            .unwrap();

        assert!(activated.iter().any(|s| s.metadata.name.as_ref() == "pdf-processing"));
    }

    #[tokio::test]
    async fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        for i in 1..=3 {
            let name = format!("skill-{}", i);
            let skill_dir = orbitx_skills.join(&name);
            create_test_skill(&skill_dir, &name).await.unwrap();
        }

        let manager = SkillManager::new();
        manager.discover_skills(None, Some(workspace)).await.unwrap();

        let all = manager.list_all();
        assert_eq!(all.len(), 3);
    }
}
