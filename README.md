# TermX

一个现代化的终端应用程序，基于 Vue.js 和 Tauri 构建。

## 🚀 特性

- 🖥️ 跨平台支持 (Windows, macOS, Linux)
- ⚡ 基于 Tauri 的高性能桌面应用
- 🎨 现代化的用户界面
- 🔧 可自定义配置
- 📱 响应式设计

## 🛠️ 技术栈

- **前端**: Vue.js 3 + TypeScript + Vite
- **桌面框架**: Tauri 2.0
- **终端组件**: xterm.js
- **状态管理**: Pinia
- **路由**: Vue Router
- **样式**: CSS3 + 自定义组件库

## 📦 安装

### 开发环境要求

- Node.js 18+
- Rust 1.70+
- 系统依赖 (根据操作系统)

### 克隆项目

```bash
git clone https://github.com/Skywang16/TermX.git
cd TermX
```

### 安装依赖

```bash
# 安装前端依赖
npm install

# 安装 Tauri CLI (如果还没有安装)
npm install -g @tauri-apps/cli
```

## 🚀 开发

### 启动开发服务器

```bash
# 启动前端开发服务器
npm run dev

# 在另一个终端启动 Tauri 开发模式
npm run tauri dev
```

### 构建项目

```bash
# 构建前端
npm run build

# 构建 Tauri 应用
npm run tauri build
```

## 📁 项目结构

```
termx/
├── src/                    # Vue.js 前端源码
│   ├── components/         # Vue 组件
│   ├── views/             # 页面视图
│   ├── stores/            # Pinia 状态管理
│   ├── router/            # Vue Router 配置
│   ├── ui/                # 自定义 UI 组件库
│   ├── utils/             # 工具函数
│   └── types/             # TypeScript 类型定义
├── src-tauri/             # Tauri 后端源码
│   ├── src/               # Rust 源码
│   ├── icons/             # 应用图标
│   └── tauri.conf.json    # Tauri 配置
├── config/                # 应用配置文件
├── docs/                  # 项目文档
└── scripts/               # 构建脚本
```

## 🎯 使用说明

1. 启动应用后，你将看到一个现代化的终端界面
2. 支持多标签页管理
3. 可以通过配置文件自定义主题和行为
4. 支持常用的终端功能和快捷键

## 🤝 贡献

欢迎贡献代码！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细的贡献指南。

### 开发流程

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [xterm.js](https://xtermjs.org/) - 终端组件库

## 📞 联系

如果你有任何问题或建议，请通过以下方式联系：

- 创建 [Issue](https://github.com/Skywang16/TermX/issues)
- 发送邮件到: your.email@example.com

---

⭐ 如果这个项目对你有帮助，请给它一个星标！
