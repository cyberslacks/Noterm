pub mod frontmatter;
pub mod markdown;
pub mod watcher;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use frontmatter::NoteFrontmatter;
pub use watcher::FileNode;

#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub relative_path: String,
    pub frontmatter: NoteFrontmatter,
    pub body: String,
    pub raw: String,
    pub content_hash: String,
    pub has_mermaid: bool,
    pub has_tasks: bool,
}

impl Note {
    pub fn from_path(path: &Path, notes_dir: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(notes_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let (frontmatter, body) = frontmatter::parse(&raw);

        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw.as_bytes());
            hex::encode(hasher.finalize())
        };

        let has_mermaid = body.contains("```mermaid");
        let has_tasks = frontmatter
            .tasks
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false);

        Ok(Note {
            path: path.to_path_buf(),
            relative_path,
            frontmatter,
            body,
            raw,
            content_hash,
            has_mermaid,
            has_tasks,
        })
    }

    pub fn title(&self) -> &str {
        self.frontmatter
            .title
            .as_deref()
            .unwrap_or(&self.relative_path)
    }

    pub fn save(&self) -> Result<()> {
        let content = frontmatter::serialize(&self.frontmatter, &self.body);
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub relative_path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub relative_path: String,
    pub title: String,
    pub similarity: f32,
}
