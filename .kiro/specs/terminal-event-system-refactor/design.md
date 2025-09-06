# 终端事件系统重构设计文档

## 概述

本设计文档详细描述了OrbitX终端事件系统的重构方案，旨在建立一个基于统一事件总线的现代化事件驱动架构。设计完全遵循Tauri应用的前后端通信机制，与现有代码风格保持一致，确保高性能、高可靠性和易维护性。

## 架构设计

### 核心理念

采用"单消费者异步事件总线"为中心的架构，将所有终端相关事件统一入队，由一个消费任务统一出站到前端，确保事件顺序性和一致性。

### 事件流路径

```
PTY 输出 ──> IoHandler/BatchProcessor ──(微批)──>
                                       MuxEventBus (Tokio mpsc, 有界)
                                                │
                                                ▼
                                单消费者异步任务（TerminalEventHandler）
                                   └─> 映射为前端事件名（app_handle.emit）
                                                │
                                                ▼
                                            前端（TerminalStore 统一分发）
```

## 组件设计

### 1. MuxEventBus（事件总线）

**文件位置：** `src-tauri/src/mux/event_bus.rs`

**职责：**

- 提供统一、可靠的事件入队接口
- 提供单消费者读取端
- 实施背压策略和微批处理
- 提供可观测性指标

**接口设计：**

```rust
/// 统一的终端事件类型
#[derive(Debug, Clone, serde::Serialize)]
pub enum TerminalEvent {
    /// 终端输出事件
    Output {
        pane_id: PaneId,
        data: bytes::Bytes
    },
    /// 终端创建事件
    Created {
        pane_id: PaneId
    },
    /// 终端大小调整事件
    Resized {
        pane_id: PaneId,
        cols: u16,
        rows: u16
    },
    /// 终端关闭事件
    Closed {
        pane_id: PaneId
    },
    /// 终端进程退出事件
    Exited {
        pane_id: PaneId,
        exit_code: Option<i32>
    },
    /// 活跃终端变化事件
    ActivePaneChanged {
        old_pane_id: Option<PaneId>,
        new_pane_id: Option<PaneId>
    },
    /// 工作目录变化事件
    CwdChanged {
        pane_id: PaneId,
        old_cwd: Option<String>,
        new_cwd: String
    },
    /// Shell集成状态变化事件
    ShellIntegrationChanged {
        pane_id: PaneId,
        enabled: bool
    },
}

/// 事件总线配置
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// 队列容量
    pub queue_capacity: usize,
    /// 批处理大小阈值
    pub batch_size: usize,
    /// 批处理时间阈值
    pub flush_interval_ms: u64,
    /// 背压策略配置
    pub backpressure_config: BackpressureConfig,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 8192,
            batch_size: 1024,
            flush_interval_ms: 16, // ~60 FPS
            backpressure_config: BackpressureConfig::default(),
        }
    }
}

/// 背压策略配置
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// 高水位阈值（队列使用率）
    pub high_watermark: f32,
    /// 输出事件合并策略
    pub output_merge_strategy: OutputMergeStrategy,
    /// 控制事件保护
    pub protect_control_events: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            high_watermark: 0.8, // 80%
            output_merge_strategy: OutputMergeStrategy::MergeConsecutive,
            protect_control_events: true,
        }
    }
}

/// 输出事件合并策略
#[derive(Debug, Clone)]
pub enum OutputMergeStrategy {
    /// 合并连续的输出块
    MergeConsecutive,
    /// 仅保留最新块并标记截断
    KeepLatest,
    /// 不合并（可能导致丢弃）
    NoMerge,
}

/// 事件总线指标
#[derive(Debug, Clone, serde::Serialize)]
pub struct EventBusMetrics {
    /// 当前队列长度
    pub current_queue_length: usize,
    /// 历史最高队列长度
    pub max_queue_length: usize,
    /// 总处理事件数
    pub total_events_processed: u64,
    /// 丢弃的事件数
    pub events_dropped: u64,
    /// 丢弃的字节数
    pub bytes_dropped: u64,
    /// 最近错误时间戳
    pub last_error_timestamp: Option<std::time::SystemTime>,
    /// 最近错误信息
    pub last_error_message: Option<String>,
    /// 平均处理延迟（毫秒）
    pub average_processing_latency_ms: f64,
}

/// 事件总线主结构
pub struct MuxEventBus {
    /// 事件发送器
    tx: tokio::sync::mpsc::Sender<TerminalEvent>,
    /// 事件接收器（单消费者）
    rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<TerminalEvent>>>,
    /// 配置
    config: EventBusConfig,
    /// 指标收集器
    metrics: Arc<tokio::sync::RwLock<EventBusMetrics>>,
    /// 关闭信号
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl MuxEventBus {
    /// 创建新的事件总线
    pub fn new(config: EventBusConfig) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(config.queue_capacity);
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Self {
            tx,
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
            config,
            metrics: Arc::new(tokio::sync::RwLock::new(EventBusMetrics::default())),
            shutdown_tx,
        }
    }

    /// 获取事件发送器
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<TerminalEvent> {
        self.tx.clone()
    }

    /// 接收事件（单消费者）
    pub async fn recv(&self) -> Option<TerminalEvent> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }

    /// 获取指标快照
    pub async fn get_metrics(&self) -> EventBusMetrics {
        self.metrics.read().await.clone()
    }

    /// 关闭事件总线
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// 实施背压策略
    async fn apply_backpressure(&self, event: TerminalEvent) -> Option<TerminalEvent> {
        let current_len = self.tx.capacity() - self.tx.capacity();
        let capacity = self.tx.capacity();
        let usage_ratio = current_len as f32 / capacity as f32;

        if usage_ratio < self.config.backpressure_config.high_watermark {
            return Some(event);
        }

        // 实施背压策略
        match event {
            TerminalEvent::Output { .. } => {
                // 对输出事件应用合并策略
                self.handle_output_backpressure(event).await
            }
            _ if self.config.backpressure_config.protect_control_events => {
                // 保护控制事件
                Some(event)
            }
            _ => {
                // 其他事件可能被丢弃
                self.update_drop_metrics().await;
                None
            }
        }
    }

    /// 处理输出事件的背压
    async fn handle_output_backpressure(&self, event: TerminalEvent) -> Option<TerminalEvent> {
        match self.config.backpressure_config.output_merge_strategy {
            OutputMergeStrategy::MergeConsecutive => {
                // 实现连续块合并逻辑
                Some(event)
            }
            OutputMergeStrategy::KeepLatest => {
                // 保留最新块，标记截断
                Some(event)
            }
            OutputMergeStrategy::NoMerge => {
                // 不合并，可能丢弃
                self.update_drop_metrics().await;
                None
            }
        }
    }

    /// 更新丢弃指标
    async fn update_drop_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.events_dropped += 1;
    }
}
```

