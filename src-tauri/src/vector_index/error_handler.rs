/*!
 * 增强的错误处理和恢复机制
 *
 * 基于Roo-Code项目的策略实现详细的错误分类、分步跟踪和自动恢复功能
 *
 * 主要功能：
 * - 维度不匹配自动修复
 * - 详细的错误上下文信息
 * - 分步错误跟踪
 * - 错误恢复策略
 */

use anyhow::{bail, Result};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::vector_index::{
    constants::error_recovery::*, qdrant::QdrantService, types::VectorIndexFullConfig,
};

/// 错误恢复策略
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 重试操作
    Retry { max_attempts: u32, delay_ms: u64 },
    /// 重新创建集合
    RecreateCollection,
    /// 降级处理
    Fallback,
    /// 跳过当前项
    Skip,
    /// 终止操作
    Abort,
}

/// 操作步骤状态
#[derive(Debug, Clone)]
pub struct OperationStep {
    pub name: String,
    pub completed: bool,
    pub error: Option<String>,
}

/// 增强的错误处理器
pub struct EnhancedErrorHandler {
    #[allow(dead_code)]
    config: VectorIndexFullConfig,
    operation_steps: Vec<OperationStep>,
}

impl EnhancedErrorHandler {
    /// 创建新的错误处理器
    pub fn new(config: VectorIndexFullConfig) -> Self {
        Self {
            config,
            operation_steps: Vec::new(),
        }
    }

    /// 开始新的操作跟踪
    pub fn start_operation(&mut self, operation_name: &str) {
        self.operation_steps.clear();
        info!("开始操作: {}", operation_name);
    }

    /// 添加操作步骤
    pub fn add_step(&mut self, step_name: &str) {
        self.operation_steps.push(OperationStep {
            name: step_name.to_string(),
            completed: false,
            error: None,
        });
    }

    /// 标记步骤完成
    pub fn complete_step(&mut self, step_name: &str) {
        for step in &mut self.operation_steps {
            if step.name == step_name {
                step.completed = true;
                info!("步骤完成: {}", step_name);
                break;
            }
        }
    }

    /// 标记步骤失败
    pub fn fail_step(&mut self, step_name: &str, error: &str) {
        for step in &mut self.operation_steps {
            if step.name == step_name {
                step.error = Some(error.to_string());
                error!("步骤失败: {} - {}", step_name, error);
                break;
            }
        }
    }

    /// 处理维度不匹配错误并尝试自动修复
    pub async fn handle_dimension_mismatch<T: QdrantService>(
        &mut self,
        storage: &T,
        expected_dimension: usize,
        actual_dimension: usize,
    ) -> Result<()> {
        warn!(
            "检测到维度不匹配: 期望 {}, 实际 {}，开始自动修复",
            expected_dimension, actual_dimension
        );

        self.start_operation("维度不匹配自动修复");

        // 步骤1: 验证集合存在
        self.add_step("验证集合存在");
        let res_info = storage.get_collection_info().await;
        if let Err(ref e) = res_info {
            self.fail_step("验证集合存在", &e.to_string());
        }
        let _collection_info = res_info?;
        self.complete_step("验证集合存在");

        // 步骤2: 删除现有集合
        self.add_step("删除现有集合");
        let res_clear = storage.clear_all_vectors().await;
        if let Err(ref e) = res_clear {
            self.fail_step("删除现有集合", &e.to_string());
        }
        res_clear?;
        self.complete_step("删除现有集合");

        // 步骤3: 等待删除完成
        self.add_step("等待删除完成");
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.complete_step("等待删除完成");

        // 步骤4: 验证删除成功
        self.add_step("验证删除成功");
        let verification_info = storage.get_collection_info().await;
        if let Ok((points, _)) = verification_info {
            if points > 0 {
                self.fail_step("验证删除成功", "集合在删除后仍然存在");
                bail!("验证删除失败: 集合在删除尝试后仍然存在");
            }
        }
        self.complete_step("验证删除成功");

        // 步骤5: 重新初始化集合
        self.add_step("重新初始化集合");
        let res_init = storage.initialize_collection().await;
        if let Err(ref e) = res_init {
            self.fail_step("重新初始化集合", &e.to_string());
        }
        res_init?;
        self.complete_step("重新初始化集合");

        info!("维度不匹配自动修复完成");
        Ok(())
    }

