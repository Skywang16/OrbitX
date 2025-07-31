# 贡献指南

感谢你对 TermX 项目的关注！我们欢迎所有形式的贡献。

## 🤝 如何贡献

### 报告问题

如果你发现了 bug 或有功能建议：

1. 在 [Issues](https://github.com/Skywang16/TermX/issues) 中搜索是否已有相关问题
2. 如果没有，创建一个新的 Issue
3. 使用清晰的标题和详细的描述
4. 如果是 bug，请提供复现步骤

### 提交代码

1. **Fork 仓库**

   ```bash
   git clone https://github.com/Skywang16/TermX.git
   cd TermX
   ```

2. **创建分支**

   ```bash
   git checkout -b feature/your-feature-name
   # 或
   git checkout -b fix/your-bug-fix
   ```

3. **设置开发环境**

   ```bash
   npm install
   npm run dev
   ```

4. **进行更改**
   - 遵循现有的代码风格
   - 添加必要的测试
   - 更新相关文档

5. **提交更改**

   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

6. **推送分支**

   ```bash
   git push origin feature/your-feature-name
   ```

7. **创建 Pull Request**
   - 提供清晰的 PR 标题和描述
   - 链接相关的 Issues
   - 等待代码审查

## 📝 代码规范

### 提交信息格式

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

类型包括：

- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `style`: 代码格式化
- `refactor`: 代码重构
- `test`: 添加测试
- `chore`: 构建过程或辅助工具的变动

### 代码风格

- 使用 ESLint 和 Prettier 进行代码格式化
- 运行 `npm run lint` 检查代码风格
- 运行 `npm run format` 自动格式化代码

### TypeScript

- 为新功能添加适当的类型定义
- 避免使用 `any` 类型
- 使用接口定义复杂对象结构

### Vue.js

- 使用 Composition API
- 组件名使用 PascalCase
- Props 和 events 使用 camelCase

## 🧪 测试

- 为新功能编写测试
- 确保所有测试通过
- 运行 `npm run test` 执行测试

## 📚 文档

- 更新相关的 README 和文档
- 为新功能添加使用示例
- 保持文档与代码同步

## 🔍 代码审查

所有的 Pull Request 都需要经过代码审查：

- 至少需要一个维护者的批准
- 解决所有审查意见
- 确保 CI 检查通过

## 🎯 开发指南

### 项目结构

```
src/
├── components/     # 可复用组件
├── views/         # 页面组件
├── stores/        # 状态管理
├── utils/         # 工具函数
├── types/         # 类型定义
└── ui/           # UI 组件库
```

### 添加新功能

1. 在 `src/components/` 或 `src/views/` 中创建组件
2. 如需状态管理，在 `src/stores/` 中添加 store
3. 更新路由配置（如果需要）
4. 添加相应的类型定义

### 调试

- 使用浏览器开发者工具调试前端
- 使用 `console.log` 或 `debugger` 进行调试
- Tauri 后端可以使用 Rust 的调试工具

## 🚀 发布流程

维护者负责版本发布：

1. 更新版本号
2. 更新 CHANGELOG
3. 创建 Git 标签
4. 发布 GitHub Release

## 📞 获取帮助

如果你在贡献过程中遇到问题：

- 查看现有的 Issues 和 Discussions
- 在 Issue 中提问
- 联系维护者

## 🙏 致谢

感谢所有为 TermX 项目做出贡献的开发者！

---

再次感谢你的贡献！🎉
