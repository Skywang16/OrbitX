/*!
 * 代码解析服务模块
 *
 * 基于Tree-sitter实现语法感知的代码分块处理。
 * 支持多种编程语言的AST解析和智能代码块提取。
 *
 * Requirements: 1.1, 1.2, 1.3, 1.4
 */

use anyhow::{ensure, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, warn};
use tree_sitter::{Language as TSLanguage, Node, Parser};

use crate::vector_index::types::{
    ChunkType, CodeChunk, Language, ParsedCode, VectorIndexFullConfig,
};

// 子模块
pub mod scanner;
pub mod smart_chunker;

// 重新导出扫描器和分块器
pub use scanner::{CodeFileScanner, ScanStats};
pub use smart_chunker::SmartChunker;

/// 代码解析器接口
pub trait CodeParser {
    /// 解析单个文件
    fn parse_file(
        &self,
        file_path: &str,
    ) -> impl std::future::Future<Output = Result<ParsedCode>> + Send;

    /// 批量解析文件
    fn parse_files(
        &self,
        file_paths: &[String],
    ) -> impl std::future::Future<Output = Result<Vec<ParsedCode>>> + Send;

    /// 检查文件是否支持解析
    fn supports_file(&self, file_path: &str) -> bool;

    /// 获取支持的语言列表
    fn supported_languages(&self) -> Vec<Language>;
}

/// Tree-sitter代码解析器实现
pub struct TreeSitterParser {
    config: VectorIndexFullConfig,
    /// 语言配置缓存
    languages: HashMap<Language, TSLanguage>,
    /// 智能分块器
    smart_chunker: SmartChunker,
}

impl TreeSitterParser {
    /// 创建新的解析器实例
    pub fn new(config: VectorIndexFullConfig) -> Result<Self> {
        let mut parser = Self {
            config,
            languages: HashMap::new(),
            smart_chunker: SmartChunker::new(),
        };

        // 初始化支持的语言
        parser
            .initialize_languages()
            .context("初始化Tree-sitter语言失败")?;

        Ok(parser)
    }

    /// 初始化各语言的Tree-sitter语言定义
    fn initialize_languages(&mut self) -> Result<()> {
        let languages = vec![
            (
                Language::TypeScript,
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            ),
            (Language::JavaScript, tree_sitter_javascript::LANGUAGE),
            (Language::Rust, tree_sitter_rust::LANGUAGE),
            (Language::Python, tree_sitter_python::LANGUAGE),
            (Language::Go, tree_sitter_go::LANGUAGE),
            (Language::Java, tree_sitter_java::LANGUAGE),
        ];

        for (lang, ts_lang) in languages {
            self.languages.insert(lang, ts_lang.into());
            debug!("注册{}语言支持", lang.as_str());
        }

        Ok(())
    }

    /// 根据文件路径获取语言类型
    fn detect_language(&self, file_path: &str) -> Option<Language> {
        let path = Path::new(file_path);
        let extension = path.extension()?.to_str()?;
        Language::from_extension(extension)
    }

    /// 解析代码文本并提取代码块
    fn extract_code_chunks(
        &self,
        content: &str,
        language: Language,
        file_path: &str,
    ) -> Result<Vec<CodeChunk>> {
        let ts_language = self
            .languages
            .get(&language)
            .ok_or_else(|| anyhow::anyhow!("不支持的语言: {}", language.as_str()))?;

        // 创建新的parser实例（Tree-sitter parser不是线程安全的）
        let mut parser = Parser::new();
        parser
            .set_language(ts_language)
            .with_context(|| format!("设置{}语言解析器失败", language.as_str()))?;

        // 解析代码生成AST
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("解析代码失败"))?;

        let root_node = tree.root_node();
        let mut raw_chunks = Vec::new();

        // 遍历AST节点提取代码块
        self.walk_node_recursive(&root_node, content, &mut raw_chunks, language)?;

