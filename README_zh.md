# OrbitX

中文 | [English](./README.md)

**开源的 AI Coding Agent，拥有精美的桌面界面。**

![CI](https://img.shields.io/github/actions/workflow/status/Skywang16/OrbitX/ci.yml?branch=main&label=CI)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Release](https://img.shields.io/github/v/release/Skywang16/OrbitX)](https://github.com/Skywang16/OrbitX/releases)

> 平台支持：当前仅适配 macOS（Windows/Linux 正在适配中）

## 特性

- **AI Coding Agent** - 强大的 AI 助手，支持代码生成、重构和分析
- **多服务商支持** - 支持 OpenAI、Claude、Gemini 及 OpenAI 兼容 API
- **内置终端** - 基于 xterm.js 的全功能终端（搜索、链接、自适应）
- **Agent 模式** - 可在不同 Agent 模式间切换以适应各种任务
- **智能补全** - 命令补全、文件路径补全、Git/NPM 集成
- **主题定制** - 多种内置主题，支持亮色/暗色模式
- **原生性能** - 基于 Tauri 构建，体积小、资源占用低

## 子代理

OrbitX 使用子代理来处理复杂任务：

- **General** - 通用子代理，用于复杂搜索和多步任务
- **Plan** - 只读子代理，用于代码分析和规划
- **Explore** - 快速子代理，用于代码库探索和信息收集

## 安装

### 下载桌面应用

可直接从 [发布页](https://github.com/Skywang16/OrbitX/releases) 下载。

| 平台                  | 下载文件                      |
| --------------------- | ----------------------------- |
| macOS (Apple Silicon) | `OrbitX_x.x.x_aarch64.dmg` |
| macOS (Intel)         | `OrbitX_x.x.x_x64.dmg`     |
| Windows               | 即将推出                      |
| Linux                 | 即将推出                      |

### 从源码构建

```bash
git clone https://github.com/Skywang16/OrbitX.git
cd OrbitX
npm install
npm run tauri build
```

## 本地开发

```bash
# 启动前端开发服务器
npm run dev

# 在另一个终端启动 Tauri 开发模式
npm run tauri dev
```

## 技术栈

- **前端**: Vue 3 + TypeScript + Vite
- **桌面框架**: Tauri 2
- **终端**: xterm.js
- **状态管理**: Pinia
- **后端**: Rust

## 配置

- 主题: `config/themes/*.json`
- 工作区设置: `.orbitx/settings.json`

## 常见问题

### 这和其他 AI 编程工具有什么不同？

- **100% 开源** - 完全透明，社区驱动开发
- **服务商无关** - 不锁定任何单一 AI 服务商
- **原生桌面应用** - 快速、轻量，支持本地模型离线使用
- **精美界面** - 为开发者设计的现代化界面

## 参与贡献

如有兴趣贡献代码，请在提交 PR 前阅读 [贡献指南](./CONTRIBUTING.md)。

## 致谢

- [Tauri](https://tauri.app/)
- [Vue.js](https://vuejs.org/)
- [xterm.js](https://xtermjs.org/)

## 许可

本项目以 GPL-3.0-or-later 授权。详见 `LICENSE` 文件。

---

**联系**: 如有问题和建议，请创建 [Issue](https://github.com/Skywang16/OrbitX/issues)。

⭐ 如果这个项目对你有帮助，请给它一个 star！