### 2. TerminalEventHandler（事件处理器）

**文件位置：** `src-tauri/src/terminal/event_handler.rs`（重构现有文件）

**职责：**

- 单消费者异步任务
- 事件映射和格式转换
- 前端事件发送
- 错误处理和恢复

**重构设计：**

```rust
/// 重构后的终端事件处理器
pub struct TerminalEventHandler<R: Runtime> {
    /// Tauri应用句柄
    app_handle: AppHandle<R>,
    /// 事件总线引用
    event_bus: Arc<MuxEventBus>,
    /// 处理器状态
    state: Arc<tokio::sync::RwLock<HandlerState>>,
    /// 关闭信号接收器
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

#[derive(Debug)]
struct HandlerState {
    /// 是否正在运行
    is_running: bool,
    /// 处理的事件总数
    events_processed: u64,
    /// 最后处理时间
    last_processed_at: Option<std::time::Instant>,
}

impl<R: Runtime> TerminalEventHandler<R> {
    /// 创建新的事件处理器
    pub fn new(
        app_handle: AppHandle<R>,
        event_bus: Arc<MuxEventBus>,
    ) -> Self {
        let shutdown_rx = event_bus.shutdown_tx.subscribe();

        Self {
            app_handle,
            event_bus,
            state: Arc::new(tokio::sync::RwLock::new(HandlerState {
                is_running: false,
                events_processed: 0,
                last_processed_at: None,
            })),
            shutdown_rx,
        }
    }

    /// 启动事件处理器
    pub async fn start(&self) -> anyhow::Result<()> {
        {
            let mut state = self.state.write().await;
            if state.is_running {
                anyhow::bail!("事件处理器已在运行");
            }
            state.is_running = true;
        }

        let event_bus = Arc::clone(&self.event_bus);
        let app_handle = self.app_handle.clone();
        let state = Arc::clone(&self.state);
        let mut shutdown_rx = self.shutdown_rx.resubscribe();

        // 启动消费者任务
        tauri::async_runtime::spawn(async move {
            tracing::info!("终端事件处理器已启动");

            loop {
                tokio::select! {
                    // 接收事件
                    event = event_bus.recv() => {
                        match event {
                            Some(event) => {
                                Self::handle_event(&app_handle, &event).await;

                                // 更新状态
                                let mut state_guard = state.write().await;
                                state_guard.events_processed += 1;
                                state_guard.last_processed_at = Some(std::time::Instant::now());
                            }
                            None => {
                                tracing::info!("事件总线已关闭，退出处理器");
                                break;
                            }
                        }
                    }
                    // 接收关闭信号
                    _ = shutdown_rx.recv() => {
                        tracing::info!("收到关闭信号，退出事件处理器");
                        break;
                    }
                }
            }

            // 更新状态
            let mut state_guard = state.write().await;
            state_guard.is_running = false;

            tracing::info!("终端事件处理器已停止");
        });

        Ok(())
    }

    /// 处理单个事件
    async fn handle_event(app_handle: &AppHandle<R>, event: &TerminalEvent) {
        let (event_name, payload) = Self::map_event_to_tauri(event);

        match app_handle.emit(event_name, payload) {
            Ok(_) => {
                tracing::debug!("事件已发送: {}", event_name);
            }
            Err(e) => {
                tracing::error!("发送事件失败: {}, 错误: {}", event_name, e);
                // 可以在这里实现重试逻辑
            }
        }
    }

    /// 将内部事件映射为Tauri事件
    fn map_event_to_tauri(event: &TerminalEvent) -> (&'static str, serde_json::Value) {
        match event {
            TerminalEvent::Output { pane_id, data } => (
                "terminal_output",
                serde_json::json!({
                    "paneId": pane_id.as_u32(),
                    "data": String::from_utf8_lossy(data)
                })
            ),
            TerminalEvent::Created { pane_id } => (
                "terminal_created",
                serde_json::json!({
                    "paneId": pane_id.as_u32()
                })
            ),
            TerminalEvent::Resized { pane_id, cols, rows } => (
                "terminal_resized",
                serde_json::json!({
                    "paneId": pane_id.as_u32(),
                    "rows": rows,
                    "cols": cols
                })
            ),
            TerminalEvent::Closed { pane_id } => (
                "terminal_closed",
                serde_json::json!({
                    "paneId": pane_id.as_u32()
                })
            ),
            TerminalEvent::Exited { pane_id, exit_code } => (
                "terminal_exit",
                serde_json::json!({
                    "paneId": pane_id.as_u32(),
                    "exitCode": exit_code
                })
            ),
            TerminalEvent::ActivePaneChanged { old_pane_id, new_pane_id } => (
                "active_pane_changed",
                serde_json::json!({
                    "oldPaneId": old_pane_id.map(|id| id.as_u32()),
                    "newPaneId": new_pane_id.map(|id| id.as_u32())
                })
            ),
            TerminalEvent::CwdChanged { pane_id, old_cwd, new_cwd } => (
                "pane_cwd_changed",
                serde_json::json!({
                    "paneId": pane_id.as_u32(),
                    "oldCwd": old_cwd,
                    "newCwd": new_cwd
                })
            ),
            TerminalEvent::ShellIntegrationChanged { pane_id, enabled } => (
                "pane_shell_integration_changed",
                serde_json::json!({
                    "paneId": pane_id.as_u32(),
                    "enabled": enabled
                })
            ),
        }
    }

    /// 获取处理器状态
    pub async fn get_status(&self) -> HandlerState {
        self.state.read().await.clone()
    }
}
```

