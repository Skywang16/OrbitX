use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "variable")]
    Variable,
    #[serde(rename = "interface")]
    Interface,
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "trait")]
    Trait,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "property")]
    Property,
    #[serde(rename = "constant")]
    Constant,
    #[serde(rename = "module")]
    Module,
    #[serde(rename = "namespace")]
    Namespace,
    #[serde(rename = "macro")]
    Macro,
}

impl SymbolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolType::Function => "function",
            SymbolType::Class => "class",
            SymbolType::Variable => "variable",
            SymbolType::Interface => "interface",
            SymbolType::Type => "type",
            SymbolType::Struct => "struct",
            SymbolType::Enum => "enum",
            SymbolType::Trait => "trait",
            SymbolType::Method => "method",
            SymbolType::Property => "property",
            SymbolType::Constant => "constant",
            SymbolType::Module => "module",
            SymbolType::Namespace => "namespace",
            SymbolType::Macro => "macro",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeCodeParams {
    pub path: String,
    pub recursive: Option<bool>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub name: String,
    #[serde(rename = "type")]
    pub symbol_type: SymbolType,
    pub line: u32,
    pub column: u32,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub file: String,
    pub language: String,
    pub symbols: Vec<CodeSymbol>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub analyses: Vec<CodeAnalysis>,
    pub total_files: usize,
    pub success_count: usize,
    pub error_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AstError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("不支持的文件类型: {0}")]
    UnsupportedFileType(String),

    #[error("不支持的语言: {0}")]
    UnsupportedLanguage(String),

    #[error("解析失败: {0}")]
    ParseError(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub type AstResult<T> = Result<T, AstError>;
