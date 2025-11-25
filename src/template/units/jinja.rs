use serde::Serialize;
use rust_embed::RustEmbed;
use minijinja::Environment;

pub use minijinja::{Value, context};

use crate::template::{traits::TemplateUnit, error::{TemplateError, TemplateResult}, types::RenderedFile};


#[derive(RustEmbed)]
#[folder = "src/template/templates/"]
#[include = "*.j2"]
pub struct JinjaTemplates;

pub struct JinjaTemplateUnit<S: Serialize> {
    pub template_name: String,
    pub context: S,
}

impl<S: Serialize> JinjaTemplateUnit<S> {
    /// Get a jinja template file by name.
    /// 
    /// # Arguments
    /// * `name` - The name of the template file (without the `.j2` extension).
    /// 
    /// # Returns
    /// An `Option<String>` containing the template content if found, or `None` if not found.
    pub fn get_jinja_template(&self, name: &str) -> Option<String> {
        JinjaTemplates::get(name)
            .and_then(|file| {
                str::from_utf8(file.data.as_ref())
                    .map(|s| s.to_string())
                    .ok()
            })
    }

    /// Render a jinja template with the given context.
    /// 
    /// # Arguments
    /// * `name` - The name of the template file (without the `.j2` extension).
    /// * `context` - A context to render the template with.
    /// 
    /// # Returns
    /// An `Option<String>` containing the rendered template if successful, or `None` if the template is not found or rendering fails.
    pub fn render_jinja_template(&self) -> Option<String> {
        let template_content = self.get_jinja_template(&format!("{}.j2", self.template_name))?;

        let mut env = Environment::new();
        env.add_template(&self.template_name, &template_content).ok()?;
        
        env.get_template(&self.template_name)
            .and_then(|template| template.render(&self.context))
            .map(|s| s.to_string())
            .ok()
    }
}

impl<S: Serialize> TemplateUnit for JinjaTemplateUnit<S> {
    fn render(&self) -> TemplateResult<Vec<RenderedFile>> {
        Ok(vec![
            RenderedFile {
                path: self.template_name.clone().into(),
                content: self.render_jinja_template()
                    .ok_or(TemplateError::RenderFailed(self.template_name.clone()))?,
            }
        ])
    }
}