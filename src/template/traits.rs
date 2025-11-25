use crate::template::{error::TemplateResult, types::RenderedFile};

pub trait TemplateUnit {
    fn render(&self) -> TemplateResult<Vec<RenderedFile>>;
}