    /// 处理连接错误并提供详细诊断
    pub async fn handle_connection_error<T: QdrantService>(
        &mut self,
        storage: &T,
        error: &anyhow::Error,
    ) -> Result<RecoveryStrategy> {
        let error_msg = error.to_string().to_lowercase();

        // 分析错误类型
        if error_msg.contains("connection refused") {
            error!("Qdrant服务器连接被拒绝");
            return Ok(RecoveryStrategy::Abort);
        }

        if error_msg.contains("timeout") {
            warn!("连接超时，建议重试");
            return Ok(RecoveryStrategy::Retry {
                max_attempts: MAX_RETRIES,
                delay_ms: NETWORK_RETRY_DELAY_MS,
            });
        }

        if error_msg.contains("authentication") || error_msg.contains("api key") {
            error!("认证失败，请检查API密钥");
            return Ok(RecoveryStrategy::Abort);
        }

        // 尝试测试连接
        match storage.test_connection().await {
            Ok(_) => {
                info!("连接测试成功，可能是临时网络问题");
                Ok(RecoveryStrategy::Retry {
                    max_attempts: 2,
                    delay_ms: CONNECTION_RETRY_DELAY_MS,
                })
            }
            Err(test_error) => {
                error!("连接测试失败: {}", test_error);
                Ok(RecoveryStrategy::Abort)
            }
        }
    }

    /// 处理批处理错误
    pub fn handle_batch_error(
        &mut self,
        batch_index: usize,
        error: &anyhow::Error,
        total_batches: usize,
    ) -> RecoveryStrategy {
        let error_msg = error.to_string().to_lowercase();

        // 如果是前期批次且错误严重，建议终止
        if batch_index < 3
            && (error_msg.contains("dimension")
                || error_msg.contains("collection")
                || error_msg.contains("authentication"))
        {
            error!("前期批次出现严重错误，建议终止操作");
            return RecoveryStrategy::Abort;
        }

        // 如果错误率过高，建议终止
        let error_rate = (batch_index as f32 + 1.0) / total_batches as f32;
        if error_rate > 0.3 {
            error!("错误率过高 ({:.1}%)，建议终止操作", error_rate * 100.0);
            return RecoveryStrategy::Abort;
        }

        // 网络相关错误，可以重试
        if error_msg.contains("network") || error_msg.contains("timeout") {
            warn!("网络错误，将重试批次 {}", batch_index);
            return RecoveryStrategy::Retry {
                max_attempts: 2,
                delay_ms: NETWORK_RETRY_DELAY_MS,
            };
        }

        // 其他错误，跳过当前批次
        warn!("跳过有问题的批次 {}", batch_index);
        RecoveryStrategy::Skip
    }

    /// 生成详细的错误报告
    pub fn generate_error_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== 操作执行报告 ===\n");

        for (i, step) in self.operation_steps.iter().enumerate() {
            report.push_str(&format!("{}. {} ", i + 1, step.name));

            if step.completed {
                report.push_str("✓ 完成\n");
            } else if let Some(error) = &step.error {
                report.push_str(&format!("✗ 失败: {}\n", error));
            } else {
                report.push_str("⏸ 未完成\n");
            }
        }

        report.push_str("=====================\n");
        report
    }

    /// 获取操作状态摘要
    pub fn get_operation_summary(&self) -> (usize, usize, usize) {
        let total = self.operation_steps.len();
        let completed = self.operation_steps.iter().filter(|s| s.completed).count();
        let failed = self
            .operation_steps
            .iter()
            .filter(|s| s.error.is_some())
            .count();

        (completed, failed, total)
    }

    /// 分类错误并提供建议
    pub fn classify_error(&self, error: &anyhow::Error) -> (String, String) {
        let error_msg = error.to_string().to_lowercase();

        if error_msg.contains("dimension") {
            return (
                "维度不匹配".to_string(),
                "检查模型配置，或使用自动修复功能".to_string(),
            );
        }

        if error_msg.contains("connection") || error_msg.contains("network") {
            return (
                "网络连接问题".to_string(),
                "检查Qdrant服务器状态和网络连接".to_string(),
            );
        }

        if error_msg.contains("authentication") || error_msg.contains("api key") {
            return (
                "认证失败".to_string(),
                "检查API密钥配置是否正确".to_string(),
            );
        }

        if error_msg.contains("parsing") || error_msg.contains("syntax") {
            return (
                "代码解析错误".to_string(),
                "检查代码文件格式，或添加到忽略列表".to_string(),
            );
        }

        if error_msg.contains("embedding") || error_msg.contains("vectorization") {
            return (
                "向量化失败".to_string(),
                "检查模型配置和API连接状态".to_string(),
            );
        }

        (
            "未分类错误".to_string(),
            "查看详细日志获取更多信息".to_string(),
        )
    }
}
