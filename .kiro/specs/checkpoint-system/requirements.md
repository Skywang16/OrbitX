# Requirements Document

## Introduction

本文档定义了 OrbitX AI 聊天 Checkpoint 系统的需求。该系统允许用户在与 AI 对话过程中自动创建文件状态快照，并能够回滚到任意历史状态。类似于 Cursor、Windsurf 等 AI 编辑器的 checkpoint 功能，本质上是一个轻量级的 Git 树结构，用于追踪 Agent 对文件的修改历史。

## Glossary

- **Checkpoint**: 一个时间点的文件状态快照，记录了该时刻所有被 Agent 修改的文件内容
- **Checkpoint Tree**: 类似 Git 的树状结构，每个 checkpoint 可以有父节点，支持分支
- **File Snapshot**: 单个文件在某个 checkpoint 时的完整内容
- **Conversation**: 用户与 Agent 的对话会话
- **Agent Execution**: 一次 Agent 执行任务的过程，可能包含多次文件修改
- **Rollback**: 将文件系统恢复到某个 checkpoint 的状态
- **Diff**: 两个 checkpoint 之间的文件差异

## Requirements

### Requirement 1

**User Story:** As a user, I want the system to automatically create a checkpoint before each AI response, so that I can restore files to any previous state if the AI makes unwanted changes.

#### Acceptance Criteria

1. WHEN a user sends a message to the Agent THEN the Checkpoint System SHALL create a new checkpoint capturing the current state of all tracked files
2. WHEN a checkpoint is created THEN the Checkpoint System SHALL store the complete content of each modified file since the last checkpoint
3. WHEN a checkpoint is created THEN the Checkpoint System SHALL link the new checkpoint to its parent checkpoint forming a tree structure
4. WHEN the first checkpoint in a conversation is created THEN the Checkpoint System SHALL mark the checkpoint as a root node with no parent

### Requirement 2

**User Story:** As a user, I want to view the checkpoint history for a conversation, so that I can understand what changes were made at each step.

#### Acceptance Criteria

1. WHEN a user requests the checkpoint history THEN the Checkpoint System SHALL return a chronologically ordered list of checkpoints for the conversation
2. WHEN displaying a checkpoint THEN the Checkpoint System SHALL show the checkpoint timestamp, associated user message, and list of modified files
3. WHEN a user requests diff between two checkpoints THEN the Checkpoint System SHALL compute and return the file-level differences

### Requirement 3

**User Story:** As a user, I want to rollback to a previous checkpoint, so that I can undo unwanted AI changes and restore my files.

#### Acceptance Criteria

1. WHEN a user initiates a rollback to a specific checkpoint THEN the Checkpoint System SHALL restore all tracked files to their state at that checkpoint
2. WHEN a rollback is performed THEN the Checkpoint System SHALL create a new checkpoint representing the post-rollback state
3. WHEN a rollback affects files that have been modified by the user since the target checkpoint THEN the Checkpoint System SHALL warn the user before proceeding
4. IF a file restoration fails during rollback THEN the Checkpoint System SHALL report the error and continue with remaining files

### Requirement 4

**User Story:** As a user, I want the checkpoint system to efficiently store file snapshots, so that disk space is not wasted on redundant data.

#### Acceptance Criteria

1. WHEN storing a file snapshot THEN the Checkpoint System SHALL use content-addressable storage with SHA-256 hash as the identifier
2. WHEN a file content matches an existing snapshot THEN the Checkpoint System SHALL reference the existing content instead of duplicating
3. WHEN a checkpoint is deleted THEN the Checkpoint System SHALL only remove orphaned file snapshots that are not referenced by other checkpoints

### Requirement 5

**User Story:** As a user, I want to see which files were changed in each checkpoint, so that I can quickly identify what the AI modified.

#### Acceptance Criteria

1. WHEN a checkpoint is created THEN the Checkpoint System SHALL record the list of files that were added, modified, or deleted
2. WHEN displaying checkpoint details THEN the Checkpoint System SHALL categorize file changes as added, modified, or deleted
3. WHEN a user requests file content at a checkpoint THEN the Checkpoint System SHALL return the exact content stored in that snapshot

### Requirement 6

**User Story:** As a developer, I want the checkpoint data to be persisted in SQLite, so that it integrates with the existing OrbitX storage architecture.

#### Acceptance Criteria

1. WHEN the application starts THEN the Checkpoint System SHALL initialize the required database tables if they do not exist
2. WHEN checkpoint operations are performed THEN the Checkpoint System SHALL use transactions to ensure data consistency
3. WHEN querying checkpoints THEN the Checkpoint System SHALL use indexed queries for efficient retrieval

### Requirement 7

**User Story:** As a user, I want to compare the current file state with a checkpoint, so that I can see what has changed since that point.

#### Acceptance Criteria

1. WHEN a user requests comparison with a checkpoint THEN the Checkpoint System SHALL compute the diff between current file content and the checkpoint snapshot
2. WHEN computing diffs THEN the Checkpoint System SHALL support unified diff format for text files
3. WHEN a file does not exist in the checkpoint THEN the Checkpoint System SHALL indicate the file was added after that checkpoint
