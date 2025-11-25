use std::result::Result;
use thiserror::Error;


#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Template render failed: {0}")]
    RenderFailed(String),
}


pub type TemplateResult<T> = Result<T, TemplateError>;