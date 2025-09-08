/*!
 * 向量索引系统 Tauri 命令接口
 *
 * 提供前端调用的API接口，实现向量索引的管理和搜索功能。
 * 遵循OrbitX错误处理开发规范，集成现有LLM服务。
 *
 * ## 架构说明
 *
 * 本模块作为Tauri API层，连接前端Vue组件和后端Rust服务：
 * - 使用统一的错误处理 (anyhow::Result -> TauriResult)
 * - 集成现有的LLM服务进行向量化
 * - 状态管理与其他OrbitX服务保持一致
 * - 遵循开发规范中的命名和结构要求
 *
 * Requirements: 6.1, 6.2 - 核心API命令和状态管理
 */

use crate::llm::commands::LLMManagerState;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use crate::vector_index::{
    monitor::FileMonitorService,
    service::VectorIndexService,
    types::{IndexStats, SearchOptions, SearchResult, TaskProgress, VectorIndexConfig, VectorIndexStatus},
};

mod app_settings;
pub use app_settings::*;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// 向量索引事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum VectorIndexEvent {
    /// 索引构建进度事件
    Progress(TaskProgress),
    /// 索引构建完成事件
    Completed(CompletedPayload),
    /// 搜索完成事件
    SearchComplete {
        query: String,
        result_count: usize,
        duration_ms: u64,
    },
    /// 错误事件
    Error {
        operation: String,
        message: String,
        timestamp: String,
    },
    /// 文件监控启动事件
    MonitorStarted {
        workspace_path: String,
    },
    /// 文件监控停止事件
    MonitorStopped,
    /// 增量更新完成事件
    IncrementalUpdateComplete {
        file_path: String,
        added_vectors: usize,
        deleted_vectors: usize,
        duration_ms: u64,
    },
    /// 服务状态变化事件
    ServiceStatus { initialized: bool, message: String },
}

/// 根据错误消息内容映射更精确的 i18n 错误键
fn map_build_index_error_key(e: &anyhow::Error) -> &'static str {
    let msg = e.to_string();
    
    // 直接匹配服务层返回的具体错误类型
    match msg.as_str() {
        "EMBEDDING_MODEL_NOT_FOUND" => "vector_index.embedding_model_not_found",
        "EMBEDDING_API_KEY_FAILED" => "vector_index.embedding_api_key_failed",
        "EMBEDDING_API_CALL_FAILED" => "vector_index.embedding_api_call_failed",
        "NO_VECTORS_GENERATED" => "vector_index.no_vectors_generated",
        "TOO_MANY_FILE_ERRORS" => "vector_index.too_many_file_errors",
        _ => "vector_index.build_index_failed", // 默认兜底
    }
}

/// 构建完成事件负载
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedPayload {
    pub total_files: usize,
    pub total_chunks: usize,
    pub uploaded_vectors: usize,
    pub elapsed_time: u64,
}

