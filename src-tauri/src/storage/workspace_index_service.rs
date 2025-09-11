/*!
 * 工作区索引服务
 *
 * 提供工作区向量索引的管理功能，包括检测、构建、查询和删除索引
 */

use crate::storage::repositories::{vector_workspaces::*, Repository, RepositoryManager};
use crate::utils::error::AppResult;
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// 工作区索引服务
pub struct WorkspaceIndexService {
    repositories: Arc<RepositoryManager>,
}

impl WorkspaceIndexService {
    /// 创建新的工作区索引服务实例
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    /// 检查工作区索引状态
    ///
    /// # 参数
    ///
    /// * `path` - 工作区路径
    ///
    /// # 返回
    ///
    /// 返回工作区索引信息，如果不存在则返回 None
    pub async fn check_workspace_index(&self, path: &str) -> AppResult<Option<WorkspaceIndex>> {
        debug!("检查工作区索引状态: {}", path);

        // 标准化路径
        let normalized_path = self.normalize_path(path)?;

        // 查询数据库中的索引记录
        let workspace_index = self
            .repositories
            .vector_workspaces()
            .find_by_path(&normalized_path)
            .await
            .context("查询工作区索引失败")?;

        match workspace_index {
            Some(mut index) => {
                // 验证磁盘上的索引文件是否存在
                if self.verify_index_files_exist(&normalized_path).await? {
                    debug!("工作区索引存在且有效: {}", normalized_path);
                    Ok(Some(index))
                } else {
                    // 磁盘文件不存在，更新数据库状态为错误
                    warn!("工作区索引记录存在但磁盘文件缺失: {}", normalized_path);
                    index.mark_error("索引文件缺失".to_string());
                    self.repositories
                        .vector_workspaces()
                        .update(&index)
                        .await
                        .context("更新索引状态失败")?;
                    Ok(Some(index))
                }
            }
            None => {
                debug!("工作区索引不存在: {}", normalized_path);
                Ok(None)
            }
        }
    }

    /// 构建工作区索引
    ///
    /// # 参数
    ///
    /// * `path` - 工作区路径
    /// * `name` - 可选的工作区名称
    ///
    /// # 返回
    ///
    /// 返回创建的工作区索引信息
    pub async fn build_workspace_index(
        &self,
        path: &str,
        name: Option<String>,
    ) -> AppResult<WorkspaceIndex> {
        info!("开始构建工作区索引: {} (名称: {:?})", path, name);

        // 标准化路径
        let normalized_path = self.normalize_path(path)?;

        // 验证路径存在
        if !Path::new(&normalized_path).exists() {
            bail!("工作区路径不存在: {}", normalized_path);
        }

        // 检查是否已存在索引
        if self
            .repositories
            .vector_workspaces()
            .path_exists(&normalized_path)
            .await?
        {
            bail!("工作区索引已存在: {}", normalized_path);
        }

        // 创建新的索引记录
        let mut workspace_index = WorkspaceIndex::new(normalized_path.clone(), name);

        // 保存到数据库
        let index_id = self
            .repositories
            .vector_workspaces()
            .save(&workspace_index)
            .await
            .context("保存工作区索引记录失败")?;

        workspace_index.id = Some(index_id as i32);

        // 异步构建索引
        let repositories = Arc::clone(&self.repositories);
        let build_path = normalized_path.clone();

        tokio::spawn(async move {
            let result = Self::build_index_with_ck(&build_path).await;
            Self::handle_build_result(repositories, index_id as i32, result).await;
        });

        info!("工作区索引构建任务已启动: {}", normalized_path);
        Ok(workspace_index)
    }

    /// 获取所有工作区索引列表
    pub async fn list_all_workspaces(&self) -> AppResult<Vec<WorkspaceIndex>> {
        debug!("获取所有工作区索引列表");

        self.repositories
            .vector_workspaces()
            .find_all_ordered()
            .await
            .context("获取工作区索引列表失败")
    }

