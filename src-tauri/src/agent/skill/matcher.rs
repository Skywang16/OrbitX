use regex::Regex;
use std::sync::OnceLock;

use super::types::SkillMetadata;

/// 技能匹配器 - 实现显式引用提取和语义匹配
pub struct SkillMatcher {
    // 正则表达式用于提取 @skill 引用
    skill_mention_regex: OnceLock<Regex>,
}

impl SkillMatcher {
    pub fn new() -> Self {
        Self {
            skill_mention_regex: OnceLock::new(),
        }
    }

    /// 提取显式 @skill 引用
    ///
    /// 示例:
    /// - "Use @pdf-processing to extract" -> ["pdf-processing"]
    /// - "Apply @code-review and @linting" -> ["code-review", "linting"]
    pub fn extract_explicit_mentions(
        &self,
        user_prompt: &str,
        available_skills: &[String],
    ) -> Vec<String> {
        let regex = self.skill_mention_regex.get_or_init(|| {
            // 匹配 @skill-name 格式
            Regex::new(r"@([\w-]+)").unwrap()
        });

        let mut mentioned = Vec::new();

        for cap in regex.captures_iter(user_prompt) {
            if let Some(name) = cap.get(1) {
                let skill_name = name.as_str().to_string();

                // 验证技能是否真实存在
                if available_skills.is_empty() || available_skills.contains(&skill_name) {
                    mentioned.push(skill_name);
                }
            }
        }

        mentioned
    }

    /// 语义匹配 - 基于关键词相似度
    ///
    /// 简单实现: 计算 user_prompt 和 skill.description 的关键词重叠度
    /// 生产环境可以用更复杂的算法 (TF-IDF, 向量相似度等)
    ///
    /// 参数:
    /// - user_prompt: 用户输入
    /// - available_skills: 可用技能列表
    /// - limit: 返回最多匹配的技能数量
    pub fn semantic_match(
        &self,
        user_prompt: &str,
        available_skills: &[SkillMetadata],
        limit: usize,
    ) -> Vec<String> {
        let prompt_tokens = Self::tokenize(user_prompt);
        let mut scores: Vec<(String, f32)> = Vec::new();

        for skill in available_skills {
            let desc_tokens = Self::tokenize(&skill.description);
            let score = Self::jaccard_similarity(&prompt_tokens, &desc_tokens);

            if score > 0.0 {
                scores.push((skill.name.to_string(), score));
            }
        }

        // 按分数降序排序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 返回 top N
        scores
            .into_iter()
            .take(limit)
            .map(|(name, _)| name)
            .collect()
    }

    /// 简单分词 (转小写 + 分割)
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|s| s.len() > 2) // 过滤短词
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    /// Jaccard 相似度: |A ∩ B| / |A ∪ B|
    fn jaccard_similarity(a: &[String], b: &[String]) -> f32 {
        if a.is_empty() && b.is_empty() {
            return 0.0;
        }

        let set_a: std::collections::HashSet<_> = a.iter().collect();
        let set_b: std::collections::HashSet<_> = b.iter().collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}

impl Default for SkillMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_explicit_mentions() {
        let matcher = SkillMatcher::new();
        let available = vec!["pdf-processing".to_string(), "code-review".to_string()];

        let prompt = "Use @pdf-processing to extract text from @code-review files";
        let mentions = matcher.extract_explicit_mentions(prompt, &available);

        assert_eq!(mentions.len(), 2);
        assert!(mentions.contains(&"pdf-processing".to_string()));
        assert!(mentions.contains(&"code-review".to_string()));
    }

    #[test]
    fn test_extract_nonexistent_skill() {
        let matcher = SkillMatcher::new();
        let available = vec!["pdf-processing".to_string()];

        let prompt = "Use @nonexistent skill";
        let mentions = matcher.extract_explicit_mentions(prompt, &available);

        // 不存在的技能应该被过滤掉
        assert_eq!(mentions.len(), 0);
    }

    #[test]
    fn test_semantic_match() {
        let matcher = SkillMatcher::new();

        let skills = vec![
            SkillMetadata {
                name: "pdf-processing".into(),
                description: "Extract text and tables from PDF files".into(),
                license: None,
                compatibility: None,
                metadata: Default::default(),
                allowed_tools: None,
                skill_dir: PathBuf::new(),
            },
            SkillMetadata {
                name: "code-review".into(),
                description: "Review code quality and style".into(),
                license: None,
                compatibility: None,
                metadata: Default::default(),
                allowed_tools: None,
                skill_dir: PathBuf::new(),
            },
        ];

        let prompt = "I need to extract text from a PDF document";
        let matches = matcher.semantic_match(prompt, &skills, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "pdf-processing");
    }

    #[test]
    fn test_jaccard_similarity() {
        let a = vec!["extract".to_string(), "text".to_string(), "pdf".to_string()];
        let b = vec!["extract".to_string(), "text".to_string(), "tables".to_string()];

        let score = SkillMatcher::jaccard_similarity(&a, &b);

        // 交集: {extract, text} = 2
        // 并集: {extract, text, pdf, tables} = 4
        // 相似度: 2/4 = 0.5
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_tokenize() {
        let text = "Extract TEXT from PDF files!";
        let tokens = SkillMatcher::tokenize(text);

        assert!(tokens.contains(&"extract".to_string()));
        assert!(tokens.contains(&"text".to_string()));
        assert!(tokens.contains(&"pdf".to_string()));
        assert!(tokens.contains(&"files".to_string()));
    }
}
