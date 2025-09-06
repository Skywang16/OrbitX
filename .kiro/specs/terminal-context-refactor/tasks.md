# 终端上下文重构实施计划

## 阶段 1: 基础设施建设

- [-] 1. 实现活跃终端上下文注册表
  - 创建 `src-tauri/src/terminal/context_registry.rs` 文件
  - 实现 `ActiveTerminalContextRegistry` 结构体和核心方法
  - 添加线程安全的活跃终端状态管理
  - 实现事件发送机制
  - 编写单元测试验证并发安全性
  - _需求: 1.1, 1.2, 1.3, 1.4_

- [ ] 2. 实现终端上下文服务核心功能
  - 创建 `src-tauri/src/terminal/context_service.rs` 文件
  - 实现 `TerminalContextService` 结构体
  - 实现核心查询方法：`get_active_context`, `get_context_by_pane`
  - 实现错误处理和回退逻辑
  - 添加基础缓存机制
  - 编写单元测试覆盖各种查询场景
  - _需求: 2.1, 2.2, 2.3, 2.4, 7.1, 7.2_

- [ ] 3. 创建统一的终端上下文数据模型
  - 在 `src-tauri/src/terminal/types.rs` 中定义 `TerminalContext` 结构体
  - 定义 `ContextError` 错误类型
  - 实现序列化和反序列化支持
  - 添加数据验证逻辑
  - _需求: 2.2, 7.1_

- [ ] 4. 添加新的 Tauri 命令接口
  - 在 `src-tauri/src/terminal/commands.rs` 中添加活跃终端管理命令
  - 实现 `set_active_pane` 和 `get_active_pane` 命令
  - 实现 `get_terminal_context` 和 `get_active_terminal_context` 命令
  - 添加参数验证和错误处理
  - 编写集成测试验证命令功能
  - _需求: 1.1, 2.1, 2.3_

## 阶段 2: 核心功能迁移

- [ ] 5. 重构 AI 上下文构建逻辑
  - 修改 `src-tauri/src/ai/commands.rs` 中的 `build_prompt_with_context` 函数
  - 移除 `current_working_directory` 参数，添加可选的 `pane_id` 参数
  - 集成 `TerminalContextService` 进行上下文解析
  - 更新 `build_intelligent_prompt_with_context` 函数签名
  - 保持向后兼容性，添加弃用警告
  - 编写测试验证新逻辑的正确性
  - _需求: 4.1, 4.2, 4.3, 4.4, 6.1_

- [ ] 6. 更新 Shell 工具使用新的上下文服务
  - 修改 `src/eko/tools/toolList/shell.ts` 中的 `getActiveTerminal` 方法
  - 使用后端 `TerminalContextService` 替代前端 Store 查询
  - 添加可选的 `paneId` 参数支持
  - 实现错误处理和回退逻辑
  - 更新工具执行逻辑以使用新的上下文接口
  - 编写测试验证工具功能正常
  - _需求: 2.1, 4.2, 7.2_

- [ ] 7. 整合 Shell 集成管理器与上下文服务
  - 修改 `src-tauri/src/shell/integration.rs` 以支持上下文服务集成
  - 确保 CWD 变化事件正确传播到上下文服务
  - 实现上下文缓存失效机制
  - 优化 Shell 集成状态查询性能
  - _需求: 3.1, 3.4, 8.4_

- [ ] 8. 实现事件系统整合
  - 创建 `src-tauri/src/terminal/event_handler.rs` 文件
  - 实现统一的 `TerminalEventHandler`
  - 整合现有的事件处理逻辑，移除重复代码
  - 确保事件的单一来源和清晰的传播路径
  - 更新 Tauri 事件发送逻辑
  - _需求: 5.1, 5.2, 5.3, 5.4_

## 阶段 3: 前端集成

- [ ] 9. 创建前端终端上下文 API 适配层
  - 创建 `src/api/terminal-context/index.ts` 文件
  - 实现 `TerminalContextApi` 类
  - 添加活跃终端管理方法：`setActivePaneId`, `getActivePaneId`
  - 添加上下文查询方法：`getTerminalContext`, `getActiveTerminalContext`
  - 实现错误处理和类型安全
  - _需求: 1.1, 2.1, 6.2_

- [ ] 10. 更新前端 Terminal Store
  - 修改 `src/stores/Terminal.ts` 中的 `setActiveTerminal` 方法
  - 添加对后端 `set_active_pane` 的调用
  - 移除前端 CWD 回写逻辑（`updatePaneCwd` 调用）
  - 保持纯订阅模式，只监听后端 CWD 变化事件
  - 更新终端切换逻辑以同步后端状态
  - 编写测试验证状态同步正确性
  - _需求: 1.1, 3.2, 3.3_

