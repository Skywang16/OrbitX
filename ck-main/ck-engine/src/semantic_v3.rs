use anyhow::Result;
use ck_core::{CkError, SearchOptions, SearchResult};
use std::path::Path;
use walkdir::WalkDir;

use super::{SearchProgressCallback, extract_content_from_span, find_nearest_index_root};

/// New semantic search implementation using span-based storage
pub async fn semantic_search_v3(options: &SearchOptions) -> Result<ck_core::SearchResults> {
    semantic_search_v3_with_progress(options, None).await
}

pub async fn semantic_search_v3_with_progress(
    options: &SearchOptions,
    progress_callback: Option<SearchProgressCallback>,
) -> Result<ck_core::SearchResults> {
    // Find the index root
    let index_root = find_nearest_index_root(&options.path).unwrap_or_else(|| {
        if options.path.is_file() {
            options.path.parent().unwrap_or(&options.path).to_path_buf()
        } else {
            options.path.clone()
        }
    });

    let index_dir = index_root.join(".ck");
    if !index_dir.exists() {
        return Err(CkError::Index(
            "No index found. Run 'ck --index' first with embeddings.".to_string(),
        )
        .into());
    }

    if let Some(ref callback) = progress_callback {
        callback("Loading embeddings from sidecar files...");
    }

    // Collect all sidecar files and their embeddings
    let mut file_chunks: Vec<(std::path::PathBuf, ck_index::ChunkEntry)> = Vec::new();

    // Determine expected sidecar extension from index dir
    let expected_ext = if index_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == ".oxi")
        .unwrap_or(false)
    {
        "oxi"
    } else {
        "ck"
    };

    for entry in WalkDir::new(&index_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some(expected_ext) {
                // Load the sidecar file
                if let Ok(index_entry) = ck_index::load_index_entry(path) {
                    let original_file = reconstruct_original_path(path, &index_dir, &index_root);
                    if let Some(original_file) = original_file {
                        for chunk in index_entry.chunks {
                            if chunk.embedding.is_some() {
                                file_chunks.push((original_file.clone(), chunk));
                            }
                        }
                    }
                }
            }
        }
    }

    if file_chunks.is_empty() {
        return Err(CkError::Index(
            "No embeddings found. Run 'ck --index' first with embeddings.".to_string(),
        )
        .into());
    }

    if let Some(ref callback) = progress_callback {
        callback(&format!(
            "Found {} chunks with embeddings",
            file_chunks.len()
        ));
    }

    // Create embedder and embed the query
    if let Some(ref callback) = progress_callback {
        callback("Loading embedding model...");
    }

    // Read the model configuration from the index manifest
    let manifest_path = index_dir.join("manifest.json");
    let resolved_model = if manifest_path.exists() {
        let manifest_data = std::fs::read(&manifest_path)?;
        let manifest: ck_index::IndexManifest = serde_json::from_slice(&manifest_data)?;
        manifest.embedding_model.clone()
    } else {
        None // Use default model for old indexes
    };

    let mut embedder = ck_embed::create_embedder(resolved_model.as_deref())?;
    let query_embeddings = embedder.embed(std::slice::from_ref(&options.query))?;

    if query_embeddings.is_empty() {
        return Ok(ck_core::SearchResults {
            matches: Vec::new(),
            closest_below_threshold: None,
        });
    }

    let query_embedding = &query_embeddings[0];

    if let Some(ref callback) = progress_callback {
        callback("Computing similarity scores...");
    }

    // Compute similarities
    let mut similarities: Vec<(f32, &std::path::PathBuf, &ck_index::ChunkEntry)> = Vec::new();

    for (file_path, chunk) in &file_chunks {
        if let Some(ref embedding) = chunk.embedding {
            let similarity = cosine_similarity(query_embedding, embedding);
            similarities.push((similarity, file_path, chunk));
        }
    }

    // Sort by similarity (highest first)
    similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Apply threshold and top_k filtering
    let mut results = Vec::new();
    let mut closest_below_threshold: Option<SearchResult> = None;
    let limit = options.top_k.unwrap_or(similarities.len());

    for (similarity, file_path, chunk) in similarities.into_iter().take(limit) {
        let is_below_threshold = options
            .threshold
            .is_some_and(|threshold| similarity < threshold);

        // Check if we're filtering by a specific file or directory (apply to both above/below threshold)
        let passes_path_filter = if options.path.is_file() {
            let target_file = options
                .path
                .canonicalize()
                .unwrap_or_else(|_| options.path.clone());
            let result_file = file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.clone());
            result_file == target_file
        } else if options.path != Path::new(".") {
            // Filter by directory path - only include files within the specified directory
            let target_dir = options
                .path
                .canonicalize()
                .unwrap_or_else(|_| options.path.clone());
            let result_file = file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.clone());
            result_file.starts_with(&target_dir)
        } else {
            true
        };

        if !passes_path_filter {
            continue;
        }

        // Extract content from the file using the span
        let content = if options.full_section {
            extract_content_from_span(file_path, &chunk.span).await?
        } else {
            let full_content = extract_content_from_span(file_path, &chunk.span).await?;
            // Take first 3 lines for preview
            full_content.lines().take(3).collect::<Vec<_>>().join("\n")
        };

        let search_result = SearchResult {
            file: file_path.clone(),
            span: chunk.span.clone(),
            score: similarity,
            preview: content,
            lang: ck_core::Language::from_path(file_path),
            symbol: None,
            chunk_hash: None,
            index_epoch: None,
        };

        if is_below_threshold {
            // Track the closest below-threshold result (first one since sorted by highest first)
            if closest_below_threshold.is_none() {
                closest_below_threshold = Some(search_result);
            }
        } else {
            // Add to main results if above threshold
            results.push(search_result);
        }
    }

    // Apply reranking if enabled
    if options.rerank && !results.is_empty() {
        if let Some(ref callback) = progress_callback {
            callback("Reranking results for improved relevance...");
        }

        let rerank_model_name = match options.rerank_model.as_deref() {
            Some("jina") => Some("jina-reranker-v1-base-en"),
            Some("bge") => Some("BAAI/bge-reranker-base"),
            Some(name) => Some(name), // Pass through custom model names
            None => Some("jina-reranker-v1-base-en"), // Default to jina
        };

        match ck_embed::create_reranker(rerank_model_name) {
            Ok(mut reranker) => {
                let documents: Vec<String> = results.iter().map(|r| r.preview.clone()).collect();

                match reranker.rerank(&options.query, &documents) {
                    Ok(rerank_results) => {
                        // Update results with reranked scores
                        // The reranker returns results in reranked order, so we match by document text
                        for rerank_result in rerank_results.iter() {
                            if let Some(result) = results
                                .iter_mut()
                                .find(|r| r.preview == rerank_result.document)
                            {
                                result.score = rerank_result.score;
                            }
                        }

                        // Re-sort by reranked scores
                        results.sort_by(|a, b| {
                            b.score
                                .partial_cmp(&a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });

                        // Apply top_k limit again after reranking
                        if let Some(limit) = options.top_k {
                            results.truncate(limit);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Reranking failed, using original scores: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create reranker, using original scores: {}", e);
            }
        }
    }

    Ok(ck_core::SearchResults {
        matches: results,
        closest_below_threshold,
    })
}

fn reconstruct_original_path(
    sidecar_path: &Path,
    index_dir: &Path,
    repo_root: &Path,
) -> Option<std::path::PathBuf> {
    // Remove the index directory prefix and sidecar extension
    let relative_path = sidecar_path.strip_prefix(index_dir).ok()?;
    let mut original_path = relative_path.with_extension("");

    // Determine expected extension from index dir and strip only that
    let expected_ext = if index_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == ".oxi")
        .unwrap_or(false)
    {
        ".oxi"
    } else {
        ".ck"
    };
    if let Some(name) = original_path.file_name() {
        let name_str = name.to_string_lossy();
        if let Some(original_name) = name_str.strip_suffix(expected_ext) {
            let mut new_path = original_path.clone();
            new_path.set_file_name(original_name);
            original_path = new_path;
        }
    }

    Some(repo_root.join(original_path))
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
