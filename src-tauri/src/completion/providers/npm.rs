//! NPM命令补全提供者

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::fs;

/// NPM搜索API响应
#[derive(Debug, Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmPackageObject>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageObject {
    package: NpmPackageInfo,
    score: NpmScore,
}

#[derive(Debug, Deserialize)]
struct NpmPackageInfo {
    name: String,
    description: Option<String>,
    version: String,
}

#[derive(Debug, Deserialize)]
struct NpmScore {
    detail: NpmScoreDetail,
}

#[derive(Debug, Deserialize)]
struct NpmScoreDetail {
    popularity: f64,
}

/// Package.json结构
#[derive(Debug, Deserialize)]
struct PackageJson {
    scripts: Option<HashMap<String, String>>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

/// NPM补全提供者
pub struct NpmCompletionProvider {
    /// HTTP客户端
    client: reqwest::Client,
    /// 使用统一缓存
    cache: crate::storage::cache::UnifiedCache,
}

impl NpmCompletionProvider {
    /// 创建新的NPM补全提供者
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .user_agent("OrbitX/1.0")
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache: crate::storage::cache::UnifiedCache::new(),
        }
    }

    /// 解析npm命令
    fn parse_npm_command(
        &self,
        context: &CompletionContext,
    ) -> Option<(String, String, Vec<String>)> {
        let parts: Vec<&str> = context.input.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0];
        if !matches!(command, "npm" | "yarn" | "pnpm") {
            return None;
        }

        if parts.len() == 1 {
            return Some((command.to_string(), "".to_string(), vec![]));
        }

        let subcommand = parts[1].to_string();
        let args = parts[2..].iter().map(|s| s.to_string()).collect();
        Some((command.to_string(), subcommand, args))
    }

    /// 获取npm子命令补全
    fn get_subcommand_completions(&self, query: &str) -> Vec<CompletionItem> {
        let subcommands = vec![
            ("install", "安装依赖包"),
            ("i", "安装依赖包（简写）"),
            ("uninstall", "卸载依赖包"),
            ("update", "更新依赖包"),
            ("run", "运行脚本"),
            ("start", "启动应用"),
            ("test", "运行测试"),
            ("build", "构建项目"),
            ("init", "初始化项目"),
            ("publish", "发布包"),
            ("info", "查看包信息"),
            ("list", "列出已安装的包"),
            ("outdated", "检查过时的包"),
        ];

        let mut completions = Vec::new();
        for (cmd, desc) in subcommands {
            if query.is_empty() || cmd.starts_with(query) {
                let score = if cmd.starts_with(query) { 10.0 } else { 5.0 };

                let item = CompletionItem::new(cmd.to_string(), CompletionType::Subcommand)
                    .with_description(desc.to_string())
                    .with_score(score)
                    .with_source("npm".to_string());

                completions.push(item);
            }
        }

        completions
    }

    /// 获取脚本补全
    async fn get_script_completions(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let package_json_path = working_directory.join("package.json");

        if !package_json_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&package_json_path)
            .await
            .context("读取package.json失败")?;

        let package_json: PackageJson =
            serde_json::from_str(&content).context("解析package.json失败")?;

        let mut completions = Vec::new();

        if let Some(scripts) = package_json.scripts {
            for (name, command) in scripts {
                if query.is_empty() || name.to_lowercase().starts_with(&query.to_lowercase()) {
                    let priority = match name.as_str() {
                        "start" => 20.0,
                        "build" => 18.0,
                        "test" => 16.0,
                        "dev" | "develop" => 15.0,
                        _ => 10.0,
                    };

                    let item = CompletionItem::new(name.clone(), CompletionType::Value)
                        .with_description(format!("脚本: {} -> {}", name, command))
                        .with_score(priority)
                        .with_source("npm".to_string())
                        .with_metadata("type".to_string(), "script".to_string())
                        .with_metadata("command".to_string(), command);

                    completions.push(item);
                }
            }
        }

        Ok(completions)
    }

    /// 获取已安装包的补全
    async fn get_installed_packages(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let package_json_path = working_directory.join("package.json");

        if !package_json_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&package_json_path)
            .await
            .context("读取package.json失败")?;

        let package_json: PackageJson =
            serde_json::from_str(&content).context("解析package.json失败")?;

        let mut completions = Vec::new();

        // 添加dependencies
        if let Some(deps) = package_json.dependencies {
            for (name, version) in deps {
                if query.is_empty() || name.to_lowercase().starts_with(&query.to_lowercase()) {
                    let item = CompletionItem::new(name.clone(), CompletionType::Value)
                        .with_description(format!("依赖包: {} {}", name, version))
                        .with_score(10.0)
                        .with_source("npm".to_string())
                        .with_metadata("type".to_string(), "dependency".to_string())
                        .with_metadata("version".to_string(), version);

                    completions.push(item);
                }
            }
        }

        // 添加devDependencies
        if let Some(dev_deps) = package_json.dev_dependencies {
            for (name, version) in dev_deps {
                if query.is_empty() || name.to_lowercase().starts_with(&query.to_lowercase()) {
                    let item = CompletionItem::new(name.clone(), CompletionType::Value)
                        .with_description(format!("开发依赖: {} {}", name, version))
                        .with_score(8.0)
                        .with_source("npm".to_string())
                        .with_metadata("type".to_string(), "dev_dependency".to_string())
                        .with_metadata("version".to_string(), version);

                    completions.push(item);
                }
            }
        }

        Ok(completions)
    }

    /// 获取包搜索补全
    async fn get_package_search_completions(&self, query: &str) -> AppResult<Vec<CompletionItem>> {
        if query.len() < 3 {
            return Ok(vec![]);
        }

        let cache_key = format!("npm_search:{}", query);
        if let Some(cached_result) = self.cache.get(&cache_key).await {
            if let Ok(items) = serde_json::from_value::<Vec<CompletionItem>>(cached_result) {
                return Ok(items);
            }
        }

        // 只搜索流行的包，避免过多请求
        let url = format!(
            "https://registry.npmjs.org/-/v1/search?text={}&size=5",
            urlencoding::encode(query)
        );

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(_) => return Ok(vec![]), // 网络错误时返回空结果
        };

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let search_result: NpmSearchResponse = match response.json().await {
            Ok(result) => result,
            Err(_) => return Ok(vec![]),
        };

        let mut completions = Vec::new();
        for obj in search_result.objects {
            let package = obj.package;
            let score = obj.score;

            let description = package.description.unwrap_or_else(|| "NPM包".to_string());

            let item = CompletionItem::new(package.name.clone(), CompletionType::Value)
                .with_description(format!("{} - {}", description, package.version))
                .with_score(score.detail.popularity * 100.0)
                .with_source("npm".to_string())
                .with_metadata("type".to_string(), "package".to_string())
                .with_metadata("version".to_string(), package.version);

            completions.push(item);
        }

        // 缓存结果
        if let Ok(cache_value) = serde_json::to_value(&completions) {
            let _ = self.cache.set(&cache_key, cache_value).await;
        }

        Ok(completions)
    }
}

