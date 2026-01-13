//! 命令注册表
//!
//! 管理所有已知命令的元数据

use super::command_spec::CommandSpec;
use crate::completion::CompletionRuntime;
use std::collections::HashMap;

/// 命令注册表
pub struct CommandRegistry {
    commands: HashMap<String, CommandSpec>,
}

impl CommandRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// 获取全局注册表实例
    pub fn global() -> &'static CommandRegistry {
        CompletionRuntime::global().registry()
    }

    /// 注册命令
    pub fn register(&mut self, spec: CommandSpec) {
        self.commands.insert(spec.name.clone(), spec);
    }

    /// 批量注册命令
    pub fn register_all(&mut self, specs: Vec<CommandSpec>) {
        for spec in specs {
            self.register(spec);
        }
    }

    /// 查找命令规范
    pub fn lookup(&self, command: &str) -> Option<&CommandSpec> {
        self.commands.get(command)
    }

    /// 检查命令是否接受文件
    pub fn accepts_files(&self, command: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.accepts_files)
            .unwrap_or(false)
    }

    /// 检查命令是否接受目录
    pub fn accepts_directories(&self, command: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.accepts_directories)
            .unwrap_or(false)
    }

    /// 检查选项是否接受文件
    pub fn is_file_option(&self, command: &str, option: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.is_file_option(option))
            .unwrap_or_else(|| Self::is_common_file_option(option))
    }

    /// 检查选项是否接受目录
    pub fn is_directory_option(&self, command: &str, option: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.is_directory_option(option))
            .unwrap_or_else(|| Self::is_common_directory_option(option))
    }

    /// 通用的文件选项（后备检查）
    fn is_common_file_option(option: &str) -> bool {
        matches!(
            option,
            "--file" | "--input" | "--output" | "--config" | "--script" | "-f" | "-i" | "-o" | "-c"
        )
    }

    /// 通用的目录选项（后备检查）
    fn is_common_directory_option(option: &str) -> bool {
        matches!(
            option,
            "--directory" | "--dir" | "--path" | "--workdir" | "-d" | "-p"
        )
    }

    /// 加载内置命令
    pub(crate) fn load_builtin_commands(&mut self) {
        let builtin_commands = super::builtin::load_builtin_commands();
        self.register_all(builtin_commands);
    }

    /// 获取所有已注册的命令数量
    pub fn count(&self) -> usize {
        self.commands.len()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = CommandRegistry::new();

        let spec = CommandSpec::new("test")
            .with_files()
            .with_file_option("--file");

        registry.register(spec);

        assert!(registry.lookup("test").is_some());
        assert!(registry.lookup("nonexistent").is_none());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_accepts_files() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandSpec::new("cat").with_files());

        assert!(registry.accepts_files("cat"));
        assert!(!registry.accepts_files("ls"));
    }

    #[test]
    fn test_is_file_option() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandSpec::new("test").with_file_option("--file"));

        assert!(registry.is_file_option("test", "--file"));
        assert!(!registry.is_file_option("test", "--dir"));

        // 测试通用后备选项
        assert!(registry.is_file_option("unknown", "--input"));
    }

    #[test]
    fn test_global_registry() {
        let registry = CommandRegistry::global();

        // 应该已经加载了内置命令
        assert!(registry.count() > 0);

        // 测试一些常见命令
        assert!(registry.accepts_files("cat"));
        assert!(registry.accepts_files("vim"));
    }
}
