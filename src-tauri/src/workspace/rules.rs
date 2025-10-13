/*!
 * Workspace Rules Management
 *
 * 项目规则文件的查找和管理
 * 从 agent/context/project_context.rs 迁移而来
 */

use super::types::RULES_FILES;
use std::path::PathBuf;

/// 获取指定目录下所有存在的规则文件列表
///
/// 按优先级顺序返回存在的规则文件名
pub fn get_available_rules_files<P: Into<PathBuf>>(project_root: P) -> Vec<String> {
    let root: PathBuf = project_root.into();
    RULES_FILES
        .iter()
        .filter_map(|&filename| {
            let file_path = root.join(filename);
            if file_path.exists() {
                Some(filename.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_available_rules_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 创建一些规则文件
        fs::write(temp_path.join("CLAUDE.md"), "test content").unwrap();
        fs::write(temp_path.join("README.md"), "readme content").unwrap();

        let available = get_available_rules_files(temp_path);

        assert_eq!(available.len(), 2);
        assert!(available.contains(&"CLAUDE.md".to_string()));
        assert!(available.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_preserves_priority_order() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 反向创建文件
        fs::write(temp_path.join("README.md"), "readme").unwrap();
        fs::write(temp_path.join("CLAUDE.md"), "claude").unwrap();

        let available = get_available_rules_files(temp_path);

        // 应该按优先级顺序返回: CLAUDE.md 在前
        assert_eq!(available[0], "CLAUDE.md");
        assert_eq!(available[1], "README.md");
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let available = get_available_rules_files(temp_path);

        assert!(available.is_empty());
    }
}