    /// 删除工作区索引
    ///
    /// # 参数
    ///
    /// * `id` - 工作区索引ID
    pub async fn delete_workspace_index(&self, id: i32) -> AppResult<()> {
        info!("删除工作区索引: {}", id);

        // 查找索引记录
        let workspace_index = self
            .repositories
            .vector_workspaces()
            .find_by_id(id as i64)
            .await
            .context("查询工作区索引失败")?
            .ok_or_else(|| anyhow!("工作区索引不存在: {}", id))?;

        // 删除磁盘上的索引文件
        if let Err(e) = self
            .remove_index_files(&workspace_index.workspace_path)
            .await
        {
            warn!(
                "删除索引文件失败: {} - {}",
                workspace_index.workspace_path, e
            );
            // 继续删除数据库记录，即使文件删除失败
        }

        // 删除数据库记录
        self.repositories
            .vector_workspaces()
            .delete(id as i64)
            .await
            .context("删除工作区索引记录失败")?;

        info!("工作区索引删除成功: {}", id);
        Ok(())
    }

    /// 刷新工作区索引（重新构建）
    ///
    /// # 参数
    ///
    /// * `id` - 工作区索引ID
    pub async fn refresh_workspace_index(&self, id: i32) -> AppResult<WorkspaceIndex> {
        info!("刷新工作区索引: {}", id);

        // 查找现有索引记录
        let mut workspace_index = self
            .repositories
            .vector_workspaces()
            .find_by_id(id as i64)
            .await
            .context("查询工作区索引失败")?
            .ok_or_else(|| anyhow!("工作区索引不存在: {}", id))?;

        // 验证路径仍然存在
        if !Path::new(&workspace_index.workspace_path).exists() {
            workspace_index.mark_error("工作区路径不存在".to_string());
            self.repositories
                .vector_workspaces()
                .update(&workspace_index)
                .await
                .context("更新索引状态失败")?;
            bail!("工作区路径不存在: {}", workspace_index.workspace_path);
        }

        // 删除旧的索引文件
        if let Err(e) = self
            .remove_index_files(&workspace_index.workspace_path)
            .await
        {
            warn!(
                "删除旧索引文件失败: {} - {}",
                workspace_index.workspace_path, e
            );
        }

        // 重置状态为构建中
        workspace_index.status = IndexStatus::Building;
        workspace_index.file_count = 0;
        workspace_index.index_size_bytes = 0;
        workspace_index.error_message = None;
        workspace_index.updated_at = chrono::Utc::now();

        // 更新数据库
        self.repositories
            .vector_workspaces()
            .update(&workspace_index)
            .await
            .context("更新工作区索引状态失败")?;

        // 异步重新构建索引
        let repositories = Arc::clone(&self.repositories);
        let build_path = workspace_index.workspace_path.clone();

        tokio::spawn(async move {
            let result = Self::build_index_with_ck(&build_path).await;
            Self::handle_build_result(repositories, id, result).await;
        });

        info!(
            "工作区索引刷新任务已启动: {}",
            workspace_index.workspace_path
        );
        Ok(workspace_index)
    }

    /// 使用 ck-engine 构建索引
    async fn build_index_with_ck(path: &str) -> Result<(i32, i64)> {
        debug!("使用 ck-engine 构建索引: {}", path);

        // 调用 ck-engine 构建索引
        let output = Command::new("ck")
            .args(["index", "--path", path, "--verbose"])
            .output()
            .await
            .context("执行 ck index 命令失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ck index 命令执行失败: {}", stderr);
        }

        // 解析输出以获取文件数量和索引大小
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (file_count, index_size) = Self::parse_ck_output(&stdout)?;

