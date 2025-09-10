/*!
 * 增强的配置管理和URL解析
 *
 * 基于Roo-Code项目的健壮URL解析策略实现：
 * - 智能URL格式检测和修正
 * - 多种输入格式的兼容处理
 * - 配置验证和错误诊断
 * - 降级处理机制
 */

use anyhow::{bail, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use url::Url;

use crate::vector_index::constants::qdrant::*;
use crate::vector_index::types::VectorIndexConfig;

/// URL解析和验证器
pub struct EnhancedConfigManager {
    /// 支持的URL模式
    known_patterns: HashMap<String, UrlPattern>,
}

/// URL模式定义
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UrlPattern {
    name: String,
    default_port: u16,
    requires_https: bool,
    description: String,
}

impl EnhancedConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        let mut known_patterns = HashMap::new();

        // 本地开发模式
        known_patterns.insert(
            "localhost".to_string(),
            UrlPattern {
                name: "本地开发".to_string(),
                default_port: DEFAULT_GRPC_PORT,
                requires_https: false,
                description: "本地Qdrant实例，默认gRPC端口".to_string(),
            },
        );

        // Qdrant Cloud模式
        known_patterns.insert(
            "cloud.qdrant.io".to_string(),
            UrlPattern {
                name: "Qdrant Cloud".to_string(),
                default_port: CLOUD_PORT,
                requires_https: true,
                description: "Qdrant云服务，要求HTTPS连接".to_string(),
            },
        );

        // 自定义部署模式
        known_patterns.insert(
            "custom".to_string(),
            UrlPattern {
                name: "自定义部署".to_string(),
                default_port: DEFAULT_GRPC_PORT,
                requires_https: false,
                description: "自定义Qdrant部署".to_string(),
            },
        );

        Self { known_patterns }
    }

    /// 解析和规范化Qdrant URL（基于Roo-Code的parseQdrantUrl逻辑）
    pub fn parse_and_validate_url(&self, url_input: &str) -> Result<String> {
        debug!("开始解析URL: {}", url_input);

        // 处理空输入
        if url_input.trim().is_empty() {
            let default_url = format!("http://localhost:{}", DEFAULT_GRPC_PORT);
            info!("URL为空，使用默认值: {}", default_url);
            return Ok(default_url);
        }

        let trimmed_url = url_input.trim();

        // 检查是否包含协议
        if !trimmed_url.starts_with("http://")
            && !trimmed_url.starts_with("https://")
            && !trimmed_url.contains("://")
        {
            // 没有协议 - 作为主机名处理
            return self.parse_hostname_only(trimmed_url);
        }

        // 尝试作为完整URL解析
        match Url::parse(trimmed_url) {
            Ok(parsed_url) => self.validate_and_enhance_url(&parsed_url, trimmed_url),
            Err(_) => {
                // 解析失败 - 尝试作为主机名处理
                warn!("URL解析失败，尝试作为主机名处理: {}", trimmed_url);
                self.parse_hostname_only(trimmed_url)
            }
        }
    }

    /// 处理仅主机名的输入
    fn parse_hostname_only(&self, hostname: &str) -> Result<String> {
        debug!("处理主机名输入: {}", hostname);

        if hostname.contains(':') {
            // 包含端口 - 添加协议前缀
            let url = if hostname.starts_with("http") {
                hostname.to_string()
            } else {
                format!("http://{}", hostname)
            };
            debug!("添加协议前缀: {}", url);
            Ok(url)
        } else {
            // 无端口 - 根据主机名模式添加默认配置
            let url = self.build_url_with_defaults(hostname)?;
            debug!("使用默认配置构建URL: {}", url);
            Ok(url)
        }
    }

    /// 根据主机名模式构建URL
    fn build_url_with_defaults(&self, hostname: &str) -> Result<String> {
        // 检查是否为已知的云服务
        if hostname.contains("cloud.qdrant.io") || hostname.ends_with(".qdrant.io") {
            let pattern = self.known_patterns.get("cloud.qdrant.io").unwrap();
            return Ok(format!("https://{}:{}", hostname, pattern.default_port));
        }

        // 检查是否为localhost
        if hostname == "localhost" || hostname == "127.0.0.1" {
            let pattern = self.known_patterns.get("localhost").unwrap();
            return Ok(format!("http://{}:{}", hostname, pattern.default_port));
        }

        // 默认处理
        let pattern = self.known_patterns.get("custom").unwrap();
        Ok(format!("http://{}:{}", hostname, pattern.default_port))
    }

    /// 验证和增强已解析的URL
    fn validate_and_enhance_url(&self, parsed_url: &Url, original: &str) -> Result<String> {
        let host = parsed_url.host_str().unwrap_or("");
        let scheme = parsed_url.scheme();
        let port = parsed_url.port();

        debug!(
            "验证URL - 主机: {}, 协议: {}, 端口: {:?}",
            host, scheme, port
        );

        // 检查gRPC端口错误使用
        if let Some(p) = port {
            if p == REST_PORT {
                bail!(
                    "检测到将gRPC客户端连接到REST端口{}。请将端口改为{}。\n示例:\n- 本地: http://localhost:{}\n- 云端: https://{}.aws.cloud.qdrant.io:{}",
                    REST_PORT, DEFAULT_GRPC_PORT, DEFAULT_GRPC_PORT, host, DEFAULT_GRPC_PORT
                );
            }
        }

        // 云端特殊处理
        if host.ends_with(".cloud.qdrant.io") || host.ends_with(".qdrant.io") {
            return self.handle_cloud_url(parsed_url, original);
        }

        // localhost特殊处理
        if host == "localhost" || host == "127.0.0.1" {
            return self.handle_localhost_url(parsed_url, original);
        }

        // 其他自定义部署
        self.handle_custom_url(parsed_url, original)
    }

    /// 处理Qdrant Cloud URL
    fn handle_cloud_url(&self, parsed_url: &Url, original: &str) -> Result<String> {
        let host = parsed_url.host_str().unwrap_or("");
        let scheme = parsed_url.scheme();
        let port = parsed_url.port();

        // 云端强烈建议使用HTTPS
        if scheme != "https" {
            warn!(
                "Qdrant Cloud建议使用HTTPS。当前为'{}'，建议切换为'https'",
                scheme
            );
        }

        // 云端gRPC端口必须为CLOUD_PORT
        let final_port = port.unwrap_or(CLOUD_PORT);
        if final_port != CLOUD_PORT {
            bail!(
                "Qdrant Cloud gRPC端点应使用端口{}。请将地址改为: https://{}:{}",
                CLOUD_PORT,
                host,
                CLOUD_PORT
            );
        }

        let final_url = format!("https://{}:{}", host, CLOUD_PORT);
        info!("Qdrant Cloud URL标准化: {} -> {}", original, final_url);
        Ok(final_url)
    }

    /// 处理localhost URL
    fn handle_localhost_url(&self, parsed_url: &Url, original: &str) -> Result<String> {
        let host = parsed_url.host_str().unwrap_or("localhost");
        let scheme = parsed_url.scheme();
        let port = parsed_url.port();

        // localhost通常使用HTTP
        let final_scheme = if scheme == "https" {
            warn!("localhost通常不需要HTTPS，但将保持您的设置");
            "https"
        } else {
            "http"
        };

        // 使用默认gRPC端口
        let final_port = port.unwrap_or(DEFAULT_GRPC_PORT);

        let final_url = format!("{}://{}:{}", final_scheme, host, final_port);
        debug!("localhost URL标准化: {} -> {}", original, final_url);
        Ok(final_url)
    }

    /// 处理自定义部署URL
    fn handle_custom_url(&self, parsed_url: &Url, original: &str) -> Result<String> {
        let host = parsed_url.host_str().unwrap_or("");
        let scheme = parsed_url.scheme();
        let port = parsed_url.port();
        let path = parsed_url.path();

        // 构建最终URL
        let final_port = port.unwrap_or(DEFAULT_GRPC_PORT);
        let mut final_url = format!("{}://{}:{}", scheme, host, final_port);

        // 处理路径前缀（用于反向代理部署）
        if path != "/" && !path.is_empty() {
            let clean_path = path.trim_end_matches('/');
            final_url.push_str(clean_path);
        }

        debug!("自定义URL标准化: {} -> {}", original, final_url);
        Ok(final_url)
    }

    /// 验证完整配置
    pub fn validate_config(&self, config: &VectorIndexConfig) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // 验证URL
        match self.parse_and_validate_url(&config.qdrant_url) {
            Ok(_) => {}
            Err(e) => {
                issues.push(format!("Qdrant URL配置错误: {}", e));
            }
        }

        // 验证集合名称
        if config.collection_name.is_empty() {
            issues.push("集合名称不能为空".to_string());
        } else if !self.is_valid_collection_name(&config.collection_name) {
            issues.push("集合名称包含无效字符，建议使用字母、数字和连字符".to_string());
        }

        // 验证embedding模型ID
        if config.embedding_model_id.is_empty() {
            issues.push("embedding模型ID不能为空".to_string());
        }

        // 验证并发设置
        if config.max_concurrent_files == 0 {
            issues.push("最大并发文件数不能为0".to_string());
        } else if config.max_concurrent_files > 20 {
            issues.push("最大并发文件数过大，建议不超过20".to_string());
        }

        Ok(issues)
    }

    /// 验证集合名称有效性
    fn is_valid_collection_name(&self, name: &str) -> bool {
        // 集合名称应该符合Qdrant的命名规范
        !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !name.starts_with('-')
            && !name.ends_with('-')
    }

    /// 获取URL诊断信息
    pub fn diagnose_url(&self, url: &str) -> String {
        match self.parse_and_validate_url(url) {
            Ok(parsed) => {
                format!("✅ URL有效: {} -> {}", url, parsed)
            }
            Err(e) => {
                format!("❌ URL无效: {} - {}", url, e)
            }
        }
    }

    /// 生成配置建议
    pub fn generate_config_suggestions(&self, config: &VectorIndexConfig) -> Vec<String> {
        let mut suggestions = Vec::new();

        // URL建议
        if let Ok(parsed_url) = Url::parse(&config.qdrant_url) {
            if let Some(host) = parsed_url.host_str() {
                if host.contains("cloud.qdrant.io") && parsed_url.scheme() == "http" {
                    suggestions.push("建议Qdrant Cloud使用HTTPS协议".to_string());
                }

                if parsed_url.port() == Some(6333) {
                    suggestions.push("建议gRPC客户端使用端口6334而非6333".to_string());
                }
            }
        }

        // 性能建议
        if config.max_concurrent_files < 2 {
            suggestions.push("增加并发文件数可以提高索引构建速度".to_string());
        }

        // 安全建议
        if config.qdrant_api_key.is_none() {
            suggestions.push("建议设置API密钥以增强安全性".to_string());
        }

        suggestions
    }
}