/// 向量索引服务状态
///
/// 管理VectorIndexService实例的生命周期，提供线程安全的访问。
/// 遵循OrbitX中其他状态管理器的模式。
pub struct VectorIndexState {
    service: Arc<RwLock<Option<Arc<VectorIndexService>>>>,
    /// 进度通知通道，用于实时报告索引构建进度
    progress_sender: Arc<RwLock<Option<mpsc::Sender<crate::vector_index::types::TaskProgress>>>>,
    /// 文件监控服务实例
    monitor_service: Arc<RwLock<Option<FileMonitorService>>>,
    /// 取消标志，用于停止正在进行的索引构建任务
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl VectorIndexState {
    /// 创建新的状态实例
    pub fn new() -> Self {
        Self {
            service: Arc::new(RwLock::new(None)),
            progress_sender: Arc::new(RwLock::new(None)),
            monitor_service: Arc::new(RwLock::new(None)),
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 检查服务是否已设置
    pub async fn has_service(&self) -> bool {
        self.service.read().await.is_some()
    }

    /// 检查服务是否已初始化
    pub async fn is_initialized(&self) -> bool {
        self.service.read().await.is_some()
    }

    /// 设置向量索引服务实例
    pub async fn set_service(&self, service: Arc<VectorIndexService>) {
        *self.service.write().await = Some(service);
    }
}

impl Clone for VectorIndexState {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            progress_sender: self.progress_sender.clone(),
            monitor_service: self.monitor_service.clone(),
            cancel_flag: self.cancel_flag.clone(),
        }
    }
}

/// 初始化向量索引服务
///
/// 创建并配置VectorIndexService实例，集成现有的LLM服务。
/// 这是使用向量索引功能前的必要步骤。
///
/// # 参数
///
/// - `config`: 向量索引配置，包含Qdrant连接信息和处理参数
/// - `embedding_model`: 用于向量化的embedding模型名称
/// - `state`: 向量索引状态管理器
/// - `llm_state`: LLM服务状态管理器，用于获取LLM服务实例
/// - `app`: Tauri应用句柄，用于发送状态变化事件
///
/// # 错误
///
/// - 配置参数验证失败
/// - LLM服务获取失败
/// - 向量索引服务创建失败
/// - Qdrant数据库连接失败
///
/// # 事件
///
/// 发送 `vector-index-event` 事件通知服务状态变化
#[tauri::command]
pub async fn init_vector_index<R: Runtime>(
    config: VectorIndexConfig,
    state: State<'_, VectorIndexState>,
    llm_state: State<'_, LLMManagerState>,
    ai_state: State<'_, crate::ai::AIManagerState>,
    app: AppHandle<R>,
) -> TauriApiResult<EmptyData> {
    let result: Result<()> = async {
        info!("开始初始化向量索引服务");

        // 1. 参数验证
        ensure!(
            !config.qdrant_url.trim().is_empty(),
            "Qdrant数据库URL不能为空"
        );
        ensure!(
            !config.collection_name.trim().is_empty(),
            "集合名称不能为空"
        );
        ensure!(
            !config.embedding_model_id.trim().is_empty(),
            "Embedding模型ID不能为空"
        );
        ensure!(
            config.max_concurrent_files > 0,
            "最大并发文件数必须大于0"
        );
        
        // 获取embedding模型配置
        let models = ai_state.ai_service.get_models().await;
        
        // 调试信息：列出所有可用的embedding模型
        let available_embedding_models: Vec<String> = models
            .iter()
            .filter(|m| m.model_type == crate::ai::types::ModelType::Embedding)
            .map(|m| format!("{} ({})", m.id, m.name))
            .collect();
        
        debug!("可用的embedding模型: {:?}", available_embedding_models);
        
        let embedding_model_config = models
            .iter()
            .find(|m| m.id == config.embedding_model_id && m.model_type == crate::ai::types::ModelType::Embedding)
            .ok_or_else(|| {
                let available_list = if available_embedding_models.is_empty() {
                    "无可用的embedding模型，请先在AI设置中添加embedding模型".to_string()
                } else {
                    format!("可用模型: {}", available_embedding_models.join(", "))
                };
                anyhow::anyhow!(
                    "找不到指定的embedding模型 '{}'。{}\n\n解决方案:\n1. 在AI设置中添加该模型\n2. 或选择已有的embedding模型\n3. 或使用默认模型 'text-embedding-3-small'", 
                    config.embedding_model_id, 
                    available_list
                )
            })?;
        
        let embedding_model = embedding_model_config.id.clone();

        debug!(
            "配置验证通过: qdrant_url={}, collection_name={}, embedding_model={}, max_concurrent_files={}",
            config.qdrant_url, config.collection_name, embedding_model, config.max_concurrent_files
        );

        // 2. 获取LLM服务
        let llm_service = llm_state.service.clone();

        // 3. 创建向量索引服务
        let service = VectorIndexService::new(config, llm_service, embedding_model)
            .await
            .context("创建向量索引服务失败")?;

        // 4. 存储服务实例
        *state.service.write().await = Some(Arc::new(service));

        info!("向量索引服务初始化成功");

        // 5. 发送服务状态变化事件
        let event = VectorIndexEvent::ServiceStatus { initialized: true, message: "向量索引服务初始化成功".to_string() };
        if let Err(e) = app.emit("vector-index-event", &event) {
            warn!("发送服务状态事件失败: {}", e);
        }

        Ok(())
    }
    .await;

    // 如果初始化失败，也发送状态事件
    if let Err(ref e) = result {
        let event = VectorIndexEvent::ServiceStatus { initialized: false, message: format!("向量索引服务初始化失败: {}", e) };
        if let Err(emit_err) = app.emit("vector-index-event", &event) {
            warn!("发送服务状态事件失败: {}", emit_err);
        }
    }

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("vector_index.init_failed")),
    }
}

