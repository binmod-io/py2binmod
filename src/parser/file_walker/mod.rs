pub mod traits;
pub mod default;

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::parser::{file_walker::traits::FileIgnoreStrategy, error::ParserResult};


pub struct FileWalker<'a> {
    ignore_strategy: &'a dyn FileIgnoreStrategy,
}

impl<'a> FileWalker<'a> {
    pub fn new(ignore_strategy: &'a dyn FileIgnoreStrategy) -> Self {
        Self { ignore_strategy }
    }

    pub async fn walk(&self, project_dir: &Path) -> ParserResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![project_dir.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut entries = fs::read_dir(&dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if self.ignore_strategy.should_ignore(&path) {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }
}