### 3. 生产者集成

**修改现有文件：**

- `src-tauri/src/mux/io_handler.rs`
- `src-tauri/src/mux/batch_processor.rs`
- `src-tauri/src/mux/terminal_mux.rs`

**集成策略：**

```rust
// 在 IoHandler 中集成事件总线
impl IoHandler {
    pub fn new(
        event_bus_sender: tokio::sync::mpsc::Sender<TerminalEvent>,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Self {
        // 替换原有的 notification_sender 为 event_bus_sender
        Self {
            event_bus_sender,
            shell_integration,
            // ... 其他字段
        }
    }

    // 修改输出处理逻辑
    fn handle_output(&self, pane_id: PaneId, data: bytes::Bytes) {
        let event = TerminalEvent::Output { pane_id, data };

        // 异步发送到事件总线
        let sender = self.event_bus_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(event).await {
                tracing::error!("发送输出事件失败: {}", e);
            }
        });
    }
}

// 在 TerminalMux 中集成事件总线
impl TerminalMux {
    pub fn new(event_bus_sender: tokio::sync::mpsc::Sender<TerminalEvent>) -> Self {
        Self {
            event_bus_sender,
            // ... 其他字段
        }
    }

    // 修改控制事件发送
    fn send_control_event(&self, event: TerminalEvent) {
        let sender = self.event_bus_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(event).await {
                tracing::error!("发送控制事件失败: {}", e);
            }
        });
    }

    pub async fn create_pane_with_config(&self, size: PtySize, config: &TerminalConfig) -> AppResult<PaneId> {
        // ... 现有创建逻辑 ...

        // 发送创建事件
        self.send_control_event(TerminalEvent::Created { pane_id });

        Ok(pane_id)
    }

    pub fn remove_pane(&self, pane_id: PaneId) -> AppResult<()> {
        // ... 现有移除逻辑 ...

        // 发送关闭事件
        self.send_control_event(TerminalEvent::Closed { pane_id });

        Ok(())
    }
}
```

