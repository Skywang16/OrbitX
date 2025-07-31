/*!
 * 测试工具模块
 * 
 * 提供通用的测试辅助函数和工具，用于改进测试的可读性和维护性
 */

use std::time::Duration;
use terminal_lib::mux::{get_mux, PaneId, PtySize, TerminalMux};

/// 测试断言宏，提供更详细的错误信息
#[macro_export]
macro_rules! assert_pane_exists {
    ($mux:expr, $pane_id:expr, $description:expr) => {
        assert!(
            $mux.get_pane($pane_id).is_some(),
            "面板应该存在: {} (ID: {:?})",
            $description,
            $pane_id
        );
    };
}

#[macro_export]
macro_rules! assert_pane_not_exists {
    ($mux:expr, $pane_id:expr, $description:expr) => {
        assert!(
            $mux.get_pane($pane_id).is_none(),
            "面板不应该存在: {} (ID: {:?})",
            $description,
            $pane_id
        );
    };
}

#[macro_export]
macro_rules! assert_operation_success {
    ($result:expr, $operation:expr) => {
        assert!(
            $result.is_ok(),
            "操作应该成功: {}，错误: {:?}",
            $operation,
            $result.err()
        );
    };
}

#[macro_export]
macro_rules! assert_operation_failure {
    ($result:expr, $operation:expr) => {
        assert!(
            $result.is_err(),
            "操作应该失败: {}，但却成功了",
            $operation
        );
    };
}

/// 测试面板配置
#[derive(Debug, Clone)]
pub struct TestPaneConfig {
    pub name: String,
    pub size: PtySize,
    pub test_data: Vec<u8>,
}

impl TestPaneConfig {
    /// 创建小尺寸面板配置
    pub fn small(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: PtySize::new(24, 80),
            test_data: format!("echo 'Testing {}'\n", name).into_bytes(),
        }
    }

    /// 创建中等尺寸面板配置
    pub fn medium(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: PtySize::new(30, 100),
            test_data: format!("echo 'Testing {}'\n", name).into_bytes(),
        }
    }

    /// 创建大尺寸面板配置
    pub fn large(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: PtySize::new(40, 120),
            test_data: format!("echo 'Testing {}'\n", name).into_bytes(),
        }
    }

    /// 创建自定义配置
    pub fn custom(name: &str, rows: u16, cols: u16) -> Self {
        Self {
            name: name.to_string(),
            size: PtySize::new(rows, cols),
            test_data: format!("echo 'Testing {}'\n", name).into_bytes(),
        }
    }
}

/// 测试面板管理器
/// 
/// 提供面板的批量创建、操作和清理功能
pub struct TestPaneManager {
    mux: &'static TerminalMux,
    created_panes: Vec<(String, PaneId)>,
    initial_pane_count: usize,
}

impl TestPaneManager {
    /// 创建新的测试面板管理器
    pub fn new() -> Self {
        let mux = get_mux();
        let initial_pane_count = mux.pane_count();
        
        Self {
            mux,
            created_panes: Vec::new(),
            initial_pane_count,
        }
    }

    /// 创建单个面板
    pub async fn create_pane(&mut self, config: &TestPaneConfig) -> Result<PaneId, String> {
        let pane_id = self.mux.create_pane(config.size).await
            .map_err(|e| format!("创建面板 '{}' 失败: {}", config.name, e))?;
        
        self.created_panes.push((config.name.clone(), pane_id));
        Ok(pane_id)
    }

    /// 批量创建面板
    pub async fn create_panes(&mut self, configs: &[TestPaneConfig]) -> Result<Vec<PaneId>, String> {
        let mut pane_ids = Vec::new();
        
        for config in configs {
            let pane_id = self.create_pane(config).await?;
            pane_ids.push(pane_id);
        }
        
        Ok(pane_ids)
    }

    /// 验证面板存在
    pub fn assert_pane_exists(&self, pane_id: PaneId, description: &str) {
        assert_pane_exists!(self.mux, pane_id, description);
    }

    /// 验证面板不存在
    pub fn assert_pane_not_exists(&self, pane_id: PaneId, description: &str) {
        assert_pane_not_exists!(self.mux, pane_id, description);
    }

