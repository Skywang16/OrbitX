//! Models.dev API client
//!
//! Fetches and caches model definitions from https://models.dev/api.json
//! This provides up-to-date model information including capabilities like reasoning.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, error, info, warn};

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
const CACHE_FILE_NAME: &str = "models_dev_cache.json";
const CACHE_TTL_HOURS: u64 = 24; // Refresh cache every 24 hours

/// Model capabilities from models.dev
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub temperature: bool,
}

/// Interleaved reasoning format (for DeepSeek etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InterleavedConfig {
    Enabled(bool),
    Config {
        field: String, // "reasoning_content" or "reasoning_details"
    },
}

/// Model cost information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCost {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
    #[serde(default)]
    pub reasoning: Option<f64>,
}

/// Model limits
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLimits {
    #[serde(default)]
    pub context: u32,
    #[serde(default)]
    pub input: Option<u32>,
    #[serde(default)]
    pub output: u32,
}

/// Model modalities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelModalities {
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
}

/// Model definition from models.dev
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default)]
    pub interleaved: Option<InterleavedConfig>,
    #[serde(default)]
    pub cost: Option<ModelCost>,
    #[serde(default)]
    pub limit: Option<ModelLimits>,
    #[serde(default)]
    pub modalities: Option<ModelModalities>,
    #[serde(default)]
    pub knowledge: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub open_weights: Option<bool>,
}

impl ModelDef {
    /// Get context window size
    pub fn context_window(&self) -> u32 {
        self.limit.as_ref().map(|l| l.context).unwrap_or(128000)
    }

    /// Get max output tokens
    pub fn max_output(&self) -> u32 {
        self.limit.as_ref().map(|l| l.output).unwrap_or(8192)
    }
}

/// Provider definition from models.dev
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub npm: Option<String>,
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub models: HashMap<String, ModelDef>,
}

/// Cached models data with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedData {
    timestamp: i64,
    providers: HashMap<String, ProviderDef>,
}

/// Global models cache
static MODELS_CACHE: Lazy<RwLock<Option<HashMap<String, ProviderDef>>>> =
    Lazy::new(|| RwLock::new(None));

/// Get cache file path
fn get_cache_path() -> Option<PathBuf> {
    let Some(data_dir) = dirs::data_dir() else {
        warn!("System data directory is unavailable; models.dev disk cache disabled");
        return None;
    };

    // Use data directory for cache storage
    let cache_dir = data_dir.join("OrbitX").join("cache");

    // Ensure directory exists
    if !cache_dir.exists() {
        if let Err(err) = std::fs::create_dir_all(&cache_dir) {
            warn!(
                "Failed to create models.dev cache directory '{}': {}",
                cache_dir.display(),
                err
            );
        }
    }

    Some(cache_dir.join(CACHE_FILE_NAME))
}

/// Check if cache is still valid
fn is_cache_valid(timestamp: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    let ttl_seconds = CACHE_TTL_HOURS * 3600;
    (now - timestamp) < ttl_seconds as i64
}

/// Load cached data from disk
async fn load_from_disk() -> Option<HashMap<String, ProviderDef>> {
    let path = get_cache_path()?;
    if !path.exists() {
        return None;
    }

    match fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<CachedData>(&content) {
            Ok(cached) => {
                if is_cache_valid(cached.timestamp) {
                    debug!(
                        "Loaded models from cache (age: {}s)",
                        chrono::Utc::now().timestamp() - cached.timestamp
                    );
                    Some(cached.providers)
                } else {
                    debug!("Cache expired, will refresh");
                    None
                }
            }
            Err(e) => {
                warn!("Failed to parse cache: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Failed to read cache file: {}", e);
            None
        }
    }
}

/// Save data to disk cache
async fn save_to_disk(providers: &HashMap<String, ProviderDef>) {
    let Some(path) = get_cache_path() else {
        return;
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent).await {
            error!(
                "Failed to create models.dev cache parent directory '{}': {}",
                parent.display(),
                err
            );
            return;
        }
    }

    let cached = CachedData {
        timestamp: chrono::Utc::now().timestamp(),
        providers: providers.clone(),
    };

    match serde_json::to_string(&cached) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, content).await {
                error!("Failed to write cache: {}", e);
            } else {
                debug!("Saved models cache to disk");
            }
        }
        Err(e) => {
            error!("Failed to serialize cache: {}", e);
        }
    }
}

