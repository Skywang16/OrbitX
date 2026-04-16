use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use crate::agent::error::{AgentError, AgentResult};

use super::loader::SkillLoader;
use super::registry::{SkillRegistry, SkillRegistryRef};
use super::types::{SkillContent, SkillMetadata};

/// Skill Manager - Core implementation of progressive disclosure
///
/// Workflow:
/// 1. `discover_skills()`: Discovery phase - only load metadata
/// 2. `activate_skills()`: Activation phase - load full content
/// 3. `load_reference()`: Execution phase - load reference files on demand
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
/// // Discover skills (global + workspace)
/// manager.discover_skills(Some(global_skills), Some(workspace)).await?;
///
/// // Activate skills
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
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(SkillRegistry::new()),
        }
    }

    /// Discovery phase: scan global and workspace and load metadata for all skills
    ///
    /// Directory scan priority (latter overrides former):
    /// 1. Global: OrbitX app data `skills/`
    /// 2. Home compatibility: `~/.claude/skills/`
    /// 3. Home compatibility: `~/.codex/skills/.system/`
    /// 4. Workspace: `workspace/.orbitx/skills/`
    /// 5. Workspace compatibility: `workspace/.claude/skills/`
    ///
    /// Returns metadata for all discovered skills
    pub async fn discover_skills(
        &self,
        global_skills_dir: Option<&Path>,
        workspace: Option<&Path>,
    ) -> AgentResult<Vec<SkillMetadata>> {
        self.discover_skills_from_dirs(
            global_skills_dir,
            workspace,
            &Self::compatible_global_skill_dirs(),
        )
        .await
    }

    async fn discover_skills_from_dirs(
        &self,
        global_skills_dir: Option<&Path>,
        workspace: Option<&Path>,
        compatible_global_dirs: &[PathBuf],
    ) -> AgentResult<Vec<SkillMetadata>> {
        // Clear old registry
        self.registry.clear();

        let mut all_metadata = Vec::new();

        // 1. Scan global directory
        if let Some(global_dir) = global_skills_dir {
            if global_dir.exists() {
                self.scan_skills_directory(global_dir, &mut all_metadata)
                    .await?;
            }
        }

        // 2. Scan home-level compatible directories
        for compatible_dir in compatible_global_dirs {
            if compatible_dir.exists() {
                self.scan_skills_directory(compatible_dir, &mut all_metadata)
                    .await?;
            }
        }

        // 3. Scan workspace directory (higher priority, can override global)
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

    fn compatible_global_skill_dirs() -> Vec<PathBuf> {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };

        vec![
            home.join(".claude").join("skills"),
            home.join(".codex").join("skills"),
            home.join(".codex").join("skills").join(".system"),
        ]
    }

    /// Scan all skills in the specified directory
    async fn scan_skills_directory(
        &self,
        skills_dir: &Path,
        all_metadata: &mut Vec<SkillMetadata>,
    ) -> AgentResult<()> {
        let mut entries = fs::read_dir(skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Must be a directory and contain SKILL.md
            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            // Load metadata
            match SkillLoader::load_metadata(&path).await {
                Ok(metadata) => {
                    let skill_name = metadata.name.clone();

                    // Check if already exists (workspace overrides global)
                    if self.registry.contains(&skill_name) {
                        self.registry.clear_content_cache(&skill_name);
                    }

                    self.registry.register(metadata.clone())?;

                    // Remove old skill with same name then add new one
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

    /// Activation phase: load full content by skill name
    ///
    /// This method will be called by SkillTool, LLM activates skills via tool calling
    pub async fn load_content(&self, skill_name: &str) -> AgentResult<SkillContent> {
        self.registry.get_or_load_content(skill_name).await
    }

    /// Execution phase: load skill reference files
    ///
    /// Parameters:
    /// - skill_name: skill name
    /// - reference_path: reference file path (relative to skill directory, e.g., "references/api.md")
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

    /// Get skill metadata (does not trigger content loading)
    pub fn get_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.registry.get_metadata(name)
    }

    /// List all discovered skills
    pub fn list_all(&self) -> Vec<SkillMetadata> {
        self.registry.list_all()
    }

    /// Reload skill if modified
    pub async fn reload_if_modified(&self, name: &str) -> AgentResult<bool> {
        self.registry.reload_if_modified(name).await
    }

    /// Get underlying registry reference (for advanced operations)
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

    // Use public test utilities
    use crate::agent::skill::test_utils::create_test_skill;

    #[tokio::test]
    async fn test_discover_skills_workspace_only() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create workspace skills
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        let skill1_dir = orbitx_skills.join("skill-1");
        create_test_skill(&skill1_dir, "skill-1").unwrap();

        let manager = SkillManager::new();
        let skills = manager
            .discover_skills_from_dirs(None, Some(workspace), &[])
            .await
            .unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_ref(), "skill-1");
    }

    #[tokio::test]
    async fn test_discover_skills_global_and_workspace() {
        let temp_dir = TempDir::new().unwrap();

        // Create global skills
        let global_dir = temp_dir.path().join("global");
        std_fs::create_dir_all(&global_dir).unwrap();
        let global_skill1 = global_dir.join("skill-global");
        create_test_skill(&global_skill1, "skill-global").unwrap();

        // Create workspace skills
        let workspace = temp_dir.path().join("workspace");
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();
        let workspace_skill1 = orbitx_skills.join("skill-workspace");
        create_test_skill(&workspace_skill1, "skill-workspace").unwrap();

        let manager = SkillManager::new();
        let skills = manager
            .discover_skills_from_dirs(Some(&global_dir), Some(&workspace), &[])
            .await
            .unwrap();

        assert_eq!(skills.len(), 2);
    }

    #[tokio::test]
    async fn test_workspace_overrides_global() {
        let temp_dir = TempDir::new().unwrap();

        // Both global and workspace have skill with same name
        let global_dir = temp_dir.path().join("global");
        std_fs::create_dir_all(&global_dir).unwrap();
        let global_skill = global_dir.join("shared-skill");
        create_test_skill(&global_skill, "shared-skill").unwrap();

        let workspace = temp_dir.path().join("workspace");
        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();
        let workspace_skill = orbitx_skills.join("shared-skill");
        create_test_skill(&workspace_skill, "shared-skill").unwrap();

        let manager = SkillManager::new();
        let skills = manager
            .discover_skills(Some(&global_dir), Some(&workspace))
            .await
            .unwrap();

        // Should only have 1 (workspace overrides global)
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_ref(), "shared-skill");

        // Verify source is workspace
        assert!(skills[0].skill_dir.starts_with(&workspace));
    }

    #[tokio::test]
    async fn test_load_content() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        let skill_dir = orbitx_skills.join("pdf-processing");
        create_test_skill(&skill_dir, "pdf-processing").unwrap();

        let manager = SkillManager::new();
        manager
            .discover_skills(None, Some(workspace))
            .await
            .unwrap();

        // Load skill content directly by name
        let content = manager.load_content("pdf-processing").await.unwrap();
        assert_eq!(content.metadata.name.as_ref(), "pdf-processing");
        assert!(content.instructions.contains("Test content"));
    }

    #[tokio::test]
    async fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        for i in 1..=3 {
            let name = format!("skill-{i}");
            let skill_dir = orbitx_skills.join(&name);
            create_test_skill(&skill_dir, &name).unwrap();
        }

        let manager = SkillManager::new();
        manager
            .discover_skills(None, Some(workspace))
            .await
            .unwrap();

        let all = manager.list_all();
        assert_eq!(all.len(), 3);
    }
}
