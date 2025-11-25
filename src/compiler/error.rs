use thiserror::Error;


#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    #[error("Unsupported target platform: {0}")]
    UnsupportedTargetPlatform(String),
    #[error("Missing build configuration")]
    MissingBuildConfiguration,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

pub type CompilerResult<T> = Result<T, CompilerError>;
