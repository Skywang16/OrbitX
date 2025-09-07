/*!
 * 文件变化检测器
 *
 * 集成现有文件监控系统，处理代码文件变化事件，支持基本路径过滤。
 * 实现防抖机制，避免重复事件处理，优化性能。
 *
 * ## 主要功能
 *
 * - **事件防抖**: 合并短时间内的重复文件变化事件
 * - **路径过滤**: 根据配置过滤需要处理的文件和目录
 * - **事件聚合**: 将多个相关文件变化合并为批量操作
 * - **性能优化**: 避免频繁的单文件更新，优化系统性能
 *
 * ## 设计原则
 *
 * - 重用现有向量索引配置和过滤规则
 * - 遵循OrbitX开发规范，保持错误处理一致性
 * - 基于现有架构扩展，避免重复造轮子
 * - 保持代码风格统一
 *
 * Requirements: 8.1, 8.2, 8.3
 */

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn};

use super::file_filter::FileFilter;
use crate::vector_index::types::VectorIndexConfig;

/// 文件变化事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    /// 文件创建
    Created,
    /// 文件修改
    Modified,
    /// 文件删除
    Deleted,
    /// 文件重命名
    Renamed,
}

/// 文件变化事件
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// 文件路径
    pub file_path: PathBuf,
    /// 变化类型
    pub change_type: FileChangeType,
    /// 事件时间戳
    pub timestamp: Instant,
    /// 原文件路径（仅用于重命名事件）
    pub old_path: Option<PathBuf>,
}

/// 防抖配置
#[derive(Debug, Clone)]
pub struct DebounceConfig {
    /// 防抖延迟时间（毫秒）
    pub delay_ms: u64,
    /// 最大等待时间（毫秒）
    pub max_wait_ms: u64,
    /// 批量处理阈值
    pub batch_threshold: usize,
}

impl Default for DebounceConfig {
    fn default() -> Self {
        Self {
            delay_ms: 300,       // 300ms防抖延迟
            max_wait_ms: 2000,   // 最大等待2秒
            batch_threshold: 10, // 批量处理阈值
        }
    }
}

/// 文件变化检测器状态
#[derive(Debug, Default)]
struct DetectorState {
    /// 待处理的事件映射 (文件路径 -> 最新事件)
    pending_events: HashMap<PathBuf, FileChangeEvent>,
    /// 最后处理时间
    last_flush_time: Option<Instant>,
    /// 统计信息
    total_events: usize,
    processed_batches: usize,
    filtered_events: usize,
}

/// 文件变化检测器
pub struct FileChangeDetector {
    /// 防抖配置
    debounce_config: DebounceConfig,
    /// 检测器状态
    state: Arc<RwLock<DetectorState>>,
    /// 文件过滤器
    file_filter: FileFilter,
    /// 事件发送器
    event_sender: Option<mpsc::Sender<Vec<FileChangeEvent>>>,
}

impl FileChangeDetector {
    /// 创建新的文件变化检测器
    pub fn new(config: VectorIndexConfig, debounce_config: Option<DebounceConfig>) -> Self {
        let file_filter = FileFilter::new(&config);

        Self {
            debounce_config: debounce_config.unwrap_or_default(),
            state: Arc::new(RwLock::new(DetectorState::default())),
            file_filter,
            event_sender: None,
        }
    }

    /// 设置事件输出通道
    pub fn set_event_sender(&mut self, sender: mpsc::Sender<Vec<FileChangeEvent>>) {
        self.event_sender = Some(sender);
    }

