/*!
 * 任务协调器
 * 
 * 专注于任务状态管理和进度报告的服务。
 * 职责单一：管理长时间运行的任务状态、进度和取消操作。
 * 
 * Requirements: 4.2, 4.3
 */

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use uuid::Uuid;

use crate::vector_index::types::TaskProgress;

/// 任务状态
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// 任务元数据
#[derive(Debug)]
pub struct TaskMetadata {
    pub id: String,
    pub name: String,
    pub total_work: usize,
    pub completed_work: AtomicUsize,
    pub status: Arc<RwLock<TaskStatus>>,
    pub start_time: Instant,
    pub is_cancellable: bool,
    pub cancel_flag: AtomicBool,
}

impl TaskMetadata {
    pub fn new(id: String, name: String, total_work: usize, is_cancellable: bool) -> Self {
        Self {
            id,
            name,
            total_work,
            completed_work: AtomicUsize::new(0),
            status: Arc::new(RwLock::new(TaskStatus::Pending)),
            start_time: Instant::now(),
            is_cancellable,
            cancel_flag: AtomicBool::new(false),
        }
    }

    pub async fn get_progress(&self) -> TaskProgress {
        let completed = self.completed_work.load(Ordering::Relaxed);
        let progress = if self.total_work > 0 {
            completed as f32 / self.total_work as f32
        } else {
            0.0
        };

        let status = self.status.read().await;
        let status_text = match &*status {
            TaskStatus::Pending => "等待中".to_string(),
            TaskStatus::Running => format!("运行中 ({}/{})", completed, self.total_work),
            TaskStatus::Completed => "已完成".to_string(),
            TaskStatus::Failed(err) => format!("失败: {}", err),
            TaskStatus::Cancelled => "已取消".to_string(),
        };

        TaskProgress {
            task_id: self.id.clone(),
            progress,
            status: status_text,
            current_file: None, // 可以根据需要设置当前处理的文件
            processed_files: completed,
            total_files: self.total_work,
            cancellable: self.is_cancellable,
        }
    }

    pub fn increment_progress(&self, amount: usize) {
        self.completed_work.fetch_add(amount, Ordering::Relaxed);
    }

    pub fn set_progress(&self, completed: usize) {
        self.completed_work.store(completed, Ordering::Relaxed);
    }

    pub async fn set_status(&self, status: TaskStatus) {
        *self.status.write().await = status;
    }

    pub fn cancel(&self) -> bool {
        if self.is_cancellable {
            self.cancel_flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

/// 任务协调器接口
pub trait TaskCoordinator: Send + Sync {
    /// 创建新任务
    async fn create_task(&self, name: String, total_work: usize, is_cancellable: bool) -> String;
    
    /// 开始任务
    async fn start_task(&self, task_id: &str) -> Result<()>;
    
    /// 更新任务进度
    async fn update_progress(&self, task_id: &str, completed: usize) -> Result<()>;
    
    /// 增加任务进度
    async fn increment_progress(&self, task_id: &str, amount: usize) -> Result<()>;
    
    /// 完成任务
    async fn complete_task(&self, task_id: &str) -> Result<()>;
    
    /// 失败任务
    async fn fail_task(&self, task_id: &str, error: String) -> Result<()>;
    
    /// 取消任务
    async fn cancel_task(&self, task_id: &str) -> Result<bool>;
    
    /// 检查任务是否被取消
    async fn is_task_cancelled(&self, task_id: &str) -> bool;
    
    /// 获取任务进度
    async fn get_task_progress(&self, task_id: &str) -> Option<TaskProgress>;
    
    /// 获取所有活跃任务
    async fn get_active_tasks(&self) -> Vec<TaskProgress>;
    
    /// 清理已完成的任务
    async fn cleanup_completed_tasks(&self, older_than: Duration) -> usize;
}

/// 默认任务协调器实现
pub struct DefaultTaskCoordinator {
    tasks: Arc<RwLock<HashMap<String, Arc<TaskMetadata>>>>,
}

impl DefaultTaskCoordinator {
    /// 创建新的任务协调器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动进度广播服务
    pub async fn start_progress_broadcaster(
        &self,
        mut receiver: mpsc::Receiver<String>, // 接收需要广播的任务ID
        sender: mpsc::Sender<TaskProgress>,   // 发送进度更新
        interval_ms: u64,
    ) {
        let tasks = self.tasks.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));
            let mut broadcasting_tasks = std::collections::HashSet::new();
            
            loop {
                tokio::select! {
                    // 接收新的广播任务请求
                    task_id = receiver.recv() => {
                        if let Some(id) = task_id {
                            broadcasting_tasks.insert(id);
                        } else {
                            break; // 通道关闭
                        }
                    }
                    // 定期广播进度
                    _ = interval.tick() => {
                        let tasks_guard = tasks.read().await;
                        let mut completed_tasks = Vec::new();
                        
                        for task_id in &broadcasting_tasks {
                            if let Some(task) = tasks_guard.get(task_id) {
                                let progress = task.get_progress().await;
                                
                                // 发送进度更新
                                if sender.send(progress.clone()).await.is_err() {
                                    tracing::debug!("进度广播发送失败");
                                    break;
                                }
                                
                                // 检查任务是否完成
                                if progress.progress >= 1.0 || 
                                   progress.status.contains("完成") || 
                                   progress.status.contains("失败") || 
                                   progress.status.contains("取消") {
                                    completed_tasks.push(task_id.clone());
                                }
                            } else {
                                completed_tasks.push(task_id.clone());
                            }
                        }
                        
                        // 移除已完成的任务
                        for task_id in completed_tasks {
                            broadcasting_tasks.remove(&task_id);
                        }
                        
                        drop(tasks_guard);
                        
                        // 如果没有活跃任务且接收通道关闭，退出
                        if broadcasting_tasks.is_empty() && receiver.is_closed() {
                            break;
                        }
                    }
                }
            }
            
            tracing::debug!("任务进度广播服务已停止");
        });
    }
}

