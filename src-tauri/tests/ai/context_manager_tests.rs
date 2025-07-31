/*!
 * 上下文管理器测试
 *
 * 测试AI模块中上下文管理器的上下文收集、命令历史管理、压缩功能和频率统计
 */

use std::collections::HashMap;
use termx::ai::{CommandHistoryEntry, ContextManager, SystemInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::TestContexts;

    /// 测试上下文管理器创建
    #[test]
    fn test_context_manager_creation() {
        let manager = ContextManager::new(100);
        let context = manager.get_context();

        // 初始状态应该是空的
        assert!(context.working_directory.is_none());
        assert!(context.command_history.is_none());
        assert!(context.environment.is_none());
        assert!(context.current_command.is_none());
        assert!(context.last_output.is_none());
        assert!(context.system_info.is_none());
    }

    /// 测试工作目录更新
    #[test]
    fn test_update_working_directory() {
        let mut manager = ContextManager::new(100);

        // 更新工作目录
        manager.update_working_directory("/home/user/project".to_string());
        let context = manager.get_context();
        assert_eq!(
            context.working_directory,
            Some("/home/user/project".to_string())
        );

        // 再次更新
        manager.update_working_directory("/home/user/another_project".to_string());
        let context = manager.get_context();
        assert_eq!(
            context.working_directory,
            Some("/home/user/another_project".to_string())
        );
    }

    /// 测试命令历史添加
    #[test]
    fn test_add_command() {
        let mut manager = ContextManager::new(5); // 小容量便于测试

        // 添加命令
        manager.add_command("ls -la".to_string());
        manager.add_command("cd project".to_string());
        manager.add_command("git status".to_string());

        let context = manager.get_context();
        let history = context.command_history.as_ref().unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "ls -la");
        assert_eq!(history[1], "cd project");
        assert_eq!(history[2], "git status");
    }

    /// 测试命令历史容量限制
    #[test]
    fn test_command_history_capacity_limit() {
        let mut manager = ContextManager::new(3); // 容量为3

        // 添加超过容量的命令
        for i in 0..5 {
            manager.add_command(format!("command-{}", i));
        }

        let context = manager.get_context();
        let history = context.command_history.as_ref().unwrap();

        // 应该只保留最后3个命令
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "command-2");
        assert_eq!(history[1], "command-3");
        assert_eq!(history[2], "command-4");
    }

    /// 测试带详细信息的命令添加
    #[test]
    fn test_add_command_with_details() {
        let mut manager = ContextManager::new(100);

        let entry = CommandHistoryEntry {
            command: "npm install".to_string(),
            timestamp: 1234567890,
            exit_code: Some(0),
            duration: Some(5000), // 5秒
            working_directory: Some("/home/user/project".to_string()),
            output_preview: Some("Installing dependencies...".to_string()),
        };

        manager.add_command_with_details(entry.clone());

        // 验证命令已添加到历史中
        let context = manager.get_context();
        let history = context.command_history.as_ref().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], "npm install");
    }

    /// 测试当前命令设置
    #[test]
    fn test_set_current_command() {
        let mut manager = ContextManager::new(100);

        // 设置当前命令
        manager.set_current_command(Some("npm test".to_string()));
        let context = manager.get_context();
        assert_eq!(context.current_command, Some("npm test".to_string()));

        // 清除当前命令
        manager.set_current_command(None);
        let context = manager.get_context();
        assert!(context.current_command.is_none());
    }

    /// 测试最后输出设置
    #[test]
    fn test_set_last_output() {
        let mut manager = ContextManager::new(100);

        // 设置最后输出
        manager.set_last_output(Some("Test passed successfully".to_string()));
        let context = manager.get_context();
        assert_eq!(
            context.last_output,
            Some("Test passed successfully".to_string())
        );

        // 清除最后输出
        manager.set_last_output(None);
        let context = manager.get_context();
        assert!(context.last_output.is_none());
    }

    /// 测试环境变量更新
    #[test]
    fn test_update_environment() {
        let mut manager = ContextManager::new(100);

        let mut env = HashMap::new();
        env.insert("NODE_ENV".to_string(), "development".to_string());
        env.insert("PATH".to_string(), "/usr/local/bin:/usr/bin".to_string());

        manager.update_environment(env.clone());
        let context = manager.get_context();
        assert_eq!(context.environment, Some(env));
    }

    /// 测试系统信息更新
    #[test]
    fn test_update_system_info() {
        let mut manager = ContextManager::new(100);

        let system_info = SystemInfo {
            platform: "linux".to_string(),
            shell: "bash".to_string(),
            user: "testuser".to_string(),
        };

        manager.update_system_info(system_info.clone());
        let context = manager.get_context();
        assert_eq!(context.system_info, Some(system_info));
    }

    /// 测试压缩上下文
    #[test]
    fn test_get_compressed_context() {
        let mut manager = ContextManager::new(100);

        // 添加大量命令历史
        for i in 0..20 {
            manager.add_command(format!("command-{}", i));
        }

        // 获取压缩上下文（限制为5个历史项）
        let compressed = manager.get_compressed_context(5);
        let history = compressed.command_history.as_ref().unwrap();

        // 应该只包含最后5个命令
        assert_eq!(history.len(), 5);
        assert_eq!(history[0], "command-15");
        assert_eq!(history[4], "command-19");
    }

    /// 测试命令频率统计
    #[test]
    fn test_command_frequency_tracking() {
        let mut manager = ContextManager::new(100);

        // 添加重复命令
        manager.add_command("git status".to_string());
        manager.add_command("ls -la".to_string());
        manager.add_command("git status".to_string());
        manager.add_command("git status".to_string());
        manager.add_command("ls -la".to_string());

        // 获取频率统计
        let frequent_commands = manager.get_frequent_commands(10);

        // git status应该是最频繁的
        assert_eq!(frequent_commands[0].0, "git status");
        assert_eq!(frequent_commands[0].1, 3);
        assert_eq!(frequent_commands[1].0, "ls -la");
        assert_eq!(frequent_commands[1].1, 2);
    }

    /// 测试相似命令查找
    #[test]
    fn test_find_similar_commands() {
        let mut manager = ContextManager::new(100);

        // 添加相似命令
        manager.add_command("git status".to_string());
        manager.add_command("git commit".to_string());
        manager.add_command("git push".to_string());
        manager.add_command("npm install".to_string());
        manager.add_command("npm test".to_string());

        // 查找以"git"开头的命令
        let git_commands = manager.find_similar_commands("git");
        assert_eq!(git_commands.len(), 3);
        assert!(git_commands.contains(&"git status".to_string()));
        assert!(git_commands.contains(&"git commit".to_string()));
        assert!(git_commands.contains(&"git push".to_string()));

        // 查找以"npm"开头的命令
        let npm_commands = manager.find_similar_commands("npm");
        assert_eq!(npm_commands.len(), 2);
        assert!(npm_commands.contains(&"npm install".to_string()));
        assert!(npm_commands.contains(&"npm test".to_string()));
    }

    /// 测试上下文清理
    #[test]
    fn test_clear_context() {
        let mut manager = ContextManager::new(100);

        // 设置各种上下文信息
        manager.update_working_directory("/home/user".to_string());
        manager.add_command("test command".to_string());
        manager.set_current_command(Some("current".to_string()));
        manager.set_last_output(Some("output".to_string()));

        // 清理上下文
        manager.clear_context();

        let context = manager.get_context();
        assert!(context.working_directory.is_none());
        assert!(
            context.command_history.is_none()
                || context.command_history.as_ref().unwrap().is_empty()
        );
        assert!(context.current_command.is_none());
        assert!(context.last_output.is_none());
    }

    /// 测试会话统计
    #[test]
    fn test_session_statistics() {
        let mut manager = ContextManager::new(100);

        // 添加一些命令
        for i in 0..10 {
            manager.add_command(format!("command-{}", i));
        }

        let stats = manager.get_session_statistics();
        assert_eq!(stats.total_commands, 10);
        assert!(stats.session_duration > 0);
        assert_eq!(stats.unique_commands, 10);
    }

    /// 测试上下文序列化
    #[test]
    fn test_context_serialization() {
        let mut manager = ContextManager::new(100);

        // 设置完整的上下文
        manager.update_working_directory("/home/user/project".to_string());
        manager.add_command("git clone repo".to_string());
        manager.add_command("cd repo".to_string());
        manager.set_current_command(Some("npm install".to_string()));
        manager.set_last_output(Some("Installing...".to_string()));

        let mut env = HashMap::new();
        env.insert("NODE_ENV".to_string(), "development".to_string());
        manager.update_environment(env);

        let system_info = SystemInfo {
            platform: "linux".to_string(),
            shell: "bash".to_string(),
            user: "developer".to_string(),
        };
        manager.update_system_info(system_info);

        // 序列化上下文
        let context = manager.get_context();
        let serialized = serde_json::to_string(context).unwrap();

        // 反序列化并验证
        let deserialized: termx::ai::AIContext = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.working_directory, context.working_directory);
        assert_eq!(deserialized.command_history, context.command_history);
        assert_eq!(deserialized.current_command, context.current_command);
        assert_eq!(deserialized.last_output, context.last_output);
        assert_eq!(deserialized.environment, context.environment);
        assert_eq!(deserialized.system_info, context.system_info);
    }

    /// 测试上下文管理器的性能
    #[test]
    fn test_context_manager_performance() {
        let mut manager = ContextManager::new(1000);

        // 测量添加大量命令的时间
        let start = std::time::Instant::now();
        for i in 0..1000 {
            manager.add_command(format!("command-{}", i));
        }
        let duration = start.elapsed();

        // 应该在合理时间内完成
        assert!(duration.as_millis() < 100); // 应该在100ms内完成

        // 验证所有命令都已添加
        let context = manager.get_context();
        let history = context.command_history.as_ref().unwrap();
        assert_eq!(history.len(), 1000);
    }
}
