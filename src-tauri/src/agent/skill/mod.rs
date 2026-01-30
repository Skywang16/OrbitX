//! Agent Skills 系统 - 符合 Agent Skills 开放标准
//!
//! 核心设计原则: 渐进式披露 (Progressive Disclosure)
//!
//! ## 工作流程
//!
//! 1. **发现阶段**: 扫描 `.claude/skills/` 和 `.orbitx/skills/`,加载所有技能的元数据
//! 2. **激活阶段**: 根据用户提示和匹配模式,加载选中技能的完整内容
//! 3. **执行阶段**: Agent 执行时可按需加载引用文件 (scripts/, references/, assets/)
//!
//! ## 标准目录结构
//!
//! ```text
//! skill-name/
//! ├── SKILL.md          # 必需: 技能定义和指令
//! ├── scripts/          # 可选: 可执行脚本
//! ├── references/       # 可选: 文档和参考资料
//! └── assets/          # 可选: 模板、资源文件
//! ```
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use orbitx_agent::skill_system::SkillManager;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let manager = SkillManager::new();
//!
//! // 1. 发现技能
//! let workspace = Path::new("/path/to/workspace");
//! let skills = manager.discover_skills(workspace).await?;
//! println!("Found {} skills", skills.len());
//!
//! // 2. 激活技能
//! let user_prompt = "Extract text from PDF using @pdf-processing";
//! let activated = manager.activate_skills(
//!     user_prompt,
//!     SkillMatchingMode::Hybrid,
//!     None
//! ).await?;
//!
//! // 3. 使用技能内容
//! for skill in activated {
//!     println!("Skill: {}", skill.metadata.name);
//!     println!("Instructions: {}", skill.instructions);
//! }
//! # Ok(())
//! # }
//! ```

mod loader;
mod manager;
mod registry;
#[cfg(test)]
mod test_utils;
mod tool;
mod types;
mod validator;

// 公开 API
pub use loader::SkillLoader;
pub use manager::SkillManager;
pub use registry::{SkillRegistry, SkillRegistryRef};
pub use tool::SkillTool;
pub use types::{SkillContent, SkillEntry, SkillMetadata, SkillSummary, ValidationResult};
pub use validator::SkillValidator;
