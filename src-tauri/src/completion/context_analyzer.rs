//! 智能命令行上下文分析器
//!
//! 基于优秀的补全系统（如zsh、fish、carapace等）的设计原理，
//! 实现智能的上下文感知补全分析

use std::collections::HashMap;

/// 补全位置类型
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionPosition {
    /// 命令名位置
    Command,
    /// 命令选项位置（如 -h, --help）
    Option,
    /// 选项的值位置（如 --file <文件名>）
    OptionValue { option: String },
    /// 子命令位置
    Subcommand { parent: String },
    /// 命令参数位置
    Argument { command: String, position: usize },
    /// 文件路径位置
    FilePath,
    /// 未知位置
    Unknown,
}

/// 命令元数据
#[derive(Debug, Clone)]
pub struct CommandMeta {
    /// 命令名
    pub name: String,
    /// 子命令列表
    pub subcommands: Vec<String>,
    /// 选项列表
    pub options: Vec<CommandOption>,
    /// 是否需要文件参数
    pub takes_files: bool,
    /// 参数类型列表
    pub arg_types: Vec<ArgType>,
}

/// 命令选项
#[derive(Debug, Clone)]
pub struct CommandOption {
    /// 短选项（如 -h）
    pub short: Option<String>,
    /// 长选项（如 --help）
    pub long: Option<String>,
    /// 是否需要值
    pub takes_value: bool,
    /// 值的类型
    pub value_type: Option<ArgType>,
    /// 描述
    pub description: String,
}

/// 参数类型
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// 任意字符串
    String,
    /// 文件路径
    File,
    /// 目录路径
    Directory,
    /// 数字
    Number,
    /// URL
    Url,
    /// 枚举值
    Enum(Vec<String>),
}

/// 智能上下文分析器
pub struct ContextAnalyzer {
    /// 内置命令知识库
    command_db: HashMap<String, CommandMeta>,
}

impl ContextAnalyzer {
    /// 创建新的上下文分析器
    pub fn new() -> Self {
        let mut analyzer = Self {
            command_db: HashMap::new(),
        };
        analyzer.load_builtin_commands();
        analyzer
    }

    /// 分析命令行上下文
    pub fn analyze(&self, input: &str, cursor_pos: usize) -> CompletionContext {
        let tokens = self.tokenize(input);
        let current_token_index = self.find_current_token_index(&tokens, cursor_pos);

        let position = self.determine_position(&tokens, current_token_index);
        let current_word = self.extract_current_word(input, cursor_pos);

        CompletionContext {
            input: input.to_string(),
            cursor_position: cursor_pos,
            tokens,
            current_token_index,
            current_word,
            position,
        }
    }

