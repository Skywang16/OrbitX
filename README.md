# OrbitX

一款跨平台终端应用，内置基础 AI 助手能力。基于 Vue 3 与 Tauri 构建。

![CI](https://img.shields.io/github/actions/workflow/status/Skywang16/OrbitX/ci.yml?branch=main&label=CI)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

## 特性

- 跨平台：Windows / macOS / Linux
- 基于 Tauri，体积小、资源占用低
- xterm.js 终端，支持常用插件（搜索、链接、自适应尺寸）
- 主题与配置可定制（见 `config/`）
- Pinia 管理应用状态

## 技术栈

- 前端：Vue 3 + TypeScript + Vite
- 桌面框架：Tauri 2
- 终端：xterm.js
- 状态管理：Pinia
- 后端（Tauri）：Rust

## 开发环境与依赖

- Node.js 18+
- Rust stable（建议与 CI 一致）
- 系统依赖：
  - macOS：Xcode Command Line Tools
  - Windows：Visual Studio Build Tools（含 C++ 工具集）、WebView2 Runtime
  - Ubuntu/Debian：`libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf`

## 安装

```bash
git clone https://github.com/Skywang16/OrbitX.git
cd OrbitX
npm install
```

### 可选：安装 Tauri CLI

```bash
npm install -g @tauri-apps/cli
```

## 本地开发

```bash
# 启动前端开发服务器
npm run dev

# 在另一个终端启动 Tauri 开发模式
npm run tauri dev
```

## 构建

```bash
# 构建前端（类型检查 + 打包）
npm run build

# 构建 Tauri 应用（多平台依赖见下文 CI/Release）
npm run tauri build
```

## 项目结构

```text
orbitx/
├── src/                     # 前端源代码（Vue 3 + TS + Vite）
│   ├── api/                 # 前端与 Tauri/Rust 的 API 调用
│   ├── components/          # Vue 组件
│   ├── composables/         # 可复用 hooks（useXxx）
│   ├── constants/           # 常量
│   ├── stores/              # Pinia 状态管理
│   ├── types/               # TypeScript 类型定义
│   └── ...
├── src-tauri/               # Tauri/Rust 后端
```

## 配置

- 主题：`config/themes/*.toml`
- 全局配置：`config/config.toml`

## 使用

常见操作：

- 多标签页、分屏与搜索（xterm.js 插件）
- 主题切换与跟随系统
- 快捷键（复制/粘贴/搜索、标签页管理等）

## 脚本

- `npm run dev`：前端开发（结合 `npm run tauri dev`）
- `npm run build`：类型检查 + 打包
- `npm run lint:check`：ESLint 检查
- `npm run format:check`：Prettier 检查

## CI/Release

- CI：见 `.github/workflows/ci.yml`（lint/format/build）
- Release：推送 `v*` 标签将触发 `.github/workflows/release.yml`，在 macOS/Windows/Ubuntu 构建并发布

Note: If the repository name or owner changes, please update badges and links accordingly.

## 致谢

- [Tauri](https://tauri.app/)
- [Vue.js](https://vuejs.org/)
- [xterm.js](https://xtermjs.org/)

- [eko](https://github.com/FellouAI/eko)

## Contact

For issues and suggestions, please create an [Issue](https://github.com/Skywang16/OrbitX/issues).

---

⭐ If this project helps you, please give it a star!
