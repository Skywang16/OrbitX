//! 智能实体提取器

use crate::utils::error::AppResult;
use anyhow::anyhow;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// 全局智能提取器实例
static GLOBAL_SMART_EXTRACTOR: OnceLock<SmartExtractor> = OnceLock::new();

/// 智能实体提取器
pub struct SmartExtractor {
    /// 提取规则
    rules: Vec<ExtractionRule>,
    /// 通用模式
    patterns: HashMap<String, Regex>,
}

/// 提取规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    /// 规则名称
    pub name: String,

    /// 适用的命令模式
    pub command_patterns: Vec<String>,

    /// 输出特征（用于识别命令输出类型）
    pub output_signatures: Vec<String>,

    /// 实体提取模式
    pub entity_patterns: Vec<EntityPattern>,

    /// 规则优先级
    pub priority: i32,

    /// 是否启用
    pub enabled: bool,
}

/// 实体模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPattern {
    /// 实体类型
    pub entity_type: String,

    /// 正则表达式模式
    pub pattern: String,

    /// 捕获组索引（默认为1）
    pub capture_group: Option<usize>,

    /// 最小置信度
    pub min_confidence: f64,

    /// 上下文要求（可选）
    pub context_requirements: Option<Vec<String>>,
}

/// 提取结果
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// 实体类型
    pub entity_type: String,

    /// 实体值
    pub value: String,

    /// 置信度
    pub confidence: f64,

    /// 上下文信息
    pub context: HashMap<String, String>,
}

impl SmartExtractor {
    /// 创建新的智能提取器
    pub fn new() -> Self {
        let mut extractor = Self {
            rules: Vec::new(),
            patterns: HashMap::new(),
        };

        // 加载默认规则
        extractor.load_default_rules();
        extractor.compile_patterns();

        extractor
    }

