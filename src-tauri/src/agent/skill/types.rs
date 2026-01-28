use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

// Arc<str> 的 serde 辅助函数
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

/// Agent Skills 标准元数据
/// 对应 SKILL.md 的 frontmatter
///
/// 注意: name 和 description 使用 `Arc<str>` 优化共享,减少克隆开销
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    /// 技能名称 (必需)
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub name: Arc<str>,

    /// 技能描述 (必需) - 用于匹配和发现
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub description: Arc<str>,

    /// 许可证 (可选)
    pub license: Option<String>,

    /// 兼容性声明 (可选)
    pub compatibility: Option<String>,

    /// 扩展元数据字段 (author, version 等)
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// 允许使用的工具列表 (可选)
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,

    /// 技能目录路径 (内部使用)
    #[serde(skip)]
    pub skill_dir: PathBuf,
}

/// 完整的技能内容 (渐进式加载后的结果)
#[derive(Debug, Clone)]
pub struct SkillContent {
    /// 元数据
    pub metadata: SkillMetadata,

    /// SKILL.md 的主体内容
    pub instructions: String,

    /// 可用的 scripts/ 文件列表
    pub scripts: Vec<String>,

    /// 可用的 references/ 文件列表
    pub references: Vec<String>,

    /// 可用的 assets/ 文件列表
    pub assets: Vec<String>,
}

/// 注册表中的技能条目
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// 元数据 (总是加载)
    pub metadata: SkillMetadata,

    /// 完整内容 (按需加载)
    pub content: Option<SkillContent>,

    /// 最后修改时间 (用于缓存失效)
    pub last_modified: SystemTime,
}

/// 技能匹配模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMatchingMode {
    /// 仅 @skill 显式引用
    Explicit,

    /// 基于 description 的语义匹配
    Semantic,

    /// 混合模式 (默认)
    Hybrid,
}

impl Default for SkillMatchingMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// 技能验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 供前端使用的技能摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub metadata: HashMap<String, String>,
    /// Skill 来源: "global" | "workspace"
    pub source: String,
    /// Skill 目录路径 (用于调试)
    pub skill_dir: String,
}

impl From<&SkillMetadata> for SkillSummary {
    fn from(metadata: &SkillMetadata) -> Self {
        // 判断来源: 如果路径包含 .orbitx/skills 或 .claude/skills 则是workspace
        let source = if metadata.skill_dir.to_string_lossy().contains(".orbitx/skills")
            || metadata.skill_dir.to_string_lossy().contains(".claude/skills")
        {
            "workspace".to_string()
        } else {
            "global".to_string()
        };

        Self {
            name: metadata.name.to_string(),
            description: metadata.description.to_string(),
            license: metadata.license.clone(),
            metadata: metadata.metadata.clone(),
            source,
            skill_dir: metadata.skill_dir.to_string_lossy().to_string(),
        }
    }
}
