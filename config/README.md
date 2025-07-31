# TermX 配置系统

<div align="center">

**完整的配置管理和主题系统**

支持 TOML 格式的分层配置，提供丰富的自定义选项

</div>

---

## 📁 目录结构

```
config/
├── 📄 config.toml              # 主配置文件
├── 📁 themes/                  # 主题配置目录
│   ├── 🌙 dark.toml            # 深色主题
│   ├── ☀️ light.toml           # 浅色主题
│   ├── 🧛 dracula.toml         # Dracula 主题
│   ├── 🟫 gruvbox-dark.toml    # Gruvbox 深色主题
│   ├── 🎨 monokai.toml         # Monokai 主题
│   ├── ❄️ nord.toml            # Nord 主题
│   ├── 🌃 one-dark.toml        # One Dark 主题
│   ├── 🌅 solarized-dark.toml  # Solarized 深色主题
│   ├── 🌞 solarized-light.toml # Solarized 浅色主题
│   ├── 🌃 tokyo-night.toml     # Tokyo Night 主题
│   └── 📋 index.toml           # 主题索引文件 (自动生成，无需手动编辑)
├── 📄 README.md                # 本说明文件
└── 📁 examples/                # 配置示例目录
    ├── 📄 minimal.toml         # 最小配置示例
    ├── 📄 developer.toml       # 开发者配置示例
    └── 📄 power-user.toml      # 高级用户配置示例
```

## 🚀 快速开始

### 配置文件位置

TermX 会按以下优先级查找配置文件：

1. **命令行指定**: `--config /path/to/config.toml`
2. **环境变量**: `TERMX_CONFIG_DIR`
3. **用户配置目录**:
   - **Windows**: `%APPDATA%\termx\config.toml`
   - **macOS**: `~/.config/termx/config.toml`
   - **Linux**: `~/.config/termx/config.toml`
4. **系统配置目录**: `/etc/termx/config.toml` (仅 Linux/macOS)

### 首次配置

```bash
# 创建配置目录
mkdir -p ~/.config/termx

# 复制默认配置
cp config/config.toml ~/.config/termx/

# 复制主题文件
cp -r config/themes ~/.config/termx/

# 编辑配置文件
nano ~/.config/termx/config.toml
```

## ⚙️ 配置文件详解

### 📋 主配置文件 (config.toml)

主配置文件采用 TOML 格式，包含应用的所有设置。配置分为以下几个主要部分：

#### 🔧 应用基础设置 `[app]`

```toml
[app]
language = "zh-CN"              # 界面语言: zh-CN, en-US, ja-JP
confirm_on_exit = true          # 退出时显示确认对话框
startup_behavior = "restore"    # 启动行为: restore, new, last
auto_update = true              # 自动检查更新
telemetry_enabled = false       # 遥测数据收集 (匿名)
```

**可选值说明**:

- `language`: 支持的界面语言
- `startup_behavior`:
  - `restore`: 恢复上次会话
  - `new`: 创建新会话
  - `last`: 打开最后一个会话

#### 🎨 外观设置 `[appearance]`

```toml
[appearance]
ui_scale = 100                  # UI 缩放比例 (50-200)
animations_enabled = true       # 启用界面动画
blur_background = false         # 背景模糊效果
show_tab_bar = true            # 显示标签栏
show_title_bar = true          # 显示标题栏

# 主题配置
[appearance.theme_config]
auto_switch_time = "18:00"      # 自动切换深色主题时间
terminal_theme = "tokyo-night"  # 当前终端主题
light_theme = "light"           # 浅色模式主题
dark_theme = "dark"             # 深色模式主题
follow_system = false           # 跟随系统主题

# 字体配置
[appearance.font]
family = "JetBrains Mono, Menlo, Monaco, 'Courier New', monospace"
size = 14.0                     # 字体大小 (8.0-72.0)
weight = "normal"               # 字体粗细: normal, bold, light
style = "normal"                # 字体样式: normal, italic
line_height = 1.2               # 行高倍数 (0.8-3.0)
letter_spacing = 0.0            # 字符间距 (-5.0-5.0)
```