/// Fetch models from models.dev API
async fn fetch_from_api() -> Result<HashMap<String, ProviderDef>, String> {
    info!("Fetching models from {}", MODELS_DEV_URL);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(MODELS_DEV_URL)
        .header("User-Agent", "OrbitX/0.2.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    let providers: HashMap<String, ProviderDef> =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    info!("Fetched {} providers from models.dev", providers.len());
    Ok(providers)
}

/// Get all providers with their models
pub async fn get_providers() -> Result<HashMap<String, ProviderDef>, String> {
    // Check memory cache first
    {
        let cache = MODELS_CACHE.read().unwrap_or_else(|e| {
            warn!("Models cache poisoned, recovering: {}", e);
            e.into_inner()
        });
        if let Some(ref providers) = *cache {
            return Ok(providers.clone());
        }
    }

    // Try disk cache
    if let Some(providers) = load_from_disk().await {
        let mut cache = MODELS_CACHE.write().unwrap_or_else(|e| {
            warn!("Models cache poisoned, recovering: {}", e);
            e.into_inner()
        });
        *cache = Some(providers.clone());
        return Ok(providers);
    }

    // Fetch from API
    let providers = fetch_from_api().await.map_err(|err| {
        error!("Failed to fetch models: {}", err);
        err
    })?;

    // Save to disk cache
    save_to_disk(&providers).await;

    // Update memory cache
    let mut cache = MODELS_CACHE.write().unwrap_or_else(|e| {
        warn!("Models cache poisoned, recovering: {}", e);
        e.into_inner()
    });
    *cache = Some(providers.clone());
    Ok(providers)
}

/// Force refresh models from API
pub async fn refresh() -> Result<(), String> {
    let providers = fetch_from_api().await?;
    save_to_disk(&providers).await;

    let mut cache = MODELS_CACHE.write().unwrap_or_else(|e| {
        warn!("Models cache poisoned, recovering: {}", e);
        e.into_inner()
    });
    *cache = Some(providers);

    Ok(())
}

/// Get a specific provider by ID
pub async fn get_provider(provider_id: &str) -> Result<Option<ProviderDef>, String> {
    let providers = get_providers().await?;
    Ok(providers.get(provider_id).cloned())
}

/// Get a specific model by provider and model ID
pub async fn get_model(provider_id: &str, model_id: &str) -> Result<Option<ModelDef>, String> {
    let provider = get_provider(provider_id).await?;
    Ok(provider.and_then(|provider| provider.models.get(model_id).cloned()))
}

/// Filter providers to only include commonly used ones
pub fn filter_common_providers(providers: &HashMap<String, ProviderDef>) -> Vec<&ProviderDef> {
    const COMMON_PROVIDERS: &[&str] = &[
        "anthropic",
        "openai",
        "google", // Gemini
        "deepseek",
        "xai",     // Grok
        "alibaba", // Qwen
        "openrouter",
        "ollama-cloud",
    ];

    COMMON_PROVIDERS
        .iter()
        .filter_map(|id| providers.get(*id))
        .collect()
}

/// Simplified model info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub reasoning: bool,
    pub tool_call: bool,
    pub attachment: bool,
    pub context_window: u32,
    pub max_output: u32,
}

impl From<&ModelDef> for ModelInfo {
    fn from(model: &ModelDef) -> Self {
        Self {
            id: model.id.clone(),
            name: model.name.clone(),
            reasoning: model.reasoning,
            tool_call: model.tool_call,
            attachment: model.attachment,
            context_window: model.context_window(),
            max_output: model.max_output(),
        }
    }
}

/// Simplified provider info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub api_url: Option<String>,
    pub env_vars: Vec<String>,
    pub models: Vec<ModelInfo>,
}

impl From<&ProviderDef> for ProviderInfo {
    fn from(provider: &ProviderDef) -> Self {
        let mut models: Vec<ModelInfo> = provider.models.values().map(ModelInfo::from).collect();

        // Sort by name
        models.sort_by(|a, b| a.name.cmp(&b.name));

        Self {
            id: provider.id.clone(),
            name: provider.name.clone(),
            api_url: provider.api.clone(),
            env_vars: provider.env.clone(),
            models,
        }
    }
}

/// Get simplified provider info for frontend
pub async fn get_provider_infos() -> Result<Vec<ProviderInfo>, String> {
    let providers = get_providers().await?;
    let common = filter_common_providers(&providers);
    Ok(common.into_iter().map(ProviderInfo::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_def_deserialization() {
        let json = r#"{
            "id": "claude-sonnet-4",
            "name": "Claude Sonnet 4",
            "reasoning": true,
            "tool_call": true,
            "attachment": true,
            "temperature": true,
            "limit": {
                "context": 200000,
                "output": 64000
            }
        }"#;

        let model: ModelDef = serde_json::from_str(json).unwrap();
        assert_eq!(model.id, "claude-sonnet-4");
        assert!(model.reasoning);
        assert!(model.tool_call);
        assert_eq!(model.context_window(), 200000);
        assert_eq!(model.max_output(), 64000);
    }

    #[test]
    fn test_provider_def_deserialization() {
        let json = r#"{
            "id": "anthropic",
            "name": "Anthropic",
            "api": "https://api.anthropic.com/v1",
            "env": ["ANTHROPIC_API_KEY"],
            "models": {
                "claude-sonnet-4": {
                    "id": "claude-sonnet-4",
                    "name": "Claude Sonnet 4",
                    "reasoning": true,
                    "tool_call": true,
                    "attachment": true,
                    "temperature": true
                }
            }
        }"#;

        let provider: ProviderDef = serde_json::from_str(json).unwrap();
        assert_eq!(provider.id, "anthropic");
        assert_eq!(provider.models.len(), 1);
        assert!(provider.models.contains_key("claude-sonnet-4"));
    }

    #[test]
    fn test_cache_validity() {
        let now = chrono::Utc::now().timestamp();

        // Fresh cache should be valid
        assert!(is_cache_valid(now - 3600)); // 1 hour ago

        // Old cache should be invalid
        assert!(!is_cache_valid(now - 100000)); // ~28 hours ago
    }
}
