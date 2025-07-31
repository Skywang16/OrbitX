# TermX 配置文件说明

本目录包含 TermX 终端应用的所有配置文件。

## 📁 目录结构

```
config/
├── config.toml          # 主配置文件
├── themes/              # 主题目录
│   ├── dark.toml        # 深色主题
│   ├── light.toml       # 浅色主题
│   ├── dracula.toml     # Dracula 主题
│   ├── gruvbox-dark.toml # Gruvbox 深色主题
│   ├── monokai.toml     # Monokai 主题
│   ├── nord.toml        # Nord 主题
│   ├── one-dark.toml    # One Dark 主题
│   ├── solarized-dark.toml  # Solarized 深色主题
│   ├── solarized-light.toml # Solarized 浅色主题
│   ├── tokyo-night.toml # Tokyo Night 主题
│   └── index.toml       # 主题索引文件
└── README.md            # 本说明文件
```

## ⚙️ 主配置文件 (config.toml)

主配置文件包含应用的所有设置，分为以下几个部分：

### 应用基础设置 `[app]`

- `language`: 界面语言
- `confirm_on_exit`: 退出确认
- `startup_behavior`: 启动行为

### 外观设置 `[appearance]`

- `ui_scale`: UI 缩放比例
- `animations_enabled`: 动画效果
- `theme_config`: 主题配置
- `font`: 字体配置

### 终端设置 `[terminal]`

- `scrollback`: 回滚缓冲区
- `shell`: Shell 配置
- `cursor`: 光标配置
- `behavior`: 终端行为

### 窗口设置 `[window]`

- `opacity`: 窗口透明度
- `always_on_top`: 置顶设置
- `startup_mode`: 启动模式
- `size`: 窗口尺寸

### AI 功能设置 `[ai]`

- `models`: AI 模型配置
- `features.chat`: 聊天功能配置

### 快捷键设置 `[shortcuts]`

- `global`: 全局快捷键
- `terminal`: 终端快捷键
- `custom`: 自定义快捷键

## 🎨 主题文件 (themes/\*.toml)

每个主题文件包含以下配置：

### 主题信息 `[theme]`

- `name`: 主题名称
- `theme_type`: 主题类型 (dark/light)

### 颜色配置 `[theme.colors]`

- `foreground`: 前景色
- `background`: 背景色
- `cursor`: 光标颜色
- `selection`: 选中颜色
- `ansi`: ANSI 标准颜色
- `bright`: ANSI 明亮颜色

### 语法高亮 `[theme.syntax]`

- `comment`: 注释颜色
- `string`: 字符串颜色
- `number`: 数字颜色
- `keyword`: 关键字颜色
- `function`: 函数颜色
- `variable`: 变量颜色
- `type_name`: 类型颜色
- `operator`: 操作符颜色

### UI 颜色 `[theme.ui]`

- `primary`: 主色调
- `secondary`: 次要色调
- `success`: 成功状态色
- `warning`: 警告状态色
- `error`: 错误状态色
- `info`: 信息状态色
- `border`: 边框颜色
- `divider`: 分割线颜色

## 🔧 配置修改

### 修改配置

1. 找到用户配置目录：
   - **macOS**: `~/.config/termx/`
   - **Windows**: `%APPDATA%\termx\`
   - **Linux**: `~/.config/termx/`

2. 编辑配置文件：

   ```bash
   # 编辑主配置
   nano ~/.config/termx/config.toml

   # 编辑主题
   nano ~/.config/termx/themes/dark.toml
   ```

3. 重启应用使配置生效

### 创建自定义主题

1. 复制现有主题文件：

   ```bash
   cp ~/.config/termx/themes/dark.toml ~/.config/termx/themes/my-theme.toml
   ```

2. 修改主题信息：

   ```toml
   [theme]
   name = "my-theme"
   theme_type = "dark"
   ```

3. 自定义颜色配置

4. 在主配置中使用新主题：
   ```toml
   [appearance.theme_config]
   terminal_theme = "my-theme"
   ```

## 📝 注意事项

1. **备份配置**: 修改前建议备份原配置文件
2. **语法检查**: 确保 TOML 语法正确
3. **颜色格式**: 颜色值使用十六进制格式 (如 `#ffffff`)
4. **重启生效**: 大部分配置修改需要重启应用
5. **权限问题**: 确保对配置目录有读写权限

## 🚀 高级配置

### 环境变量

可以通过环境变量覆盖部分配置：

- `TERMX_CONFIG_DIR`: 自定义配置目录
- `TERMX_THEME`: 指定默认主题

### 命令行参数

启动时可以使用命令行参数：

- `--config <path>`: 指定配置文件路径
- `--theme <name>`: 指定主题名称
- `--no-config`: 使用默认配置（不加载用户配置）

## 🔗 相关链接

- [TOML 语法参考](https://toml.io/)
- [颜色选择器](https://htmlcolorcodes.com/)
- [主题设计指南](https://github.com/termx/themes)
