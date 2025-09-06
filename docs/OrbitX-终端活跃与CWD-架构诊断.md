# OrbitX 后端“活跃终端/CWD”获取架构诊断报告

日期：2025-09-06
作者：Cascade（架构审阅）

---

## 执行摘要（Summary）

- 现状问题
  - 前端在构建 AI 提示词时主动传入 `currentWorkingDirectory`，形成前后端“职责穿透”和耦合（`src/components/AIChatSidebar/store.ts` → `aiApi.buildPromptWithContext` → 后端 `build_prompt_with_context`）。
  - CWD 的“单一事实来源”不清晰。后端 `ShellIntegrationManager` 能感知并广播 CWD，但前端也在解析 OSC 序列并反向 `updatePaneCwd` 到后端，形成“双源状态”，存在竞态和漂移风险。
  - 后端没有“活跃终端”的概念（仅 `TerminalMux` 的 pane 集合）。活跃终端只存在于前端 `useTerminalStore` 中，导致后端无法自主推断上下文。
  - 事件/集成模块存在重复与潜在混乱：`src-tauri/src/mux/tauri_integration.rs` 与 `src-tauri/src/ai/tool/shell/mux_terminal.rs::setup_tauri_integration` 均承担 Tauri 事件集成职责。
  - 工具层（如 `ShellTool`）依赖前端 Store 获取活跃终端（UI 概念），与执行层耦合，降低可测试性与可移植性。

- 关键建议
  - 以“后端为 CWD 单一事实来源”。CWD 的采集、存储、广播均在后端完成；前端仅订阅 `pane_cwd_changed` 更新 UI，不再向后端回写。
  - 在后端新增“活跃终端注册表”（Active Pane Registry），暴露 `set_active_pane` / `get_active_pane` 命令，前端在切换终端时同步一次；AI 后端基于此进行上下文推断。
  - 调整 AI Prompt 构建接口：可传 `pane_id`（或不传，由后端从注册表解析）；彻底移除 `currentWorkingDirectory` 参数。注意：`pane_id` 仅用于后端解析，不会进入 LLM Prompt 内容。
  - 统一 Tauri 事件集成路径，移除/合并重复实现，确保事件源清晰唯一。
  - 强 UI 取向：保留工具对前端 Store（`useTerminalStore`）的依赖，后端负责状态一致性与上下文统一，不以“无 UI 复用”为目标。

---

## 现状与数据流梳理

### 前端链路

- `src/stores/Terminal.ts`
  - 维护 `terminals[]` 与 `activeTerminalId`，监听 `pane_cwd_changed` 事件并更新 `terminal.cwd`（参考 `listen('pane_cwd_changed', ...)`）。
  - `setActiveTerminal(id)` 仅更新前端状态，后端无感知。

- `src/composables/useShellIntegration.ts`
  - 在浏览器中解析 OSC 133/7 序列（`parseOSCSequences`），触发 `onCwdUpdate`，并调用 `shellIntegrationApi.updatePaneCwd(backendId, newCwd)` 将 CWD 反向同步到后端。
  - 与后端 `ShellIntegrationManager` 的解析功能重复，形成双源状态。

- `src/components/AIChatSidebar/store.ts`
  - 发送消息时：
    - 取 `activeTerminal?.cwd` 作为 `currentWorkingDirectory`（行 250-259）。
    - 调用 `aiApi.buildPromptWithContext(conversationId, content, userMessageId, currentWorkingDirectory)`。
  - 这将 UI 状态注入到 AI 后端，形成耦合。

- `src/eko/tools/toolList/shell.ts`
  - `ShellTool.getActiveTerminal()` 直接读取前端 Store 的 `activeTerminalId`，并依赖面板 backendId 执行命令。

### 后端链路

- `src-tauri/src/mux/terminal_mux.rs`
  - 管理 Pane，集成 `ShellIntegrationManager`，在解析到 CWD 变化时发出 `MuxNotification::PaneCwdChanged` 并映射为 Ta uri 事件 `pane_cwd_changed`（621-625）。

- `src-tauri/src/shell/integration.rs`
  - `ShellIntegrationManager` 是 CWD/命令生命周期的权威来源：
    - `process_output()` 解析 OSC 序列（包括 133/7），更新 `PaneShellState.current_working_directory`。
    - 提供 `get_current_working_directory(pane_id)` 等查询接口。

- `src-tauri/src/shell/commands.rs`
  - 暴露 `get_pane_cwd` 与 `update_pane_cwd`。前者用于读，后者被前端调用进行“回写”。后者在单一事实来源原则下应裁剪。

- `src-tauri/src/ai/enhanced_context.rs` 与 `src-tauri/src/ai/context.rs`
  - Prompt 构建接受 `current_working_directory: Option<&str>`，若传入则将其以文本片段拼接进 Prompt（265-267）。后端自身并不尝试从终端态导出 CWD。