#### 🖥️ 终端设置 `[terminal]`

```toml
[terminal]
scrollback = 10000              # 回滚缓冲区行数 (100-100000)
word_separators = " \t\n\"'`()[]{}|"  # 单词分隔符

# Shell 配置
[terminal.shell]
default = "zsh"                 # 默认 Shell: bash, zsh, fish, powershell
args = ["-l"]                   # Shell 启动参数
working_directory = "~"         # 默认工作目录
env = { TERM = "xterm-256color" }  # 环境变量

# 光标配置
[terminal.cursor]
style = "block"                 # 光标样式: block, underline, bar
blink = true                    # 光标闪烁
color = "#ffffff"               # 光标颜色
thickness = 0.15                # 光标粗细 (0.1-1.0)

# 终端行为
[terminal.behavior]
close_on_exit = true            # Shell 退出时关闭标签页
confirm_close = false           # 关闭标签页确认
bell_sound = true               # 响铃声音
visual_bell = false             # 视觉响铃
```

#### 🪟 窗口设置 `[window]`

```toml
[window]
opacity = 1.0                   # 窗口透明度 (0.1-1.0)
always_on_top = false           # 窗口置顶
startup_mode = "windowed"       # 启动模式: windowed, maximized, fullscreen
decorations = true              # 窗口装饰 (标题栏等)

# 窗口尺寸
[window.size]
width = 1200                    # 窗口宽度
height = 800                    # 窗口高度
min_width = 400                 # 最小宽度
min_height = 300                # 最小高度
```

#### 🤖 AI 功能设置 `[ai]`

```toml
[ai]
enabled = true                  # 启用 AI 功能
default_model = "gpt-3.5-turbo" # 默认模型

# AI 模型配置
[[ai.models]]
id = "gpt-3.5-turbo"
name = "GPT-3.5 Turbo"
provider = "openai"
api_key = "your-api-key"
api_url = "https://api.openai.com/v1/chat/completions"
max_tokens = 2048
temperature = 0.7

# AI 功能配置
[ai.features]
completion = true               # 智能补全
explanation = true              # 命令解释
error_analysis = true           # 错误分析
```

#### ⌨️ 快捷键设置 `[shortcuts]`

```toml
# 全局快捷键 (在任何情况下都生效)
[[shortcuts.global]]
key = "c"
modifiers = ["cmd"]             # 修饰键: cmd, ctrl, alt, shift
action = "copy_to_clipboard"

[[shortcuts.global]]
key = "v"
modifiers = ["cmd"]
action = "paste_from_clipboard"

# 终端专用快捷键
[[shortcuts.terminal]]
key = "t"
modifiers = ["cmd"]
action = "new_tab"

[[shortcuts.terminal]]
key = "w"
modifiers = ["cmd"]
action = "close_tab"

# 自定义快捷键
[[shortcuts.custom]]
key = "l"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "ls -la\n" }
```

## 🎨 主题系统

### � 主题加载机制

TermX 的主题系统采用**自动扫描 + 索引缓存**的机制：

1. **自动扫描**: 系统启动时扫描 `themes/` 目录下的所有 `.toml` 文件
2. **生成索引**: 自动创建/更新 `index.toml` 文件作为主题索引缓存
3. **动态加载**: 添加新主题文件后，系统会自动识别并更新索引

**添加新主题的步骤**:

1. 将新的 `.toml` 主题文件放入 `themes/` 目录
2. 重启应用或调用刷新命令
3. 系统自动扫描并更新 `index.toml`

> **注意**: `index.toml` 是自动生成的缓存文件，请勿手动编辑！

### �📋 主题文件结构 (themes/\*.toml)

每个主题文件包含完整的颜色配置：

```toml
# 主题信息
[theme]
name = "Tokyo Night"            # 主题显示名称
theme_type = "dark"             # 主题类型: dark, light
author = "TermX Team"           # 主题作者
version = "1.0.0"               # 主题版本

