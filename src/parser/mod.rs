pub mod file_walker;
pub mod metadata_parser;
pub mod ast_analyzer;
pub mod layout_resolver;
pub mod error;
pub mod traits;

use std::path::{Path, PathBuf};
use futures::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    parser::{
        file_walker::{FileWalker, traits::FileIgnoreStrategy, default::DefaultFileIgnoreStrategy},
        metadata_parser::{traits::MetadataParser, pep621::Pep621MetadataParser},
        ast_analyzer::AstAnalyzer,
        layout_resolver::{LayoutResolver, LayoutHints},
        error::{ParserError, ParserResult},
    },
    types::ProjectContext,
};


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProjectParserOptions {
    pub venv_override: Option<PathBuf>,
    pub module_root_override: Option<PathBuf>,
    pub module_override: Option<String>,
}

pub struct ProjectParser {
    ignore_strategy: Box<dyn FileIgnoreStrategy + Send + Sync>,
    metadata_parser: Box<dyn MetadataParser + Send + Sync>,
    ast_analyzer: AstAnalyzer,
    layout_resolver: LayoutResolver,
    options: ProjectParserOptions,
}

impl ProjectParser {
    pub fn new(
        ignore_strategy: Box<dyn FileIgnoreStrategy + Send + Sync>,
        metadata_parser: Box<dyn MetadataParser + Send + Sync>,
        options: ProjectParserOptions,
    ) -> Self {
        Self {
            ignore_strategy,
            metadata_parser,
            ast_analyzer: AstAnalyzer::new(),
            layout_resolver: LayoutResolver::new(),
            options,
        }
    }

    pub fn builder() -> ProjectParserBuilder {
        ProjectParserBuilder::default()
    }

    pub async fn parse_project(&self, project_dir: &Path) -> ParserResult<ProjectContext> {
        if !project_dir.is_dir() {
            return Err(ParserError::InvalidProjectDir(project_dir.to_path_buf()));
        }

        let files = FileWalker::new(self.ignore_strategy.as_ref())
            .walk(project_dir)
            .await?;

        let metadata = self.metadata_parser
            .parse(project_dir)
            .await?;

        let layout = self.layout_resolver
            .resolve(
                project_dir,
                &files,
                &LayoutHints {
                    venv: self.options.venv_override.clone()
                        .or_else(|| metadata.py2binmod.as_ref().and_then(|c| c.venv.clone())),
                    module_root: self.options.module_root_override.clone()
                        .or_else(|| metadata.py2binmod.as_ref().and_then(|c| c.module_root.clone())),
                    module: self.options.module_override.clone()
                        .or_else(|| metadata.py2binmod.as_ref().and_then(|c| c.module.clone())),
                }
            )?;

        let modules = stream::iter(
                files
                    .into_iter()
                    .filter(|p| p.extension().is_some_and(|ext| ext == "py" && p.starts_with(&layout.module_root)))
            )
            .then(|p| async move { self.ast_analyzer.analyze_file(&p).await })
            .map_ok(|m| m.into_iter())
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(ProjectContext {
            venv_dir: layout.venv_dir,
            site_packages_dir: layout.site_packages_dir,
            project_dir: project_dir.to_path_buf(),
            module_root: layout.module_root,
            module_name: layout.module_name,
            metadata: metadata,
            modules: modules,
        })
    }
}


#[derive(Default)]
pub struct ProjectParserBuilder {
    ignore_strategy: Option<Box<dyn FileIgnoreStrategy + Send + Sync>>,
    metadata_parser: Option<Box<dyn MetadataParser + Send + Sync>>,
    options: Option<ProjectParserOptions>,
}

impl ProjectParserBuilder {
    pub fn new() -> Self {
        Self {
            ignore_strategy: None,
            metadata_parser: None,
            options: None,
        }
    }

    pub fn ignore_strategy(
        mut self,
        strategy: impl FileIgnoreStrategy + Send + Sync + 'static,
    ) -> Self {
        self.ignore_strategy = Some(Box::new(strategy));
        self
    }

    pub fn metadata_parser(
        mut self,
        parser: impl MetadataParser + Send + Sync + 'static,
    ) -> Self {
        self.metadata_parser = Some(Box::new(parser));
        self
    }

    pub fn options(mut self, options: ProjectParserOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn build(self) -> ProjectParser {
        ProjectParser::new(
            self.ignore_strategy.unwrap_or_else(|| Box::new(DefaultFileIgnoreStrategy::new())),
            self.metadata_parser.unwrap_or_else(|| Box::new(Pep621MetadataParser::new())),
            self.options.unwrap_or_default(),
        )
    }
}