- `src-tauri/src/ai/commands.rs :: build_prompt_with_context`
  - 直接透传前端传入的 `current_working_directory` 给 `build_intelligent_prompt_with_tags`（202-211），未实现任何“本地推断”。

- 事件集成重复
  - `src-tauri/src/mux/tauri_integration.rs` 与 `src-tauri/src/ai/tool/shell/mux_terminal.rs::setup_tauri_integration` 都有将 `MuxNotification` 映射为 Tauri 事件的逻辑，存在职责重复/容易混淆的问题。

---

## 问题清单（含证据与影响）

1. **职责穿透与前后端耦合**（高影响）
   - 证据：`src/components/AIChatSidebar/store.ts` 在 `sendMessage()` 中取 `activeTerminal?.cwd` 传给 `aiApi.buildPromptWithContext`；后端 `ai/commands.rs` 再透传给 `enhanced_context.rs`。
   - 影响：
     - AI 后端对前端 UI 状态产生依赖，测试与复用困难。
     - 多来源 CWD 导致错误上下文的风险（UI 状态不等于真实 Shell 状态）。

2. **CWD 双源状态与竞态**（高影响）
   - 证据：前端 `useShellIntegration.ts` 与后端 `ShellIntegrationManager` 均解析 OSC 序列并能更新 CWD；前端还调用 `updatePaneCwd` 回写。
   - 影响：
     - 同一步状态有两处产生者，易出现时序竞态与漂移（尤其在高频命令/切目录场景）。
     - 增加维护复杂度与调试成本。

3. **后端无“活跃终端”概念**（中高影响）
   - 证据：`TerminalMux` 仅管理 Pane 集合；`ai/commands.rs` 无法在不依赖前端的情况下推断 CWD。
   - 影响：
     - 后端无法在“无 UI 参与/服务端回放/批处理”场景下独立工作。
     - 任一后端模块要感知当前上下文都需额外参数或前端配合。

4. **事件集成实现重复**（中影响）
   - 证据：`mux/tauri_integration.rs` 与 `ai/tool/shell/mux_terminal.rs` 都封装了 `MuxNotification` → Tauri 事件发送。
   - 影响：
     - 职责重复，容易出现逻辑分叉或不一致。
     - 新人上手成本上升。

5. **工具层与 UI 状态强耦合**（设计取向）
   - 证据：`ShellTool.getActiveTerminal()` 直接读取前端 Store 的 `activeTerminalId`。
   - 影响：
     - 在强 UI 架构下可接受，不作为本次重构优化目标。
     - 集成测试需要模拟前端 Store 状态，属于预期成本。

6. **Prompt 对 CWD 的文本化拼接**（低中影响）
   - 现状：CWD 仅以文本片段加入 Prompt，不具备强类型/一致性保障，且与工具执行时上下文可能脱节。

---

## 重构蓝图（Design Proposal）

### A. 单一事实来源（SSoT）：后端权威

- CWD 的解析、存储、广播完全由后端 `ShellIntegrationManager` 承担。
- 前端不再解析 OSC 序列用于状态回写，不再调用 `updatePaneCwd`。前端仅订阅 `pane_cwd_changed` 更新 UI。

### B. 活跃终端注册表（后端）

- 新增模块：`ActivePaneRegistry`
  - 职责：记录“当前活跃”的 PaneId（可细化维度：全局级 / 每窗口级 / 每会话级）。
  - Tauri 命令：
    - `set_active_pane(pane_id: u32)`
    - `get_active_pane() -> Option<u32>`
  - 前端在 `setActiveTerminal(id)` 时调用 `set_active_pane(backendId)`，保证后端与 UI 一致。

### C. AI Prompt 后端自动推断 CWD

- 破坏性变更（不保留兼容）：
  - 新接口：`build_prompt_with_context(conversation_id, current_message, up_to_message_id, pane_id?: u32, tag_context?: json)`
  - 逻辑：
    1) 如传入 `pane_id`，则使用 `mux.get_pane_cwd(pane_id)` 获取 CWD。
    2) 如未传入，则使用 `ActivePaneRegistry.get_active_pane()` → `get_pane_cwd`。
    3) 若两者都无，省略 CWD 信息。
  - 明确移除：不再接受 `currentWorkingDirectory` 参数；`pane_id` 不进入 Prompt，仅用于后端解析。

### D. 统一事件集成

- 选择并保留一个事件集成实现（建议保留 `ai/tool/shell/mux_terminal.rs::setup_tauri_integration`，并移除/合并 `mux/tauri_integration.rs`）。
- 在文档中明确事件映射表，避免命名/字段的不一致。

### E. 强 UI 取向的工具链策略