### 4. 状态管理器集成

**新增文件：** `src-tauri/src/mux/event_bus_state.rs`

```rust
/// 事件总线状态管理器
pub struct EventBusState {
    /// 事件总线实例
    pub event_bus: Arc<MuxEventBus>,
    /// 事件处理器
    pub event_handler: Arc<TerminalEventHandler<tauri::Wry>>,
}

impl EventBusState {
    /// 创建新的事件总线状态
    pub fn new(app_handle: tauri::AppHandle) -> anyhow::Result<Self> {
        let config = EventBusConfig::default();
        let event_bus = Arc::new(MuxEventBus::new(config));
        let event_handler = Arc::new(TerminalEventHandler::new(
            app_handle,
            Arc::clone(&event_bus),
        ));

        Ok(Self {
            event_bus,
            event_handler,
        })
    }

    /// 启动事件系统
    pub async fn start(&self) -> anyhow::Result<()> {
        self.event_handler.start().await?;
        tracing::info!("事件总线系统已启动");
        Ok(())
    }

    /// 关闭事件系统
    pub fn shutdown(&self) {
        self.event_bus.shutdown();
        tracing::info!("事件总线系统已关闭");
    }

    /// 获取事件发送器
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<TerminalEvent> {
        self.event_bus.sender()
    }

    /// 获取系统指标
    pub async fn get_metrics(&self) -> EventBusMetrics {
        self.event_bus.get_metrics().await
    }
}
```

### 5. Tauri命令接口

**新增文件：** `src-tauri/src/mux/commands.rs`

```rust
/// 获取事件总线指标
#[tauri::command]
pub async fn get_mux_metrics(
    state: tauri::State<'_, EventBusState>,
) -> Result<EventBusMetrics, String> {
    match state.get_metrics().await {
        metrics => {
            tracing::debug!("获取事件总线指标: {:?}", metrics);
            Ok(metrics)
        }
    }
}

/// 重置事件总线指标
#[tauri::command]
pub async fn reset_mux_metrics(
    state: tauri::State<'_, EventBusState>,
) -> Result<(), String> {
    // 实现指标重置逻辑
    tracing::info!("事件总线指标已重置");
    Ok(())
}

/// 获取事件处理器状态
#[tauri::command]
pub async fn get_event_handler_status(
    state: tauri::State<'_, EventBusState>,
) -> Result<HandlerState, String> {
    let status = state.event_handler.get_status().await;
    tracing::debug!("获取事件处理器状态: {:?}", status);
    Ok(status)
}
```

## 前端设计

### 1. TerminalStore重构

**文件位置：** `src/stores/Terminal.ts`

**重构策略：**

