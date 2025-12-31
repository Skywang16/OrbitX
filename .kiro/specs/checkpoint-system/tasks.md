# 实现计划

## 1. 数据库表结构与基础类型

- [x] 1.1 添加 checkpoint 相关数据库表
  - 在 `src-tauri/sql/01_tables.sql` 中添加 `checkpoints`、`checkpoint_file_snapshots`、`checkpoint_blobs` 表
  - 添加必要的索引
  - _需求: 6.1_

- [x] 1.2 创建 Rust 数据模型
  - 在 `src-tauri/src/checkpoint/` 目录下创建 `models.rs`
  - 定义 `Checkpoint`、`CheckpointSummary`、`FileSnapshot`、`FileChangeType`、`FileDiff`、`RollbackResult` 类型
  - _需求: 2.2, 5.2_

- [x] 1.3 创建模块入口文件
  - 创建 `src-tauri/src/checkpoint/mod.rs`
  - 导出公共类型和服务
  - _需求: 6.1_

## 2. BlobStore 内容寻址存储

- [x] 2.1 实现 BlobStore 核心功能
  - 创建 `src-tauri/src/checkpoint/blob_store.rs`
  - 实现 `store()` 方法：计算 SHA-256 哈希，存储内容
  - 实现 `get()` 方法：根据哈希获取内容
  - 实现 `exists()` 方法：检查哈希是否存在
  - 实现引用计数增减逻辑
  - _需求: 4.1, 4.2, 5.3_

- [x] 2.2 编写属性测试：内容寻址存储完整性
  - **属性 2：内容寻址存储完整性**
  - **验证需求: 4.1, 4.2, 5.3**

- [x] 2.3 实现垃圾回收功能
  - 实现 `gc()` 方法：清理 ref_count 为 0 的 blob
  - _需求: 4.3_

- [x] 2.4 编写属性测试：垃圾回收安全性
  - **属性 6：垃圾回收安全性**
  - **验证需求: 4.3**

## 3. CheckpointStorage 数据访问层

- [x] 3.1 实现 CheckpointStorage 基础 CRUD
  - 创建 `src-tauri/src/checkpoint/storage.rs`
  - 实现 `insert_checkpoint()` 方法
  - 实现 `get_checkpoint()` 方法
  - 实现 `list_by_conversation()` 方法
  - 实现 `delete_checkpoint()` 方法
  - _需求: 2.1, 6.2, 6.3_

- [x] 3.2 实现文件快照存储
  - 实现 `insert_file_snapshot()` 方法
  - 实现 `get_file_snapshots()` 方法
  - 实现 `get_latest_checkpoint()` 方法（获取会话最新 checkpoint）
  - _需求: 1.2, 5.1_

- [x] 3.3 编写属性测试：Checkpoint 历史排序
  - **属性 4：Checkpoint 历史排序**
  - **验证需求: 2.1**

## 4. Checkpoint - 确保所有测试通过

- [x] 确保所有测试通过，如有问题请询问用户。

## 5. CheckpointService 核心服务

- [x] 5.1 实现 checkpoint 创建功能
  - 创建 `src-tauri/src/checkpoint/service.rs`
  - 实现 `create_checkpoint()` 方法
  - 读取文件内容，计算变更类型
  - 存储 blob 和文件快照
  - 维护 parent_id 树结构
  - _需求: 1.1, 1.2, 1.3, 1.4, 5.1_

- [x] 5.2 编写属性测试：Checkpoint 创建完整性
  - **属性 1：Checkpoint 创建完整性**
  - **验证需求: 1.1, 1.2, 1.3, 5.1, 5.2**

- [x] 5.3 实现 checkpoint 列表和详情查询
  - 实现 `list_checkpoints()` 方法
  - 实现 `get_checkpoint_details()` 方法
  - 返回文件数量和总大小统计
  - _需求: 2.1, 2.2_

- [x] 5.4 编写属性测试：Checkpoint 数据完整性
  - **属性 7：Checkpoint 数据完整性**
  - **验证需求: 2.2**

- [x] 5.5 实现回滚功能
  - 实现 `rollback_to()` 方法
  - 从 blob 读取内容并写入文件
  - 处理部分失败情况
  - 回滚后创建新 checkpoint
  - _需求: 3.1, 3.2, 3.4_

- [x] 5.6 编写属性测试：回滚往返一致性
  - **属性 3：回滚往返一致性**
  - **验证需求: 3.1, 3.2**

- [x] 5.7 实现 diff 计算功能
  - 实现 `diff_checkpoints()` 方法
  - 实现 `diff_with_current()` 方法
  - 使用 unified diff 格式
  - _需求: 2.3, 7.1, 7.2, 7.3_

- [x] 5.8 编写属性测试：Diff 计算正确性
  - **属性 5：Diff 计算正确性**
  - **验证需求: 2.3, 7.1, 7.2**

## 6. Checkpoint - 确保所有测试通过

- [x] 确保所有测试通过，如有问题请询问用户。

## 7. Tauri 命令接口

- [x] 7.1 创建 Tauri 命令模块
  - 创建 `src-tauri/src/checkpoint/commands.rs`
  - 定义 `CheckpointState` 状态管理结构
  - _需求: 6.1_

- [x] 7.2 实现 checkpoint 命令
  - 实现 `checkpoint_create` 命令
  - 实现 `checkpoint_list` 命令
  - 实现 `checkpoint_rollback` 命令
  - 实现 `checkpoint_diff` 命令
  - 实现 `checkpoint_get_file_content` 命令
  - _需求: 1.1, 2.1, 3.1, 2.3, 5.3_

- [x] 7.3 注册命令到 Tauri 应用
  - 在 `src-tauri/src/lib.rs` 中注册 checkpoint 命令
  - 初始化 CheckpointState
  - _需求: 6.1_

## 8. 与 Agent 执行器集成

- [x] 8.1 在 Agent 执行前自动创建 checkpoint
  - 修改 `src-tauri/src/agent/core/executor/lifecycle.rs`
  - 在 `execute_task` 开始时调用 checkpoint 创建
  - 传递被追踪的文件列表
  - _需求: 1.1_

- [x] 8.2 追踪 Agent 修改的文件
  - 利用现有的 `file_context_entries` 表获取被修改的文件
  - 在 checkpoint 创建时包含这些文件
  - 添加 `get_agent_edited_files()` 方法获取被 Agent 编辑过的文件
  - _需求: 1.2, 5.1_

## 9. 最终 Checkpoint - 确保所有测试通过

- [x] 确保所有测试通过，如有问题请询问用户。
