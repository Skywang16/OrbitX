/*!
 * 文件监控模块
 *
 * 集成现有文件监控系统，处理代码文件变化事件，支持基本路径过滤。
 * 实现增量索引更新，删除旧向量数据，重新处理变化的文件。
 *
 * ## 主要功能
 *
 * - **文件变化检测**: 使用notify库监控代码文件变化
 * - **增量更新**: 智能更新向量索引，避免重复处理
 * - **路径过滤**: 根据配置过滤需要监控的文件和目录
 * - **事件处理**: 统一处理文件创建、修改、删除事件
 *
 * ## 设计原则
 *
 * - 重用现有向量索引服务的配置和接口
 * - 遵循OrbitX开发规范，使用anyhow进行错误处理
 * - 避免重复造轮子，基于现有架构扩展
 * - 保持代码风格一致性
 *
 * Requirements: 8.1, 8.2, 8.3, 8.4, 8.5
 */

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::vector_index::{service::VectorIndexService, types::VectorIndexConfig};

pub mod file_change_detector;
pub mod file_filter;
pub mod incremental_updater;

use file_filter::FileFilter;

/// 文件监控事件类型
#[derive(Debug, Clone)]
pub enum FileMonitorEvent {
    /// 文件被创建
    Created(PathBuf),
    /// 文件被修改
    Modified(PathBuf),
    /// 文件被删除
    Deleted(PathBuf),
    /// 文件被重命名
    Renamed { from: PathBuf, to: PathBuf },
}

/// 监控统计信息
#[derive(Debug, Clone, Default)]
pub struct MonitorStats {
    /// 监控开始时间
    pub start_time: Option<Instant>,
    /// 总事件数
    pub total_events: usize,
    /// 处理的事件数
    pub processed_events: usize,
    /// 跳过的事件数（由于过滤等原因）
    pub skipped_events: usize,
    /// 增量更新次数
    pub incremental_updates: usize,
    /// 最近更新时间
    pub last_update_time: Option<Instant>,
}

/// 文件监控服务
pub struct FileMonitorService {
    /// 配置
    config: VectorIndexConfig,
    /// 向量索引服务
    vector_service: Arc<VectorIndexService>,
    /// 监控器
    watcher: Option<RecommendedWatcher>,
    /// 事件接收器
    event_receiver: Option<mpsc::Receiver<Result<Event, notify::Error>>>,
    /// 监控统计
    stats: Arc<RwLock<MonitorStats>>,
    /// 正在监控的根目录
    watch_root: Option<PathBuf>,
    /// 文件过滤器
    file_filter: FileFilter,
}

impl FileMonitorService {
    /// 创建新的文件监控服务
    pub fn new(config: VectorIndexConfig, vector_service: Arc<VectorIndexService>) -> Result<Self> {
        // 创建文件过滤器
        let full_config = crate::vector_index::types::VectorIndexFullConfig::new(config.clone());
        let file_filter = FileFilter::new(&full_config);

        Ok(Self {
            config,
            vector_service,
            watcher: None,
            event_receiver: None,
            stats: Arc::new(RwLock::new(MonitorStats::default())),
            watch_root: None,
            file_filter,
        })
    }

    /// 开始监控指定目录
    pub async fn start_monitoring<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        ensure!(
            path.exists() && path.is_dir(),
            "监控路径必须是一个存在的目录: {}",
            path.display()
        );

        info!("开始监控目录: {}", path.display());

        // 创建事件通道
        let (tx, rx) = mpsc::channel(1000);

        // 配置监控器
        let config = Config::default()
            .with_poll_interval(Duration::from_millis(100))
            .with_compare_contents(false);