    /// 获取全局实例
    pub fn global() -> &'static SmartExtractor {
        GLOBAL_SMART_EXTRACTOR.get_or_init(SmartExtractor::new)
    }

    /// 加载默认规则
    fn load_default_rules(&mut self) {
        // 进程相关规则
        self.rules.push(ExtractionRule {
            name: "process_list".to_string(),
            command_patterns: vec!["lsof.*".to_string(), "ps.*".to_string()],
            output_signatures: vec![
                "COMMAND.*PID.*USER".to_string(),
                "PID.*TTY.*TIME.*CMD".to_string(),
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "pid".to_string(),
                    pattern: r"\b(\d{1,6})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "process_name".to_string(),
                    pattern: r"^(\S+)\s+\d+".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: None,
                },
            ],
            priority: 10,
            enabled: true,
        });

        // 网络相关规则
        self.rules.push(ExtractionRule {
            name: "network_info".to_string(),
            command_patterns: vec![
                "netstat.*".to_string(),
                "ss.*".to_string(),
                "lsof.*-i.*".to_string(),
            ],
            output_signatures: vec![
                "Proto.*Local Address.*Foreign Address".to_string(),
                "Netid.*State.*Recv-Q".to_string(),
                "COMMAND.*PID.*USER.*FD.*TYPE".to_string(),
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "port".to_string(),
                    pattern: r":(\d{1,5})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.9,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "ip_address".to_string(),
                    pattern: r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "pid".to_string(),
                    pattern: r"\b(\d{1,6})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: Some(vec!["COMMAND".to_string()]),
                },
            ],
            priority: 10,
            enabled: true,
        });

        // 文件系统相关规则
        self.rules.push(ExtractionRule {
            name: "filesystem".to_string(),
            command_patterns: vec!["ls.*".to_string(), "find.*".to_string()],
            output_signatures: vec![
                r"^[drwx-]{10}".to_string(), // ls -l输出
                r"^\./".to_string(),         // find输出
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "file_path".to_string(),
                    pattern: r"([^\s]+\.[a-zA-Z0-9]+)$".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.6,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "directory_path".to_string(),
                    pattern: r"^d[rwx-]{9}\s+\d+\s+\S+\s+\S+\s+\d+\s+\S+\s+\d+\s+[\d:]+\s+(.+)$"
                        .to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: None,
                },
            ],
            priority: 5,
            enabled: true,
        });

        // Git 相关规则
        self.rules.push(ExtractionRule {
            name: "git_info".to_string(),
            command_patterns: vec!["git.*".to_string()],
            output_signatures: vec![
                "commit [a-f0-9]{40}".to_string(),
                r"\* \w+".to_string(), // git branch输出
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "git_commit".to_string(),
                    pattern: r"commit ([a-f0-9]{7,40})".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.9,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "git_branch".to_string(),
                    pattern: r"^\*?\s*([^\s]+)$".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
            ],
            priority: 8,
            enabled: true,
        });

        // 通用数字模式（作为后备）
        self.rules.push(ExtractionRule {
            name: "generic_numbers".to_string(),
            command_patterns: vec![".*".to_string()],
            output_signatures: vec![],
            entity_patterns: vec![EntityPattern {
                entity_type: "number".to_string(),
                pattern: r"\b(\d+)\b".to_string(),
                capture_group: Some(1),
                min_confidence: 0.3,
                context_requirements: None,
            }],
            priority: 1,
            enabled: true,
        });
    }

    /// 编译正则表达式模式
    fn compile_patterns(&mut self) {
        for rule in &self.rules {
            for pattern in &rule.entity_patterns {
                if let Ok(regex) = Regex::new(&pattern.pattern) {
                    let key = format!("{}_{}", rule.name, pattern.entity_type);
                    self.patterns.insert(key, regex);
                }
            }

            // 编译命令模式
            for (i, cmd_pattern) in rule.command_patterns.iter().enumerate() {
                if let Ok(regex) = Regex::new(cmd_pattern) {
                    let key = format!("{}_cmd_{}", rule.name, i);
                    self.patterns.insert(key, regex);
                }
            }

            // 编译输出特征模式
            for (i, sig_pattern) in rule.output_signatures.iter().enumerate() {
                if let Ok(regex) = Regex::new(sig_pattern) {
                    let key = format!("{}_sig_{}", rule.name, i);
                    self.patterns.insert(key, regex);
                }
            }
        }
    }

    /// 提取实体
    pub fn extract_entities(
        &self,
        command: &str,
        output: &str,
    ) -> AppResult<Vec<ExtractionResult>> {
        let mut results = Vec::new();

        // 找到适用的规则
        let applicable_rules = self.find_applicable_rules(command, output);

        for rule in applicable_rules {
            for pattern in &rule.entity_patterns {
                if let Some(entities) = self.extract_with_pattern(pattern, output, rule)? {
                    results.extend(entities);
                }
            }
        }

        // 按置信度排序并去重
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        self.deduplicate_results(results)
    }

    /// 查找适用的规则
    fn find_applicable_rules(&self, command: &str, output: &str) -> Vec<&ExtractionRule> {
        let mut applicable = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let command_matches = rule.command_patterns.iter().any(|pattern| {
                if let Some(regex) = self.patterns.get(&format!(
                    "{}_cmd_{}",
                    rule.name,
                    rule.command_patterns
                        .iter()
                        .position(|p| p == pattern)
                        .unwrap()
                )) {
                    regex.is_match(command)
                } else {
                    false
                }
            });

            let output_matches = rule.output_signatures.is_empty()
                || rule.output_signatures.iter().any(|signature| {
                    if let Some(regex) = self.patterns.get(&format!(
                        "{}_sig_{}",
                        rule.name,
                        rule.output_signatures
                            .iter()
                            .position(|s| s == signature)
                            .unwrap()
                    )) {
                        regex.is_match(output)
                    } else {
                        false
                    }
                });

            if command_matches && output_matches {
                applicable.push(rule);
            }
        }

        // 按优先级排序
        applicable.sort_by_key(|rule| std::cmp::Reverse(rule.priority));
        applicable
    }

    /// 使用模式提取实体
    fn extract_with_pattern(
        &self,
        pattern: &EntityPattern,
        output: &str,
        rule: &ExtractionRule,
    ) -> AppResult<Option<Vec<ExtractionResult>>> {
        let pattern_key = format!("{}_{}", rule.name, pattern.entity_type);
        let regex = self
            .patterns
            .get(&pattern_key)
            .ok_or_else(|| anyhow!("未找到编译的正则表达式: {}", pattern_key))?;

        let mut results = Vec::new();
        let capture_group = pattern.capture_group.unwrap_or(1);

        for captures in regex.captures_iter(output) {
            if let Some(matched) = captures.get(capture_group) {
                let value = matched.as_str().to_string();

                if let Some(requirements) = &pattern.context_requirements {
                    if !self.check_context_requirements(requirements, output, matched.start()) {
                        continue;
                    }
                }

                let confidence = self.calculate_confidence(pattern, &value, output);

                if confidence >= pattern.min_confidence {
                    results.push(ExtractionResult {
                        entity_type: pattern.entity_type.clone(),
                        value,
                        confidence,
                        context: HashMap::new(),
                    });
                }
            }
        }

        Ok(if results.is_empty() {
            None
        } else {
            Some(results)
        })
    }

    /// 检查上下文要求
    fn check_context_requirements(
        &self,
        requirements: &[String],
        output: &str,
        position: usize,
    ) -> bool {
        let line_start = output[..position]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);
        let line_end = output[position..]
            .find('\n')
            .map(|pos| position + pos)
            .unwrap_or(output.len());
        let line = &output[line_start..line_end];

        requirements.iter().any(|req| line.contains(req))
    }

    /// 计算置信度
    fn calculate_confidence(&self, pattern: &EntityPattern, value: &str, _output: &str) -> f64 {
        let mut confidence = pattern.min_confidence;

        // 根据实体类型调整置信度
        match pattern.entity_type.as_str() {
            "pid" => {
                if let Ok(pid) = value.parse::<u32>() {
                    if pid > 0 && pid < 65536 {
                        confidence += 0.1;
                    }
                }
            }
            "port" => {
                if let Ok(port) = value.parse::<u16>() {
                    if port > 0 {
                        confidence += 0.1;
                    }
                }
            }
            "ip_address" => {
                // 简单的IP地址验证
                let parts: Vec<&str> = value.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|part| part.parse::<u8>().is_ok()) {
                    confidence += 0.1;
                }
            }
            _ => {}
        }

        confidence.min(1.0)
    }

    /// 去重结果
    fn deduplicate_results(
        &self,
        mut results: Vec<ExtractionResult>,
    ) -> AppResult<Vec<ExtractionResult>> {
        let mut seen = std::collections::HashSet::new();
        results.retain(|result| {
            let key = format!("{}:{}", result.entity_type, result.value);
            seen.insert(key)
        });

        Ok(results)
    }

    /// 添加自定义规则
    pub fn add_rule(&mut self, rule: ExtractionRule) -> AppResult<()> {
        self.rules.push(rule);
        self.compile_patterns();
        Ok(())
    }

    /// 从配置文件加载规则
    pub fn load_rules_from_config(&mut self, config_path: &str) -> AppResult<()> {
        use std::fs;

        let config_content =
            fs::read_to_string(config_path).map_err(|e| anyhow!("读取配置文件失败: {}", e))?;

        let config: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| anyhow!("解析配置文件失败: {}", e))?;

        if let Some(rules_array) = config.get("rules").and_then(|r| r.as_array()) {
            for rule_value in rules_array {
                if let Ok(rule) = serde_json::from_value::<ExtractionRule>(rule_value.clone()) {
                    self.add_rule(rule)?;
                }
            }
        }

        Ok(())
    }

    /// 导出当前规则到配置文件
    pub fn export_rules_to_config(&self, config_path: &str) -> AppResult<()> {
        use std::fs;

        let config = serde_json::json!({
            "rules": self.rules,
            "global_settings": {
                "max_entities_per_type": 50,
                "min_global_confidence": 0.5,
                "enable_fuzzy_matching": true,
                "cache_extraction_results": true
            }
        });

        let config_content =
            serde_json::to_string_pretty(&config).map_err(|e| anyhow!("序列化配置失败: {}", e))?;

        fs::write(config_path, config_content).map_err(|e| anyhow!("写入配置文件失败: {}", e))?;

        Ok(())
    }

    /// 重新加载规则（热更新）
    pub fn reload_rules(&mut self, config_path: &str) -> AppResult<()> {
        // 清空现有规则
        self.rules.clear();
        self.patterns.clear();

        // 重新加载默认规则
        self.load_default_rules();

        // 加载配置文件中的规则
        if std::path::Path::new(config_path).exists() {
            self.load_rules_from_config(config_path)?;
        }

        // 重新编译模式
        self.compile_patterns();

        Ok(())
    }

    /// 获取规则统计信息
    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "total_rules".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.rules.len())),
        );

        stats.insert(
            "enabled_rules".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                self.rules.iter().filter(|r| r.enabled).count(),
            )),
        );

        stats.insert(
            "total_patterns".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.patterns.len())),
        );

        let entity_types: std::collections::HashSet<String> = self
            .rules
            .iter()
            .flat_map(|r| r.entity_patterns.iter().map(|p| p.entity_type.clone()))
            .collect();

        stats.insert(
            "supported_entity_types".to_string(),
            serde_json::Value::Array(
                entity_types
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        stats
    }
}

impl Default for SmartExtractor {
    fn default() -> Self {
        Self::new()
    }
}
