use crate::vector_db::core::{ChunkId, FileMetadata, Result, VectorDbError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Vector data for a single file
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileVectors {
    /// chunk_id -> vector
    pub chunks: HashMap<ChunkId, Vec<f32>>,
}

/// File system storage manager
pub struct FileStore {
    /// Index root directory
    root_path: PathBuf,
    /// Vector data directory
    vectors_path: PathBuf,
    /// Metadata directory
    metadata_path: PathBuf,
    /// Cache directory
    cache_path: PathBuf,
    /// Project root directory
    project_root: PathBuf,
}

impl FileStore {
    /// Create new file storage
    pub fn new(project_root: &Path) -> Result<Self> {
        let root_path = project_root.join(".orbitx").join("index");
        let vectors_path = root_path.join("vectors");
        let metadata_path = root_path.join("metadata");
        let cache_path = root_path.join("cache");

        Ok(Self {
            root_path,
            vectors_path,
            metadata_path,
            cache_path,
            project_root: project_root.to_path_buf(),
        })
    }

    /// Initialize storage directory structure
    pub fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root_path)?;
        fs::create_dir_all(&self.vectors_path)?;
        fs::create_dir_all(&self.metadata_path)?;
        fs::create_dir_all(&self.cache_path)?;
        Ok(())
    }

    /// Get vector file path corresponding to file
    fn get_vector_file_path(&self, source_file: &Path) -> Result<PathBuf> {
        // Convert source file path to path relative to project root
        let relative_path = source_file.strip_prefix(&self.project_root).map_err(|_| {
            VectorDbError::Index(format!(
                "Source file is outside project root: {}",
                source_file.display()
            ))
        })?;

        // Create corresponding directory structure under vectors directory
        let vector_dir = self.vectors_path.join(match relative_path.parent() {
            Some(parent) => parent,
            None => Path::new(""),
        });

        // Use source file name + .oxi suffix
        let file_name = format!(
            "{}.oxi",
            relative_path
                .file_name()
                .ok_or_else(|| {
                    VectorDbError::Index(format!(
                        "Source file has no file name: {}",
                        source_file.display()
                    ))
                })?
                .to_string_lossy()
        );

        Ok(vector_dir.join(file_name))
    }

    /// Save all vector data for a single file
    pub fn save_file_vectors(
        &self,
        source_file: &Path,
        chunks: &[(ChunkId, Vec<f32>)],
    ) -> Result<()> {
        let vector_file = self.get_vector_file_path(source_file)?;

        // Ensure directory exists
        if let Some(parent) = vector_file.parent() {
            fs::create_dir_all(parent)?;
        }

        // Build vector data
        let file_vectors = FileVectors {
            chunks: chunks.iter().cloned().collect(),
        };

        // Serialize and save
        let data = bincode::serialize(&file_vectors)?;
        fs::write(&vector_file, data)?;

        Ok(())
    }

    /// Load all vector data for a single file
    pub fn load_file_vectors(&self, source_file: &Path) -> Result<FileVectors> {
        let vector_file = self.get_vector_file_path(source_file)?;

        if !vector_file.exists() {
            return Err(VectorDbError::FileNotFound(format!(
                "Vector file not found: {}",
                vector_file.display()
            )));
        }

        let data = fs::read(&vector_file)?;
        let file_vectors: FileVectors = bincode::deserialize(&data)?;
        Ok(file_vectors)
    }

    /// Delete vector data for file
    pub fn delete_file_vectors(&self, source_file: &Path) -> Result<()> {
        let vector_file = self.get_vector_file_path(source_file)?;
        if vector_file.exists() {
            fs::remove_file(&vector_file)?;
        }
        Ok(())
    }

    /// Save file metadata
    pub fn save_file_metadata(&self, metadata: &FileMetadata) -> Result<()> {
        let mut all_metadata = self.load_all_file_metadata()?;
        all_metadata.insert(metadata.path.clone(), metadata.clone());

        let file_path = self.metadata_path.join("files.json");
        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    /// Load all file metadata
    pub fn load_all_file_metadata(&self) -> Result<HashMap<PathBuf, FileMetadata>> {
        let file_path = self.metadata_path.join("files.json");
        if !file_path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(file_path)?;
        let metadata: HashMap<PathBuf, FileMetadata> = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// Clean up expired data
    pub fn cleanup(&self) -> Result<()> {
        // Implement cleanup logic
        // 1. Check for orphaned vector files
        // 2. Delete unreferenced cache
        Ok(())
    }

    /// Get storage root directory
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Get vectors directory
    pub fn vectors_path(&self) -> &Path {
        &self.vectors_path
    }

    /// Calculate disk space used by index directory (bytes)
    pub fn disk_usage_bytes(&self) -> Result<u64> {
        let mut total = 0u64;
        let mut stack = vec![self.root_path.clone()];

        while let Some(dir) = stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(v) => v,
                Err(e) => return Err(VectorDbError::Io(e)),
            };

            for entry in entries {
                let entry = entry.map_err(VectorDbError::Io)?;
                let path = entry.path();
                let meta = entry.metadata().map_err(VectorDbError::Io)?;
                if meta.is_dir() {
                    stack.push(path);
                } else if meta.is_file() {
                    total = total.saturating_add(meta.len());
                }
            }
        }

        Ok(total)
    }
}
