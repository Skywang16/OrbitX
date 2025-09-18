use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use ignore::WalkBuilder;
use ignore::gitignore::GitignoreBuilder;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub async fn fs_list_directory(path: String, recursive: bool) -> TauriApiResult<Vec<String>> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(api_error!("common.not_found"));
    }
    if !root.is_dir() {
        return Ok(api_error!("common.invalid_path"));
    }

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .standard_filters(true)
        .sort_by_file_name(|a, b| a.cmp(b));

    if !recursive {
        builder.max_depth(Some(1));
    }

    let mut entries: Vec<(String, bool)> = Vec::new();

    // Load root .gitignore explicitly to ensure ignore semantics even when walker misses them
    let gitignore = {
        let mut builder = GitignoreBuilder::new(&root);
        let gi_path = root.join(".gitignore");
        if gi_path.exists() {
            let _ = builder.add(gi_path);
        }
        builder.build().ok()
    };

    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.depth() == 0 {
            continue;
        }
        let p = entry.path();
        let rel = p.strip_prefix(&root).unwrap_or(p);
        let is_dir = entry
            .file_type()
            .map(|ft| ft.is_dir())
            .unwrap_or_else(|| p.is_dir());
        // Extra guard: apply root .gitignore matcher if available
        if let Some(matcher) = &gitignore {
            if matcher.matched_path_or_any_parents(rel, is_dir).is_ignore() {
                continue;
            }
        }
        let mut name = rel.to_string_lossy().to_string();
        if is_dir && !name.ends_with('/') {
            name.push('/');
        }
        entries.push((name, is_dir));
    }

    // 排序：目录在前，字典序
    entries.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.cmp(&b.0),
    });

    let out: Vec<String> = entries.into_iter().map(|(s, _)| s).collect();
    Ok(api_success!(out))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDefItem {
    pub file: String,
    pub kind: String,
    pub name: String,
    pub line: usize,
    pub exported: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

fn is_source_file(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
    {
        Some(ext) => matches!(ext.as_str(), "ts" | "tsx" | "js" | "jsx"),
        None => false,
    }
}

fn compute_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn offset_to_line(offsets: &[usize], pos: usize) -> usize {
    match offsets.binary_search(&pos) {
        Ok(i) => i + 1,
        Err(i) => i, // line number starting at 1
    }
}

#[tauri::command]
pub async fn code_list_definition_names(path: String) -> TauriApiResult<Vec<CodeDefItem>> {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::*;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let p = PathBuf::from(&path);
    if !p.exists() {
        return Ok(api_error!("common.not_found"));
    }

    let mut files: Vec<PathBuf> = Vec::new();
    if p.is_dir() {
        // 顶层非递归
        if let Ok(rd) = fs::read_dir(&p) {
            for entry in rd.flatten() {
                let fp = entry.path();
                if fp.is_file() && is_source_file(&fp) {
                    files.push(fp);
                }
            }
        }
    } else if p.is_file() {
        if is_source_file(&p) {
            files.push(p.clone());
        }
    }

    let mut defs: Vec<CodeDefItem> = Vec::new();

    for file in files {
        let Ok(source) = fs::read_to_string(&file) else {
            continue;
        };
        let allocator = Allocator::default();

        let source_type = SourceType::from_path(&file).unwrap_or_else(|_| {
            // fallback by extension
            let st = SourceType::default();
            st
        });

        let parser = Parser::new(&allocator, &source, source_type);
        let parsed = parser.parse();
        let program = parsed.program;

        let line_offsets = compute_line_offsets(&source);

        for item in &program.body {
            match item {
                Statement::FunctionDeclaration(func) => {
                    if let Some(ident) = &func.id {
                        let name = ident.name.to_string();
                        let line = offset_to_line(&line_offsets, func.span.start as usize);
                        defs.push(CodeDefItem {
                            file: file.display().to_string(),
                            kind: "function".into(),
                            name,
                            line,
                            exported: None,
                            is_default: None,
                        });
                    }
                }
                Statement::ClassDeclaration(class) => {
                    if let Some(ident) = &class.id {
                        let name = ident.name.to_string();
                        let line = offset_to_line(&line_offsets, class.span.start as usize);
                        defs.push(CodeDefItem {
                            file: file.display().to_string(),
                            kind: "class".into(),
                            name,
                            line,
                            exported: None,
                            is_default: None,
                        });
                    }
                }
                Statement::TSInterfaceDeclaration(iface) => {
                    let name = iface.id.name.to_string();
                    let line = offset_to_line(&line_offsets, iface.span.start as usize);
                    defs.push(CodeDefItem {
                        file: file.display().to_string(),
                        kind: "interface".into(),
                        name,
                        line,
                        exported: None,
                        is_default: None,
                    });
                }
                Statement::TSTypeAliasDeclaration(ty) => {
                    let name = ty.id.name.to_string();
                    let line = offset_to_line(&line_offsets, ty.span.start as usize);
                    defs.push(CodeDefItem {
                        file: file.display().to_string(),
                        kind: "type".into(),
                        name,
                        line,
                        exported: None,
                        is_default: None,
                    });
                }
                Statement::TSEnumDeclaration(enm) => {
                    let name = enm.id.name.to_string();
                    let line = offset_to_line(&line_offsets, enm.span.start as usize);
                    defs.push(CodeDefItem {
                        file: file.display().to_string(),
                        kind: "enum".into(),
                        name,
                        line,
                        exported: None,
                        is_default: None,
                    });
                }
                Statement::VariableDeclaration(var) => {
                    for decl in &var.declarations {
                        if let Some(init) = &decl.init {
                            let is_fn_like = matches!(
                                init,
                                Expression::FunctionExpression(_)
                                    | Expression::ArrowFunctionExpression(_)
                            );
                            if is_fn_like {
                                if let Some(ident) = decl.id.kind.get_binding_identifier() {
                                    let name = ident.name.to_string();
                                    let line =
                                        offset_to_line(&line_offsets, decl.span.start as usize);
                                    defs.push(CodeDefItem {
                                        file: file.display().to_string(),
                                        kind: "var-function".into(),
                                        name,
                                        line,
                                        exported: None,
                                        is_default: None,
                                    });
                                }
                            }
                        }
                    }
                }
                Statement::ExportDefaultDeclaration(ed) => {
                    use oxc_ast::ast::ExportDefaultDeclarationKind as K;
                    match &ed.declaration {
                        K::FunctionDeclaration(f) => {
                            let name =
                                f.id.as_ref()
                                    .map(|i| i.name.to_string())
                                    .unwrap_or_else(|| "(anonymous)".into());
                            let line = offset_to_line(&line_offsets, ed.span.start as usize);
                            defs.push(CodeDefItem {
                                file: file.display().to_string(),
                                kind: "default".into(),
                                name,
                                line,
                                exported: Some(true),
                                is_default: Some(true),
                            });
                        }
                        K::ClassDeclaration(c) => {
                            let name =
                                c.id.as_ref()
                                    .map(|i| i.name.to_string())
                                    .unwrap_or_else(|| "(anonymous)".into());
                            let line = offset_to_line(&line_offsets, ed.span.start as usize);
                            defs.push(CodeDefItem {
                                file: file.display().to_string(),
                                kind: "default".into(),
                                name,
                                line,
                                exported: Some(true),
                                is_default: Some(true),
                            });
                        }
                        _ => {
                            if let Some(expr) = ed.declaration.as_expression() {
                                if let Expression::Identifier(ident) = expr {
                                    let name = ident.name.to_string();
                                    let line =
                                        offset_to_line(&line_offsets, ed.span.start as usize);
                                    defs.push(CodeDefItem {
                                        file: file.display().to_string(),
                                        kind: "default".into(),
                                        name,
                                        line,
                                        exported: Some(true),
                                        is_default: Some(true),
                                    });
                                } else {
                                    let line =
                                        offset_to_line(&line_offsets, ed.span.start as usize);
                                    defs.push(CodeDefItem {
                                        file: file.display().to_string(),
                                        kind: "default".into(),
                                        name: "(anonymous)".into(),
                                        line,
                                        exported: Some(true),
                                        is_default: Some(true),
                                    });
                                }
                            } else {
                                let line = offset_to_line(&line_offsets, ed.span.start as usize);
                                defs.push(CodeDefItem {
                                    file: file.display().to_string(),
                                    kind: "default".into(),
                                    name: "(anonymous)".into(),
                                    line,
                                    exported: Some(true),
                                    is_default: Some(true),
                                });
                            }
                        }
                    }
                }
                Statement::ExportNamedDeclaration(e) => {
                    use oxc_ast::ast::Declaration as D;
                    if let Some(decl) = &e.declaration {
                        match decl {
                            D::FunctionDeclaration(func) => {
                                if let Some(ident) = &func.id {
                                    let name = ident.name.to_string();
                                    let line = offset_to_line(&line_offsets, func.span.start as usize);
                                    defs.push(CodeDefItem {
                                        file: file.display().to_string(),
                                        kind: "function".into(),
                                        name,
                                        line,
                                        exported: Some(true),
                                        is_default: Some(false),
                                    });
                                }
                            }
                            D::ClassDeclaration(class) => {
                                if let Some(ident) = &class.id {
                                    let name = ident.name.to_string();
                                    let line = offset_to_line(&line_offsets, class.span.start as usize);
                                    defs.push(CodeDefItem {
                                        file: file.display().to_string(),
                                        kind: "class".into(),
                                        name,
                                        line,
                                        exported: Some(true),
                                        is_default: Some(false),
                                    });
                                }
                            }
                            D::TSEnumDeclaration(enm) => {
                                let name = enm.id.name.to_string();
                                let line = offset_to_line(&line_offsets, enm.span.start as usize);
                                defs.push(CodeDefItem {
                                    file: file.display().to_string(),
                                    kind: "enum".into(),
                                    name,
                                    line,
                                    exported: Some(true),
                                    is_default: Some(false),
                                });
                            }
                            D::TSInterfaceDeclaration(iface) => {
                                let name = iface.id.name.to_string();
                                let line = offset_to_line(&line_offsets, iface.span.start as usize);
                                defs.push(CodeDefItem {
                                    file: file.display().to_string(),
                                    kind: "interface".into(),
                                    name,
                                    line,
                                    exported: Some(true),
                                    is_default: Some(false),
                                });
                            }
                            D::TSTypeAliasDeclaration(ty) => {
                                let name = ty.id.name.to_string();
                                let line = offset_to_line(&line_offsets, ty.span.start as usize);
                                defs.push(CodeDefItem {
                                    file: file.display().to_string(),
                                    kind: "type".into(),
                                    name,
                                    line,
                                    exported: Some(true),
                                    is_default: Some(false),
                                });
                            }
                            D::VariableDeclaration(var) => {
                                for decl in &var.declarations {
                                    if let Some(init) = &decl.init {
                                        let is_fn_like = matches!(
                                            init,
                                            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                                        );
                                        if is_fn_like {
                                            if let Some(ident) = decl.id.kind.get_binding_identifier() {
                                                let name = ident.name.to_string();
                                                let line = offset_to_line(&line_offsets, decl.span.start as usize);
                                                defs.push(CodeDefItem {
                                                    file: file.display().to_string(),
                                                    kind: "var-function".into(),
                                                    name,
                                                    line,
                                                    exported: Some(true),
                                                    is_default: Some(false),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(api_success!(defs))
}
