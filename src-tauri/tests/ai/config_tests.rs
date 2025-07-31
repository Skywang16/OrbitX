/*!
 * AI配置管理测试
 *
 * 测试AI配置管理器的基本操作、模型配置CRUD、验证和默认配置功能
 */

use std::path::PathBuf;
use tempfile::TempDir;
use termx::ai::{AIConfigManager, AIProvider, SecureStorage};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::{TestModelConfigs, TestSettings};

    /// 创建测试用的配置管理器
    fn create_test_config_manager() -> (TempDir, AIConfigManager) {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_config.json");
        let storage = SecureStorage::new(storage_path);
        let config_manager = AIConfigManager::with_storage(storage);
        (temp_dir, config_manager)
    }

    /// 测试配置管理器创建
    #[test]
    fn test_config_manager_creation() {
        // 测试默认创建
        let config_manager = AIConfigManager::new();
        assert!(config_manager.get_models().is_empty());

        // 测试带存储创建
        let (_temp_dir, config_manager) = create_test_config_manager();
        assert!(config_manager.get_models().is_empty());
    }

    /// 测试模型配置添加
    #[test]
    fn test_add_model() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加OpenAI模型
        let openai_model = TestModelConfigs::openai();
        assert!(config_manager.add_model(openai_model.clone()).is_ok());

        // 验证模型已添加
        assert_eq!(config_manager.get_models().len(), 1);
        let added_model = config_manager.get_model(&openai_model.id).unwrap();
        assert_eq!(added_model.id, openai_model.id);
        assert_eq!(added_model.provider, openai_model.provider);

        // 添加Claude模型
        let claude_model = TestModelConfigs::claude();
        assert!(config_manager.add_model(claude_model.clone()).is_ok());
        assert_eq!(config_manager.get_models().len(), 2);

        // 尝试添加重复ID的模型应该失败
        let duplicate_model = TestModelConfigs::openai();
        assert!(config_manager.add_model(duplicate_model).is_err());
        assert_eq!(config_manager.get_models().len(), 2); // 数量不变
    }

    /// 测试模型配置更新
    #[test]
    fn test_update_model() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加初始模型
        let mut model = TestModelConfigs::openai();
        assert!(config_manager.add_model(model.clone()).is_ok());

        // 更新模型配置
        model.name = "Updated OpenAI Model".to_string();
        assert!(config_manager
            .update_model(&model.id, model.clone())
            .is_ok());

        // 验证更新
        let updated_model = config_manager.get_model(&model.id).unwrap();
        assert_eq!(updated_model.name, "Updated OpenAI Model");

        // 尝试更新不存在的模型应该失败
        assert!(config_manager.update_model("nonexistent", model).is_err());
    }

    /// 测试模型配置删除
    #[test]
    fn test_remove_model() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加多个模型
        let openai_model = TestModelConfigs::openai();
        let claude_model = TestModelConfigs::claude();
        assert!(config_manager.add_model(openai_model.clone()).is_ok());
        assert!(config_manager.add_model(claude_model.clone()).is_ok());
        assert_eq!(config_manager.get_models().len(), 2);

        // 删除一个模型
        assert!(config_manager.remove_model(&openai_model.id).is_ok());
        assert_eq!(config_manager.get_models().len(), 1);
        assert!(config_manager.get_model(&openai_model.id).is_none());
        assert!(config_manager.get_model(&claude_model.id).is_some());

        // 尝试删除不存在的模型应该失败
        assert!(config_manager.remove_model("nonexistent").is_err());

        // 删除剩余模型
        assert!(config_manager.remove_model(&claude_model.id).is_ok());
        assert!(config_manager.get_models().is_empty());
    }

    /// 测试模型配置查询
    #[test]
    fn test_get_model() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加测试模型
        let models = vec![
            TestModelConfigs::openai(),
            TestModelConfigs::claude(),
            TestModelConfigs::local(),
        ];

        for model in &models {
            assert!(config_manager.add_model(model.clone()).is_ok());
        }

        // 测试按ID查询
        for model in &models {
            let found_model = config_manager.get_model(&model.id);
            assert!(found_model.is_some());
            assert_eq!(found_model.unwrap().id, model.id);
        }

        // 测试查询不存在的模型
        assert!(config_manager.get_model("nonexistent").is_none());
    }

    /// 测试默认模型管理
    #[test]
    fn test_default_model_management() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 初始状态没有默认模型
        assert!(config_manager.get_default_model_id().is_none());

        // 添加模型
        let openai_model = TestModelConfigs::openai();
        let claude_model = TestModelConfigs::claude();
        assert!(config_manager.add_model(openai_model.clone()).is_ok());
        assert!(config_manager.add_model(claude_model.clone()).is_ok());

        // 设置默认模型
        assert!(config_manager.set_default_model(&openai_model.id).is_ok());
        assert_eq!(
            config_manager.get_default_model_id(),
            Some(&openai_model.id)
        );

        // 更改默认模型
        assert!(config_manager.set_default_model(&claude_model.id).is_ok());
        assert_eq!(
            config_manager.get_default_model_id(),
            Some(&claude_model.id)
        );

        // 尝试设置不存在的模型为默认应该失败
        assert!(config_manager.set_default_model("nonexistent").is_err());
        assert_eq!(
            config_manager.get_default_model_id(),
            Some(&claude_model.id)
        ); // 保持不变

        // 清除默认模型
        config_manager.clear_default_model();
        assert!(config_manager.get_default_model_id().is_none());
    }

    /// 测试按提供商查询模型
    #[test]
    fn test_get_models_by_provider() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加不同提供商的模型
        let openai_model1 = TestModelConfigs::openai();
        let mut openai_model2 = TestModelConfigs::openai();
        openai_model2.id = "openai-gpt3".to_string();
        openai_model2.model = "gpt-3.5-turbo".to_string();

        let claude_model = TestModelConfigs::claude();
        let local_model = TestModelConfigs::local();

        assert!(config_manager.add_model(openai_model1).is_ok());
        assert!(config_manager.add_model(openai_model2).is_ok());
        assert!(config_manager.add_model(claude_model).is_ok());
        assert!(config_manager.add_model(local_model).is_ok());

        // 测试按提供商查询
        let openai_models = config_manager.get_models_by_provider(AIProvider::OpenAI);
        assert_eq!(openai_models.len(), 2);

        let claude_models = config_manager.get_models_by_provider(AIProvider::Claude);
        assert_eq!(claude_models.len(), 1);

        let local_models = config_manager.get_models_by_provider(AIProvider::Local);
        assert_eq!(local_models.len(), 1);

        let custom_models = config_manager.get_models_by_provider(AIProvider::Custom);
        assert_eq!(custom_models.len(), 0);
    }

    /// 测试配置验证
    #[test]
    fn test_config_validation() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 测试有效配置
        let valid_model = TestModelConfigs::openai();
        assert!(config_manager.validate_model_config(&valid_model).is_ok());

        // 测试无效配置 - 空ID
        let mut invalid_model = TestModelConfigs::openai();
        invalid_model.id = "".to_string();
        assert!(config_manager
            .validate_model_config(&invalid_model)
            .is_err());

        // 测试无效配置 - 空名称
        let mut invalid_model = TestModelConfigs::openai();
        invalid_model.name = "".to_string();
        assert!(config_manager
            .validate_model_config(&invalid_model)
            .is_err());

        // 测试无效配置 - 空API URL
        let mut invalid_model = TestModelConfigs::openai();
        invalid_model.api_url = "".to_string();
        assert!(config_manager
            .validate_model_config(&invalid_model)
            .is_err());

        // 测试无效配置 - 空模型名
        let mut invalid_model = TestModelConfigs::openai();
        invalid_model.model = "".to_string();
        assert!(config_manager
            .validate_model_config(&invalid_model)
            .is_err());
    }

    /// 测试配置保存和加载
    #[test]
    fn test_save_and_load_settings() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加模型配置
        let models = vec![
            TestModelConfigs::openai(),
            TestModelConfigs::claude(),
            TestModelConfigs::local(),
        ];

        for model in &models {
            assert!(config_manager.add_model(model.clone()).is_ok());
        }

        // 设置默认模型
        assert!(config_manager.set_default_model(&models[0].id).is_ok());

        // 保存设置
        assert!(config_manager.save_settings().is_ok());

        // 创建新的配置管理器并加载设置
        let storage_path = config_manager.get_storage_path().to_path_buf();
        let storage = SecureStorage::new(storage_path);
        let loaded_config_manager = AIConfigManager::with_storage(storage);

        // 验证加载的配置
        assert_eq!(loaded_config_manager.get_models().len(), 3);
        assert_eq!(
            loaded_config_manager.get_default_model_id(),
            Some(&models[0].id)
        );

        for model in &models {
            let loaded_model = loaded_config_manager.get_model(&model.id);
            assert!(loaded_model.is_some());
            assert_eq!(loaded_model.unwrap().provider, model.provider);
        }
    }

    /// 测试配置重置
    #[test]
    fn test_reset_config() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 添加配置
        let models = TestModelConfigs::all();
        for model in &models {
            assert!(config_manager.add_model(model.clone()).is_ok());
        }
        assert!(config_manager.set_default_model(&models[0].id).is_ok());

        // 验证配置已添加
        assert_eq!(config_manager.get_models().len(), 4);
        assert!(config_manager.get_default_model_id().is_some());

        // 重置配置
        config_manager.reset_to_defaults();

        // 验证配置已重置
        assert!(config_manager.get_models().is_empty());
        assert!(config_manager.get_default_model_id().is_none());
    }

    /// 测试配置导入导出
    #[test]
    fn test_import_export_config() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 创建测试设置
        let settings = TestSettings::complete();

        // 导入设置
        assert!(config_manager.import_settings(settings.clone()).is_ok());

        // 验证导入
        assert_eq!(config_manager.get_models().len(), settings.models.len());
        assert_eq!(
            config_manager.get_default_model_id(),
            settings.default_model_id.as_ref()
        );

        // 导出设置
        let exported_settings = config_manager.export_settings();

        // 验证导出
        assert_eq!(exported_settings.models.len(), settings.models.len());
        assert_eq!(
            exported_settings.default_model_id,
            settings.default_model_id
        );

        for (original, exported) in settings.models.iter().zip(exported_settings.models.iter()) {
            assert_eq!(original.id, exported.id);
            assert_eq!(original.provider, exported.provider);
        }
    }

    /// 测试并发配置操作
    #[tokio::test]
    async fn test_concurrent_config_operations() {
        let (_temp_dir, config_manager) = create_test_config_manager();
        let config_manager = std::sync::Arc::new(tokio::sync::Mutex::new(config_manager));

        // 并发添加模型
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let config_manager = config_manager.clone();
                tokio::spawn(async move {
                    let mut manager = config_manager.lock().await;
                    let mut model = TestModelConfigs::openai();
                    model.id = format!("concurrent-model-{}", i);
                    model.name = format!("Concurrent Model {}", i);
                    manager.add_model(model)
                })
            })
            .collect();

        // 等待所有操作完成
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // 验证所有模型都已添加
        let manager = config_manager.lock().await;
        assert_eq!(manager.get_models().len(), 5);
    }

    /// 测试配置管理器的错误处理
    #[test]
    fn test_config_manager_error_handling() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 测试添加无效模型
        let mut invalid_model = TestModelConfigs::openai();
        invalid_model.id = "".to_string();
        assert!(config_manager.add_model(invalid_model).is_err());

        // 测试更新不存在的模型
        let model = TestModelConfigs::openai();
        assert!(config_manager.update_model("nonexistent", model).is_err());

        // 测试删除不存在的模型
        assert!(config_manager.remove_model("nonexistent").is_err());

        // 测试设置不存在的默认模型
        assert!(config_manager.set_default_model("nonexistent").is_err());
    }

    /// 测试配置管理器的边界条件
    #[test]
    fn test_config_manager_edge_cases() {
        let (_temp_dir, mut config_manager) = create_test_config_manager();

        // 测试空配置状态
        assert!(config_manager.get_models().is_empty());
        assert!(config_manager.get_default_model_id().is_none());

        // 测试单个模型操作
        let model = TestModelConfigs::openai();
        assert!(config_manager.add_model(model.clone()).is_ok());
        assert_eq!(config_manager.get_models().len(), 1);

        // 删除唯一模型
        assert!(config_manager.remove_model(&model.id).is_ok());
        assert!(config_manager.get_models().is_empty());

        // 测试大量模型
        for i in 0..100 {
            let mut model = TestModelConfigs::openai();
            model.id = format!("model-{}", i);
            assert!(config_manager.add_model(model).is_ok());
        }
        assert_eq!(config_manager.get_models().len(), 100);
    }
}
