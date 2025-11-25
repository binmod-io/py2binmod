use std::path::Path;
use async_trait::async_trait;

use crate::parser::file_walker::traits::FileIgnoreStrategy;


pub struct DefaultFileIgnoreStrategy;

impl DefaultFileIgnoreStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FileIgnoreStrategy for DefaultFileIgnoreStrategy {
    fn should_ignore(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(
                name,
                ".venv" | "venv" | "__pycache__" | ".git" | ".hg" | ".svn" |
                "node_modules" | "dist" | "build" | "*.egg-info" | "*.pyc" |
                "*.pyo" | "*.pyd" | "*.so" | "*.dll" | "*.dylib" | ".mypy_cache" |
                ".ruff_cache" | ".pytest_cache"
            )
        } else {
            false
        }
    }
}

