# OrbitX 终端模拟器应用 · 演示PPT大纲

## 1. 封面

- 产品名称：OrbitX – 智能终端（Vue + Tauri）
- 一句话介绍：现代化终端 + AI 助手 + Sidecar 实时伴随分析
- 作者 / 时间：

## 2. 目标与受众

- 目标：展示产品价值、架构设计、AI集成亮点与演示案例
- 受众：开发者、架构师、AIGC 应用评审者、潜在用户

## 3. 产品价值与亮点

- 跨平台与高性能：Tauri 2 + Rust，体积小、资源占用低（`src-tauri/Cargo.toml`）
- 现代化UI与易用性：Vue 3 + Vite + Pinia，响应式与可定制主题（`src/`、`config/themes/`）
- AI深度集成：上下文压缩、KV Cache、Eko 工具调用（`src-tauri/src/ai/`、`src/components/AIChatSidebar/`）
- Sidecar模式：自动监听终端命令，执行后自动交给AI分析并回填建议

## 4. 现场演示脚本（建议）

- 演示1：基本终端操作与多标签工作流（`src/components/terminal/Terminal.vue`）
- 演示2：AI聊天侧边栏提问 + 工具调用流式过程（`AIChatSidebar/index.vue` + `store.ts`）
- 演示3：Sidecar 模式下执行一条命令，查看自动分析与建议
- 演示4：切换终端主题、调整外观与快捷键（`config/config.toml` + `config/themes/`）

## 5. 架构总览

- 前端：Vue 3、Pinia、xterm.js，API 模块化（`src/api/*`）
- 后端：Tauri 2（Rust），AI服务、会话与存储仓库（`src-tauri/src/`）
- 通信：Tauri Commands（如 `build_prompt_with_context`）（`src-tauri/src/ai/commands.rs`）
- 第三方：Eko 框架集成、OpenAI 兼容接口（`@eko-ai/eko`、`async-openai`）

## 6. 前端模块

- 终端层：`Terminal.vue`、`TerminalCompletion.vue`，xterm.js 插件与交互
- AI侧边栏：`AIChatSidebar/index.vue` + `store.ts`
  - 发送消息流程、会话管理、流式回调、步骤数据持久化
  - 关键方法：`sendMessage()`、`initializeEko()`、`truncateAndResend()`
- 输入组件：`ChatInput.vue`
  - Enter 发送/Shift+Enter 换行，模式切换、模型选择，插入终端选中文本
- API层：`src/api/ai/index.ts` 与 `src/api/terminal/index.ts`
  - 对应 Tauri 命令的调用封装、错误处理、数据类型（`types.ts`）
- 状态与组合式函数：`useTerminalSelection.ts`、`useConfig.ts`、`useTheme.ts`

## 7. 后端（Tauri/Rust）模块

- 命令入口：`src-tauri/src/ai/commands.rs`
  - 会话CRUD、消息保存与状态更新、上下文构建、KV缓存管理命令
  - 重点命令：`create_conversation()`、`get_compressed_context()`、`build_prompt_with_context()`、`truncate_conversation()`
- 上下文与缓存：`src-tauri/src/ai/enhanced_context.rs`
  - `ContextManager`、`CompressionStrategy`、`KVCache`、`MessageScorer`
  - `build_prompt()` 将【前置提示】【历史对话】【当前问题】拼装并结合 KV Cache
- 类型与配置：`src-tauri/src/ai/types.rs`
  - `AIContext`、`AIResponse`、`Conversation/Message`、`AIConfig` 等
- 模型与服务：`AIService`（在 `mod.rs` re-export，服务初始化与模型管理）
- 依赖选型：`async-openai`、`reqwest`、`sqlx`、`tree-sitter` 等（`Cargo.toml`）

## 8. AI 上下文压缩与 KV 缓存机制

