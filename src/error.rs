use thiserror::Error;

use crate::{compiler::error::CompilerError, parser::error::ParserError, template::error::TemplateError};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Compilation error: {0}")]
    CompilationError(#[from] CompilerError),
    #[error("Parser error: {0}")]
    ParserError(#[from] ParserError),
    #[error("Template error: {0}")]
    TemplateError(#[from] TemplateError),
    #[error("Generator error: {0}")]
    GeneratorError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;