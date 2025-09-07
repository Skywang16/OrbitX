/*!
 * 向量索引服务层
 * 
 * 包含各种专门的服务组件，每个组件都有单一职责：
 * - CodeAnalysisService: 代码解析和分块
 * - TaskCoordinator: 任务状态和进度管理
 * 
 * 这些服务可以被更高层的协调器组合使用。
 */

pub mod code_analysis;
pub mod task_coordinator;

// 重新导出主要接口
pub use code_analysis::{CodeAnalysisService, TreeSitterCodeAnalysisService, CodeAnalysisResult};
pub use task_coordinator::{TaskCoordinator, DefaultTaskCoordinator, TaskStatus, TaskMetadata};