- 保留工具对前端 Store 的依赖（`useTerminalStore` 是一等公民）。
- 后端通过 Active Pane Registry 保证与前端活跃终端状态一致。
- 如需覆盖执行目标，可在调用层显式提供 `pane_id`（用于后端解析），但不对 LLM 暴露。

### F. Prompt 语义与一致性

- 在 `enhanced_context.rs` 中，将 CWD（必要时连同 Shell）作为可读文本加入 Prompt，不在 Prompt 中暴露 `pane_id`。
- 如内部需要，可在后端维持结构化上下文对象用于执行一致性校验，但不传递给 LLM。

---

## API 变化草案（Tauri）

- 新增
  - `set_active_pane(pane_id: u32) -> Result<(), String>`
  - `get_active_pane() -> Result<Option<u32>, String>`

- 移除/裁剪
  - `currentWorkingDirectory` 参数（彻底移除）。
  - `update_pane_cwd(pane_id, cwd)`（建议移除；如确需保留，仅限调试用途并标注“非权威接口”）。

---

## 前端改动点

- `src/stores/Terminal.ts`
  - 在 `setActiveTerminal(id)` 内部新增调用 `set_active_pane(backendId)`，保证后端与 UI 一致。

- `src/components/AIChatSidebar/store.ts`
  - 调用 `aiApi.buildPromptWithContext` 时可传 `paneId`（`activeTerminal.backendId`），不再传 `currentWorkingDirectory`。

- `src/composables/useShellIntegration.ts`
  - 移除对 OSC 7/133 的解析与 `updatePaneCwd` 回写逻辑，仅保留必要的 UI 级事件（如键入提示）或完全依赖后端发出的 `pane_cwd_changed`。

- `src/eko/tools/toolList/shell.ts`
  - 增加可选参数 `paneId`；默认读取后端注册表，而不是前端 Store。

---

## 风险评估与回滚

- 风险
  - 前端不再回写 CWD 后，若后端解析异常，UI 的 CWD 更新可能延迟或缺失。
  - `ActivePaneRegistry` 的粒度（全局/窗口/会话）选择不当可能造成跨上下文串扰。

- 缓解
  - 为 `ActivePaneRegistry` 提供按窗口 ID（Webview Window）命名空间的能力。
  - 在后端保留 `get_pane_shell_state` 调试命令，便于快速定位解析问题。

---

## 验收标准（DoD）

- 用例 1：仅切换 UI 活跃终端，不输入命令，AI Prompt 仍正确展示后端解析得到的 CWD。
- 用例 2：快速 `cd` 切换目录并立刻发起 AI 请求，Prompt 中 CWD 与后端 `get_pane_cwd` 一致。
- 用例 3：移除前端 `updatePaneCwd` 后，系统无报错，`pane_cwd_changed` 事件仍能驱动 UI 更新。

---

## 代码引用（关键位置）

- 前端
  - `src/components/AIChatSidebar/store.ts`：`sendMessage()` 中 `currentWorkingDirectory` 的获取与传递。
  - `src/stores/Terminal.ts`：`activeTerminalId` 管理、`pane_cwd_changed` 事件处理。
  - `src/composables/useShellIntegration.ts`：OSC 133/7 解析与 `updatePaneCwd` 回写逻辑。
  - `src/eko/tools/toolList/shell.ts`：`getActiveTerminal()` 依赖 UI Store。

- 后端
  - `src-tauri/src/shell/integration.rs`：`ShellIntegrationManager`（CWD/命令生命周期权威）。
  - `src-tauri/src/mux/terminal_mux.rs`：`PaneCwdChanged` 事件与 `notification_to_tauri_event`。
  - `src-tauri/src/shell/commands.rs`：`get_pane_cwd` / `update_pane_cwd`。
  - `src-tauri/src/ai/commands.rs`：`build_prompt_with_context` 透传 `current_working_directory`。
  - 事件集成重复：`src-tauri/src/mux/tauri_integration.rs` vs `src-tauri/src/ai/tool/shell/mux_terminal.rs::setup_tauri_integration`。

---

## 下一步（建议落地清单）

1. 后端新增 `ActivePaneRegistry` 模块与 Tauri 命令（set/get）。
2. 修改 `build_prompt_with_context`：支持 `pane_id` 并默认从注册表解析 CWD；彻底移除 `currentWorkingDirectory` 参数。
3. 统一事件集成：择一保留、另一处标注弃用并删除调用。
4. 前端移除 `updatePaneCwd` 回写与 OSC 解析，转为纯订阅模式。
5. 回归测试与性能对齐，补充端到端集成测试用例（见“验收标准”）。

---

如需，我可以直接按上述蓝图提交一版最小可行改造（Minimal Viable Refactor），包括后端注册表 + API 变更 + 前端调用梳理与兼容层，并附集成测试。
