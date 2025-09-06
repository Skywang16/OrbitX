/*!
 * TOML配置管理模块的单元测试
 */

#[cfg(test)]
mod tests {
    use super::manager::TomlConfigManager;
    use crate::config::{defaults::create_default_config, paths::ConfigPaths, types::AppConfig};
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;
    use tokio::sync::broadcast;
    use tokio::time::Duration;

    /// 创建测试用的配置管理器
    async fn create_test_manager() -> (TomlConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();

        // 使用测试路径创建管理器的各个组件
        let reader = super::reader::TomlConfigReader::new().unwrap();
        let writer = super::writer::TomlConfigWriter::new(paths.config_file());
        let validator = super::validator::TomlConfigValidator::new();
        let event_sender = super::events::ConfigEventSender::new().0;

        let manager = TomlConfigManager {
            config_cache: Arc::new(RwLock::new(create_default_config())),
            reader,
            writer,
            validator,
            event_sender,
        };

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = TomlConfigManager::new().await.unwrap();

        // 验证管理器创建成功
        let config = manager.get_config().await.unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");
    }

    #[tokio::test]
    async fn test_load_default_config() {
        let (manager, _temp_dir) = create_test_manager().await;

        let config = manager.load_config().await.unwrap();

        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");
        assert!(config.app.confirm_on_exit);
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        let (manager, _temp_dir) = create_test_manager().await;

        let mut config = create_default_config();
        config.app.language = "en-US".to_string();
        config.appearance.font.size = 16.0;

        manager.save_config(&config).await.unwrap();
        let loaded_config = manager.load_config().await.unwrap();

        assert_eq!(loaded_config.app.language, "en-US");
        assert_eq!(loaded_config.appearance.font.size, 16.0);
    }

    #[tokio::test]
    async fn test_update_section() {
        let (manager, _temp_dir) = create_test_manager().await;

        // 加载初始配置
        manager.load_config().await.unwrap();

        // 更新语言设置
        manager
            .update_section("app.language", "en-US")
            .await
            .unwrap();

        let config = manager.get_config().await.unwrap();
        assert_eq!(config.app.language, "en-US");

        // 更新字体大小
        manager
            .update_section("appearance.font.size", 18.0)
            .await
            .unwrap();

        let config = manager.get_config().await.unwrap();
        assert_eq!(config.appearance.font.size, 18.0);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let (manager, _temp_dir) = create_test_manager().await;

        // 测试有效配置
        let valid_config = create_default_config();
        assert!(manager.validate_config(&valid_config).is_ok());

        // 测试无效配置
        let mut invalid_config = create_default_config();
        invalid_config.app.language = "invalid-lang".to_string();
        assert!(manager.validate_config(&invalid_config).is_err());

        // 测试字体大小超出范围
        let mut invalid_font_config = create_default_config();
        invalid_font_config.appearance.font.size = 100.0;
        assert!(manager.validate_config(&invalid_font_config).is_err());
    }

    #[tokio::test]
    async fn test_config_events() {
        let (manager, _temp_dir) = create_test_manager().await;

        let mut event_receiver = manager.subscribe_changes();

        // 加载配置应该触发事件
        manager.load_config().await.unwrap();

        // 检查是否收到加载事件
        let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, super::events::ConfigEvent::Loaded { .. }));

        // 保存配置应该触发事件
        let config = create_default_config();
        manager.save_config(&config).await.unwrap();

        let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(event, super::events::ConfigEvent::Saved { .. }));
    }

    #[tokio::test]
    async fn test_config_merge() {
        let (manager, _temp_dir) = create_test_manager().await;

        let base_config = create_default_config();
        let partial_config = serde_json::json!({
            "app": {
                "language": "ja-JP"
            },
            "appearance": {
                "font": {
                    "size": 20.0
                }
            }
        });

        let merged_config = manager.merge_config(&base_config, partial_config).unwrap();

        assert_eq!(merged_config.app.language, "ja-JP");
        assert_eq!(merged_config.appearance.font.size, 20.0);
        // 其他字段应该保持默认值
        assert!(merged_config.app.confirm_on_exit);
        assert_eq!(merged_config.terminal.scrollback, 1000);
    }

    #[tokio::test]
    async fn test_error_handling_comprehensive() {
        let (manager, _temp_dir) = create_test_manager().await;

        // 测试无效配置节更新
        let result = manager.update_section("invalid.section", "test").await;
        assert!(result.is_err(), "无效配置节应该返回错误");

        // 测试无效数据类型
        let result = manager.update_section("app.language", 123).await;
        assert!(result.is_err(), "无效数据类型应该返回错误");

        // 测试验证失败的配置
        let mut invalid_config = create_default_config();
        invalid_config.app.language = "invalid-language".to_string();
        let result = manager.validate_config(&invalid_config);
        assert!(result.is_err(), "无效配置应该验证失败");

        // 测试字体大小超出范围
        invalid_config.appearance.font.size = 1000.0;
        let result = manager.validate_config(&invalid_config);
        assert!(result.is_err(), "字体大小超出范围应该验证失败");

        // 测试滚动缓冲区超出范围
        invalid_config.terminal.scrollback = 50;
        let result = manager.validate_config(&invalid_config);
        assert!(result.is_err(), "滚动缓冲区超出范围应该验证失败");
    }
}