# 基础颜色配置
[theme.colors]
foreground = "#a9b1d6"          # 前景色 (文本颜色)
background = "#1a1b26"          # 背景色
cursor = "#c0caf5"              # 光标颜色
selection = "#33467c"           # 选中区域背景色

# ANSI 标准颜色 (0-7)
[theme.colors.ansi]
black = "#15161e"
red = "#f7768e"
green = "#9ece6a"
yellow = "#e0af68"
blue = "#7aa2f7"
magenta = "#bb9af7"
cyan = "#7dcfff"
white = "#a9b1d6"

# ANSI 明亮颜色 (8-15)
[theme.colors.bright]
black = "#414868"
red = "#f7768e"
green = "#9ece6a"
yellow = "#e0af68"
blue = "#7aa2f7"
magenta = "#bb9af7"
cyan = "#7dcfff"
white = "#c0caf5"

# 语法高亮颜色
[theme.syntax]
comment = "#565f89"             # 注释
string = "#9ece6a"              # 字符串
number = "#ff9e64"              # 数字
keyword = "#bb9af7"             # 关键字
function = "#7aa2f7"            # 函数名
variable = "#c0caf5"            # 变量名
type_name = "#2ac3de"           # 类型名
operator = "#89ddff"            # 操作符

# UI 界面颜色
[theme.ui]
primary = "#7aa2f7"             # 主色调
secondary = "#bb9af7"           # 次要色调
success = "#9ece6a"             # 成功状态
warning = "#e0af68"             # 警告状态
error = "#f7768e"               # 错误状态
info = "#7dcfff"                # 信息状态
border = "#29a4bd"              # 边框颜色
divider = "#414868"             # 分割线颜色
```

### 🎨 内置主题预览

| 主题名称            | 类型  | 特色           | 适用场景   |
| ------------------- | ----- | -------------- | ---------- |
| **Tokyo Night**     | Dark  | 现代紫蓝色调   | 夜间编程   |
| **One Dark**        | Dark  | 经典深色主题   | 通用开发   |
| **Dracula**         | Dark  | 高对比度紫色   | 长时间使用 |
| **Nord**            | Dark  | 冷色调极简     | 专注工作   |
| **Gruvbox Dark**    | Dark  | 复古暖色调     | 护眼舒适   |
| **Solarized Dark**  | Dark  | 科学配色       | 减少眼疲劳 |
| **Solarized Light** | Light | 浅色科学配色   | 白天使用   |
| **Light**           | Light | 简洁浅色       | 明亮环境   |
| **Monokai**         | Dark  | 经典编辑器主题 | 代码编辑   |

## 🔧 配置管理

### 📝 修改配置

#### 1. 定位配置文件

```bash
# 查看当前配置文件位置
termx --config-path

# 或手动定位
# macOS/Linux: ~/.config/termx/config.toml
# Windows: %APPDATA%\termx\config.toml
```

#### 2. 编辑配置文件

```bash
# 使用默认编辑器
termx --edit-config

# 或手动编辑
nano ~/.config/termx/config.toml
code ~/.config/termx/config.toml
```

#### 3. 验证配置

```bash
# 验证配置文件语法
termx --validate-config

# 查看当前配置
termx --show-config
```

#### 4. 重新加载配置

- **热重载**: `Cmd/Ctrl + Shift + R`
- **重启应用**: 完全重启以应用所有更改

### 🎨 自定义主题

#### 创建新主题

```bash
# 1. 复制现有主题作为模板
cp ~/.config/termx/themes/tokyo-night.toml ~/.config/termx/themes/my-theme.toml

# 2. 编辑主题文件
nano ~/.config/termx/themes/my-theme.toml
```

#### 主题配置示例

```toml
[theme]
name = "My Custom Theme"
theme_type = "dark"
author = "Your Name"
version = "1.0.0"
description = "My personal terminal theme"

[theme.colors]
foreground = "#e1e1e1"
background = "#1e1e1e"
cursor = "#ffcc00"
selection = "#264f78"

