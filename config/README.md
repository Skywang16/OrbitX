# OrbitX 配置系统

<div align="center">

**终端应用配置管理**

支持 TOML 格式的配置文件和丰富的主题系统

</div>

---

## 📁 目录结构

```
config/
├── 📄 config.toml              # 主配置文件
├── 📁 themes/                  # 主题配置目录
│   ├── dark.toml               # 深色主题
│   ├── light.toml              # 浅色主题
│   ├── dracula.toml            # Dracula 主题
│   ├── gruvbox-dark.toml       # Gruvbox 深色主题
│   ├── monokai.toml            # Monokai 主题
│   ├── nord.toml               # Nord 主题
│   ├── one-dark.toml           # One Dark 主题
│   ├── solarized-dark.toml     # Solarized 深色主题
│   ├── solarized-light.toml    # Solarized 浅色主题
│   ├── tokyo-night.toml        # Tokyo Night 主题
│   └── index.toml              # 主题索引文件 (自动生成)
└── 📄 README.md                # 本说明文件
```

## ⚙️ 配置文件说明

### 主配置文件 (config.toml)

当前的配置文件包含以下主要部分：

```toml
# OrbitX 终端应用配置文件
version = "1.0.0"

# ==================== 应用基础设置 ====================
[app]
language = "zh-CN"              # 界面语言 (zh-CN, en-US)
confirm_on_exit = true          # 退出时是否显示确认对话框
startup_behavior = "restore"    # 启动行为: restore(恢复上次会话), new(新建会话), last(打开最后一个会话)

# ==================== 外观设置 ====================
[appearance]
ui_scale = 100                  # UI 缩放比例 (50-200)
animations_enabled = true       # 是否启用界面动画效果

# 主题配置
[appearance.theme_config]
auto_switch_time = "18:00"      # 自动切换深色主题的时间 (24小时制)
terminal_theme = "solarized-light"  # 当前使用的终端主题名称
light_theme = "light"           # 浅色模式使用的主题
dark_theme = "dark"             # 深色模式使用的主题
follow_system = false           # 是否跟随系统主题自动切换

# 字体配置
[appearance.font]
family = "Menlo, Monaco, \"SF Mono\", \"Microsoft YaHei UI\", \"PingFang SC\", \"Hiragino Sans GB\", \"Source Han Sans CN\", \"WenQuanYi Micro Hei\", \"Courier New\", monospace"
size = 14.0                     # 字体大小 (8.0-72.0)
weight = "normal"               # 字体粗细: normal, bold, light
style = "normal"                # 字体样式: normal, italic
lineHeight = 1.2                # 行高倍数 (0.8-3.0)
letterSpacing = 0.0             # 字符间距 (-5.0-5.0)

# ==================== 终端设置 ====================
[terminal]
scrollback = 1000               # 回滚缓冲区行数 (100-100000)

# Shell 配置
[terminal.shell]
default = "zsh"                 # 默认 Shell: bash, zsh, fish, powershell
args = []                       # Shell 启动参数
working_directory = "~"         # 默认工作目录 (~表示用户主目录)

# 光标配置
[terminal.cursor]
style = "block"                 # 光标样式: block(块状), underline(下划线), bar(竖线)
blink = true                    # 光标是否闪烁
color = "#ffffff"               # 光标颜色 (十六进制颜色值)
thickness = 0.15                # 光标粗细 (0.1-1.0，仅对 underline 和 bar 有效)

# 终端行为
[terminal.behavior]
close_on_exit = true            # Shell 退出时是否自动关闭标签页
confirm_close = false           # 关闭标签页时是否显示确认对话框
```

### 快捷键配置

配置文件中包含三种类型的快捷键：

#### 全局快捷键

在任何情况下都生效的快捷键：

```toml
[[shortcuts.global]]
key = "c"
modifiers = ["cmd"]
action = "copy_to_clipboard"

[[shortcuts.global]]
key = "v"
modifiers = ["cmd"]
action = "paste_from_clipboard"
```

#### 终端快捷键

仅在终端界面生效的快捷键：

```toml
[[shortcuts.terminal]]
key = "t"
modifiers = ["cmd"]
action = "new_tab"

[[shortcuts.terminal]]
key = "w"
modifiers = ["cmd"]
action = "close_tab"
```

#### 自定义快捷键

用户可以自定义的快捷键，支持发送文本：

```toml
[[shortcuts.custom]]
key = "l"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "ls -la\n" }

[[shortcuts.custom]]
key = "g"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "git status\n" }
```

### 主题切换

在配置文件中修改 `terminal_theme` 字段：

```toml
[appearance.theme_config]
terminal_theme = "tokyo-night"  # 改为你想要的主题名称
```

### 跟随系统主题

启用跟随系统主题自动切换：

```toml
[appearance.theme_config]
follow_system = true
light_theme = "solarized-light"
dark_theme = "tokyo-night"
```

## 🔧 配置修改

### 1. 定位配置文件

配置文件位置：

- **macOS**: `~/.config/OrbitX/config.toml`
- **Windows**: `%APPDATA%\OrbitX\config.toml`
- **Linux**: `~/.config/OrbitX/config.toml`

### 2. 编辑配置文件

```bash
# 使用你喜欢的编辑器
nano ~/.config/OrbitX/config.toml
code ~/.config/OrbitX/config.toml
vim ~/.config/OrbitX/config.toml
```

### 3. 重新加载配置

修改配置后重启应用生效。

## 🎯 常用配置调整

### 调整字体

```toml
[appearance.font]
family = "JetBrains Mono, Fira Code, monospace"
size = 16.0
lineHeight = 1.3
```

### 调整终端行为

```toml
[terminal]
scrollback = 5000

[terminal.behavior]
close_on_exit = false
confirm_close = true
```

### 添加自定义快捷键

```toml
[[shortcuts.custom]]
key = "d"
modifiers = ["cmd", "shift"]
action = { type = "send_text", text = "docker ps\n" }
```

---

<div align="center">

**需要帮助？** [创建 Issue](https://github.com/Skywang16/OrbitX/issues) | [查看文档](../docs/)

</div>