/// 构建代码索引
///
/// 扫描指定工作空间的代码文件，生成向量索引并存储到Qdrant数据库。
/// 支持实时进度报告和错误恢复，通过事件系统向前端发送进度更新。
///
/// # 参数
///
/// - `workspace_path`: 工作空间路径，将扫描此目录下的代码文件
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送事件通知
///
/// # 返回
///
/// 返回索引构建统计信息，包括处理的文件数量、向量数量和耗时等
///
/// # 错误
///
/// - 向量索引服务未初始化
/// - 工作空间路径无效
/// - 代码文件扫描失败
/// - 向量化处理失败
/// - Qdrant上传失败
///
/// # 事件
///
/// 在构建过程中会发送以下事件：
/// - `vector-index-progress`: 进度更新事件
/// - `vector-index-complete`: 构建完成事件
/// - `vector-index-error`: 错误事件
#[tauri::command]
pub async fn build_code_index<R: Runtime>(
    workspace_path: String,
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<IndexStats> {
    // 参数与前置条件优雅返回：直接用 i18n key 提前失败，避免后续字符串匹配
    if workspace_path.trim().is_empty() {
        return Ok(api_error!("vector_index.workspace_path_empty"));
    }

    let workspace_path_trimmed = workspace_path.trim().to_string();
    let path = std::path::Path::new(&workspace_path_trimmed);
    if !path.exists() {
        return Ok(api_error!("vector_index.workspace_not_exist"));
    }
    if !path.is_dir() {
        return Ok(api_error!("vector_index.workspace_not_directory"));
    }

    // 获取服务实例（未初始化时直接返回明确错误）
    let service = {
        let service_guard = state.service.read().await;
        if let Some(svc) = service_guard.as_ref() {
            svc.clone()
        } else {
            return Ok(api_error!("vector_index.service_not_initialized"));
        }
    };

    let result: Result<IndexStats> = async {
        info!("开始构建代码索引: {}", workspace_path);

        let workspace_path = workspace_path_trimmed.as_str();

        // 3. 重置取消标志
        state.cancel_flag.store(false, std::sync::atomic::Ordering::Relaxed);

        // 4. 准备进度通知和事件发送
        let (progress_sender, mut progress_receiver) = mpsc::channel::<TaskProgress>(100);
        let app_clone = app.clone();

        // 启动进度报告任务 - 将进度转发为Tauri事件
        let progress_task = tokio::spawn(async move {
            while let Some(progress) = progress_receiver.recv().await {
                debug!(
                    "构建进度: {:.1}% - {}",
                    progress.progress * 100.0,
                    progress.status
                );

                // 向前端发送进度事件
                let event = VectorIndexEvent::Progress(progress);
                if let Err(e) = app_clone.emit("vector-index-event", &event) {
                    warn!("发送进度事件失败: {}", e);
                }
            }
        });

        // 5. 执行索引构建
        let build_result = service
            .build_index(&workspace_path, Some(progress_sender), Some(state.cancel_flag.clone()))
            .await;

        // 6. 等待进度报告任务结束
        progress_task.abort();

        // 7. 处理构建结果并发送事件
        match &build_result {
            Ok(stats) => {
                info!(
                    "代码索引构建完成: {}/{} 文件, {} 个向量, 耗时 {}ms",
                    stats.total_files - stats.failed_files.len(),
                    stats.total_files,
                    stats.uploaded_vectors,
                    stats.processing_time
                );

                // 发送构建完成事件（与前端契约一致）
                let payload = CompletedPayload {
                    total_files: stats.total_files,
                    total_chunks: stats.total_chunks,
                    uploaded_vectors: stats.uploaded_vectors,
                    elapsed_time: stats.processing_time,
                };
                let event = VectorIndexEvent::Completed(payload);
                if let Err(e) = app.emit("vector-index-event", &event) {
                    warn!("发送构建完成事件失败: {}", e);
                }
            }
            Err(e) => {
                let error_msg = format!("构建代码索引失败: {}", e);
                warn!("{}", error_msg);

                // 发送错误事件
                let event = VectorIndexEvent::Error {
                    operation: "build_index".to_string(),
                    message: error_msg,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                };
                if let Err(emit_err) = app.emit("vector-index-event", &event) {
                    warn!("发送错误事件失败: {}", emit_err);
                }
            }
        }

        build_result
    }
    .await;

    match result {
        Ok(stats) => Ok(api_success!(stats)),
        Err(e) => {
            let error_key = map_build_index_error_key(&e);
            Ok(api_error!(error_key))
        }
    }
}

/// 搜索代码向量
///
/// 使用自然语言查询在代码向量索引中进行语义搜索，
/// 返回最相关的代码片段。向前端发送搜索完成事件。
///
/// # 参数
///
/// - `options`: 搜索选项，包含查询文本、结果数量限制、过滤条件等
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送搜索完成事件
///
/// # 返回
///
/// 返回匹配的代码片段列表，按相似度分数降序排列
///
/// # 错误
///
/// - 向量索引服务未初始化
/// - 查询文本过短或过长
/// - 向量化处理失败
/// - Qdrant搜索失败
///
/// # 事件
///
/// 发送 `vector-index-event` 事件通知搜索完成
#[tauri::command]
pub async fn search_code_vectors<R: Runtime>(
    options: SearchOptions,
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<Vec<SearchResult>> {
    let result: Result<Vec<SearchResult>> = async {
        debug!("开始搜索代码向量: '{}'", options.query);

        // 1. 参数验证
        ensure!(
            options.query.len() >= 3,
            "查询文本过短，需要至少3个字符，当前{}个字符",
            options.query.len()
        );
        ensure!(
            options.query.len() <= 1000,
            "查询文本过长，最多支持1000个字符，当前{}个字符",
            options.query.len()
        );

        if let Some(max_results) = options.max_results {
            ensure!(
                max_results > 0 && max_results <= 100,
                "结果数量限制必须在1-100之间，当前为{}",
                max_results
            );
        }

        if let Some(min_score) = options.min_score {
            ensure!(
                min_score >= 0.0 && min_score <= 1.0,
                "最小相似度分数必须在0.0-1.0之间，当前为{}",
                min_score
            );
        }

        // 2. 获取服务实例
        let service_guard = state.service.read().await;
        let service = service_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("向量索引服务未初始化，请先调用 init_vector_index"))?
            .clone();

        // 3. 执行搜索并计时
        let search_start = std::time::Instant::now();
        let search_result = service.search_vectors(options.clone()).await;
        let search_duration = search_start.elapsed();

        // 4. 处理搜索结果并发送事件
        match &search_result {
            Ok(results) => {
                debug!("搜索完成，找到 {} 个结果", results.len());

                // 发送搜索完成事件
                let event = VectorIndexEvent::SearchComplete {
                    query: options.query.clone(),
                    result_count: results.len(),
                    duration_ms: search_duration.as_millis() as u64,
                };
                if let Err(e) = app.emit("vector-index-event", &event) {
                    warn!("发送搜索完成事件失败: {}", e);
                }
            }
            Err(e) => {
                let error_msg = format!("搜索代码向量失败: {}", e);
                warn!("{}", error_msg);

                // 发送搜索错误事件
                let event = VectorIndexEvent::Error {
                    operation: "search_vectors".to_string(),
                    message: error_msg,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                };
                if let Err(emit_err) = app.emit("vector-index-event", &event) {
                    warn!("发送搜索错误事件失败: {}", emit_err);
                }
            }
        }

        search_result.context("搜索代码向量失败")
    }
    .await;

    match result {
        Ok(results) => Ok(api_success!(results)),
        Err(_) => Ok(api_error!("vector_index.search_failed")),
    }
}

/// 测试Qdrant数据库连接
///
/// 验证给定的Qdrant配置是否能够正常连接，
/// 用于配置验证和故障排除。
///
/// # 参数
///
/// - `config`: Qdrant数据库配置
///
/// # 返回
///
/// 返回连接状态信息，包括连接成功或失败的详细信息
///
/// # 错误
///
/// - 配置参数无效
/// - 网络连接失败
/// - 认证失败
/// - 服务不可用
#[tauri::command]
pub async fn test_qdrant_connection(config: VectorIndexConfig) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("开始测试Qdrant数据库连接: {}", config.qdrant_url);

        // 1. 参数验证
        ensure!(
            !config.qdrant_url.trim().is_empty(),
            "Qdrant数据库URL不能为空"
        );
        ensure!(
            !config.collection_name.trim().is_empty(),
            "集合名称不能为空"
        );

        // 2. 创建临时的Qdrant客户端进行测试
        let full_config = crate::vector_index::types::VectorIndexFullConfig::new(config.clone());
        let qdrant_client = crate::vector_index::qdrant::QdrantClientImpl::new(full_config)
            .await
            .context("创建Qdrant客户端失败")?;

        // 3. 测试连接
        use crate::vector_index::qdrant::QdrantService;
        let status = qdrant_client
            .test_connection()
            .await
            .context("Qdrant连接测试失败")?;

        info!("Qdrant连接测试成功: {}", status);

        Ok(status)
    }
    .await;

    match result {
        Ok(status) => Ok(api_success!(status)),
        Err(_) => Ok(api_error!("vector_index.connection_test_failed")),
    }
}