- 压缩触发：估算 tokens 超阈值触发（`ContextConfig.compress_threshold`）
- 混合策略：保留系统消息、最近消息、关键词加权高分消息（`HybridStrategy` → `MessageScorer`）
- 循环去重：`LoopDetector` 降低重复问答噪音
- KV Cache：稳定前缀提取 + 哈希键；命中后增量拼接“当前问题”（`KVCache.get/put()`）
- 收益：降低重复拼装与推理成本，提高响应速度与稳定性

## 9. Sidecar（边车）模式工作流

- 模式定位：伴随终端执行，自动感知命令生命周期并生成 AI 分析
- 监听机制：注册终端回调、解析 OSC 序列识别命令开始/结束（前端终端层与 `terminalStore.registerTerminalCallbacks` 集成位）
- 数据管道：命令输入/输出 → 结构化封装 → 交给 Eko → `build_prompt_with_context()` → AI 回复 → 渲染到侧边栏
- 可靠性：防抖与队列、错误处理、模式切换时资源清理
- 价值：零打扰/低摩擦的“伴随式”智能协作

## 10. 配置与主题

- 统一配置：`config/config.toml`（App 行为、终端、快捷键、字体、主题等）
- 主题体系：`config/themes/*.toml` 多款主题（light/dark/one-dark/nord…）
- 运行时策略：组合式函数 `useTheme.ts`、`useConfig.ts` 动态应用

## 11. 性能与稳定性

- Tauri 构建优化：`[profile.release] opt-level="s", lto, panic=abort`（`Cargo.toml`）
- 前端体积与加载：Vite + 按需模块、流式渲染
- AI链路：KV Cache 命中率、请求超时与并发控制（`AIPerformanceSettings`）
- 日志与观测：`tracing`、命令级 info/debug，错误链路可追踪

## 12. 安全与合规

- 秘钥管理：模型 `apiKey` 输入与持久化策略（前端设置模块与仓库侧）
- 数据最小化：按会话截断、只上传必要上下文
- 沙箱与权限：Tauri 插件使用边界（fs/http/process/opener）

## 13. 可扩展性与二次开发

- 模型扩展：`AIModelConfig`/`AISettings` 支持多模型与默认模型切换（`types.rs`）
- 策略扩展：可插拔压缩策略（实现 `CompressionStrategy`）
- 工具生态：Eko 工具执行链与统一步骤记录（`createToolExecution`、流式回调 → `saveStepsToDatabase()`）
- 前端扩展点：API 层封装与 Pinia Store 合理分层，易于新增视图与功能

## 14. 路线图（Roadmap）

- 更智能上下文：语义摘要与向量检索增强
- 更多Sidecar能力：多终端/多会话并行策略、命令级知识库
- 插件市场：工具与主题生态、脚本运行器
- 团队协作：共享会话、组织知识库

## 15. 总结

- 一句话回顾：将终端与 AI 深度耦合，提升开发与运维生产力
- 关键信息：高性能、可扩展、可观测、好用且好看
- Call to Action：体验 Sidecar、提交模型配置、试用主题

---

### 建议配图（供后续补充）

- 架构图：前端/后端/AI 服务与数据流
- Sidecar流程时序图：命令→OSC→Eko→AI→UI
- 上下文压缩示意图：原始消息→压缩策略→Prompt
- KV Cache 命中/未命中对比：延迟与成本曲线

### 附录（备份）

- 目录结构：参考 `README.md`
- 关键函数索引：
  - `build_prompt_with_context()`（`src-tauri/src/ai/commands.rs`）
  - `ContextManager.build_prompt()`（`src-tauri/src/ai/enhanced_context.rs`）
  - `useAIChatStore.sendMessage()`（`src/components/AIChatSidebar/store.ts`）
  - `ChatInput` 键盘与模式切换（`src/components/AIChatSidebar/components/ChatInput.vue`）
- 配置清单：`config/config.toml`
- 依赖清单：`package.json` 与 `Cargo.toml`
