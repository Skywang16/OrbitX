//! Agent Skills system - Compliant with Agent Skills open standard
//!
//! Core design principle: Progressive Disclosure
//!
//! ## Workflow
//!
//! 1. **Discovery phase**: Scan `.claude/skills/` and `.orbitx/skills/`, load metadata for all skills
//! 2. **Activation phase**: Based on user prompt and matching mode, load full content of selected skills
//! 3. **Execution phase**: Agent can load reference files on demand during execution (scripts/, references/, assets/)
//!
//! ## Standard Directory Structure
//!
//! ```text
//! skill-name/
//! ├── SKILL.md          # Required: skill definition and instructions
//! ├── scripts/          # Optional: executable scripts
//! ├── references/       # Optional: documentation and reference materials
//! └── assets/          # Optional: templates, resource files
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use orbitx_agent::skill_system::SkillManager;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let manager = SkillManager::new();
//!
//! // 1. Discover skills
//! let workspace = Path::new("/path/to/workspace");
//! let skills = manager.discover_skills(workspace).await?;
//! println!("Found {} skills", skills.len());
//!
//! // 2. Activate skills
//! let user_prompt = "Extract text from PDF using @pdf-processing";
//! let activated = manager.activate_skills(
//!     user_prompt,
//!     SkillMatchingMode::Hybrid,
//!     None
//! ).await?;
//!
//! // 3. Use skill content
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

// Public API
pub use loader::SkillLoader;
pub use manager::SkillManager;
pub use registry::{SkillRegistry, SkillRegistryRef};
pub use tool::SkillTool;
pub use types::{SkillContent, SkillEntry, SkillMetadata, SkillSummary, ValidationResult};
pub use validator::SkillValidator;