/// 获取向量索引状态信息（对象）
#[tauri::command]
pub async fn get_vector_index_status(state: State<'_, VectorIndexState>) -> TauriApiResult<VectorIndexStatus> {
    let result: Result<VectorIndexStatus> = async {
        if !state.is_initialized().await {
            return Ok(VectorIndexStatus { is_initialized: false, total_vectors: 0, collection_name: None, last_updated: None });
        }

        let service_guard = state.service.read().await;
        if let Some(service) = service_guard.as_ref() {
            match service.get_collection_info().await {
                Ok((_points, vectors)) => Ok(VectorIndexStatus {
                    is_initialized: true,
                    total_vectors: vectors,
                    collection_name: Some(service.collection_name().to_string()),
                    last_updated: None,
                }),
                Err(_) => Ok(VectorIndexStatus { is_initialized: true, total_vectors: 0, collection_name: Some(service.collection_name().to_string()), last_updated: None }),
            }
        } else {
            Ok(VectorIndexStatus { is_initialized: false, total_vectors: 0, collection_name: None, last_updated: None })
        }
    }
    .await;

    match result {
        Ok(status) => Ok(api_success!(status)),
        Err(_) => Ok(api_error!("vector_index.get_status_failed")),
    }
}

/// 启动文件监控服务
///
/// 创建并启动文件监控服务，监控指定目录的代码文件变化。
/// 当检测到文件变化时，自动触发增量索引更新。
///
/// # 参数
///
/// - `workspace_path`: 要监控的工作空间路径
/// - `config`: 向量索引配置
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送事件通知
///
/// # 错误
///
/// - 配置参数验证失败
/// - 向量索引服务未初始化
/// - 监控路径无效
/// - 文件监控服务创建失败
///
/// # 事件
///
/// - `VectorIndexEvent::MonitorStarted`: 监控启动成功
/// - `VectorIndexEvent::Error`: 启动失败
#[tauri::command]
pub async fn start_file_monitoring<R: Runtime>(
    workspace_path: String,
    config: VectorIndexConfig,
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("启动文件监控: {}", workspace_path);

        // 1. 参数验证
        ensure!(
            !workspace_path.trim().is_empty(),
            "工作空间路径不能为空"
        );

        let workspace_path = std::path::Path::new(&workspace_path);
        ensure!(
            workspace_path.exists() && workspace_path.is_dir(),
            "工作空间路径必须是一个存在的目录: {}",
            workspace_path.display()
        );

        // 2. 检查向量索引服务是否已初始化
        ensure!(
            state.is_initialized().await,
            "必须先初始化向量索引服务才能启动文件监控"
        );

        // 3. 获取向量索引服务实例
        let vector_service = {
            let service_guard = state.service.read().await;
            service_guard
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("向量索引服务未初始化"))?
                .clone()
        };

        // 4. 创建文件监控服务
        let mut monitor_service = FileMonitorService::new(config, vector_service)
            .context("创建文件监控服务失败")?;

        // 5. 启动监控
        monitor_service
            .start_monitoring(&workspace_path)
            .await
            .context("启动文件监控失败")?;

        // 6. 存储监控服务实例
        {
            let mut monitor_guard = state.monitor_service.write().await;
            *monitor_guard = Some(monitor_service);
        }

        // 7. 发送启动成功事件
        let _ = app.emit(
            "vector-index-event",
            VectorIndexEvent::MonitorStarted {
                workspace_path: workspace_path.to_string_lossy().to_string(),
            },
        );

        let success_msg = format!("文件监控已启动，监控目录: {}", workspace_path.display());
        info!("{}", success_msg);

        Ok(success_msg)
    }
    .await;

    match result {
        Ok(msg) => Ok(api_success!(msg)),
        Err(_) => Ok(api_error!("vector_index.start_monitoring_failed")),
    }
}

