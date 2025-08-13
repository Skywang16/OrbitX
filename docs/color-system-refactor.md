# OrbitX 颜色系统重构文档

## 概述

当前的颜色系统存在以下问题：

1. **层次不清晰**：颜色差异太小，用户难以区分不同层级的元素
2. **变量重复**：多个变量指向同一个颜色值
3. **缺少扩展性**：难以添加新的颜色层次
4. **命名不统一**：前端CSS变量和后端TOML配置命名不一致

本文档提供完整的重构方案，采用基于数字层次的颜色系统设计。

## 新颜色系统设计

### 核心原则

- **数字层次**：使用50-900的数字表示颜色深浅，数字越大越"突出"
- **预留空间**：每个层次之间留有足够空间，方便插入新层次
- **主题适配**：浅色主题通过重新定义数值实现适配
- **语义清晰**：组件直接使用对应层级，无需额外的语义化变量
- **结构简化**：删除冗余的 theme.colors 基础字段，只保留必要的 ANSI 颜色

### 颜色层次定义

#### 背景色层次（深色主题）

```css
--bg-50: #0a0a0a; /* 最深层，预留 */
--bg-100: #1a1a1a; /* 编辑器背景 */
--bg-200: #1e1e1e; /* 主要内容区 */
--bg-300: #252526; /* 侧边栏背景 */
--bg-400: #2d2d30; /* 面板背景 */
--bg-500: #3c3c3c; /* 卡片/工具块背景 */
--bg-600: #4d4d4d; /* 悬停状态 */
--bg-700: #5a5a5a; /* 激活状态 */
--bg-800: #6a6a6a; /* 高亮状态，预留 */
--bg-900: #7a7a7a; /* 最亮背景，预留 */
```

#### 边框层次

```css
--border-100: rgba(255, 255, 255, 0.04); /* 最淡 */
--border-200: rgba(255, 255, 255, 0.08); /* 很淡 */
--border-300: rgba(255, 255, 255, 0.12); /* 正常 */
--border-400: rgba(255, 255, 255, 0.16); /* 明显 */
--border-500: rgba(255, 255, 255, 0.2); /* 很明显 */
```

#### 文本层次

```css
--text-100: #ffffff; /* 最重要文本 */
--text-200: #e0e0e0; /* 重要文本 */
--text-300: #cccccc; /* 正常文本 */
--text-400: #999999; /* 次要文本 */
--text-500: #666666; /* 最淡文本 */
```

### 主题独立配置原则

每个主题都有独立的TOML配置文件，包含完整的颜色层次定义：

- `config/themes/dark.toml` - 深色主题配置
- `config/themes/light.toml` - 浅色主题配置
- `config/themes/xxx.toml` - 其他主题配置

前端CSS只定义变量名，具体颜色值完全由后端从TOML文件加载并动态应用。

## 组件颜色映射

### 常用组件的颜色使用

- **编辑器背景**: `var(--bg-100)`
- **侧边栏背景**: `var(--bg-300)`
- **面板背景**: `var(--bg-400)`
- **ToolBlock背景**: `var(--bg-500)`
- **ToolBlock悬停**: `var(--bg-600)`
- **卡片背景**: `var(--bg-500)`
- **按钮背景**: `var(--bg-500)`
- **输入框背景**: `var(--bg-400)`

### 边框使用

- **淡边框**: `var(--border-200)`
- **正常边框**: `var(--border-300)`
- **强调边框**: `var(--border-400)`

### 文本使用

- **标题文本**: `var(--text-100)`
- **正文文本**: `var(--text-300)`
- **次要文本**: `var(--text-400)`
- **禁用文本**: `var(--text-500)`

## 重构计划

### 阶段1：完全重构前端CSS变量

1. **删除** `src/styles/themes/default.css` 中所有旧变量
2. **重新定义** 新的颜色层次变量（仅变量名，无默认值）
3. **删除** 所有兼容性变量和旧代码

### 阶段2：完全重构后端配置结构

1. **重新设计** `UIColors` 结构体，删除旧字段
2. **完全重写** 主题TOML文件格式
3. **重写** CSS解析器和映射逻辑
4. **重写** 主题应用逻辑

### 阶段3：重写所有配置文件

1. **重写** 所有主题TOML文件，使用新格式
2. **删除** 所有旧的颜色定义
3. **不保留** 任何向后兼容性

