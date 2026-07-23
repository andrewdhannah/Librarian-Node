//! Task pack loader — loads versioned fixtures from the filesystem.
//!
//! Each task pack has a fixture_hash and an optional fixture_path.
//! The loader reads the fixture file, verifies its hash, and returns
//! the validated content.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Loaded and validated task pack fixture content.
#[derive(Debug, Clone)]
pub struct LoadedFixture {
    /// The task pack ID this fixture belongs to.
    pub task_pack_id: String,

    /// The fixture content (prompt text).
    pub content: String,

    /// SHA-256 hash of the content (hex).
    pub content_hash: String,

    /// The path the fixture was loaded from.
    pub path: PathBuf,
}

/// Loads and validates task pack fixtures from the filesystem.
pub struct TaskPackLoader {
    /// Base directory for task pack fixtures.
    fixtures_dir: PathBuf,
}

impl TaskPackLoader {
    /// Create a new loader with the given fixtures directory.
    pub fn new(fixtures_dir: impl Into<PathBuf>) -> Self {
        Self {
            fixtures_dir: fixtures_dir.into(),
        }
    }

    /// Load a fixture from a file path.
    /// Verifies the content hash matches the expected hash.
    pub fn load_fixture(
        &self,
        task_pack_id: &str,
        fixture_path: &str,
        expected_hash: &str,
    ) -> Result<LoadedFixture> {
        let path = if Path::new(fixture_path).is_absolute() {
            PathBuf::from(fixture_path)
        } else {
            self.fixtures_dir.join(fixture_path)
        };

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))?;

        let content_hash = compute_hash(&content);

        if content_hash != expected_hash {
            anyhow::bail!(
                "Fixture hash mismatch for '{}': expected '{}', got '{}'",
                task_pack_id,
                expected_hash,
                content_hash
            );
        }

        Ok(LoadedFixture {
            task_pack_id: task_pack_id.to_string(),
            content,
            content_hash,
            path,
        })
    }

    /// Load a fixture from raw content (no filesystem).
    /// Verifies the content hash matches the expected hash.
    pub fn load_fixture_from_content(
        &self,
        task_pack_id: &str,
        content: &str,
        expected_hash: &str,
    ) -> Result<LoadedFixture> {
        let content_hash = compute_hash(content);

        if content_hash != expected_hash {
            anyhow::bail!(
                "Fixture hash mismatch for '{}': expected '{}', got '{}'",
                task_pack_id,
                expected_hash,
                content_hash
            );
        }

        Ok(LoadedFixture {
            task_pack_id: task_pack_id.to_string(),
            content: content.to_string(),
            content_hash,
            path: PathBuf::from(format!("<inline:{}>", task_pack_id)),
        })
    }

    /// Create a fixture file on disk from content.
    /// Returns the path and computed hash.
    pub fn create_fixture(
        &self,
        task_pack_id: &str,
        content: &str,
    ) -> Result<(PathBuf, String)> {
        let filename = format!("{}.txt", task_pack_id);
        let path = self.fixtures_dir.join(&filename);

        std::fs::create_dir_all(&self.fixtures_dir)
            .with_context(|| format!("Failed to create fixtures dir: {}", self.fixtures_dir.display()))?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write fixture: {}", path.display()))?;

        let hash = compute_hash(content);
        Ok((path, hash))
    }
}

/// Compute SHA-256 hash of content (hex).
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
        assert_eq!(hash, compute_hash("hello world")); // deterministic
    }

    #[test]
    fn test_load_fixture_from_content() {
        let loader = TaskPackLoader::new(PathBuf::from("/tmp/fixtures"));
        let content = "Write a function that adds two numbers.";
        let hash = compute_hash(content);

        let loaded = loader
            .load_fixture_from_content("tp-1", content, &hash)
            .unwrap();
        assert_eq!(loaded.task_pack_id, "tp-1");
        assert_eq!(loaded.content, content);
        assert_eq!(loaded.content_hash, hash);
    }

    #[test]
    fn test_load_fixture_hash_mismatch() {
        let loader = TaskPackLoader::new(PathBuf::from("/tmp/fixtures"));
        let content = "Write a function that adds two numbers.";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = loader.load_fixture_from_content("tp-1", content, wrong_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_and_load_fixture() {
        let dir = tempdir().unwrap();
        let loader = TaskPackLoader::new(dir.path().to_path_buf());

        let content = "Summarize the following text.";
        let (path, hash) = loader.create_fixture("tp-1", content).unwrap();
        assert!(path.exists());

        // Load back
        let loaded = loader.load_fixture("tp-1", path.to_str().unwrap(), &hash).unwrap();
        assert_eq!(loaded.content, content);
    }

    #[test]
    fn test_load_fixture_file_not_found() {
        let loader = TaskPackLoader::new(PathBuf::from("/tmp/fixtures"));
        let result = loader.load_fixture("tp-1", "nonexistent.txt", "abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_loaded_fixture_clone() {
        let loader = TaskPackLoader::new(PathBuf::from("/tmp/fixtures"));
        let content = "Test content.";
        let hash = compute_hash(content);

        let loaded = loader
            .load_fixture_from_content("tp-1", content, &hash)
            .unwrap();
        let cloned = loaded.clone();
        assert_eq!(cloned.content, loaded.content);
    }
}
