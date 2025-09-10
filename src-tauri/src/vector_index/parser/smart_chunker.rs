/*!
 * 智能代码分块器
 *
 * 基于Roo-Code项目的优化策略实现智能代码分割，包括：
 * - 动态chunk size管理
 * - 重新平衡逻辑防止微小余块
 * - 超大行的分段处理
 * - 重复检测机制
 *
 * 设计原则：遵循用户代码简洁性偏好，保持逻辑简单
 */

use crate::vector_index::constants::smart_chunker::*;
use crate::vector_index::types::{ChunkType, CodeChunk};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// 智能分块器
pub struct SmartChunker {
    min_chunk_size: usize,
    max_chunk_size: usize,
    min_remainder_size: usize,
    effective_max_size: usize,
}

impl SmartChunker {
    /// 创建新的智能分块器
    pub fn new() -> Self {
        let effective_max_size = (MAX_CHUNK_SIZE as f32 * MAX_TOLERANCE_FACTOR) as usize;

        Self {
            min_chunk_size: MIN_CHUNK_SIZE,
            max_chunk_size: MAX_CHUNK_SIZE,
            min_remainder_size: MIN_REMAINDER_SIZE,
            effective_max_size,
        }
    }

    /// 智能分块处理，处理过大的代码块
    pub fn chunk_large_content(
        &self,
        content: &str,
        file_path: &str,
        file_hash: &str,
        chunk_type: ChunkType,
        start_line: u32,
    ) -> Result<Vec<CodeChunk>> {
        let lines: Vec<&str> = content.lines().collect();

        if content.len() <= self.max_chunk_size {
            // 内容不大，直接返回单个块
            return Ok(vec![self.create_single_chunk(
                content,
                file_path,
                file_hash,
                chunk_type,
                start_line,
                start_line + lines.len() as u32 - 1,
            )?]);
        }

        // 需要分块处理
        self.chunk_lines_with_rebalancing(lines, file_path, file_hash, &chunk_type, start_line)
    }

    /// 按行进行智能分块，包含重新平衡逻辑
    fn chunk_lines_with_rebalancing(
        &self,
        lines: Vec<&str>,
        file_path: &str,
        file_hash: &str,
        chunk_type: &ChunkType,
        base_start_line: u32,
    ) -> Result<Vec<CodeChunk>> {
        let mut chunks = Vec::new();
        let mut current_lines = Vec::new();
        let mut current_size = 0;
        let mut chunk_start_index = 0;

        for (i, line) in lines.iter().enumerate() {
            let line_size = line.len() + 1; // +1 for newline
            let current_line_number = base_start_line + i as u32;

            // 处理超大单行
            if line_size > self.effective_max_size {
                // 先完成当前块
                if !current_lines.is_empty() {
                    if let Some(chunk) = self.finalize_current_chunk(
                        &current_lines,
                        file_path,
                        file_hash,
                        &chunk_type,
                        base_start_line + chunk_start_index as u32,
                    )? {
                        chunks.push(chunk);
                    }
                    current_lines.clear();
                    current_size = 0;
                }

                // 分段处理超大行
                let segments = self.segment_oversized_line(line, current_line_number);
                for segment in segments {
                    chunks.push(self.create_segment_chunk(
                        &segment,
                        file_path,
                        file_hash,
                        current_line_number,
                    )?);
                }

                chunk_start_index = i + 1;
                continue;
            }

            // 检查是否需要完成当前块
            if current_size > 0 && current_size + line_size > self.effective_max_size {
                // 重新平衡逻辑：检查余块大小
                let remaining_lines = &lines[i..];
                let remaining_size: usize = remaining_lines.iter().map(|l| l.len() + 1).sum();

                if remaining_size < self.min_remainder_size && current_lines.len() > 1 {
                    // 余块太小，尝试重新分配
                    if let Some(better_split) = self.find_better_split(&lines, chunk_start_index, i)
                    {
                        // 使用更好的分割点
                        let chunk_lines = &lines[chunk_start_index..=better_split];
                        if let Some(chunk) = self.create_chunk_from_lines(
                            chunk_lines,
                            file_path,
                            file_hash,
                            &chunk_type,
                            base_start_line + chunk_start_index as u32,
                        )? {
                            chunks.push(chunk);
                        }

                        // 重置状态到新的分割点
                        chunk_start_index = better_split + 1;
                        current_lines.clear();
                        current_size = 0;

                        // 重新处理当前行
                        if chunk_start_index <= i {
                            for j in chunk_start_index..=i {
                                current_lines.push(lines[j]);
                                current_size += lines[j].len() + 1;
                            }
                        }
                        continue;
                    }
                }

                // 没有找到更好的分割点，直接完成当前块
                if let Some(chunk) = self.finalize_current_chunk(
                    &current_lines,
                    file_path,
                    file_hash,
                    &chunk_type,
                    base_start_line + chunk_start_index as u32,
                )? {
                    chunks.push(chunk);
                }

                current_lines.clear();
                current_size = 0;
                chunk_start_index = i;
            }

            // 添加当前行到当前块
            current_lines.push(line);
            current_size += line_size;
        }

        // 处理最后剩余的块
        if !current_lines.is_empty() {
            if let Some(chunk) = self.finalize_current_chunk(
                &current_lines,
                file_path,
                file_hash,
                &chunk_type,
                base_start_line + chunk_start_index as u32,
            )? {
                chunks.push(chunk);
            }
        }

        Ok(chunks)
    }