### 阶段4：重写所有组件样式

1. **重写** 所有组件的CSS，使用新变量
2. **删除** 所有硬编码颜色和旧变量引用
3. **清理** 所有废弃代码

## 详细实施步骤

### 步骤1：前端CSS变量重构

#### 1.1 完全重写 `src/styles/themes/default.css`

```css
/* 全新的颜色层次系统 - 仅定义变量名，值由后端动态设置 */
:root {
  /* 背景色层次 */
  --bg-100: ;
  --bg-200: ;
  --bg-300: ;
  --bg-400: ;
  --bg-500: ;
  --bg-600: ;
  --bg-700: ;

  /* 边框层次 */
  --border-200: ;
  --border-300: ;
  --border-400: ;

  /* 文本层次 */
  --text-100: ;
  --text-200: ;
  --text-300: ;
  --text-400: ;
  --text-500: ;

  /* 状态颜色 */
  --color-primary: ;
  --color-success: ;
  --color-warning: ;
  --color-error: ;
  --color-info: ;
}

/* 删除所有旧变量，不保留任何兼容性代码 */
```

#### 1.2 更新组件样式

以ToolBlock为例：

```css
.tool-block {
  background: var(--bg-500);
  border: 1px solid var(--border-300);
}

.tool-block:hover {
  background: var(--bg-600);
}

.tool-name {
  color: var(--text-200);
}

.tool-command {
  color: var(--text-400);
}
```

### 步骤2：后端结构调整

#### 2.1 完全重写 `UIColors` 结构体

```rust
// src-tauri/src/config/theme/types.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UIColors {
    // 背景色层次
    pub bg_100: String,
    pub bg_200: String,
    pub bg_300: String,
    pub bg_400: String,
    pub bg_500: String,
    pub bg_600: String,
    pub bg_700: String,

    // 边框层次
    pub border_200: String,
    pub border_300: String,
    pub border_400: String,

    // 文本层次
    pub text_100: String,
    pub text_200: String,
    pub text_300: String,
    pub text_400: String,
    pub text_500: String,

    // 状态颜色
    pub primary: String,
    pub primary_hover: String,
    pub primary_alpha: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,

    // 交互状态
    pub hover: String,
    pub active: String,
    pub focus: String,
    pub selection: String,
}

// 删除所有旧字段：secondary, border, divider 等
```

#### 2.2 完全重写CSS解析器

```rust
// src-tauri/src/config/theme/css_parser.rs
// 删除整个CSS解析器，因为不再从CSS解析，而是直接从TOML读取

// 新的TOML解析逻辑
fn parse_toml_theme(toml_content: &str) -> AppResult<Theme> {
    let theme_data: TomlTheme = toml::from_str(toml_content)?;

    Ok(Theme {
        name: theme_data.theme.name,
        theme_type: theme_data.theme.theme_type,
        ansi: theme_data.theme.ansi,
        bright: theme_data.theme.bright,
        syntax: theme_data.theme.syntax,
        ui: UIColors {
            bg_100: theme_data.theme.ui.bg_100,
            bg_200: theme_data.theme.ui.bg_200,
            bg_300: theme_data.theme.ui.bg_300,
            bg_400: theme_data.theme.ui.bg_400,
            bg_500: theme_data.theme.ui.bg_500,
            bg_600: theme_data.theme.ui.bg_600,
            bg_700: theme_data.theme.ui.bg_700,

            border_200: theme_data.theme.ui.border_200,
            border_300: theme_data.theme.ui.border_300,
            border_400: theme_data.theme.ui.border_400,

            text_100: theme_data.theme.ui.text_100,
            text_200: theme_data.theme.ui.text_200,
            text_300: theme_data.theme.ui.text_300,
            text_400: theme_data.theme.ui.text_400,
            text_500: theme_data.theme.ui.text_500,

            primary: theme_data.theme.ui.primary,
            primary_hover: theme_data.theme.ui.primary_hover,
            primary_alpha: theme_data.theme.ui.primary_alpha,
            success: theme_data.theme.ui.success,
            warning: theme_data.theme.ui.warning,
            error: theme_data.theme.ui.error,
            info: theme_data.theme.ui.info,

            hover: theme_data.theme.ui.hover,
            active: theme_data.theme.ui.active,
            focus: theme_data.theme.ui.focus,
            selection: theme_data.theme.ui.selection,
        },
    })
}
```