/// 停止文件监控服务
///
/// 停止当前运行的文件监控服务，释放相关资源。
///
/// # 参数
///
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送事件通知
///
/// # 返回
///
/// 返回操作状态信息
#[tauri::command]
pub async fn stop_file_monitoring<R: Runtime>(
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("停止文件监控");

        // 1. 获取并停止监控服务
        let mut monitor_guard = state.monitor_service.write().await;
        
        if let Some(mut monitor_service) = monitor_guard.take() {
            monitor_service
                .stop_monitoring()
                .await
                .context("停止文件监控失败")?;

            // 2. 发送停止事件
            let _ = app.emit(
                "vector-index-event",
                VectorIndexEvent::MonitorStopped,
            );

            let success_msg = "文件监控已停止".to_string();
            info!("{}", success_msg);
            Ok(success_msg)
        } else {
            Ok("文件监控服务未运行".to_string())
        }
    }
    .await;

    match result {
        Ok(msg) => Ok(api_success!(msg)),
        Err(_) => Ok(api_error!("vector_index.stop_monitoring_failed")),
    }
}

/// 获取文件监控状态
///
/// 返回文件监控服务的当前状态信息，包括是否正在运行、
/// 监控目录、统计信息等。
///
/// # 参数
///
/// - `state`: 向量索引状态管理器
///
/// # 返回
///
/// 返回监控状态信息
#[tauri::command]
pub async fn get_file_monitoring_status(
    state: State<'_, VectorIndexState>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        let monitor_guard = state.monitor_service.read().await;
        
        if let Some(monitor_service) = monitor_guard.as_ref() {
            if monitor_service.is_monitoring() {
                let stats = monitor_service.get_stats().await;
                let watch_root = monitor_service
                    .get_watch_root()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "未知".to_string());

                Ok(format!(
                    "监控状态: 运行中\n监控目录: {}\n总事件数: {}\n处理事件数: {}\n跳过事件数: {}\n增量更新次数: {}",
                    watch_root,
                    stats.total_events,
                    stats.processed_events,
                    stats.skipped_events,
                    stats.incremental_updates
                ))
            } else {
                Ok("监控状态: 已停止".to_string())
            }
        } else {
            Ok("监控状态: 未初始化".to_string())
        }
    }
    .await;

    match result {
        Ok(status) => Ok(api_success!(status)),
        Err(_) => Ok(api_error!("vector_index.get_monitoring_status_failed")),
    }
}

