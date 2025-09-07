use crate::ai::tool::ast::types::{CodeAnalysis, CodeSymbol, SymbolType};
use anyhow::{anyhow, Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::{ast::*, Visit};
use oxc_parser::Parser as OxcParser;
use oxc_span::SourceType;
use std::path::Path;
use syn::visit::Visit as SynVisit;

// 使用 tree-sitter crate 提供的语言

pub struct AstParser;

impl AstParser {
    pub fn new() -> Self {
        Self
    }

    pub async fn analyze_file(&self, file_path: &str) -> Result<CodeAnalysis> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(anyhow!("文件不存在: {}", file_path));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("读取文件失败: {}", file_path))?;
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension {
            "ts" | "tsx" => self.analyze_typescript(&content, file_path, extension == "tsx"),
            "js" | "jsx" => self.analyze_javascript(&content, file_path, extension == "jsx"),
            "py" => self.analyze_python(&content, file_path).await,
            "rs" => self.analyze_rust_code(&content, file_path),
            _ => Err(anyhow!("不支持的文件类型: {}", extension)),
        }
    }

    fn analyze_typescript(
        &self,
        content: &str,
        file_path: &str,
        is_tsx: bool,
    ) -> Result<CodeAnalysis> {
        let source_type = if is_tsx {
            SourceType::tsx()
        } else {
            SourceType::ts()
        };
        let language = if is_tsx { "tsx" } else { "typescript" };

        self.analyze_with_oxc(content, file_path, source_type, language, "TypeScript")
    }

    fn analyze_javascript(
        &self,
        content: &str,
        file_path: &str,
        is_jsx: bool,
    ) -> Result<CodeAnalysis> {
        let source_type = if is_jsx {
            SourceType::jsx()
        } else {
            SourceType::mjs()
        };
        let language = if is_jsx { "jsx" } else { "javascript" };

        self.analyze_with_oxc(content, file_path, source_type, language, "JavaScript")
    }

    /// 通用的 Oxc 解析函数，消除代码重复
    fn analyze_with_oxc(
        &self,
        content: &str,
        file_path: &str,
        source_type: SourceType,
        language: &str,
        error_prefix: &str,
    ) -> Result<CodeAnalysis> {
        let allocator = Allocator::default();
        let parser_return = OxcParser::new(&allocator, content, source_type).parse();

        if !parser_return.errors.is_empty() {
            return Err(anyhow!(
                "{}解析错误: {:?}",
                error_prefix,
                parser_return.errors
            ));
        }

        let mut visitor = OxcSymbolVisitor::new(file_path);
        visitor.visit_program(&parser_return.program);

        Ok(CodeAnalysis {
            file: file_path.to_string(),
            language: language.to_string(),
            symbols: visitor.symbols,
            imports: visitor.imports,
            exports: visitor.exports,
        })
    }

    fn analyze_rust_code(&self, content: &str, file_path: &str) -> Result<CodeAnalysis> {
        let syntax_tree = syn::parse_file(content).with_context(|| "Rust代码解析失败")?;

        let mut visitor = RustSymbolVisitor::new(file_path);
        visitor.visit_file(&syntax_tree);

        Ok(CodeAnalysis {
            file: file_path.to_string(),
            language: "rust".to_string(),
            symbols: visitor.symbols,
            imports: visitor.imports,
            exports: visitor.exports,
        })
    }

    async fn analyze_python(&self, _content: &str, _file_path: &str) -> Result<CodeAnalysis> {
        // 暂时禁用 Python 分析，因为需要正确配置 tree-sitter-python
        Err(anyhow!("Python 分析功能暂时禁用"))
    }
}

// Oxc AST 访问者，用于 TypeScript/JavaScript
struct OxcSymbolVisitor {
    symbols: Vec<CodeSymbol>,
    imports: Vec<String>,
    exports: Vec<String>,
    file_path: String,
}

impl OxcSymbolVisitor {
    fn new(file_path: &str) -> Self {
        Self {
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            file_path: file_path.to_string(),
        }
    }
}

impl<'a> Visit<'a> for OxcSymbolVisitor {
    fn visit_function(&mut self, func: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {
        if let Some(id) = &func.id {
            self.symbols.push(CodeSymbol {
                name: id.name.to_string(),
                symbol_type: SymbolType::Function,
                line: id.span.start as u32,
                column: 0,
                file: self.file_path.clone(),
            });
        }
    }

    fn visit_class(&mut self, class: &Class<'a>) {
        if let Some(id) = &class.id {
            self.symbols.push(CodeSymbol {
                name: id.name.to_string(),
                symbol_type: SymbolType::Class,
                line: id.span.start as u32,
                column: 0,
                file: self.file_path.clone(),
            });
        }
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let BindingPatternKind::BindingIdentifier(id) = &declarator.id.kind {
            self.symbols.push(CodeSymbol {
                name: id.name.to_string(),
                symbol_type: SymbolType::Variable,
                line: id.span.start as u32,
                column: 0,
                file: self.file_path.clone(),
            });
        }
    }

    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        self.imports.push(import.source.value.to_string());
    }

    fn visit_export_named_declaration(&mut self, export: &ExportNamedDeclaration<'a>) {
        if let Some(Declaration::FunctionDeclaration(func)) = &export.declaration {
            if let Some(id) = &func.id {
                self.exports.push(id.name.to_string());
            }
        }
    }
}

// Syn 访问者，用于 Rust
struct RustSymbolVisitor {
    symbols: Vec<CodeSymbol>,
    imports: Vec<String>,
    exports: Vec<String>,
    file_path: String,
}

impl RustSymbolVisitor {
    fn new(file_path: &str) -> Self {
        Self {
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            file_path: file_path.to_string(),
        }
    }
}

impl<'ast> SynVisit<'ast> for RustSymbolVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.symbols.push(CodeSymbol {
            name: node.sig.ident.to_string(),
            symbol_type: SymbolType::Function,
            line: 1, // Syn doesn't provide line info easily, use default
            column: 0,
            file: self.file_path.clone(),
        });
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.symbols.push(CodeSymbol {
            name: node.ident.to_string(),
            symbol_type: SymbolType::Struct,
            line: 1,
            column: 0,
            file: self.file_path.clone(),
        });
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.symbols.push(CodeSymbol {
            name: node.ident.to_string(),
            symbol_type: SymbolType::Enum,
            line: 1,
            column: 0,
            file: self.file_path.clone(),
        });
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.symbols.push(CodeSymbol {
            name: node.ident.to_string(),
            symbol_type: SymbolType::Trait,
            line: 1,
            column: 0,
            file: self.file_path.clone(),
        });
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.imports.push(format!("{}", quote::quote!(#node)));
    }
}