#### 2.3 完全重写主题应用逻辑

```typescript
// src/utils/themeApplier.ts - 完全重写
export const applyThemeToUI = (theme: Theme): void => {
  const root = document.documentElement
  const style = root.style

  // 清除所有旧变量
  clearAllOldVariables(style)

  // 应用新的颜色层次
  style.setProperty('--bg-100', theme.ui.bg_100)
  style.setProperty('--bg-200', theme.ui.bg_200)
  style.setProperty('--bg-300', theme.ui.bg_300)
  style.setProperty('--bg-400', theme.ui.bg_400)
  style.setProperty('--bg-500', theme.ui.bg_500)
  style.setProperty('--bg-600', theme.ui.bg_600)
  style.setProperty('--bg-700', theme.ui.bg_700)

  style.setProperty('--border-200', theme.ui.border_200)
  style.setProperty('--border-300', theme.ui.border_300)
  style.setProperty('--border-400', theme.ui.border_400)

  style.setProperty('--text-100', theme.ui.text_100)
  style.setProperty('--text-200', theme.ui.text_200)
  style.setProperty('--text-300', theme.ui.text_300)
  style.setProperty('--text-400', theme.ui.text_400)
  style.setProperty('--text-500', theme.ui.text_500)

  style.setProperty('--color-primary', theme.ui.primary)
  style.setProperty('--color-primary-hover', theme.ui.primary_hover)
  style.setProperty('--color-primary-alpha', theme.ui.primary_alpha)
  style.setProperty('--color-success', theme.ui.success)
  style.setProperty('--color-warning', theme.ui.warning)
  style.setProperty('--color-error', theme.ui.error)
  style.setProperty('--color-info', theme.ui.info)

  style.setProperty('--color-hover', theme.ui.hover)
  style.setProperty('--color-active', theme.ui.active)
  style.setProperty('--color-focus', theme.ui.focus)
  style.setProperty('--color-selection', theme.ui.selection)

  // 设置主题类型
  root.setAttribute('data-theme', theme.theme_type)
}

const clearAllOldVariables = (style: CSSStyleDeclaration) => {
  // 删除所有旧变量
  const oldVariables = [
    '--color-background',
    '--color-foreground',
    '--color-cursor',
    '--color-background-secondary',
    '--color-background-hover',
    '--color-border',
    '--border-color',
    '--text-primary',
    '--text-secondary',
    '--text-muted',
    // ... 所有旧变量
  ]

  oldVariables.forEach(variable => {
    style.removeProperty(variable)
  })
}
```

### 步骤3：重写所有配置文件

#### 3.1 完全重写深色主题配置

```toml
# config/themes/dark.toml - 完全重写
[theme]
name = "dark"
theme_type = "dark"

[theme.ansi]
black = "#000000"
red = "#cd3131"
green = "#0dbc79"
yellow = "#e5e510"
blue = "#2472c8"
magenta = "#bc3fbc"
cyan = "#11a8cd"
white = "#e5e5e5"

[theme.bright]
black = "#666666"
red = "#f14c4c"
green = "#23d18b"
yellow = "#f5f543"
blue = "#3b8eea"
magenta = "#d670d6"
cyan = "#29b8db"
white = "#ffffff"

[theme.ui]
# 背景色层次
bg_100 = "#1a1a1a"
bg_200 = "#1e1e1e"
bg_300 = "#252526"
bg_400 = "#2d2d30"
bg_500 = "#3c3c3c"
bg_600 = "#4d4d4d"
bg_700 = "#5a5a5a"

# 边框层次
border_200 = "rgba(255, 255, 255, 0.08)"
border_300 = "rgba(255, 255, 255, 0.12)"
border_400 = "rgba(255, 255, 255, 0.16)"

# 文本层次
text_100 = "#ffffff"
text_200 = "#e0e0e0"
text_300 = "#cccccc"
text_400 = "#999999"
text_500 = "#666666"

# 状态颜色
primary = "#007acc"
primary_hover = "#005a9e"
primary_alpha = "rgba(0, 122, 204, 0.1)"
success = "#0dbc79"
warning = "#ffcc02"
error = "#f44747"
info = "#75beff"

# 交互状态
hover = "#2a2d2e"        # 通用悬停背景
active = "#3c3c3c"       # 激活状态背景
focus = "#007acc"        # 焦点边框颜色
selection = "rgba(173, 214, 255, 0.3)"  # 选择背景
```