/// 获取向量索引配置
///
/// 从数据库加载向量索引配置信息，用于前端配置页面显示。
/// 如果配置不存在，返回默认配置。
///
/// # 参数
///
/// - `storage_state`: 存储系统状态，用于访问数据库
///
/// # 返回
///
/// 返回VectorIndexConfig结构，包含所有配置参数
#[tauri::command]
pub async fn get_vector_index_config(
    storage_state: State<'_, crate::ai::tool::storage::StorageCoordinatorState>,
) -> TauriApiResult<VectorIndexConfig> {
    let result: Result<VectorIndexConfig> = async {
        debug!("获取向量索引配置");

        // 1. 创建配置服务
        let config_service = crate::vector_index::VectorIndexConfigService::new(
            storage_state.coordinator.repositories()
        );

        // 2. 加载配置或使用默认配置
        let config = config_service
            .get_config_or_default()
            .await
            .context("获取向量索引配置失败")?;

        debug!("向量索引配置获取成功");
        Ok(config)
    }
    .await;

    match result {
        Ok(config) => Ok(api_success!(config)),
        Err(_) => Ok(api_error!("vector_index.get_config_failed")),
    }
}

/// 保存向量索引配置
///
/// 验证并保存向量索引配置到数据库中。
/// 配置包括Qdrant连接信息、embedding模型选择和性能参数。
///
/// # 参数
///
/// - `config`: 要保存的向量索引配置
/// - `storage_state`: 存储系统状态，用于访问数据库
///
/// # 返回
///
/// 返回操作成功的消息
///
/// # 错误
///
/// - 配置参数验证失败
/// - 数据库保存失败
#[tauri::command]
pub async fn save_vector_index_config(
    config: VectorIndexConfig,
    storage_state: State<'_, crate::ai::tool::storage::StorageCoordinatorState>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("保存向量索引配置");

        // 1. 创建配置服务
        let config_service = crate::vector_index::VectorIndexConfigService::new(
            storage_state.coordinator.repositories()
        );

        // 2. 保存配置（包含验证逻辑）
        config_service
            .save_config(&config)
            .await
            .context("保存向量索引配置失败")?;

        info!("向量索引配置保存成功");
        Ok("向量索引配置保存成功".to_string())
    }
    .await;

    match result {
        Ok(msg) => Ok(api_success!(msg)),
        Err(_) => Ok(api_error!("vector_index.save_config_failed")),
    }
}