        // 创建监控器
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.blocking_send(res) {
                    error!("发送文件监控事件失败: {}", e);
                }
            },
            config,
        )
        .context("创建文件监控器失败")?;

        // 开始监控
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .with_context(|| format!("开始监控目录失败: {}", path.display()))?;

        // 更新状态
        self.watcher = Some(watcher);
        self.event_receiver = Some(rx);
        self.watch_root = Some(path.clone());

        // 初始化统计信息
        {
            let mut stats = self.stats.write().await;
            stats.start_time = Some(Instant::now());
            *stats = MonitorStats::default();
            stats.start_time = Some(Instant::now());
        }

        info!("文件监控服务已启动，监控目录: {}", path.display());
        Ok(())
    }

    /// 停止监控
    pub async fn stop_monitoring(&mut self) -> Result<()> {
        if let Some(mut watcher) = self.watcher.take() {
            // 停止监控
            if let Some(watch_path) = &self.watch_root {
                watcher
                    .unwatch(watch_path)
                    .with_context(|| format!("停止监控目录失败: {}", watch_path.display()))?;
            }
        }

        // 清理状态
        self.event_receiver = None;
        self.watch_root = None;

        info!("文件监控服务已停止");
        Ok(())
    }

    /// 处理文件监控事件（高效的异步接收模式）
    pub async fn process_events(&mut self) -> Result<()> {
        info!("开始处理文件监控事件");

        loop {
            // 接收单个事件，避免长期借用
            let event_result = match self.event_receiver.as_mut() {
                Some(receiver) => match receiver.recv().await {
                    Some(result) => result,
                    None => {
                        debug!("事件接收器通道关闭，退出事件处理");
                        break;
                    }
                },
                None => {
                    debug!("事件接收器不存在，退出事件处理");
                    break;
                }
            };

            // 更新总事件计数
            {
                let mut stats = self.stats.write().await;
                stats.total_events += 1;
            }

            // 处理事件（现在可以安全地借用self）
            let process_result = match event_result {
                Ok(event) => self.handle_file_event(event).await,
                Err(e) => {
                    error!("文件监控事件错误: {}", e);
                    Err(anyhow::anyhow!("监控事件错误: {}", e))
                }
            };

            // 根据处理结果更新统计
            {
                let mut stats = self.stats.write().await;
                match process_result {
                    Ok(_) => {
                        stats.processed_events += 1;
                        debug!("文件事件处理成功");
                    }
                    Err(e) => {
                        stats.skipped_events += 1;
                        error!("处理文件事件失败: {}", e);
                    }
                }
            }
        }

        info!("文件监控事件处理结束");
        Ok(())
    }

    /// 处理单个文件事件
    async fn handle_file_event(&self, event: Event) -> Result<()> {
        let monitor_event = self.convert_notify_event(event)?;

        debug!("收到文件监控事件: {:?}", monitor_event);

        match monitor_event {
            FileMonitorEvent::Created(path) | FileMonitorEvent::Modified(path) => {
                if self.file_filter.should_process_file(&path) {
                    self.handle_file_updated(path).await?;
                }
            }
            FileMonitorEvent::Deleted(path) => {
                if self.file_filter.should_process_file(&path) {
                    self.handle_file_deleted(path).await?;
                }
            }
            FileMonitorEvent::Renamed { from, to } => {
                if self.file_filter.should_process_file(&from)
                    || self.file_filter.should_process_file(&to)
                {
                    self.handle_file_renamed(from, to).await?;
                }
            }
        }

        Ok(())
    }

    /// 转换notify事件为内部事件格式
    fn convert_notify_event(&self, event: Event) -> Result<FileMonitorEvent> {
        use notify::EventKind::*;

        match event.kind {
            Create(_) => {
                let path = event
                    .paths
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("创建事件缺少路径信息"))?;
                Ok(FileMonitorEvent::Created(path.clone()))
            }
            Modify(_) => {
                let path = event
                    .paths
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("修改事件缺少路径信息"))?;
                Ok(FileMonitorEvent::Modified(path.clone()))
            }
            Remove(_) => {
                let path = event
                    .paths
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("删除事件缺少路径信息"))?;
                Ok(FileMonitorEvent::Deleted(path.clone()))
            }
            _ => {
                // 对于其他类型的事件，尝试作为修改事件处理
                let path = event
                    .paths
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("未知事件缺少路径信息"))?;
                Ok(FileMonitorEvent::Modified(path.clone()))
            }
        }
    }

    /// 处理文件更新事件
    async fn handle_file_updated(&self, path: PathBuf) -> Result<()> {
        info!("处理文件更新: {}", path.display());

        // 使用增量更新器处理单个文件
        let updater = incremental_updater::IncrementalUpdater::new(
            self.config.clone(),
            self.vector_service.clone(),
        );

        updater
            .update_single_file(&path)
            .await
            .with_context(|| format!("增量更新文件失败: {}", path.display()))?;

        // 更新增量更新统计
        {
            let mut stats = self.stats.write().await;
            stats.incremental_updates += 1;
            stats.last_update_time = Some(std::time::Instant::now());
        }

        debug!("文件更新处理完成: {}", path.display());
        Ok(())
    }

    /// 处理文件删除事件
    async fn handle_file_deleted(&self, path: PathBuf) -> Result<()> {
        info!("处理文件删除: {}", path.display());

        // 使用增量更新器删除文件对应的向量
        let updater = incremental_updater::IncrementalUpdater::new(
            self.config.clone(),
            self.vector_service.clone(),
        );

        updater
            .delete_file_vectors(&path)
            .await
            .with_context(|| format!("删除文件向量失败: {}", path.display()))?;

        // 更新增量更新统计
        {
            let mut stats = self.stats.write().await;
            stats.incremental_updates += 1;
            stats.last_update_time = Some(std::time::Instant::now());
        }

        debug!("文件删除处理完成: {}", path.display());
        Ok(())
    }

    /// 处理文件重命名事件
    async fn handle_file_renamed(&self, from: PathBuf, to: PathBuf) -> Result<()> {
        info!("处理文件重命名: {} -> {}", from.display(), to.display());

        // 先删除旧文件的向量（如果是支持的文件类型）
        if self.file_filter.should_process_file(&from) {
            self.handle_file_deleted(from).await?;
        }

        // 再为新文件创建向量（如果是支持的文件类型）
        if self.file_filter.should_process_file(&to) {
            self.handle_file_updated(to).await?;
        }

        debug!("文件重命名处理完成");
        Ok(())
    }

    /// 获取监控统计信息
    pub async fn get_stats(&self) -> MonitorStats {
        self.stats.read().await.clone()
    }

    /// 重置监控统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = MonitorStats::default();
        stats.start_time = Some(Instant::now());
    }

    /// 检查监控服务是否正在运行
    pub fn is_monitoring(&self) -> bool {
        self.watcher.is_some() && self.watch_root.is_some()
    }

    /// 获取当前监控的根目录
    pub fn get_watch_root(&self) -> Option<&Path> {
        self.watch_root.as_deref()
    }
}

impl Drop for FileMonitorService {
    fn drop(&mut self) {
        if self.is_monitoring() {
            warn!("FileMonitorService被释放时仍在监控状态，尝试停止监控");

            // 注意：这里不能使用async，所以只能尽力而为
            if let Some(mut watcher) = self.watcher.take() {
                if let Some(watch_path) = &self.watch_root {
                    let _ = watcher.unwatch(watch_path);
                }
            }
        }
    }
}
