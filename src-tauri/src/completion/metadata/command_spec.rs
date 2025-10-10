//! 命令规范定义
//!
//! 定义命令的元数据，用于替代硬编码的命令列表

use serde::{Deserialize, Serialize};

/// 命令规范
///
/// 描述一个命令的行为特征，用于智能补全
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    /// 命令名称
    pub name: String,

    /// 是否接受文件作为参数
    pub accepts_files: bool,

    /// 是否接受目录作为参数
    pub accepts_directories: bool,

    /// 接受文件的选项列表（如 --file, -f）
    pub file_options: Vec<String>,

    /// 接受目录的选项列表（如 --dir, -d）
    pub directory_options: Vec<String>,

    /// 命令描述
    pub description: Option<String>,
}

impl CommandSpec {
    /// 创建新的命令规范
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            accepts_files: false,
            accepts_directories: false,
            file_options: Vec::new(),
            directory_options: Vec::new(),
            description: None,
        }
    }

    /// 设置接受文件
    pub fn with_files(mut self) -> Self {
        self.accepts_files = true;
        self
    }

    /// 设置接受目录
    pub fn with_directories(mut self) -> Self {
        self.accepts_directories = true;
        self
    }

    /// 添加文件选项
    pub fn with_file_option(mut self, option: impl Into<String>) -> Self {
        self.file_options.push(option.into());
        self
    }

    /// 添加目录选项
    pub fn with_directory_option(mut self, option: impl Into<String>) -> Self {
        self.directory_options.push(option.into());
        self
    }

    /// 批量添加文件选项
    pub fn with_file_options(mut self, options: Vec<String>) -> Self {
        self.file_options.extend(options);
        self
    }

    /// 批量添加目录选项
    pub fn with_directory_options(mut self, options: Vec<String>) -> Self {
        self.directory_options.extend(options);
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 检查选项是否接受文件
    pub fn is_file_option(&self, option: &str) -> bool {
        self.file_options.iter().any(|opt| opt == option)
    }

    /// 检查选项是否接受目录
    pub fn is_directory_option(&self, option: &str) -> bool {
        self.directory_options.iter().any(|opt| opt == option)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_spec_builder() {
        let spec = CommandSpec::new("test")
            .with_files()
            .with_file_option("--file")
            .with_file_option("-f")
            .with_directory_option("--dir")
            .with_description("Test command");

        assert_eq!(spec.name, "test");
        assert!(spec.accepts_files);
        assert!(!spec.accepts_directories);
        assert_eq!(spec.file_options.len(), 2);
        assert_eq!(spec.directory_options.len(), 1);
        assert!(spec.description.is_some());
    }

    #[test]
    fn test_is_file_option() {
        let spec = CommandSpec::new("test")
            .with_file_options(vec!["--file".to_string(), "-f".to_string()]);

        assert!(spec.is_file_option("--file"));
        assert!(spec.is_file_option("-f"));
        assert!(!spec.is_file_option("--dir"));
    }

    #[test]
    fn test_is_directory_option() {
        let spec = CommandSpec::new("test")
            .with_directory_options(vec!["--dir".to_string(), "-d".to_string()]);

        assert!(spec.is_directory_option("--dir"));
        assert!(spec.is_directory_option("-d"));
        assert!(!spec.is_directory_option("--file"));
    }
}