impl TaskCoordinator for DefaultTaskCoordinator {
    async fn create_task(&self, name: String, total_work: usize, is_cancellable: bool) -> String {
        let task_id = Uuid::new_v4().to_string();
        let task = Arc::new(TaskMetadata::new(
            task_id.clone(),
            name,
            total_work,
            is_cancellable,
        ));
        
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task);
        
        tracing::debug!("创建任务: {} ({})", task_id, name);
        task_id
    }

    async fn start_task(&self, task_id: &str) -> Result<()> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            task.set_status(TaskStatus::Running).await;
            tracing::debug!("启动任务: {}", task_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn update_progress(&self, task_id: &str, completed: usize) -> Result<()> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            task.set_progress(completed);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn increment_progress(&self, task_id: &str, amount: usize) -> Result<()> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            task.increment_progress(amount);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn complete_task(&self, task_id: &str) -> Result<()> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            task.set_status(TaskStatus::Completed).await;
            tracing::info!("任务完成: {}", task_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn fail_task(&self, task_id: &str, error: String) -> Result<()> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            task.set_status(TaskStatus::Failed(error.clone())).await;
            tracing::error!("任务失败: {} - {}", task_id, error);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            let cancelled = task.cancel();
            if cancelled {
                task.set_status(TaskStatus::Cancelled).await;
                tracing::info!("任务已取消: {}", task_id);
            }
            Ok(cancelled)
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    async fn is_task_cancelled(&self, task_id: &str) -> bool {
        let tasks = self.tasks.read().await;
        tasks.get(task_id)
            .map(|task| task.is_cancelled())
            .unwrap_or(false)
    }

    async fn get_task_progress(&self, task_id: &str) -> Option<TaskProgress> {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(task_id) {
            Some(task.get_progress().await)
        } else {
            None
        }
    }

    async fn get_active_tasks(&self) -> Vec<TaskProgress> {
        let tasks = self.tasks.read().await;
        let mut progresses = Vec::new();
        
        for task in tasks.values() {
            progresses.push(task.get_progress().await);
        }
        
        progresses
    }

    async fn cleanup_completed_tasks(&self, older_than: Duration) -> usize {
        let mut tasks = self.tasks.write().await;
        let cutoff_time = Instant::now() - older_than;
        let mut to_remove = Vec::new();
        
        for (task_id, task) in tasks.iter() {
            let status = task.status.read().await;
            let is_finished = matches!(*status, TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled);
            
            if is_finished && task.start_time < cutoff_time {
                to_remove.push(task_id.clone());
            }
        }
        
        let removed_count = to_remove.len();
        for task_id in to_remove {
            tasks.remove(&task_id);
        }
        
        if removed_count > 0 {
            tracing::debug!("清理了 {} 个已完成的任务", removed_count);
        }
        
        removed_count
    }
}

impl Default for DefaultTaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