    /// 处理单个文件变化事件
    pub async fn handle_file_change(&self, event: FileChangeEvent) -> Result<()> {
        // 1. 过滤不需要处理的文件
        if !self.file_filter.should_process_file(&event.file_path) {
            debug!("跳过文件事件: {}", event.file_path.display());

            let mut state = self.state.write().await;
            state.filtered_events += 1;
            return Ok(());
        }

        // 2. 更新检测器状态
        {
            let mut state = self.state.write().await;
            state.total_events += 1;

            // 更新或添加待处理事件
            state
                .pending_events
                .insert(event.file_path.clone(), event.clone());

            debug!(
                "添加待处理事件: {} ({:?}), 当前待处理: {}",
                event.file_path.display(),
                event.change_type,
                state.pending_events.len()
            );
        }

        // 3. 检查是否需要立即处理
        self.check_and_flush_events().await?;

        Ok(())
    }

    /// 启动防抖处理任务
    pub async fn start_debounce_processing(&self) -> Result<()> {
        let state = Arc::clone(&self.state);
        let debounce_config = self.debounce_config.clone();
        let event_sender = self.event_sender.clone();

        if event_sender.is_none() {
            return Err(anyhow::anyhow!("事件发送器未设置"));
        }

        let sender = event_sender.unwrap();

        // 启动定期刷新任务
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(debounce_config.delay_ms));

