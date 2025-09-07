use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Variable,
    Interface,
    Type,
    Struct,
    Enum,
    Trait,
    Method,
    Property,
    Constant,
    Module,
    Namespace,
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
