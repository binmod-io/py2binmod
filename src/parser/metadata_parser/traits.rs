use std::path::Path;
use async_trait::async_trait;

use crate::{types::ProjectMetadata, parser::error::ParserResult};


#[async_trait]
pub trait MetadataParser: Send + Sync {
    async fn parse(&self, project_dir: &Path) -> ParserResult<ProjectMetadata>;
}