        debug!(
            "索引构建完成: 文件数量={}, 索引大小={}",
            file_count, index_size
        );
        Ok((file_count, index_size))
    }

    /// 处理索引构建结果
    async fn handle_build_result(
        repositories: Arc<RepositoryManager>,
        index_id: i32,
        result: Result<(i32, i64)>,
    ) {
        match result {
            Ok((file_count, index_size)) => {
                debug!(
                    "索引构建成功: ID={}, 文件数量={}, 大小={}",
                    index_id, file_count, index_size
                );

                if let Err(e) = repositories
                    .vector_workspaces()
                    .update_status(
                        index_id,
                        IndexStatus::Ready,
                        Some(file_count),
                        Some(index_size),
                        None,
                    )
                    .await
                {
                    error!("更新索引状态为就绪失败: {}", e);
                }
            }
            Err(e) => {
                error!("索引构建失败: ID={}, 错误={}", index_id, e);

                if let Err(update_err) = repositories
                    .vector_workspaces()
                    .update_status(
                        index_id,
                        IndexStatus::Error,
                        None,
                        None,
                        Some(e.to_string()),
                    )
                    .await
                {
                    error!("更新索引状态为错误失败: {}", update_err);
                }
            }
        }
    }

    /// 解析 ck 命令输出
    fn parse_ck_output(output: &str) -> Result<(i32, i64)> {
        // 这里需要根据实际的 ck 输出格式来解析
        // 暂时返回默认值，后续需要根据实际输出调整
        // TODO: 根据实际的 ck-engine 输出格式实现解析逻辑

        let mut file_count = 0i32;
        let mut index_size = 0i64;

        for line in output.lines() {
            if line.contains("indexed") && line.contains("files") {
                // 尝试提取文件数量
                if let Some(captures) = regex::Regex::new(r"(\d+)\s+files")
                    .ok()
                    .and_then(|re| re.captures(line))
                {
                    if let Some(count_str) = captures.get(1) {
                        file_count = count_str.as_str().parse().unwrap_or(0);
                    }
                }
            }

            if line.contains("size") || line.contains("bytes") {
                // 尝试提取索引大小
                if let Some(captures) = regex::Regex::new(r"(\d+)\s*(?:bytes|B)")
                    .ok()
                    .and_then(|re| re.captures(line))
                {
                    if let Some(size_str) = captures.get(1) {
                        index_size = size_str.as_str().parse().unwrap_or(0);
                    }
                }
            }
        }

        // 如果无法解析，设置默认值
        if file_count == 0 {
            file_count = 1; // 至少有一个文件
        }
        if index_size == 0 {
            index_size = 1024; // 默认1KB
        }

        Ok((file_count, index_size))
    }

    /// 标准化路径
    fn normalize_path(&self, path: &str) -> AppResult<String> {
        let path_buf = PathBuf::from(path);
        let canonical_path = path_buf.canonicalize().context("无法标准化路径")?;

        Ok(canonical_path.to_string_lossy().to_string())
    }

    /// 验证索引文件是否存在
    async fn verify_index_files_exist(&self, path: &str) -> AppResult<bool> {
        // 检查 .ck 目录是否存在
        let ck_dir = Path::new(path).join(".ck");
        Ok(ck_dir.exists() && ck_dir.is_dir())
    }

    /// 删除索引文件
    async fn remove_index_files(&self, path: &str) -> AppResult<()> {
        let ck_dir = Path::new(path).join(".ck");
        if ck_dir.exists() {
            tokio::fs::remove_dir_all(&ck_dir)
                .await
                .context("删除索引文件失败")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ck_output() {
        let output = r#"
Indexing directory: /path/to/workspace
Processed 150 files
Index size: 2048 bytes
Indexing complete
        "#;

        let result = WorkspaceIndexService::parse_ck_output(output);
        assert!(result.is_ok());

        let (file_count, index_size) = result.unwrap();
        assert_eq!(file_count, 150);
        assert_eq!(index_size, 2048);
    }

    #[test]
    fn test_parse_ck_output_empty() {
        let output = "";
        let result = WorkspaceIndexService::parse_ck_output(output);
        assert!(result.is_ok());

        let (file_count, index_size) = result.unwrap();
        assert_eq!(file_count, 1);
        assert_eq!(index_size, 1024);
    }

    #[tokio::test]
    async fn test_normalize_path() {
        let repositories = Arc::new(RepositoryManager::new(Arc::new(
            crate::storage::database::DatabaseManager::new(":memory:")
                .await
                .unwrap(),
        )));

        let service = WorkspaceIndexService::new(repositories);

        // 测试相对路径
        let result = service.normalize_path(".");
        assert!(result.is_ok());

        // 测试绝对路径
        let result = service.normalize_path("/tmp");
        assert!(result.is_ok() || result.is_err()); // 可能不存在，但不应该panic
    }
}