#### 3.2 完全重写浅色主题配置

```toml
# config/themes/light.toml - 完全重写
[theme]
name = "light"
theme_type = "light"

[theme.ansi]
black = "#24292e"
red = "#d73a49"
green = "#28a745"
yellow = "#ffd33d"
blue = "#0366d6"
magenta = "#ea4aaa"
cyan = "#17a2b8"
white = "#f6f8fa"

[theme.bright]
black = "#586069"
red = "#cb2431"
green = "#22863a"
yellow = "#b08800"
blue = "#005cc5"
magenta = "#e36209"
cyan = "#0598bc"
white = "#fafbfc"

[theme.ui]
# 浅色主题的背景色层次
bg_100 = "#ffffff"
bg_200 = "#fafafa"
bg_300 = "#f5f5f5"
bg_400 = "#f0f0f0"
bg_500 = "#e8e8e8"
bg_600 = "#e0e0e0"
bg_700 = "#d8d8d8"

# 浅色主题的边框层次
border_200 = "rgba(0, 0, 0, 0.08)"
border_300 = "rgba(0, 0, 0, 0.12)"
border_400 = "rgba(0, 0, 0, 0.16)"

# 浅色主题的文本层次
text_100 = "#000000"
text_200 = "#1a1a1a"
text_300 = "#333333"
text_400 = "#666666"
text_500 = "#999999"

# 状态颜色
primary = "#0366d6"
primary_hover = "#005cc5"
primary_alpha = "rgba(3, 102, 214, 0.1)"
success = "#28a745"
warning = "#ffc107"
error = "#dc3545"
info = "#17a2b8"

# 交互状态
hover = "#e8e8e8"        # 通用悬停背景
active = "#e0e0e0"       # 激活状态背景
focus = "#0366d6"        # 焦点边框颜色
selection = "rgba(3, 102, 214, 0.3)"  # 选择背景
```

### 步骤4：重写所有组件样式

#### 4.1 ToolBlock组件完全重写

```vue
<!-- src/components/AIChatSidebar/components/ToolBlock.vue -->
<style scoped>
  .tool-block {
    padding: 8px 12px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: background-color 0.2s ease;
    max-width: 100%;
  }

  .tool-block:hover {
    background: var(--bg-600);
  }

  .tool-name {
    color: var(--text-200);
    font-weight: 500;
    white-space: nowrap;
  }

  .tool-command {
    color: var(--text-400);
    font-family: monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
  }

  .status-dot.running {
    background: var(--color-info);
    animation: pulse 1.5s infinite;
  }

  .status-dot.error {
    background: var(--color-error);
  }

  .tool-result {
    margin-top: 8px;
    padding: 8px;
    background: var(--bg-400);
    border-radius: 4px;
  }

  .tool-result-content {
    font-family: monospace;
    color: var(--text-300);
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: 12px;
    line-height: 1.4;
  }
</style>
```

## 实施时间表

### 第1周：完全重构基础设施

- [ ] 删除所有旧的CSS变量和代码
- [ ] 重写前端颜色变量定义
- [ ] 重写后端数据结构
- [ ] 重写主题解析和应用逻辑

### 第2周：重写配置文件

- [ ] 重写所有主题TOML文件
- [ ] 删除所有旧的颜色定义
- [ ] 测试新的主题加载

### 第3周：重写核心组件

- [ ] 重写ToolBlock组件
- [ ] 重写侧边栏组件
- [ ] 重写面板组件

### 第4周：重写所有其他组件

- [ ] 重写所有UI组件样式
- [ ] 删除所有硬编码颜色
- [ ] 全面测试

### 第5周：清理和验证

- [ ] 删除所有废弃代码
- [ ] 验证所有主题正常工作
- [ ] 性能测试和优化

## 总结

这是一个**完全重构**的方案：

1. **删除所有旧代码** - 不保留任何向后兼容性
2. **重新设计颜色系统** - 基于数字层次的清晰结构
3. **独立主题配置** - 每个主题有完整的TOML配置
4. **统一变量命名** - 前后端使用一致的命名规范

通过这个彻底的重构，OrbitX将拥有一个现代化、可扩展、易维护的颜色系统。
