//! Token 估算器 - 用于估算文本的 token 数量

/// 简单的 token 估算器
/// 基于经验分析：代码约 4.2 字符/token，文本约 4.8 字符/token
pub struct TokenEstimator;

impl TokenEstimator {
    /// 估算文本的 token 数量
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let char_count = text.chars().count();

        // 检测内容是代码还是自然语言
        let code_indicators = Self::count_code_indicators(text);
        let total_lines = text.lines().count().max(1);
        let code_density = code_indicators as f32 / total_lines as f32;

        // 根据代码密度调整比例
        let chars_per_token = if code_density > 0.3 {
            // 代码 - 更多 token（符号、标识符）
            4.2
        } else if code_density > 0.1 {
            // 混合内容
            4.4
        } else {
            // 主要是自然语言
            4.8
        };

        (char_count as f32 / chars_per_token).ceil() as usize
    }

    /// 检查文本是否超过 token 限制
    pub fn exceeds_limit(text: &str, max_tokens: usize) -> bool {
        Self::estimate_tokens(text) > max_tokens
    }

    /// 获取不同模型的 token 限制
    pub fn get_model_limit(model_name: &str) -> usize {
        match model_name {
            "BAAI/bge-small-en-v1.5" => 512,
            "BAAI/bge-base-en-v1.5" => 512,
            "BAAI/bge-large-en-v1.5" => 512,
            "BAAI/bge-m3" => 8192,
            "sentence-transformers/all-MiniLM-L6-v2" => 512,
            "nomic-embed-text-v1" | "nomic-embed-text-v1.5" => 8192,
            "jina-embeddings-v2-base-code" => 8192,
            _ => 8192, // 默认使用大模型限制
        }
    }

    /// 获取模型的 chunk 配置 (target_tokens, overlap_tokens)
    pub fn get_model_chunk_config(model_name: Option<&str>) -> (usize, usize) {
        let model = model_name.unwrap_or("BAAI/bge-m3");

        match model {
            // 小模型 - 保持较小的 chunk 以提高精度
            "BAAI/bge-small-en-v1.5" | "sentence-transformers/all-MiniLM-L6-v2" => {
                (400, 80) // 400 tokens 目标, 80 token 重叠 (~20%)
            }

            // 大上下文模型 - 可以使用更大的 chunk
            "nomic-embed-text-v1" | "nomic-embed-text-v1.5" | "jina-embeddings-v2-base-code" => {
                (1024, 200) // 1024 tokens 目标, 200 token 重叠 (~20%)
            }

            // BGE 变体
            "BAAI/bge-base-en-v1.5" | "BAAI/bge-large-en-v1.5" => (400, 80),

            // BGE-M3 - 大上下文
            "BAAI/bge-m3" => (1024, 200),

            // 默认使用大模型配置
            _ => (1024, 200),
        }
    }

    /// 统计代码特征指标
    fn count_code_indicators(text: &str) -> usize {
        let mut count = 0;

        for line in text.lines() {
            let trimmed = line.trim();

            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            // 查找代码模式
            if trimmed.contains('{') || trimmed.contains('}') {
                count += 1;
            }
            if trimmed.contains(';') && !trimmed.ends_with('.') {
                count += 1;
            }
            if trimmed.contains("fn ")
                || trimmed.contains("def ")
                || trimmed.contains("function ")
                || trimmed.contains("func ")
            {
                count += 1;
            }
            if trimmed.contains("->") || trimmed.contains("=>") || trimmed.contains("::") {
                count += 1;
            }
            if trimmed.starts_with("pub ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("public ")
            {
                count += 1;
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(TokenEstimator::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let code = r#"
fn main() {
    println!("Hello, world!");
    let x = 42;
    return x;
}
"#;
        let tokens = TokenEstimator::estimate_tokens(code);
        assert!((15..=25).contains(&tokens), "Got {tokens} tokens");
    }

    #[test]
    fn test_exceeds_limit() {
        assert!(!TokenEstimator::exceeds_limit("short text", 100));

        let long_text = "word ".repeat(200);
        assert!(TokenEstimator::exceeds_limit(&long_text, 100));
    }
}