# 自定义 ANSI 颜色
[theme.colors.ansi]
black = "#000000"
red = "#cd3131"
green = "#0dbc79"
yellow = "#e5e510"
blue = "#2472c8"
magenta = "#bc3fbc"
cyan = "#11a8cd"
white = "#e5e5e5"
```

#### 应用自定义主题

```toml
# 在 config.toml 中设置
[appearance.theme_config]
terminal_theme = "my-theme"
```

### ⚙️ 配置示例

#### 最小配置 (minimal.toml)

```toml
# 最简配置，适合快速开始
[app]
language = "zh-CN"

[appearance]
ui_scale = 100

[appearance.theme_config]
terminal_theme = "dark"

[terminal]
scrollback = 1000

[terminal.shell]
default = "zsh"
```

#### 开发者配置 (developer.toml)

```toml
# 开发者优化配置
[app]
language = "zh-CN"
startup_behavior = "restore"

[appearance]
ui_scale = 110
animations_enabled = true

[appearance.theme_config]
terminal_theme = "one-dark"
follow_system = true

[appearance.font]
family = "JetBrains Mono, Fira Code, monospace"
size = 13.0
line_height = 1.3

[terminal]
scrollback = 50000
word_separators = " \t\n\"'`()[]{}|"

[terminal.shell]
default = "zsh"
args = ["-l"]
env = { TERM = "xterm-256color", COLORTERM = "truecolor" }

# 开发者快捷键
[[shortcuts.custom]]
key = "g"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "git status\n" }

[[shortcuts.custom]]
key = "l"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "ls -la\n" }
```

#### 高级用户配置 (power-user.toml)

```toml
# 高级功能完整配置
[app]
language = "zh-CN"
startup_behavior = "restore"
auto_update = true

[appearance]
ui_scale = 125
animations_enabled = true
blur_background = true

[appearance.theme_config]
terminal_theme = "tokyo-night"
auto_switch_time = "18:00"
follow_system = false

[appearance.font]
family = "JetBrains Mono, SF Mono, Menlo, monospace"
size = 14.0
weight = "normal"
line_height = 1.2
letter_spacing = 0.5

[terminal]
scrollback = 100000

[terminal.cursor]
style = "block"
blink = true
color = "#ffcc00"

[window]
opacity = 0.95
always_on_top = false
startup_mode = "maximized"

# AI 功能配置
[ai]
enabled = true
default_model = "gpt-4"

[[ai.models]]
id = "gpt-4"
name = "GPT-4"
provider = "openai"
api_key = "your-api-key"
max_tokens = 4096
temperature = 0.7

# 丰富的快捷键配置
[[shortcuts.custom]]
key = "d"
modifiers = ["cmd", "alt"]
action = { type = "send_text", text = "docker ps\n" }

[[shortcuts.custom]]
key = "k"
modifiers = ["cmd", "alt"]
action = { type = "send_text", text = "kubectl get pods\n" }
```

## � 最佳实践

### 🔒 配置安全

```bash
# 1. 备份配置文件
cp ~/.config/termx/config.toml ~/.config/termx/config.toml.backup

# 2. 设置适当的文件权限
chmod 600 ~/.config/termx/config.toml

# 3. 敏感信息使用环境变量
export OPENAI_API_KEY="your-api-key"
```

### 🎯 性能优化

```toml
# 优化配置以提升性能
[terminal]
scrollback = 10000              # 适中的缓冲区大小
word_separators = " \t\n"       # 简化分隔符

[appearance]
animations_enabled = false      # 低配置设备可关闭动画
blur_background = false         # 关闭模糊效果节省资源

[window]
opacity = 1.0                   # 完全不透明以提升性能
```

### 🔧 开发环境配置

```toml
# 针对开发工作的优化配置
[terminal.shell]
env = {
    TERM = "xterm-256color",
    COLORTERM = "truecolor",
    EDITOR = "code",
    PAGER = "less -R"
}

# 开发者常用快捷键
[[shortcuts.custom]]
key = "r"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "npm run dev\n" }
```

### 🎨 主题设计原则

1. **对比度**: 确保文本和背景有足够对比度
2. **一致性**: 保持颜色方案的一致性
3. **可读性**: 优先考虑长时间使用的舒适度
4. **兼容性**: 测试在不同屏幕和环境下的效果

## 🔧 故障排除

### 常见问题

#### 1. 配置文件不生效

**症状**: 修改配置后没有变化

**解决方案**:

```bash
# 检查配置文件语法
termx --validate-config

