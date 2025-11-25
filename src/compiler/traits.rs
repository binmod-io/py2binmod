use std::path::Path;
use async_trait::async_trait;

use crate::compiler::{error::CompilerResult, types::Artifact};


#[async_trait]
pub trait Compiler {
    async fn compile(&self, project_dir: &Path) -> CompilerResult<Artifact>;
}

#[async_trait]
pub trait OutputSink {
    async fn stdout(&self, line: &str);
    async fn stderr(&self, line: &str);
}

pub struct NullOutputSink;

#[async_trait]
impl OutputSink for NullOutputSink {
    async fn stdout(&self, _line: &str) {}
    async fn stderr(&self, _line: &str) {}
}