impl Default for EnhancedConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_localhost_url() {
        let manager = EnhancedConfigManager::new();

        // 基础localhost
        assert_eq!(
            manager.parse_and_validate_url("localhost").unwrap(),
            "http://localhost:6334"
        );

        // 带端口的localhost
        assert_eq!(
            manager.parse_and_validate_url("localhost:6334").unwrap(),
            "http://localhost:6334"
        );
    }

    #[test]
    fn test_parse_cloud_url() {
        let manager = EnhancedConfigManager::new();

        // Qdrant Cloud URL
        let cloud_url = "my-cluster.us-east-1.aws.cloud.qdrant.io";
        assert_eq!(
            manager.parse_and_validate_url(cloud_url).unwrap(),
            format!("https://{}:6334", cloud_url)
        );
    }

    #[test]
    fn test_invalid_port_detection() {
        let manager = EnhancedConfigManager::new();

        // 错误的REST端口
        let result = manager.parse_and_validate_url("http://localhost:6333");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("6333"));
    }

    #[test]
    fn test_collection_name_validation() {
        let manager = EnhancedConfigManager::new();

        assert!(manager.is_valid_collection_name("valid-name"));
        assert!(manager.is_valid_collection_name("valid_name"));
        assert!(manager.is_valid_collection_name("valid123"));

        assert!(!manager.is_valid_collection_name(""));
        assert!(!manager.is_valid_collection_name("-invalid"));
        assert!(!manager.is_valid_collection_name("invalid-"));
        assert!(!manager.is_valid_collection_name("invalid name"));
    }
}