```typescript
// 统一事件监听设置
const setupGlobalListeners = async () => {
  if (_isListenerSetup) return

  // 统一的事件处理函数
  const handleTerminalEvent = (eventType: string, payload: any) => {
    const terminal = findTerminalByBackendId(payload.paneId)
    if (!terminal) return

    switch (eventType) {
      case 'terminal_output':
        const listeners = _listeners.value.get(terminal.id) || []
        listeners.forEach(listener => listener.callbacks.onOutput(payload.data))
        break

      case 'terminal_exit':
        const exitListeners = _listeners.value.get(terminal.id) || []
        exitListeners.forEach(listener => listener.callbacks.onExit(payload.exitCode))
        closeTerminal(terminal.id)
        break

      case 'pane_cwd_changed':
        terminal.cwd = payload.newCwd
        updateTerminalTitle(terminal, payload.newCwd)
        break

      case 'active_pane_changed':
        // 处理活跃终端变化
        if (payload.newPaneId) {
          const newActiveTerminal = findTerminalByBackendId(payload.newPaneId)
          if (newActiveTerminal) {
            setActiveTerminal(newActiveTerminal.id)
          }
        }
        break

      // 其他事件类型...
    }
  }

  // 注册所有事件监听器
  const eventTypes = [
    'terminal_output',
    'terminal_created',
    'terminal_resized',
    'terminal_closed',
    'terminal_exit',
    'active_pane_changed',
    'pane_cwd_changed',
    'pane_shell_integration_changed',
  ]

  const unlisteners = await Promise.all(
    eventTypes.map(eventType =>
      listen(eventType, (event: any) => {
        try {
          handleTerminalEvent(eventType, event.payload)
        } catch (error) {
          console.error(`处理${eventType}事件时发生错误:`, error)
        }
      })
    )
  )

  _globalListenersUnlisten = unlisteners
  _isListenerSetup = true
}
```

### 2. 性能优化

**节流和防抖：**

```typescript
// 使用 requestAnimationFrame 优化更新频率
const throttledUpdate = (() => {
  let rafId: number | null = null
  let pendingUpdates = new Set<string>()

  return (terminalId: string, updateFn: () => void) => {
    pendingUpdates.add(terminalId)

    if (rafId === null) {
      rafId = requestAnimationFrame(() => {
        pendingUpdates.forEach(id => {
          const terminal = terminals.value.find(t => t.id === id)
          if (terminal) {
            updateFn()
          }
        })

        pendingUpdates.clear()
        rafId = null
      })
    }
  }
})()

// 在事件处理中使用节流
const handleTerminalOutput = (terminalId: string, data: string) => {
  throttledUpdate(terminalId, () => {
    // 执行实际的UI更新
    const listeners = _listeners.value.get(terminalId) || []
    listeners.forEach(listener => listener.callbacks.onOutput(data))
  })
}
```

## 错误处理和恢复

### 1. 后端错误处理

```rust
/// 错误恢复策略
#[derive(Debug)]
pub enum RecoveryStrategy {
    /// 重试发送
    Retry { max_attempts: u32, delay_ms: u64 },
    /// 降级处理
    Degrade,
    /// 忽略错误
    Ignore,
    /// 停止处理
    Stop,
}

impl TerminalEventHandler<R> {
    async fn handle_event_with_recovery(
        app_handle: &AppHandle<R>,
        event: &TerminalEvent,
    ) -> anyhow::Result<()> {
        let mut attempts = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            match Self::handle_event(app_handle, event).await {
                Ok(_) => return Ok(()),
                Err(e) if attempts < MAX_RETRIES => {
                    attempts += 1;
                    tracing::warn!("事件处理失败，重试 {}/{}: {}", attempts, MAX_RETRIES, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                }
                Err(e) => {
                    tracing::error!("事件处理最终失败: {}", e);
                    return Err(e);
                }
            }
        }
    }
}
```

### 2. 前端错误处理

```typescript
// 错误边界和恢复
const handleEventError = (eventType: string, error: Error, payload: any) => {
  console.error(`处理${eventType}事件时发生错误:`, error, payload)

  // 记录错误指标
  errorMetrics.value.totalErrors++
  errorMetrics.value.lastError = {
    eventType,
    error: error.message,
    timestamp: Date.now(),
    payload,
  }

  // 尝试恢复策略
  switch (eventType) {
    case 'terminal_output':
      // 输出错误可以忽略
      break
    case 'terminal_exit':
      // 退出事件错误需要手动清理
      attemptManualCleanup(payload.paneId)
      break
    default:
      // 其他错误记录但继续处理
      break
  }
}
```