        // 使用智能分块器处理过大的代码块
        let mut final_chunks = Vec::new();
        let file_hash = self.smart_chunker.generate_content_hash(
            content,
            file_path,
            1,
            content.lines().count() as u32,
        );
        let raw_chunks_len = raw_chunks.len();

        for chunk in raw_chunks {
            let chunk_size = chunk.content.len();
            if chunk_size > self.config.chunk_size_range()[1] {
                // 使用智能分块器处理过大的块
                let sub_chunks = self.smart_chunker.chunk_large_content(
                    &chunk.content,
                    file_path,
                    &file_hash,
                    chunk.chunk_type,
                    chunk.start_line,
                )?;
                final_chunks.extend(sub_chunks);
            } else {
                final_chunks.push(chunk);
            }
        }

        debug!(
            "从文件 {} 提取了 {} 个代码块（智能分块后 {} 个）",
            file_path,
            raw_chunks_len,
            final_chunks.len()
        );
        Ok(final_chunks)
    }

    /// 递归遍历AST节点，提取有意义的代码块
    fn walk_node_recursive(
        &self,
        node: &Node,
        source_code: &str,
        chunks: &mut Vec<CodeChunk>,
        language: Language,
    ) -> Result<()> {
        // 检查当前节点是否是我们感兴趣的代码块类型
        if let Some(chunk_type) = self.classify_node(node, language) {
            let start_line = node.start_position().row as u32 + 1; // Tree-sitter从0开始，我们从1开始
            let end_line = node.end_position().row as u32 + 1;

            // 提取节点对应的源代码
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();

            if start_byte < source_code.len() && end_byte <= source_code.len() {
                let content = source_code[start_byte..end_byte].to_string();

                // 过滤太短或太长的代码块
                if content.len() >= self.config.chunk_size_range()[0]
                    && content.len() <= self.config.chunk_size_range()[1]
                {
                    let mut metadata = HashMap::new();
                    metadata.insert("node_kind".to_string(), node.kind().to_string());

                    // 尝试提取函数名等额外信息
                    if let Some(name) = self.extract_node_name(node, source_code) {
                        metadata.insert("name".to_string(), name);
                    }

                    chunks.push(CodeChunk {
                        content,
                        start_line,
                        end_line,
                        chunk_type,
                        metadata,
                    });
                }
            }
        }

        // 递归处理子节点
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node_recursive(&child, source_code, chunks, language)?;
        }

        Ok(())
    }

    /// 根据节点类型和语言分类代码块
    fn classify_node(&self, node: &Node, language: Language) -> Option<ChunkType> {
        let kind = node.kind();

        match language {
            Language::Rust => match kind {
                "function_item" => Some(ChunkType::Function),
                "impl_item" => Some(ChunkType::Method),
                "struct_item" => Some(ChunkType::Struct),
                "enum_item" => Some(ChunkType::Enum),
                "trait_item" => Some(ChunkType::Interface),
                "mod_item" => Some(ChunkType::Module),
                "line_comment" | "block_comment" => Some(ChunkType::Comment),
                "use_declaration" => Some(ChunkType::Import),
                _ => None,
            },
            Language::TypeScript | Language::JavaScript => match kind {
                "function_declaration" | "function_expression" | "arrow_function" => {
                    Some(ChunkType::Function)
                }
                "method_definition" => Some(ChunkType::Method),
                "class_declaration" => Some(ChunkType::Class),
                "interface_declaration" => Some(ChunkType::Interface),
                "enum_declaration" => Some(ChunkType::Enum),
                "module_declaration" => Some(ChunkType::Module),
                "comment" => Some(ChunkType::Comment),
                "import_statement" | "import_declaration" => Some(ChunkType::Import),
                _ => None,
            },
            Language::Python => match kind {
                "function_definition" => Some(ChunkType::Function),
                "class_definition" => Some(ChunkType::Class),
                "comment" => Some(ChunkType::Comment),
                "import_statement" | "import_from_statement" => Some(ChunkType::Import),
                _ => None,
            },
            Language::Go => match kind {
                "function_declaration" | "method_declaration" => Some(ChunkType::Function),
                "type_declaration" => Some(ChunkType::Struct),
                "interface_type" => Some(ChunkType::Interface),
                "comment" => Some(ChunkType::Comment),
                "import_declaration" => Some(ChunkType::Import),
                _ => None,
            },
            Language::Java => match kind {
                "method_declaration" => Some(ChunkType::Method),
                "constructor_declaration" => Some(ChunkType::Function),
                "class_declaration" => Some(ChunkType::Class),
                "interface_declaration" => Some(ChunkType::Interface),
                "enum_declaration" => Some(ChunkType::Enum),
                "comment" => Some(ChunkType::Comment),
                "import_declaration" => Some(ChunkType::Import),
                _ => None,
            },
            Language::C | Language::Cpp => match kind {
                "function_definition" => Some(ChunkType::Function),
                "struct_specifier" => Some(ChunkType::Struct),
                "enum_specifier" => Some(ChunkType::Enum),
                "comment" => Some(ChunkType::Comment),
                "preproc_include" => Some(ChunkType::Import),
                _ => None,
            },
        }
    }

    /// 提取节点名称（如函数名、类名等）
    fn extract_node_name(&self, node: &Node, source_code: &str) -> Option<String> {
        // 查找标识符子节点
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let start_byte = child.start_byte();
                let end_byte = child.end_byte();
                if start_byte < source_code.len() && end_byte <= source_code.len() {
                    return Some(source_code[start_byte..end_byte].to_string());
                }
                break;
            }
        }
        None
    }
}