/// 获取当前工作空间路径
///
/// 返回当前应用的工作空间路径，用于代码索引构建。
///
/// # 返回
///
/// 返回工作空间路径字符串
///
/// # 错误
///
/// - 无法确定工作空间路径
/// - 路径不存在或无权限访问
#[tauri::command]
pub async fn get_current_workspace_path() -> TauriApiResult<String> {
    let result: Result<String> = async {
        // 尝试获取当前工作目录
        let current_dir = std::env::current_dir()
            .context("无法获取当前工作目录")?;

        let workspace_path = current_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("工作空间路径包含无效字符"))?
            .to_string();

        debug!("当前工作空间路径: {}", workspace_path);

        Ok(workspace_path)
    }
    .await;

    match result {
        Ok(path) => Ok(api_success!(path)),
        Err(_) => Ok(api_error!("vector_index.get_workspace_path_failed")),
    }
}

/// 取消正在进行的索引构建
///
/// 停止当前正在执行的代码索引构建任务，释放相关资源。
///
/// # 参数
///
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送取消事件
///
/// # 返回
///
/// 返回取消操作的状态信息
#[tauri::command]
pub async fn cancel_build_index<R: Runtime>(
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("请求取消索引构建");

        // 检查服务是否已初始化
        if !state.is_initialized().await {
            return Ok("向量索引服务未初始化，无需取消".to_string());
        }

        // 设置取消标志
        info!("触发索引构建取消");
        state.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // 发送取消事件
        let event = VectorIndexEvent::Error {
            operation: "cancel_build".to_string(),
            message: "用户取消了索引构建".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string(),
        };
        if let Err(e) = app.emit("vector-index-event", &event) {
            warn!("发送取消事件失败: {}", e);
        }

        Ok("索引构建已取消".to_string())
    }
    .await;

    match result {
        Ok(msg) => Ok(api_success!(msg)),
        Err(_) => Ok(api_error!("vector_index.cancel_build_failed")),
    }
}

/// 清空向量索引
///
/// 删除Qdrant集合中的所有向量数据，重置索引状态。
///
/// # 参数
///
/// - `state`: 向量索引状态管理器
/// - `app`: Tauri应用句柄，用于发送事件通知
///
/// # 返回
///
/// 返回清空操作的结果信息
///
/// # 错误
///
/// - 向量索引服务未初始化
/// - Qdrant数据库操作失败
#[tauri::command]
pub async fn clear_vector_index<R: Runtime>(
    state: State<'_, VectorIndexState>,
    app: AppHandle<R>,
) -> TauriApiResult<String> {
    let result: Result<String> = async {
        info!("开始清空向量索引");

        // 1. 检查服务是否已初始化
        ensure!(
            state.is_initialized().await,
            "向量索引服务未初始化，请先调用 init_vector_index"
        );

        // 2. 获取服务实例
        let service_guard = state.service.read().await;
        let service = service_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("向量索引服务未初始化"))?
            .clone();

        // 3. 执行清空操作
        service
            .clear_all_vectors()
            .await
            .context("清空向量索引失败")?;

        info!("向量索引清空成功");

        // 4. 发送清空完成事件
        let event = VectorIndexEvent::ServiceStatus {
            initialized: true,
            message: "向量索引已清空".to_string(),
        };
        if let Err(e) = app.emit("vector-index-event", &event) {
            warn!("发送清空完成事件失败: {}", e);
        }

        Ok("向量索引清空成功".to_string())
    }
    .await;

    match result {
        Ok(msg) => Ok(api_success!(msg)),
        Err(_) => Ok(api_error!("vector_index.clear_index_failed")),
    }
}