## 测试策略

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic_functionality() {
        let config = EventBusConfig::default();
        let bus = MuxEventBus::new(config);
        let sender = bus.sender();

        // 发送测试事件
        let test_event = TerminalEvent::Created {
            pane_id: PaneId::new(1)
        };
        sender.send(test_event.clone()).await.unwrap();

        // 接收事件
        let received = bus.recv().await.unwrap();
        assert!(matches!(received, TerminalEvent::Created { .. }));
    }

    #[tokio::test]
    async fn test_backpressure_handling() {
        let mut config = EventBusConfig::default();
        config.queue_capacity = 2; // 小容量用于测试

        let bus = MuxEventBus::new(config);
        let sender = bus.sender();

        // 填满队列
        for i in 0..3 {
            let event = TerminalEvent::Output {
                pane_id: PaneId::new(1),
                data: bytes::Bytes::from(format!("test {}", i)),
            };

            let result = sender.try_send(event);
            if i < 2 {
                assert!(result.is_ok());
            } else {
                // 第三个应该失败（队列满）
                assert!(result.is_err());
            }
        }
    }
}
```

### 2. 集成测试

```rust
#[tokio::test]
async fn test_end_to_end_event_flow() {
    // 创建测试应用
    let app = create_test_app().await;

    // 创建事件总线
    let event_bus_state = EventBusState::new(app.handle()).unwrap();
    event_bus_state.start().await.unwrap();

    // 模拟终端创建
    let sender = event_bus_state.sender();
    let create_event = TerminalEvent::Created {
        pane_id: PaneId::new(1)
    };
    sender.send(create_event).await.unwrap();

    // 验证前端接收到事件
    // 这里需要模拟前端监听器
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 验证指标更新
    let metrics = event_bus_state.get_metrics().await;
    assert_eq!(metrics.total_events_processed, 1);
}
```

## 性能考虑

### 1. 内存管理

- 使用 `bytes::Bytes` 进行零拷贝数据传输
- 实施智能缓存策略，避免内存泄漏
- 定期清理过期的事件和指标数据

### 2. CPU优化

- 使用批处理减少系统调用
- 实施事件合并策略减少处理开销
- 使用异步处理避免阻塞主线程

### 3. 网络优化

- 前端使用节流和防抖优化更新频率
- 实施智能重连机制
- 压缩大量数据传输

## 可观测性

### 1. 指标收集

- 事件处理延迟
- 队列长度和使用率
- 错误率和类型分布
- 内存和CPU使用情况

### 2. 日志策略

- 结构化日志记录关键事件
- 可配置的日志级别
- 错误日志包含足够的上下文信息

### 3. 调试工具

- 开发者面板显示实时指标
- 事件流可视化工具
- 性能分析和瓶颈识别

## 部署和迁移

### 1. 彻底重构策略

1. **完全替换：** 移除现有的事件处理代码，用新的统一事件总线替换
2. **保持接口：** 确保前端接收到的事件名称和数据格式完全一致
3. **一次性迁移：** 不需要渐进式迁移，直接切换到新架构
4. **代码清理：** 删除所有旧的事件处理逻辑，简化代码库

### 2. 前端接口保证

- **事件名称不变：** terminal_output、terminal_created、terminal_resized等保持原样
- **数据格式不变：** JSON结构和字段名称保持完全一致
- **行为一致：** 事件触发时机和频率保持相同的用户体验
- **无需前端修改：** 前端代码无需任何改动即可正常工作

### 3. 重构优势

- **代码简化：** 移除复杂的兼容性代码，架构更清晰
- **性能提升：** 无需维护多套系统，资源利用更高效
- **维护性：** 单一事件处理路径，更容易调试和维护
- **现代化：** 采用最新的异步架构模式，为未来扩展奠定基础

### 4. 风险控制

- **充分测试：** 确保所有现有功能在新架构下正常工作
- **快速回滚：** 保留旧代码分支，必要时可快速回滚
- **监控验证：** 部署后密切监控系统行为，确保稳定性

这个设计通过彻底重构实现了架构现代化，同时确保前端用户体验完全不受影响。
