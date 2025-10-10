//! 内置命令定义
//!
//! 定义常见命令的元数据，替代硬编码的命令列表

use super::command_spec::CommandSpec;

/// 加载所有内置命令
pub fn load_builtin_commands() -> Vec<CommandSpec> {
    let mut commands = Vec::new();

    // 文件查看/编辑命令
    commands.extend(file_viewer_commands());
    commands.extend(text_editor_commands());

    // 文件操作命令
    commands.extend(file_operation_commands());

    // 文本处理命令
    commands.extend(text_processing_commands());

    // 系统命令
    commands.extend(system_commands());

    commands
}

/// 文件查看命令
fn file_viewer_commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("cat")
            .with_files()
            .with_description("Concatenate and print files"),
        CommandSpec::new("less")
            .with_files()
            .with_description("View file contents with paging"),
        CommandSpec::new("more")
            .with_files()
            .with_description("View file contents page by page"),
        CommandSpec::new("head")
            .with_files()
            .with_description("Output the first part of files"),
        CommandSpec::new("tail")
            .with_files()
            .with_description("Output the last part of files"),
        CommandSpec::new("bat")
            .with_files()
            .with_description("Cat clone with syntax highlighting"),
    ]
}

/// 文本编辑器命令
fn text_editor_commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("vim")
            .with_files()
            .with_description("Vi IMproved text editor"),
        CommandSpec::new("vi")
            .with_files()
            .with_description("Text editor"),
        CommandSpec::new("nvim")
            .with_files()
            .with_description("Neovim text editor"),
        CommandSpec::new("nano")
            .with_files()
            .with_description("Simple text editor"),
        CommandSpec::new("emacs")
            .with_files()
            .with_description("Extensible text editor"),
        CommandSpec::new("code")
            .with_files()
            .with_directories()
            .with_description("Visual Studio Code"),
    ]
}

/// 文件操作命令
fn file_operation_commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("cp")
            .with_files()
            .with_description("Copy files and directories"),
        CommandSpec::new("mv")
            .with_files()
            .with_description("Move or rename files"),
        CommandSpec::new("rm")
            .with_files()
            .with_description("Remove files or directories"),
        CommandSpec::new("chmod")
            .with_files()
            .with_description("Change file permissions"),
        CommandSpec::new("chown")
            .with_files()
            .with_description("Change file owner and group"),
        CommandSpec::new("file")
            .with_files()
            .with_description("Determine file type"),
        CommandSpec::new("ln")
            .with_files()
            .with_description("Create links between files"),
        CommandSpec::new("touch")
            .with_files()
            .with_description("Change file timestamps"),
    ]
}

/// 文本处理命令
fn text_processing_commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("grep")
            .with_files()
            .with_file_option("-f")
            .with_file_option("--file")
            .with_description("Search text patterns in files"),
        CommandSpec::new("awk")
            .with_files()
            .with_file_option("-f")
            .with_description("Pattern scanning and processing"),
        CommandSpec::new("sed")
            .with_files()
            .with_file_option("-f")
            .with_file_option("--file")
            .with_description("Stream editor"),
        CommandSpec::new("wc")
            .with_files()
            .with_description("Word, line, character count"),
        CommandSpec::new("sort")
            .with_files()
            .with_description("Sort lines of text"),
        CommandSpec::new("uniq")
            .with_files()
            .with_description("Report or omit repeated lines"),
        CommandSpec::new("cut")
            .with_files()
            .with_description("Remove sections from lines"),
        CommandSpec::new("tr").with_description("Translate or delete characters"),
        CommandSpec::new("diff")
            .with_files()
            .with_description("Compare files line by line"),
        CommandSpec::new("patch")
            .with_files()
            .with_description("Apply a diff file to an original"),
    ]
}

/// 系统命令
fn system_commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("cd")
            .with_directories()
            .with_description("Change directory"),
        CommandSpec::new("ls")
            .with_files()
            .with_directories()
            .with_description("List directory contents"),
        CommandSpec::new("mkdir")
            .with_directories()
            .with_description("Make directories"),
        CommandSpec::new("rmdir")
            .with_directories()
            .with_description("Remove empty directories"),
        CommandSpec::new("pwd").with_description("Print working directory"),
        CommandSpec::new("find")
            .with_directories()
            .with_file_option("-f")
            .with_description("Search for files in a directory hierarchy"),
        CommandSpec::new("locate")
            .with_files()
            .with_description("Find files by name"),
        CommandSpec::new("which").with_description("Locate a command"),
        CommandSpec::new("tar")
            .with_files()
            .with_file_option("-f")
            .with_file_option("--file")
            .with_description("Archive files"),
        CommandSpec::new("zip")
            .with_files()
            .with_description("Package and compress files"),
        CommandSpec::new("unzip")
            .with_files()
            .with_description("Extract compressed files"),
        CommandSpec::new("gzip")
            .with_files()
            .with_description("Compress files"),
        CommandSpec::new("gunzip")
            .with_files()
            .with_description("Decompress files"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_builtin_commands() {
        let commands = load_builtin_commands();
        assert!(!commands.is_empty(), "Should have builtin commands");

        // 检查常见命令是否存在
        let command_names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
        assert!(command_names.contains(&"cat"));
        assert!(command_names.contains(&"vim"));
        assert!(command_names.contains(&"grep"));
        assert!(command_names.contains(&"cd"));
    }

    #[test]
    fn test_file_accepts_files() {
        let commands = load_builtin_commands();
        let cat = commands.iter().find(|c| c.name == "cat").unwrap();
        assert!(cat.accepts_files);
    }

    #[test]
    fn test_cd_accepts_directories() {
        let commands = load_builtin_commands();
        let cd = commands.iter().find(|c| c.name == "cd").unwrap();
        assert!(cd.accepts_directories);
    }

    #[test]
    fn test_grep_has_file_options() {
        let commands = load_builtin_commands();
        let grep = commands.iter().find(|c| c.name == "grep").unwrap();
        assert!(grep.is_file_option("-f"));
        assert!(grep.is_file_option("--file"));
    }
}
