/// 命令序列预测模块
///
/// 提供智能命令预测功能：
/// - 基于历史命令预测下一步操作
/// - 从输出自动提取实体（PID、容器ID、文件路径等）
/// - 根据工作目录上下文智能加分
mod command_pairs;
mod predictor;

pub use command_pairs::{get_suggested_commands, matches_command_pattern, COMMAND_PAIRS};
pub use predictor::{CommandPredictor, PredictionResult};
