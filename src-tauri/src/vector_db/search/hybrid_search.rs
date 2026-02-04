use crate::vector_db::core::{Result, SearchResult};
use std::collections::HashMap;

/// Hybrid 搜索引擎
/// 结合语义搜索和关键词匹配，使用 Reciprocal Rank Fusion (RRF) 算法融合结果
pub struct HybridSearchEngine;

impl HybridSearchEngine {
    /// 执行 Hybrid 搜索
    ///
    /// # 参数
    /// - `query`: 查询字符串
    /// - `semantic_results`: 语义搜索结果
    /// - `keyword_results`: 关键词搜索结果  
    /// - `semantic_weight`: 语义搜索权重 (0.0-1.0)
    /// - `keyword_weight`: 关键词搜索权重 (0.0-1.0)
    /// - `k`: RRF 常数，通常为 60
    pub fn hybrid_search(
        _query: &str,
        semantic_results: Vec<SearchResult>,
        keyword_results: Vec<SearchResult>,
        semantic_weight: f32,
        keyword_weight: f32,
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        // 使用 RRF (Reciprocal Rank Fusion) 算法融合结果
        let total_capacity = semantic_results.len() + keyword_results.len();
        let mut scores: HashMap<String, f32> = HashMap::with_capacity(total_capacity);
        let mut results_map: HashMap<String, SearchResult> = HashMap::with_capacity(total_capacity);

        // 处理语义搜索结果
        for (rank, result) in semantic_results.iter().enumerate() {
            let key = result.unique_key();
            let rrf_score = semantic_weight / (k as f32 + (rank + 1) as f32);
            *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
            results_map.insert(key, result.clone());
        }

        // 处理关键词搜索结果
        for (rank, result) in keyword_results.iter().enumerate() {
            let key = result.unique_key();
            let rrf_score = keyword_weight / (k as f32 + (rank + 1) as f32);
            *scores.entry(key.clone()).or_insert(0.0) += rrf_score;
            results_map.insert(key, result.clone());
        }

        // 按融合后的分数排序
        let mut final_results: Vec<_> = scores
            .into_iter()
            .filter_map(|(key, score)| {
                results_map.get(&key).map(|result| {
                    let mut result = result.clone();
                    result.score = score; // 更新为融合后的分数
                    result
                })
            })
            .collect();

        final_results.sort_by(|a, b| b.score.total_cmp(&a.score));

        Ok(final_results)
    }

    /// 执行简单的关键词搜索
    /// 在所有块中搜索包含查询关键词的内容
    pub fn keyword_search(query: &str, all_results: &[SearchResult]) -> Result<Vec<SearchResult>> {
        // 将查询分词
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();

        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_results = Vec::with_capacity(all_results.len());

        for result in all_results {
            let content_lower = result.preview.to_lowercase();
            let mut match_count = 0;
            let mut total_positions = 0;

            // 计算每个关键词的匹配次数
            for keyword in &keywords {
                if keyword.len() < 2 {
                    continue; // 跳过太短的词
                }

                let matches: Vec<_> = content_lower.match_indices(keyword).collect();
                match_count += matches.len();

                // 记录匹配位置以计算相关性
                for (pos, _) in matches {
                    total_positions += pos;
                }
            }

            if match_count > 0 {
                // 计算关键词匹配分数
                // 考虑：匹配次数、匹配关键词数量、匹配位置（越靠前越好）
                let keyword_coverage = match_count as f32 / keywords.len() as f32;
                let position_score = if total_positions > 0 {
                    1.0 / (1.0 + (total_positions as f32 / content_lower.len() as f32))
                } else {
                    1.0
                };

                let score = keyword_coverage * 0.7 + position_score * 0.3;

                let mut result = result.clone();
                result.score = score;
                scored_results.push(result);
            }
        }

        // 按分数排序
        scored_results.sort_by(|a, b| b.score.total_cmp(&a.score));

        Ok(scored_results)
    }
}

impl SearchResult {
    /// 生成唯一键，用于结果去重
    fn unique_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.file_path.display(),
            self.span.line_start,
            self.span.line_end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::core::{ChunkType, Span};
    use std::path::PathBuf;

    fn create_test_result(
        file: &str,
        line_start: usize,
        line_end: usize,
        preview: &str,
        score: f32,
    ) -> SearchResult {
        SearchResult {
            file_path: PathBuf::from(file),
            span: Span::new(0, 100, line_start, line_end),
            score,
            preview: preview.to_string(),
            language: None,
            chunk_type: Some(ChunkType::Function),
        }
    }

    #[test]
    fn test_hybrid_search() {
        let semantic_results = vec![
            create_test_result("file1.rs", 1, 10, "semantic match", 0.9),
            create_test_result("file2.rs", 5, 15, "another semantic", 0.8),
        ];

        let keyword_results = vec![
            create_test_result("file1.rs", 1, 10, "keyword match", 0.7),
            create_test_result("file3.rs", 20, 30, "keyword only", 0.6),
        ];

        let results = HybridSearchEngine::hybrid_search(
            "test query",
            semantic_results,
            keyword_results,
            0.7,
            0.3,
            60,
        )
        .unwrap();

        assert!(!results.is_empty());
        // file1.rs 应该排在最前面，因为它同时出现在语义和关键词结果中
        assert_eq!(results[0].file_path, PathBuf::from("file1.rs"));
    }

    #[test]
    fn test_keyword_search() {
        let all_results = vec![
            create_test_result("file1.rs", 1, 10, "This is a test function", 0.0),
            create_test_result("file2.rs", 5, 15, "Another test here", 0.0),
            create_test_result("file3.rs", 20, 30, "No match", 0.0),
        ];

        let results = HybridSearchEngine::keyword_search("test", &all_results).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].score > 0.0);
    }
}
