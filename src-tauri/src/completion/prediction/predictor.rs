/// 命令序列预测引擎
///
/// Linus式设计：三个步骤，没有抽象
/// 1. 根据上一条命令查表
/// 2. 从输出提取实体
/// 3. 生成带参数的建议
use super::command_pairs::get_suggested_commands;
use crate::completion::smart_extractor::SmartExtractor;
use crate::completion::types::{CompletionItem, CompletionType};
use std::path::PathBuf;

/// 预测结果
#[derive(Debug, Clone)]
pub struct PredictionResult {
    /// 预测的命令
    pub command: String,
    /// 自动注入的参数
    pub arguments: Vec<String>,
    /// 置信度分数
    pub confidence: f64,
    /// 来源说明
    pub source: String,
}

impl PredictionResult {
    /// 生成完整的命令字符串
    pub fn full_command(&self) -> String {
        if self.arguments.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.arguments.join(" "))
        }
    }

    /// 转换为补全项
    pub fn to_completion_item(&self) -> CompletionItem {
        let score = (90.0 + (self.confidence / 10.0)).min(100.0);
        CompletionItem::new(self.full_command(), CompletionType::Command)
            .with_score(score)
            .with_source(self.source.clone())
            .with_description(format!("预测的后续命令 (置信度: {:.0}%)", self.confidence))
    }
}

/// 命令序列预测器
pub struct CommandPredictor {
    /// 实体提取器
    extractor: &'static SmartExtractor,
}

impl CommandPredictor {
    /// 创建新的预测器
    pub fn new(_current_dir: PathBuf) -> Self {
        Self {
            extractor: SmartExtractor::global(),
        }
    }

    /// 预测下一条命令
    ///
    /// Linus: "Keep it simple - 三步走"
    pub fn predict_next_commands(
        &self,
        last_command: &str,
        last_output: Option<&str>,
        input_prefix: &str,
    ) -> Vec<PredictionResult> {
        let mut predictions = Vec::new();

        // 步骤1: 查找相关命令
        if let Some(suggested) = get_suggested_commands(last_command) {
            for cmd in suggested {
                // 过滤：只保留匹配输入前缀的
                if cmd.starts_with(input_prefix) || input_prefix.is_empty() {
                    // 步骤2: 提取实体并注入参数
                    let prediction = self.build_prediction(&cmd, last_command, last_output);
                    predictions.push(prediction);
                }
            }
        }

        // 步骤3: 按置信度排序
        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        predictions
    }

    pub fn build_prediction_for_suggestion(
        &self,
        suggested_cmd: &str,
        last_command: &str,
        last_output: Option<&str>,
    ) -> PredictionResult {
        self.build_prediction(suggested_cmd, last_command, last_output)
    }

    /// 构建预测结果（包含参数注入）
    fn build_prediction(
        &self,
        suggested_cmd: &str,
        last_command: &str,
        last_output: Option<&str>,
    ) -> PredictionResult {
        let mut arguments = Vec::new();
        let mut confidence = 50.0;

        // 根据命令类型尝试注入参数
        if let Some(output) = last_output {
            match suggested_cmd {
                cmd if cmd.starts_with("kill") => {
                    // 从 lsof/ps 输出提取 PID
                    if let Some(pid) = self.extract_first_pid(last_command, output) {
                        arguments.push(pid);
                        confidence = 85.0;
                    }
                }
                cmd if cmd.starts_with("docker stop") || cmd.starts_with("docker logs") => {
                    // 从 docker ps 输出提取容器 ID
                    if let Some(container_id) = self.extract_container_id(output) {
                        arguments.push(container_id);
                        confidence = 85.0;
                    }
                }
                cmd if cmd.starts_with("git add") => {
                    // 从 git status 输出提取修改的文件
                    if let Some(file) = self.extract_modified_file(output) {
                        arguments.push(file);
                        confidence = 80.0;
                    }
                }
                _ => {
                    // 默认置信度
                    confidence = 50.0;
                }
            }
        }

        PredictionResult {
            command: suggested_cmd.to_string(),
            arguments,
            confidence,
            source: format!(
                "基于 '{}' 的预测",
                last_command.split_whitespace().next().unwrap_or("")
            ),
        }
    }

    /// 提取第一个 PID
    ///
    /// Linus: "不需要完美，需要的是能用"
    fn extract_first_pid(&self, last_command: &str, output: &str) -> Option<String> {
        // 使用 SmartExtractor 提取 PID
        match self.extractor.extract_entities(last_command, output) {
            Ok(results) => {
                // 找第一个 PID
                results
                    .into_iter()
                    .find(|r| r.entity_type == "pid")
                    .map(|r| r.value)
            }
            Err(_) => None,
        }
    }

    /// 提取容器 ID
    fn extract_container_id(&self, output: &str) -> Option<String> {
        // 简单模式匹配 docker ps 输出
        // 第一列是容器 ID
        output
            .lines()
            .nth(1)
            .and_then(|line| line.split_whitespace().next())
            .map(|id| id.to_string())
    }

    /// 提取修改的文件
    fn extract_modified_file(&self, output: &str) -> Option<String> {
        // 匹配 git status 输出中的文件名
        // 示例: "modified:   src/main.rs"
        for line in output.lines() {
            if line.contains("modified:") || line.contains("new file:") {
                if let Some(file) = line.split(':').nth(1) {
                    return Some(file.trim().to_string());
                }
            }
            // 也支持短格式: " M src/main.rs"
            if line.starts_with(" M ") || line.starts_with("?? ") {
                if let Some(file) = line.split_whitespace().nth(1) {
                    return Some(file.to_string());
                }
            }
        }
        None
    }

    // 上下文加分已移除：学习模型负责“项目/用户行为”的动态权重，
    // 静态启发式只会制造特殊情况和不可解释的排序。
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_lsof_kill_prediction() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "lsof -i :8080";
        let last_output = Some("COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\nnode    12345 user   23u  IPv4 0x1234      0t0  TCP *:8080 (LISTEN)");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        assert!(predictions.iter().any(|p| p.command.starts_with("kill")));

        // 应该自动提取到 PID
        let kill_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("kill"))
            .unwrap();
        assert!(kill_pred.arguments.contains(&"12345".to_string()));
    }

    #[test]
    fn test_docker_workflow() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "docker ps";
        let last_output = Some("CONTAINER ID   IMAGE     COMMAND                  CREATED         STATUS         PORTS                    NAMES\nabc123def456   nginx     \"/docker-entrypoint.…\"   2 hours ago     Up 2 hours     0.0.0.0:8080->80/tcp     my-nginx");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        let stop_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("docker stop"));
        assert!(stop_pred.is_some());

        // 应该自动提取到容器 ID
        assert!(stop_pred
            .unwrap()
            .arguments
            .contains(&"abc123def456".to_string()));
    }

    #[test]
    fn test_git_workflow() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "git status";
        let last_output =
            Some("On branch main\nChanges not staged for commit:\n  modified:   src/main.rs");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        let add_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("git add"));
        assert!(add_pred.is_some());

        // 应该自动提取到文件名
        assert!(add_pred
            .unwrap()
            .arguments
            .contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_input_prefix_filter() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "git status";

        // 只返回以 "git a" 开头的预测
        let predictions = predictor.predict_next_commands(last_cmd, None, "git a");

        assert!(predictions.iter().all(|p| p.command.starts_with("git a")));
    }
}
