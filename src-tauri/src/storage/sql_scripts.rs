use crate::storage::error::{SqlScriptError, SqlScriptResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct SqlScript {
    pub path: PathBuf,
    pub name: Arc<str>,
    pub order: u32,
    pub statements: Arc<[String]>,
}

#[derive(Debug, Clone)]
pub struct SqlScriptCatalog {
    scripts: Arc<[SqlScript]>,
}

impl SqlScriptCatalog {
    pub async fn load(dir: impl AsRef<Path>) -> SqlScriptResult<Self> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Err(SqlScriptError::DirectoryMissing {
                path: dir.to_path_buf(),
            });
        }

        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|err| SqlScriptError::read_directory(dir.to_path_buf(), err))?;

        let mut scripts = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|err| SqlScriptError::walk_directory(dir.to_path_buf(), err))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("sql") {
                continue;
            }

            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| SqlScriptError::InvalidFileName { path: path.clone() })?;
            let order = parse_order(file_name)?;
            let content = fs::read_to_string(&path)
                .await
                .map_err(|err| SqlScriptError::read_file(path.clone(), err))?;
            let statements = parse_statements(&content)?;
            if statements.is_empty() {
                continue;
            }

            scripts.push(SqlScript {
                path: path.clone(),
                name: Arc::<str>::from(file_name.to_string()),
                order,
                statements: statements.into(),
            });
        }

        scripts.sort_by(|a, b| match a.order.cmp(&b.order) {
            std::cmp::Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        });

        Ok(Self {
            scripts: scripts.into(),
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &SqlScript> {
        self.scripts.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.scripts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.scripts.len()
    }
}

pub fn parse_statements(content: &str) -> SqlScriptResult<Vec<String>> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_block_comment = false;
    let mut in_trigger = false;

    for line in content.lines() {
        let Some(stripped) = strip_line(line, &mut in_block_comment) else {
            continue;
        };

        if !in_trigger {
            let upper = stripped.to_ascii_uppercase();
            if upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE TEMP TRIGGER") {
                in_trigger = true;
            }
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&stripped);

        if in_trigger {
            let upper = stripped.to_ascii_uppercase();
            if upper.ends_with("END;") || upper == "END;" {
                if current.ends_with(';') {
                    current.pop();
                }
                let stmt = current.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current.clear();
                in_trigger = false;
            }
            continue;
        }

        if stripped.ends_with(';') {
            if current.ends_with(';') {
                current.pop();
            }
            let stmt = current.trim();
            if !stmt.is_empty() {
                statements.push(stmt.to_string());
            }
            current.clear();
        }
    }

    let tail = current.trim();
    if !tail.is_empty() {
        statements.push(tail.to_string());
    }

    Ok(statements)
}

fn strip_line(line: &str, in_block_comment: &mut bool) -> Option<String> {
    let mut out = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && matches!(chars.peek(), Some('/')) {
                chars.next();
                *in_block_comment = false;
            }
            continue;
        }

        if ch == '/' && matches!(chars.peek(), Some('*')) {
            chars.next();
            *in_block_comment = true;
            continue;
        }

        if ch == '-' && matches!(chars.peek(), Some('-')) {
            break;
        }

        out.push(ch);
    }

    let trimmed = out.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_order(filename: &str) -> SqlScriptResult<u32> {
    let digits: String = filename
        .split(['_', '-'])
        .next()
        .unwrap_or_default()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return Err(SqlScriptError::MissingOrder {
            filename: filename.to_string(),
        });
    }
    digits
        .parse::<u32>()
        .map_err(|err| SqlScriptError::ParseOrder {
            filename: filename.to_string(),
            source: err,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn parses_sql_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sql_dir = temp_dir.path().join("sql");
        fs::create_dir_all(&sql_dir).await.unwrap();

        fs::write(
            sql_dir.join("01_tables.sql"),
            "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        )
        .await
        .unwrap();

        fs::write(
            sql_dir.join("02_triggers.sql"),
            "CREATE TRIGGER demo AFTER INSERT ON users BEGIN SELECT 1; END;",
        )
        .await
        .unwrap();

        let catalog = SqlScriptCatalog::load(&sql_dir).await.unwrap();
        assert_eq!(catalog.len(), 2);
        let names: Vec<_> = catalog.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names[0].as_ref(), "01_tables");
        assert_eq!(names[1].as_ref(), "02_triggers");
    }

    #[test]
    fn parses_statements() {
        let content = r#"
            -- comment
            CREATE TABLE test (id INTEGER);
            INSERT INTO test VALUES (1);
            CREATE TRIGGER trg AFTER INSERT ON test BEGIN
                INSERT INTO audit VALUES (NEW.id);
            END;
        "#;
        let statements = parse_statements(content).unwrap();
        assert_eq!(statements.len(), 3);
        assert!(statements[0].starts_with("CREATE TABLE"));
        assert!(statements[2].starts_with("CREATE TRIGGER"));
    }
}