    /// 分词
    fn tokenize(&self, input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut start_pos = 0;

        for (i, ch) in input.char_indices() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(Token {
                            text: current.clone(),
                            start: start_pos,
                            end: i,
                        });
                        current.clear();
                    }
                    in_quotes = true;
                    quote_char = ch;
                    start_pos = i;
                }
                ch if ch == quote_char && in_quotes => {
                    current.push(ch);
                    tokens.push(Token {
                        text: current.clone(),
                        start: start_pos,
                        end: i + 1,
                    });
                    current.clear();
                    in_quotes = false;
                    start_pos = i + 1;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(Token {
                            text: current.clone(),
                            start: start_pos,
                            end: i,
                        });
                        current.clear();
                    }
                    start_pos = i + 1;
                    while start_pos < input.len()
                        && input.chars().nth(start_pos).unwrap().is_whitespace()
                    {
                        start_pos += 1;
                    }
                }
                ch => {
                    if current.is_empty() && !ch.is_whitespace() {
                        start_pos = i;
                    }
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(Token {
                text: current,
                start: start_pos,
                end: input.len(),
            });
        }

        tokens
    }

    /// 查找当前token索引
    fn find_current_token_index(&self, tokens: &[Token], cursor_pos: usize) -> Option<usize> {
        for (i, token) in tokens.iter().enumerate() {
            if cursor_pos >= token.start && cursor_pos <= token.end {
                return Some(i);
            }
        }

        if let Some(last_token) = tokens.last() {
            if cursor_pos > last_token.end {
                return Some(tokens.len());
            }
        }

        None
    }

    /// 确定补全位置类型
    fn determine_position(
        &self,
        tokens: &[Token],
        current_index: Option<usize>,
    ) -> CompletionPosition {
        if tokens.is_empty() {
            return CompletionPosition::Command;
        }

        let current_index = current_index.unwrap_or(tokens.len());

        if current_index == 0 || (current_index == 1 && tokens.len() == 1) {
            return CompletionPosition::Command;
        }

        let command_name = &tokens[0].text;

        if let Some(cmd_meta) = self.command_db.get(command_name) {
            return self.analyze_with_metadata(tokens, current_index, cmd_meta);
        }

        // 基于启发式规则分析
        self.analyze_heuristic(tokens, current_index)
    }

    /// 基于命令元数据分析
    fn analyze_with_metadata(
        &self,
        tokens: &[Token],
        current_index: usize,
        meta: &CommandMeta,
    ) -> CompletionPosition {
        if current_index >= tokens.len() {
            // 在最后位置，检查前一个token
            if let Some(prev_token) = tokens.get(current_index - 1) {
                for option in &meta.options {
                    if option.takes_value {
                        if let Some(long) = &option.long {
                            if prev_token.text == *long {
                                return CompletionPosition::OptionValue {
                                    option: prev_token.text.clone(),
                                };
                            }
                        }
                        if let Some(short) = &option.short {
                            if prev_token.text == *short {
                                return CompletionPosition::OptionValue {
                                    option: prev_token.text.clone(),
                                };
                            }
                        }
                    }
                }
            }
        }

        let current_token = tokens.get(current_index);

        if let Some(token) = current_token {
            if token.text.starts_with('-') {
                return CompletionPosition::Option;
            }
        }

        if !meta.subcommands.is_empty() {
            let non_option_args: Vec<&Token> = tokens[1..]
                .iter()
                .filter(|t| !t.text.starts_with('-'))
                .collect();

            if non_option_args.is_empty() {
                return CompletionPosition::Subcommand {
                    parent: meta.name.clone(),
                };
            }
        }

        // 默认为参数位置
        let arg_position = tokens[1..current_index]
            .iter()
            .filter(|t| !t.text.starts_with('-'))
            .count();

        CompletionPosition::Argument {
            command: meta.name.clone(),
            position: arg_position,
        }
    }

    /// 基于启发式规则分析
    fn analyze_heuristic(&self, tokens: &[Token], current_index: usize) -> CompletionPosition {
        let current_token = tokens.get(current_index);

        if let Some(token) = current_token {
            if token.text.starts_with('-') {
                return CompletionPosition::Option;
            }
        }

        if current_index > 0 {
            if let Some(prev_token) = tokens.get(current_index - 1) {
                if self.is_option_that_takes_value(&prev_token.text) {
                    return CompletionPosition::OptionValue {
                        option: prev_token.text.clone(),
                    };
                }
            }
        }

        if let Some(token) = current_token {
            if self.looks_like_path(&token.text) {
                return CompletionPosition::FilePath;
            }
        }

        // 默认为参数
        let command_name = &tokens[0].text;
        let arg_position = tokens[1..current_index]
            .iter()
            .filter(|t| !t.text.starts_with('-'))
            .count();

        CompletionPosition::Argument {
            command: command_name.clone(),
            position: arg_position,
        }
    }

    /// 提取当前单词
    fn extract_current_word(&self, input: &str, cursor_pos: usize) -> String {
        if input.is_empty() || cursor_pos == 0 {
            return String::new();
        }

        let chars: Vec<char> = input.chars().collect();
        let mut start = cursor_pos.min(chars.len());
        let mut end = cursor_pos.min(chars.len());

        // 向前查找单词开始
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }

        // 向后查找单词结束
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    /// 检查是否是需要值的选项
    fn is_option_that_takes_value(&self, option: &str) -> bool {
        // 常见的需要值的选项模式
        matches!(
            option,
            "--file"
                | "--output"
                | "--input"
                | "--config"
                | "--directory"
                | "--format"
                | "--type"
                | "--name"
                | "--path"
                | "--url"
                | "-f"
                | "-o"
                | "-i"
                | "-c"
                | "-d"
                | "-t"
                | "-n"
                | "-p"
        )
    }

    /// 检查是否看起来像路径
    fn looks_like_path(&self, text: &str) -> bool {
        text.contains('/') || text.contains('\\') || text.starts_with('.') || text.starts_with('~')
    }

    /// 加载内置命令知识库
    fn load_builtin_commands(&mut self) {
        // Git 命令
        self.command_db.insert(
            "git".to_string(),
            CommandMeta {
                name: "git".to_string(),
                subcommands: vec![
                    "add".to_string(),
                    "commit".to_string(),
                    "push".to_string(),
                    "pull".to_string(),
                    "status".to_string(),
                    "branch".to_string(),
                    "checkout".to_string(),
                    "merge".to_string(),
                    "log".to_string(),
                    "diff".to_string(),
                    "clone".to_string(),
                    "init".to_string(),
                    "fetch".to_string(),
                    "reset".to_string(),
                    "rebase".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--help".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "显示帮助信息".to_string(),
                    },
                    CommandOption {
                        short: None,
                        long: Some("--version".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "显示版本信息".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::String],
            },
        );

        // Docker 命令
        self.command_db.insert(
            "docker".to_string(),
            CommandMeta {
                name: "docker".to_string(),
                subcommands: vec![
                    "run".to_string(),
                    "build".to_string(),
                    "pull".to_string(),
                    "push".to_string(),
                    "ps".to_string(),
                    "images".to_string(),
                    "stop".to_string(),
                    "start".to_string(),
                    "restart".to_string(),
                    "rm".to_string(),
                    "rmi".to_string(),
                    "exec".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--help".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "显示帮助信息".to_string(),
                    },
                    CommandOption {
                        short: Some("-f".to_string()),
                        long: Some("--file".to_string()),
                        takes_value: true,
                        value_type: Some(ArgType::File),
                        description: "指定Dockerfile".to_string(),
                    },
                ],
                takes_files: false,
                arg_types: vec![ArgType::String],
            },
        );

        // NPM 命令
        self.command_db.insert(
            "npm".to_string(),
            CommandMeta {
                name: "npm".to_string(),
                subcommands: vec![
                    "install".to_string(),
                    "run".to_string(),
                    "start".to_string(),
                    "test".to_string(),
                    "build".to_string(),
                    "publish".to_string(),
                    "init".to_string(),
                    "update".to_string(),
                    "uninstall".to_string(),
                ],
                options: vec![
                    CommandOption {
                        short: Some("-g".to_string()),
                        long: Some("--global".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "全局安装".to_string(),
                    },
                    CommandOption {
                        short: Some("-D".to_string()),
                        long: Some("--save-dev".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "保存为开发依赖".to_string(),
                    },
                ],
                takes_files: false,
                arg_types: vec![ArgType::String],
            },
        );

        // 添加更多常用命令...
        self.add_ls_command();
        self.add_cd_command();
        self.add_mkdir_command();
    }

    /// 添加 ls 命令
    fn add_ls_command(&mut self) {
        self.command_db.insert(
            "ls".to_string(),
            CommandMeta {
                name: "ls".to_string(),
                subcommands: vec![],
                options: vec![
                    CommandOption {
                        short: Some("-l".to_string()),
                        long: Some("--long".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "长格式显示".to_string(),
                    },
                    CommandOption {
                        short: Some("-a".to_string()),
                        long: Some("--all".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "显示所有文件".to_string(),
                    },
                    CommandOption {
                        short: Some("-h".to_string()),
                        long: Some("--human-readable".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "人类可读格式".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::Directory, ArgType::File],
            },
        );
    }

    /// 添加 cd 命令
    fn add_cd_command(&mut self) {
        self.command_db.insert(
            "cd".to_string(),
            CommandMeta {
                name: "cd".to_string(),
                subcommands: vec![],
                options: vec![],
                takes_files: true,
                arg_types: vec![ArgType::Directory],
            },
        );
    }

    /// 添加 mkdir 命令
    fn add_mkdir_command(&mut self) {
        self.command_db.insert(
            "mkdir".to_string(),
            CommandMeta {
                name: "mkdir".to_string(),
                subcommands: vec![],
                options: vec![
                    CommandOption {
                        short: Some("-p".to_string()),
                        long: Some("--parents".to_string()),
                        takes_value: false,
                        value_type: None,
                        description: "创建父目录".to_string(),
                    },
                    CommandOption {
                        short: Some("-m".to_string()),
                        long: Some("--mode".to_string()),
                        takes_value: true,
                        value_type: Some(ArgType::String),
                        description: "设置权限模式".to_string(),
                    },
                ],
                takes_files: true,
                arg_types: vec![ArgType::Directory],
            },
        );
    }

    /// 获取命令元数据
    pub fn get_command_meta(&self, command: &str) -> Option<&CommandMeta> {
        self.command_db.get(command)
    }
}

/// 令牌结构
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// 补全上下文
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub input: String,
    pub cursor_position: usize,
    pub tokens: Vec<Token>,
    pub current_token_index: Option<usize>,
    pub current_word: String,
    pub position: CompletionPosition,
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
