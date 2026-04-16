use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

// Serde helper functions for Arc<str>
fn serialize_arc_str<S>(value: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value)
}

fn deserialize_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(|s| s.into())
}

/// Agent Skills standard metadata
/// Corresponds to SKILL.md frontmatter
///
/// Note: name and description use `Arc<str>` to optimize sharing and reduce clone overhead
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    /// Skill name (required)
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub name: Arc<str>,

    /// Skill description (required) - used for matching and discovery
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub description: Arc<str>,

    /// License (optional)
    pub license: Option<String>,

    /// Compatibility statement (optional)
    pub compatibility: Option<String>,

    /// Extended metadata fields (author, version, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// List of allowed tools (optional)
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,

    /// Skill directory path (internal use)
    #[serde(skip)]
    pub skill_dir: PathBuf,
}

/// Complete skill content (result after progressive loading)
#[derive(Debug, Clone)]
pub struct SkillContent {
    /// Metadata
    pub metadata: SkillMetadata,

    /// SKILL.md main content
    pub instructions: String,

    /// Available scripts/ file list
    pub scripts: Vec<String>,

    /// Available references/ file list
    pub references: Vec<String>,

    /// Available assets/ file list
    pub assets: Vec<String>,
}

/// Skill entry in registry
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Metadata (always loaded)
    pub metadata: SkillMetadata,

    /// Full content (loaded on demand)
    pub content: Option<SkillContent>,

    /// Last modification time (for cache invalidation)
    pub last_modified: SystemTime,
}

// SkillMatchingMode has been removed, replaced with Tool mechanism to let LLM decide when to activate skills

/// Skill validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Skill summary for frontend use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub name: Arc<str>,
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub description: Arc<str>,
    pub license: Option<String>,
    pub metadata: HashMap<String, String>,
    /// Skill source: "global" | "workspace"
    pub source: String,
    /// Skill directory path (for debugging)
    pub skill_dir: String,
}

impl From<&SkillMetadata> for SkillSummary {
    fn from(metadata: &SkillMetadata) -> Self {
        let skill_dir = &metadata.skill_dir;
        let path_str = skill_dir.to_string_lossy();

        let is_home_compatible_global = dirs::home_dir().is_some_and(|home| {
            let home_claude = home.join(".claude").join("skills");
            let home_codex = home.join(".codex").join("skills");
            skill_dir.starts_with(&home_claude) || skill_dir.starts_with(&home_codex)
        });

        let source = if is_home_compatible_global {
            "global".to_string()
        } else if path_str.contains(".orbitx/skills") || path_str.contains(".claude/skills") {
            "workspace".to_string()
        } else {
            "global".to_string()
        };

        Self {
            name: Arc::clone(&metadata.name),
            description: Arc::clone(&metadata.description),
            license: metadata.license.clone(),
            metadata: metadata.metadata.clone(),
            source,
            skill_dir: path_str.into_owned(),
        }
    }
}
