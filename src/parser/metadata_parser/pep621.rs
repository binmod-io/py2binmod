use std::path::{Path, PathBuf};
use serde::Deserialize;
use async_trait::async_trait;
use tokio::fs;

use crate::{
    types::{ProjectMetadata, Py2BinmodConfig},
    parser::error::{ParserError, ParserResult},
    parser::metadata_parser::traits::MetadataParser
};


#[derive(Deserialize, Debug)]
struct PyProjectToml {
    project: Option<ProjectSection>,
    tool: Option<ToolSection>,
}

#[derive(Deserialize, Debug)]
struct ToolSection {
    #[serde(rename = "py2binmod")]
    py2binmod: Option<Py2BinmodToml>,
}

#[derive(Deserialize, Debug)]
struct ProjectSection {
    name: String,
    version: String,
    description: Option<String>,
    authors: Option<Vec<Author>>,
    license: Option<License>,
    #[serde(rename = "requires-python")]
    requires_python: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Author {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum License {
    Simple(String),
    Detailed { text: Option<String>, file: Option<String> },
}

#[derive(Deserialize, Debug)]
struct Py2BinmodToml {
    pub venv: Option<String>,
    #[serde(rename = "module-root")]
    pub module_root: Option<String>,
    pub module: Option<String>,
}

pub struct Pep621MetadataParser;

impl Pep621MetadataParser {
    pub fn new() -> Self {
        Pep621MetadataParser
    }
}

#[async_trait]
impl MetadataParser for Pep621MetadataParser {
    async fn parse(&self, project_dir: &Path) -> ParserResult<ProjectMetadata> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let content = fs::read_to_string(&pyproject_path)
            .await
            .map_err(|_| ParserError::MissingProjectMetadata)?;
        let pyproject: PyProjectToml = toml::from_str(&content)?;
        let py2binmod_config = pyproject
            .tool
            .and_then(|tool| tool.py2binmod)
            .map(|c| Py2BinmodConfig {
                venv: c.venv.map(PathBuf::from),
                module_root: c.module_root.map(PathBuf::from),
                module: c.module,
            });

        Ok(ProjectMetadata {
            name: pyproject
                .project
                .as_ref()
                .ok_or(ParserError::MissingProjectMetadata)?
                .name
                .clone(),
            version: pyproject
                .project
                .as_ref()
                .ok_or(ParserError::MissingProjectMetadata)?
                .version
                .clone(),
            requires_python: pyproject
                .project
                .as_ref()
                .and_then(|p| p.requires_python.clone()),
            description: pyproject
                .project
                .as_ref()
                .and_then(|p| p.description.clone()),
            authors: pyproject
                .project
                .as_ref()
                .and_then(|p| p.authors.as_ref())
                .map(|authors| {
                    authors
                        .iter()
                        .filter_map(|a| a.name.clone().or_else(|| a.email.clone()))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default(),
            license: pyproject
                .project
                .as_ref()
                .and_then(|p| p.license.as_ref())
                .and_then(|lic| match lic {
                    License::Simple(s) => Some(s.clone()),
                    License::Detailed { text, file } => text.clone().or_else(|| file.clone()),
                }),
            py2binmod: py2binmod_config,
        })
    }
}