            loop {
                interval.tick().await;

                let mut should_flush = false;
                let mut max_wait_exceeded = false;

                // 检查是否需要刷新
                let pending_count = {
                    let state_guard = state.read().await;
                    let current_pending = state_guard.pending_events.len();

                    if current_pending > 0 {
                        // 检查批量阈值
                        if current_pending >= debounce_config.batch_threshold {
                            should_flush = true;
                        }

                        // 检查最大等待时间
                        if let Some(last_flush) = state_guard.last_flush_time {
                            let elapsed = last_flush.elapsed();
                            if elapsed.as_millis() > debounce_config.max_wait_ms as u128 {
                                max_wait_exceeded = true;
                                should_flush = true;
                            }
                        } else if current_pending > 0 {
                            // 首次有事件，设置初始时间
                            should_flush = true;
                        }
                    }

                    current_pending
                };

                // 执行刷新
                if should_flush {
                    if let Err(e) = Self::flush_pending_events(
                        Arc::clone(&state),
                        sender.clone(),
                        max_wait_exceeded,
                    )
                    .await
                    {
                        warn!("刷新待处理事件失败: {}", e);
                    }
                }

                // 如果没有待处理事件，休眠更长时间
                if pending_count == 0 {
                    sleep(Duration::from_millis(debounce_config.delay_ms)).await;
                }
            }
        });

        info!("文件变化检测器防抖处理已启动");
        Ok(())
    }

    /// 检查并刷新事件
    async fn check_and_flush_events(&self) -> Result<()> {
        let should_flush;
        let pending_count;

        {
            let state = self.state.read().await;
            pending_count = state.pending_events.len();
            should_flush = pending_count >= self.debounce_config.batch_threshold;
        }

        if should_flush {
            if let Some(sender) = &self.event_sender {
                Self::flush_pending_events(Arc::clone(&self.state), sender.clone(), false).await?;
            }
        }

        Ok(())
    }

    /// 刷新待处理事件
    async fn flush_pending_events(
        state: Arc<RwLock<DetectorState>>,
        sender: mpsc::Sender<Vec<FileChangeEvent>>,
        force_flush: bool,
    ) -> Result<()> {
        let events_to_process;

        // 提取待处理事件
        {
            let mut state_guard = state.write().await;

            if state_guard.pending_events.is_empty() {
                return Ok(());
            }

            // 收集所有待处理事件
            events_to_process = state_guard
                .pending_events
                .values()
                .cloned()
                .collect::<Vec<_>>();

            // 清空待处理事件
            state_guard.pending_events.clear();
            state_guard.last_flush_time = Some(Instant::now());
            state_guard.processed_batches += 1;

            debug!(
                "刷新事件批次: {} 个事件, 强制刷新: {}",
                events_to_process.len(),
                force_flush
            );
        }

        // 发送事件批次
        if !events_to_process.is_empty() {
            if let Err(e) = sender.send(events_to_process).await {
                return Err(anyhow::anyhow!("发送事件批次失败: {}", e));
            }
        }

        Ok(())
    }

    /// 获取检测器统计信息
    pub async fn get_stats(&self) -> (usize, usize, usize, usize) {
        let state = self.state.read().await;
        (
            state.total_events,
            state.pending_events.len(),
            state.processed_batches,
            state.filtered_events,
        )
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut state = self.state.write().await;
        state.total_events = 0;
        state.processed_batches = 0;
        state.filtered_events = 0;
        state.last_flush_time = None;
    }

    /// 手动触发刷新
    pub async fn manual_flush(&self) -> Result<usize> {
        if let Some(sender) = &self.event_sender {
            let pending_count = {
                let state = self.state.read().await;
                state.pending_events.len()
            };

            if pending_count > 0 {
                Self::flush_pending_events(Arc::clone(&self.state), sender.clone(), true).await?;
            }

            Ok(pending_count)
        } else {
            Err(anyhow::anyhow!("事件发送器未设置"))
        }
    }

    /// 获取当前待处理事件列表
    pub async fn get_pending_events(&self) -> Vec<FileChangeEvent> {
        let state = self.state.read().await;
        state.pending_events.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_file_change_detector_creation() {
        let config = VectorIndexConfig::default();
        let detector = FileChangeDetector::new(config, None);

        // 验证基本属性
        assert!(!detector.file_filter.get_supported_extensions().is_empty());
        assert!(detector
            .file_filter
            .get_supported_extensions()
            .contains("ts"));
        assert!(detector
            .file_filter
            .get_supported_extensions()
            .contains("rs"));
    }

    #[tokio::test]
    async fn test_should_process_file() {
        let config = VectorIndexConfig::default();
        let detector = FileChangeDetector::new(config, None);

        // 创建测试文件
        let temp_dir = tempdir().unwrap();
        let ts_file = temp_dir.path().join("test.ts");
        let txt_file = temp_dir.path().join("test.txt");

        fs::write(&ts_file, "console.log('test');").await.unwrap();
        fs::write(&txt_file, "plain text").await.unwrap();

        // 测试支持的文件类型
        assert!(detector.file_filter.should_process_file(&ts_file));

        // 测试不支持的文件类型
        assert!(!detector.file_filter.should_process_file(&txt_file));

        // 测试目录
        assert!(!detector.file_filter.should_process_file(temp_dir.path()));
    }

    #[tokio::test]
    async fn test_event_handling() {
        let config = VectorIndexConfig::default();
        let mut detector = FileChangeDetector::new(config, None);

        // 创建事件通道
        let (sender, _receiver) = mpsc::channel(100);
        detector.set_event_sender(sender);

        // 创建测试事件
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.ts");
        fs::write(&test_file, "console.log('test');").await.unwrap();

        let event = FileChangeEvent {
            file_path: test_file,
            change_type: FileChangeType::Modified,
            timestamp: Instant::now(),
            old_path: None,
        };

        // 处理事件
        detector.handle_file_change(event).await.unwrap();

        // 验证统计信息
        let (total, pending, _batches, _filtered) = detector.get_stats().await;
        assert_eq!(total, 1);
        assert_eq!(pending, 1);
    }

    #[tokio::test]
    async fn test_path_filtering() {
        let config = VectorIndexConfig::default();
        let detector = FileChangeDetector::new(config, None);

        // 测试忽略模式
        let node_modules_path = PathBuf::from("/project/node_modules/package/index.js");
        let git_path = PathBuf::from("/project/.git/config");
        let _normal_path = PathBuf::from("/project/src/main.ts");

        assert!(!detector.file_filter.should_process_file(&node_modules_path));
        assert!(!detector.file_filter.should_process_file(&git_path));

        // 注意：_normal_path不存在，所以is_file()会返回false，但扩展名检查会通过
        // 这个测试主要验证路径过滤逻辑
    }
}