    /// 寻找更好的分割点以避免产生过小的余块
    fn find_better_split(&self, lines: &[&str], start: usize, current: usize) -> Option<usize> {
        // 从当前位置向前查找，寻找既能满足最小块大小又能保证余块足够大的分割点
        for split_point in (start..current - 1).rev() {
            let chunk_lines = &lines[start..=split_point];
            let chunk_size: usize = chunk_lines.iter().map(|l| l.len() + 1).sum();

            let remaining_lines = &lines[split_point + 1..];
            let remaining_size: usize = remaining_lines.iter().map(|l| l.len() + 1).sum();

            if chunk_size >= self.min_chunk_size && remaining_size >= self.min_remainder_size {
                return Some(split_point);
            }
        }
        None
    }

    /// 完成当前块的创建
    fn finalize_current_chunk(
        &self,
        lines: &[&str],
        file_path: &str,
        file_hash: &str,
        chunk_type: &ChunkType,
        start_line: u32,
    ) -> Result<Option<CodeChunk>> {
        if lines.is_empty() {
            return Ok(None);
        }

        let content = lines.join("\n");
        if content.len() < self.min_chunk_size {
            return Ok(None);
        }

        let end_line = start_line + lines.len() as u32 - 1;
        Ok(Some(self.create_single_chunk(
            &content,
            file_path,
            file_hash,
            chunk_type.clone(),
            start_line,
            end_line,
        )?))
    }

    /// 从行列表创建代码块
    fn create_chunk_from_lines(
        &self,
        lines: &[&str],
        file_path: &str,
        file_hash: &str,
        chunk_type: &ChunkType,
        start_line: u32,
    ) -> Result<Option<CodeChunk>> {
        self.finalize_current_chunk(lines, file_path, file_hash, chunk_type, start_line)
    }

    /// 分段处理超大行
    fn segment_oversized_line(&self, line: &str, _line_number: u32) -> Vec<String> {
        let mut segments = Vec::new();
        let mut remaining = line;

        while !remaining.is_empty() {
            let segment_size = self.max_chunk_size.min(remaining.len());
            let segment = remaining[..segment_size].to_string();
            segments.push(segment);
            remaining = &remaining[segment_size..];
        }

        segments
    }

    /// 创建行段代码块
    fn create_segment_chunk(
        &self,
        segment: &str,
        _file_path: &str,
        _file_hash: &str,
        line_number: u32,
    ) -> Result<CodeChunk> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "oversized_line_segment".to_string());
        metadata.insert("original_line".to_string(), line_number.to_string());

        Ok(CodeChunk {
            content: segment.to_string(),
            start_line: line_number,
            end_line: line_number,
            chunk_type: ChunkType::Other,
            metadata,
        })
    }

    /// 创建单个代码块
    fn create_single_chunk(
        &self,
        content: &str,
        _file_path: &str,
        file_hash: &str,
        chunk_type: ChunkType,
        start_line: u32,
        end_line: u32,
    ) -> Result<CodeChunk> {
        let mut metadata = HashMap::new();
        metadata.insert("file_hash".to_string(), file_hash.to_string());
        metadata.insert("chunk_size".to_string(), content.len().to_string());

        Ok(CodeChunk {
            content: content.to_string(),
            start_line,
            end_line,
            chunk_type,
            metadata,
        })
    }

    /// 生成内容哈希
    pub fn generate_content_hash(
        &self,
        content: &str,
        file_path: &str,
        start_line: u32,
        end_line: u32,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}-{}-{}-{}",
            file_path,
            start_line,
            end_line,
            content.len()
        ));
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for SmartChunker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_content_no_chunking() {
        let chunker = SmartChunker::new();
        let content = "fn small() { println!(\"hello\"); }";

        let result = chunker
            .chunk_large_content(content, "test.rs", "hash123", ChunkType::Function, 1)
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, content);
    }

    #[test]
    fn test_oversized_line_segmentation() {
        let chunker = SmartChunker::new();
        let large_line = "a".repeat(3000); // 超过 effective_max_size

        let segments = chunker.segment_oversized_line(&large_line, 1);
        assert!(segments.len() > 1);
        assert!(segments.iter().all(|s| s.len() <= MAX_CHUNK_SIZE));
    }

    #[test]
    fn test_rebalancing_logic() {
        let chunker = SmartChunker::new();

        // 创建一个场景：大块 + 小余块
        let mut lines = Vec::new();

        // 添加足够的行构成一个大块
        for i in 0..50 {
            lines.push("fn test_function() { println!(\"line\"); }"); // 每行约40字符
        }

        // 添加一个小余块
        lines.push("// small remainder");

        let result = chunker
            .chunk_lines_with_rebalancing(lines, "test.rs", "hash123", ChunkType::Function, 1)
            .unwrap();

        // 应该通过重新平衡避免产生过小的块
        assert!(result
            .iter()
            .all(|chunk| chunk.content.len() >= MIN_CHUNK_SIZE));
    }
}
