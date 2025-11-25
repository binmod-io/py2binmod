use thiserror::Error;


#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Missing file: {0}")]
    MissingFile(String),
    #[error("Invalid syntax at line {line}, column {column}: {message}")]
    InvalidSyntax {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("Unsupported metadata strategy: {0}")]
    UnsupportedMetadataStrategy(String),
    #[error("Parameter '{0}' is missing a type annotation")]
    ParameterMissingTypeAnnotation(String),
    #[error("Missing project metadata")]
    MissingProjectMetadata,
    #[error("Missing module")]
    MissingModule,
    #[error("Missing virtual environment")]
    MissingVirtualEnv,
    #[error("Missing site packages")]
    MissingSitePackages,
    #[error("Invalid project directory: {0}")]
    InvalidProjectDir(std::path::PathBuf),
    #[error("Invalid TOML: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

pub type ParserResult<T> = Result<T, ParserError>;