- [ ] 11. 修改 AI Chat Store 调用方式
  - 更新 `src/components/AIChatSidebar/store.ts` 中的 `sendMessage` 方法
  - 移除 `currentWorkingDirectory` 参数获取逻辑
  - 传递 `activeTerminal.backendId` 作为 `paneId` 参数
  - 更新 `aiApi.buildPromptWithContext` 调用
  - 添加错误处理和回退逻辑
  - 编写测试验证 AI 上下文构建正确性
  - _需求: 4.1, 4.3, 6.1_

- [ ] 12. 移除前端 Shell 集成的 CWD 回写逻辑
  - 修改 `src/composables/useShellIntegration.ts`
  - 移除 `shellIntegrationApi.updatePaneCwd` 调用
  - 保留 OSC 序列解析用于 UI 级别的功能（如命令提示）
  - 确保前端只订阅后端 CWD 变化事件
  - 简化 Shell 集成逻辑，专注于 UI 交互
  - _需求: 3.2, 3.3_

## 阶段 4: 性能优化和缓存

- [ ] 13. 实现高级缓存机制
  - 在 `TerminalContextService` 中实现多级缓存
  - 添加基于 TTL 的缓存失效策略
  - 实现 LRU 缓存淘汰算法
  - 添加缓存统计和监控功能
  - 优化并发访问性能
  - 编写性能测试验证缓存效果
  - _需求: 8.1, 8.3, 8.4_

- [ ] 14. 优化上下文查询性能
  - 实现异步上下文查询接口
  - 添加查询超时和重试机制
  - 优化数据库查询和内存访问
  - 实现批量上下文查询支持
  - 添加性能监控和指标收集
  - 进行性能基准测试和调优
  - _需求: 8.1, 8.2, 8.3_

- [ ] 15. 实现错误处理和弹性机制
  - 完善 `ContextError` 错误类型定义
  - 实现优雅降级和回退策略
  - 添加自动重试机制
  - 实现健康检查和自我修复功能
  - 添加详细的错误日志和监控
  - 编写错误场景测试用例
  - _需求: 7.1, 7.2, 7.3, 7.4_

## 阶段 5: 测试和验证

- [ ] 16. 编写全面的单元测试
  - 为 `ActiveTerminalContextRegistry` 编写并发安全测试
  - 为 `TerminalContextService` 编写各种查询场景测试
  - 为新的 Tauri 命令编写功能测试
  - 为错误处理逻辑编写边界条件测试
  - 确保测试覆盖率达到 90% 以上
  - _需求: 所有需求的验证_

- [ ] 17. 编写端到端集成测试
  - 测试前端切换终端到后端状态更新的完整流程
  - 测试 AI 上下文构建的端到端功能
  - 测试 CWD 变化事件的传播和处理
  - 测试错误场景和回退逻辑
  - 验证性能要求的达成
  - _需求: 1.1, 2.1, 3.1, 4.1, 8.1, 8.2_

- [ ] 18. 进行向后兼容性测试
  - 验证现有前端代码继续正常工作
  - 测试弃用警告的正确显示
  - 验证新旧 API 行为的一致性
  - 测试迁移路径的可行性
  - 确保无破坏性变更
  - _需求: 6.1, 6.2, 6.3_

## 阶段 6: 清理和发布

- [ ] 19. 清理重复和遗留代码
  - 移除 `src-tauri/src/mux/tauri_integration.rs` 中的重复事件处理逻辑
  - 清理 `src-tauri/src/ai/tool/shell/mux_terminal.rs` 中的冗余集成代码
  - 移除前端中不再使用的 CWD 回写相关代码
  - 整理和优化导入语句
  - 更新代码注释和文档
  - _需求: 5.1, 5.2_

- [ ] 20. 更新文档和 API 说明
  - 更新 API 文档，标注新接口和弃用接口
  - 编写迁移指南和最佳实践文档
  - 更新架构文档反映新的设计
  - 添加故障排除和调试指南
  - 更新开发者文档和示例代码
  - _需求: 6.2, 6.3_

- [ ] 21. 最终集成测试和性能验证
  - 在完整系统中进行端到端测试
  - 验证所有性能指标达到要求
  - 进行压力测试和稳定性测试
  - 验证内存使用和资源消耗
  - 确保系统在各种场景下稳定运行
  - _需求: 8.1, 8.2, 8.3, 8.4_

- [ ] 22. 准备发布和部署
  - 创建发布说明和变更日志
  - 准备数据库迁移脚本（如需要）
  - 设置监控和告警机制
  - 准备回滚计划和应急预案
  - 进行最终的代码审查和质量检查
  - _需求: 所有需求的最终验证_
