use std::path::Path;

use tree_sitter::Parser;

use crate::vector_db::core::Language;

#[derive(Debug, thiserror::Error)]
pub enum TreeSitterError {
    #[error("language {0:?} not supported for tree-sitter parsing")]
    Unsupported(Language),
    #[error("failed to set tree-sitter language: {0}")]
    SetLanguage(String),
}

pub fn configure_parser_for_language(
    parser: &mut Parser,
    file_path: &Path,
    language: Language,
) -> Result<(), TreeSitterError> {
    match language {
        Language::Python => parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::TypeScript => {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let ts_lang = if ext.eq_ignore_ascii_case("tsx") {
                tree_sitter_typescript::LANGUAGE_TSX
            } else {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT
            };
            parser
                .set_language(&ts_lang.into())
                .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?;
        }
        Language::JavaScript => parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Rust => parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Go => parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Java => parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::C => parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Cpp => parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::CSharp => parser
            .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Ruby => parser
            .set_language(&tree_sitter_ruby::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Php => parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Swift => parser
            .set_language(&tree_sitter_swift::LANGUAGE.into())
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?,
        Language::Kotlin => return Err(TreeSitterError::Unsupported(language)),
    }

    Ok(())
}