#[async_trait]
impl CompletionProvider for NpmCompletionProvider {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        let input = context.input.trim_start();
        input.starts_with("npm ") || input.starts_with("yarn ") || input.starts_with("pnpm ")
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        let (_command, subcommand, _args) = match self.parse_npm_command(context) {
            Some(parsed) => parsed,
            None => return Ok(vec![]),
        };

        if subcommand.is_empty() {
            return Ok(self.get_subcommand_completions(&context.current_word));
        }

        // 根据子命令提供相应的补全
        match subcommand.as_str() {
            "run" | "run-script" => {
                self.get_script_completions(&context.working_directory, &context.current_word)
                    .await
            }
            "uninstall" | "remove" | "rm" => {
                self.get_installed_packages(&context.working_directory, &context.current_word)
                    .await
            }
            "install" | "i" | "add" => {
                // 优先显示已安装的包，然后搜索新包
                let mut completions = self
                    .get_installed_packages(&context.working_directory, &context.current_word)
                    .await?;

                if context.current_word.len() >= 3 {
                    let search_results = self
                        .get_package_search_completions(&context.current_word)
                        .await?;
                    completions.extend(search_results);
                }

                Ok(completions)
            }
            _ => Ok(vec![]),
        }
    }

    fn priority(&self) -> i32 {
        12 // 中等优先级
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for NpmCompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}