impl CodeParser for TreeSitterParser {
    async fn parse_file(&self, file_path: &str) -> Result<ParsedCode> {
        // 检查文件是否支持
        ensure!(
            self.supports_file(file_path),
            "不支持的文件类型: {}",
            file_path
        );

        // 读取文件内容
        let content = fs::read_to_string(file_path)
            .await
            .with_context(|| format!("读取文件失败: {}", file_path))?;

        // 检测语言类型
        let language = self
            .detect_language(file_path)
            .ok_or_else(|| anyhow::anyhow!("无法检测文件语言: {}", file_path))?;

        debug!("解析文件: {} ({})", file_path, language.as_str());

        // 提取代码块
        let mut errors = Vec::new();
        let chunks = match self.extract_code_chunks(&content, language, file_path) {
            Ok(chunks) => chunks,
            Err(e) => {
                let error_msg = format!("解析代码块失败: {}", e);
                error!("{}", error_msg);
                errors.push(error_msg);
                Vec::new()
            }
        };

        Ok(ParsedCode {
            file_path: file_path.to_string(),
            chunks,
            errors,
        })
    }

    async fn parse_files(&self, file_paths: &[String]) -> Result<Vec<ParsedCode>> {
        let mut results = Vec::new();

        // 分批处理文件以控制内存使用和并发度
        let batch_size = self
            .config
            .user_config
            .max_concurrent_files
            .min(file_paths.len());

        for chunk in file_paths.chunks(batch_size) {
            let mut batch_results = Vec::new();

            // 顺序处理一批文件（在TreeSitterParser支持Clone之前的临时方案）
            for file_path in chunk {
                match self.parse_file(file_path).await {
                    Ok(parsed) => batch_results.push(parsed),
                    Err(e) => {
                        warn!("解析文件失败: {} - {}", file_path, e);
                        // 创建错误结果而不是跳过
                        batch_results.push(ParsedCode {
                            file_path: file_path.clone(),
                            chunks: Vec::new(),
                            errors: vec![format!("解析失败: {}", e)],
                        });
                    }
                }
            }

            results.extend(batch_results);

            // 在批次之间让出控制权，避免长时间阻塞
            tokio::task::yield_now().await;
        }

        Ok(results)
    }

    fn supports_file(&self, file_path: &str) -> bool {
        self.detect_language(file_path).is_some()
    }

    fn supported_languages(&self) -> Vec<Language> {
        self.languages.keys().copied().collect()
    }
}