    /// 验证面板大小
    pub fn assert_pane_size(&self, pane_id: PaneId, expected_size: PtySize, description: &str) {
        let pane = self.mux.get_pane(pane_id)
            .expect(&format!("面板应该存在: {}", description));
        
        let actual_size = pane.get_size();
        assert_eq!(
            actual_size.rows, expected_size.rows,
            "{}的行数应该匹配: 期望 {}, 实际 {}",
            description, expected_size.rows, actual_size.rows
        );
        assert_eq!(
            actual_size.cols, expected_size.cols,
            "{}的列数应该匹配: 期望 {}, 实际 {}",
            description, expected_size.cols, actual_size.cols
        );
    }

    /// 写入数据到面板
    pub fn write_to_pane(&self, pane_id: PaneId, data: &[u8], description: &str) {
        let result = self.mux.write_to_pane(pane_id, data);
        assert_operation_success!(result, format!("向{}写入数据", description));
    }

    /// 调整面板大小
    pub fn resize_pane(&self, pane_id: PaneId, new_size: PtySize, description: &str) {
        let result = self.mux.resize_pane(pane_id, new_size);
        assert_operation_success!(result, format!("调整{}大小", description));
    }

    /// 移除面板
    pub fn remove_pane(&mut self, pane_id: PaneId, description: &str) {
        let result = self.mux.remove_pane(pane_id);
        assert_operation_success!(result, format!("移除{}", description));
        
        // 从跟踪列表中移除
        self.created_panes.retain(|(_, id)| *id != pane_id);
    }

    /// 验证面板总数
    pub fn assert_pane_count(&self, expected_additional: usize, description: &str) {
        let current_count = self.mux.pane_count();
        let expected_count = self.initial_pane_count + expected_additional;
        
        assert_eq!(
            current_count, expected_count,
            "{}: 期望面板总数 {}, 实际 {}",
            description, expected_count, current_count
        );
    }

    /// 验证面板在列表中
    pub fn assert_pane_in_list(&self, pane_id: PaneId, description: &str) {
        let pane_list = self.mux.list_panes();
        assert!(
            pane_list.contains(&pane_id),
            "面板列表应该包含{} (ID: {:?})",
            description, pane_id
        );
    }

    /// 获取创建的面板列表
    pub fn get_created_panes(&self) -> &[(String, PaneId)] {
        &self.created_panes
    }

    /// 清理所有创建的面板
    pub fn cleanup_all(&mut self) {
        let panes_to_remove: Vec<_> = self.created_panes.clone();
        
        for (name, pane_id) in panes_to_remove {
            self.remove_pane(pane_id, &name);
        }
        
        // 验证清理完成
        self.assert_pane_count(0, "清理后");
    }
}

impl Drop for TestPaneManager {
    /// 自动清理资源
    fn drop(&mut self) {
        if !self.created_panes.is_empty() {
            eprintln!("警告: TestPaneManager被销毁时还有 {} 个面板未清理", self.created_panes.len());
            self.cleanup_all();
        }
    }
}

/// 测试辅助函数
pub mod helpers {
    use super::*;

    /// 等待指定时间
    pub async fn wait_for(duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    /// 创建测试数据
    pub fn create_test_data(content: &str) -> Vec<u8> {
        format!("{}\n", content).into_bytes()
    }

    /// 验证错误类型
    pub fn assert_error_contains<E: std::fmt::Debug>(result: Result<(), E>, expected_msg: &str) {
        match result {
            Ok(_) => panic!("期望操作失败，但却成功了"),
            Err(e) => {
                let error_msg = format!("{:?}", e);
                assert!(
                    error_msg.contains(expected_msg),
                    "错误消息应该包含 '{}', 实际错误: {}",
                    expected_msg, error_msg
                );
            }
        }
    }

    /// 创建标准测试配置集合
    pub fn standard_test_configs() -> Vec<TestPaneConfig> {
        vec![
            TestPaneConfig::small("小面板"),
            TestPaneConfig::medium("中面板"),
            TestPaneConfig::large("大面板"),
        ]
    }
}
