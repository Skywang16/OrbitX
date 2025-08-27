/*!
 * SQL脚本加载器模块
 *
 * 负责读取和解析SQL脚本文件，支持注释过滤和语句分割
 * 提供简洁的SQL文件管理接口
 */

use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

pub struct SqlScriptLoader {
    sql_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SqlScript {
    pub path: PathBuf,
    pub name: String,
    pub order: u32,
    pub statements: Vec<String>,
}

impl SqlScriptLoader {
    pub fn new<P: AsRef<Path>>(sql_dir: P) -> Self {
        Self {
            sql_dir: sql_dir.as_ref().to_path_buf(),
        }
    }

    pub async fn load_all_scripts(&self) -> AppResult<Vec<SqlScript>> {
        debug!("开始加载SQL脚本文件，目录: {}", self.sql_dir.display());

        // 确保SQL目录存在
        if !self.sql_dir.exists() {
            return Err(anyhow!("SQL脚本目录不存在: {}", self.sql_dir.display()));
        }

        let mut scripts = Vec::new();
        let mut entries = fs::read_dir(&self.sql_dir)
            .await
            .with_context(|| format!("读取SQL脚本目录失败: {}", self.sql_dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| "遍历SQL脚本目录失败")?
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
                match self.load_script_file(&path).await {
                    Ok(script) => {
                        debug!("成功加载SQL脚本: {}", script.name);
                        scripts.push(script);
                    }
                    Err(e) => {
                        warn!("加载SQL脚本失败 {}: {}", path.display(), e);
                    }
                }
            }
        }

        // 按执行顺序排序
        scripts.sort_by_key(|s| s.order);

        debug!("成功加载 {} 个SQL脚本", scripts.len());
        Ok(scripts)
    }

    pub async fn load_script_file(&self, path: &Path) -> AppResult<SqlScript> {
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无效的文件名: {}", path.display()))?;

        // 从文件名解析执行顺序（假设格式为 01_tables.sql）
        let order = self.parse_order_from_filename(file_name)?;

        // 读取文件内容
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("读取SQL文件失败: {}", path.display()))?;

        // 解析SQL语句
        let statements = self.parse_sql_statements(&content)?;

        Ok(SqlScript {
            path: path.to_path_buf(),
            name: file_name.to_string(),
            order,
            statements,
        })
    }

    fn parse_order_from_filename(&self, filename: &str) -> AppResult<u32> {
        // 假设文件名格式为 "01_tables" 或 "01-tables"
        let raw = filename
            .split('_')
            .next()
            .or_else(|| filename.split('-').next())
            .ok_or_else(|| anyhow!("无法从文件名解析执行顺序: {}", filename))?;

        // 支持前缀包含非数字字符的情况，如 "01-tables" 或 "01_tables"
        let digits: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Err(anyhow!("执行顺序解析失败: {}", raw));
        }
        digits
            .parse::<u32>()
            .with_context(|| format!("执行顺序解析失败: {}", raw))
    }

    fn parse_sql_statements(&self, content: &str) -> AppResult<Vec<String>> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut in_multiline_comment = false;
        let mut in_trigger_block = false; // 处理 CREATE TRIGGER ... BEGIN ... END;

        for line in content.lines() {
            let trimmed = line.trim();

            // 跳过空行
            if trimmed.is_empty() {
                continue;
            }

            // 处理多行注释
            if trimmed.starts_with("/*") {
                in_multiline_comment = true;
                continue;
            }
            if trimmed.ends_with("*/") {
                in_multiline_comment = false;
                continue;
            }
            if in_multiline_comment {
                continue;
            }

            // 跳过单行注释
            if trimmed.starts_with("--") {
                continue;
            }

            // 进入触发器块检测：以 CREATE TRIGGER 开头（不区分大小写）
            if !in_trigger_block {
                let upper = trimmed.to_uppercase();
                if upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE TEMP TRIGGER") {
                    in_trigger_block = true;
                }
            }

            // 追加本行到当前语句
            if !current_statement.is_empty() {
                current_statement.push(' ');
            }
            current_statement.push_str(trimmed);

            // 如果在触发器块中，仅当遇到 END; 才结束一个完整语句
            if in_trigger_block {
                let upper = trimmed.to_uppercase();
                if upper.ends_with("END;") || upper == "END;" {
                    // 去掉末尾分号
                    if current_statement.ends_with(';') {
                        current_statement.pop();
                    }
                    let statement = current_statement.trim().to_string();
                    if !statement.is_empty() {
                        statements.push(statement);
                    }
                    current_statement.clear();
                    in_trigger_block = false;
                }
                continue; // 触发器块内不按普通分号拆分
            }

            // 普通语句：以分号结尾即结束
            if trimmed.ends_with(';') {
                // 移除末尾的分号
                if current_statement.ends_with(';') {
                    current_statement.pop();
                }

                let statement = current_statement.trim().to_string();
                if !statement.is_empty() {
                    statements.push(statement);
                }
                current_statement.clear();
            }
        }

        // 处理最后一个语句（如果没有分号结尾）
        let final_statement = current_statement.trim();
        if !final_statement.is_empty() {
            statements.push(final_statement.to_string());
        }

        debug!("解析出 {} 条SQL语句", statements.len());
        Ok(statements)
    }

    pub async fn load_script_by_order(&self, order: u32) -> AppResult<Option<SqlScript>> {
        let scripts = self.load_all_scripts().await?;
        Ok(scripts.into_iter().find(|s| s.order == order))
    }

    pub async fn get_max_order(&self) -> AppResult<u32> {
        let scripts = self.load_all_scripts().await?;
        Ok(scripts.iter().map(|s| s.order).max().unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_sql_file(dir: &Path, filename: &str, content: &str) -> AppResult<()> {
        let file_path = dir.join(filename);
        fs::write(file_path, content).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_sql_statement_parsing() {
        let loader = SqlScriptLoader::new(".");

        let content = r#"
            -- 这是注释
            CREATE TABLE test (
                id INTEGER PRIMARY KEY
            );
            
            /* 多行注释
               继续注释 */
            INSERT INTO test VALUES (1);
            
            -- 另一个注释
            SELECT * FROM test
        "#;

        let statements = loader.parse_sql_statements(content).unwrap();
        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE test"));
        assert!(statements[1].contains("INSERT INTO test"));
        assert!(statements[2].contains("SELECT * FROM test"));
    }

    #[tokio::test]
    async fn test_order_parsing() {
        let loader = SqlScriptLoader::new(".");

        assert_eq!(loader.parse_order_from_filename("01_tables").unwrap(), 1);
        assert_eq!(loader.parse_order_from_filename("42_indexes").unwrap(), 42);
        assert_eq!(loader.parse_order_from_filename("01-tables").unwrap(), 1);
    }

    #[tokio::test]
    async fn test_load_sql_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let sql_dir = temp_dir.path().join("sql");
        fs::create_dir_all(&sql_dir).await.unwrap();

        // 创建测试SQL文件
        create_test_sql_file(
            &sql_dir,
            "01_tables.sql",
            "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        )
        .await
        .unwrap();

        create_test_sql_file(
            &sql_dir,
            "02_indexes.sql",
            "CREATE INDEX idx_users_id ON users(id);",
        )
        .await
        .unwrap();

        let loader = SqlScriptLoader::new(&sql_dir);
        let scripts = loader.load_all_scripts().await.unwrap();

        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].order, 1);
        assert_eq!(scripts[1].order, 2);
        assert_eq!(scripts[0].statements.len(), 1);
        assert_eq!(scripts[1].statements.len(), 1);
    }
}