# 查看配置文件位置
termx --config-path

# 强制重新加载
termx --reload-config
```

#### 2. 主题显示异常

**症状**: 主题颜色不正确或缺失

**解决方案**:

```bash
# 检查主题文件是否存在
ls ~/.config/termx/themes/

# 验证主题文件语法
termx --validate-theme tokyo-night

# 重置为默认主题
termx --reset-theme
```

#### 3. 字体显示问题

**症状**: 字体不显示或显示异常

**解决方案**:

```toml
# 使用系统默认等宽字体
[appearance.font]
family = "monospace"

# 或指定多个备选字体
family = "JetBrains Mono, Menlo, Consolas, monospace"
```

#### 4. 快捷键冲突

**症状**: 快捷键不响应或与系统冲突

**解决方案**:

```bash
# 查看当前快捷键配置
termx --list-shortcuts

# 重置快捷键配置
termx --reset-shortcuts

# 检查系统快捷键冲突
# macOS: 系统偏好设置 > 键盘 > 快捷键
# Windows: 设置 > 系统 > 关于 > 高级系统设置
```

#### 5. 性能问题

**症状**: 应用运行缓慢或卡顿

**解决方案**:

```toml
# 性能优化配置
[terminal]
scrollback = 1000               # 减少缓冲区大小

[appearance]
animations_enabled = false      # 关闭动画
blur_background = false         # 关闭背景模糊

[window]
opacity = 1.0                   # 关闭透明度
```

### 🔍 调试工具

#### 启用调试模式

```bash
# 启动调试模式
termx --debug

# 查看详细日志
termx --log-level debug

# 生成诊断报告
termx --diagnose > termx-debug.log
```

#### 日志文件位置

- **macOS**: `~/Library/Logs/termx/`
- **Windows**: `%APPDATA%\termx\logs\`
- **Linux**: `~/.local/share/termx/logs/`

### 🆘 获取帮助

如果遇到无法解决的问题：

1. **查看文档**: [完整文档](../docs/)
2. **搜索问题**: [GitHub Issues](https://github.com/Skywang16/TermX/issues)
3. **社区讨论**: [GitHub Discussions](https://github.com/Skywang16/TermX/discussions)
4. **提交问题**: 使用 issue 模板提供详细信息

## 🚀 高级功能

### 环境变量配置

```bash
# 配置目录
export TERMX_CONFIG_DIR="$HOME/.config/termx"

# 默认主题
export TERMX_THEME="tokyo-night"

# 调试模式
export TERMX_DEBUG=1

# API 密钥 (推荐方式)
export OPENAI_API_KEY="your-api-key"
export CLAUDE_API_KEY="your-claude-key"
```

### 命令行参数

```bash
# 指定配置文件
termx --config /path/to/config.toml

# 指定主题
termx --theme tokyo-night

# 无配置模式 (使用默认设置)
termx --no-config

# 安全模式 (禁用插件和自定义配置)
termx --safe-mode

# 性能分析模式
termx --profile
```

### 配置同步

```bash
# 导出配置
termx --export-config > my-termx-config.json

# 导入配置
termx --import-config my-termx-config.json

# 同步到云端 (需要配置)
termx --sync-config
```

## 🔗 相关资源

- 📚 [TOML 语法参考](https://toml.io/cn/)
- 🎨 [颜色选择器](https://htmlcolorcodes.com/)
- 🖼️ [主题设计指南](../docs/THEME_DESIGN.md)
- ⌨️ [快捷键参考](../docs/SHORTCUTS.md)
- 🔧 [API 文档](../docs/API.md)
- 🐛 [故障排除指南](../docs/TROUBLESHOOTING.md)

---

<div align="center">

**需要帮助？** [创建 Issue](https://github.com/Skywang16/TermX/issues) | [加入讨论](https://github.com/Skywang16/TermX/discussions)

</div>
