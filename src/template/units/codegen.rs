use std::path::PathBuf;
use syn::parse2;
use prettyplease::unparse;
use proc_macro2::TokenStream;

use crate::{codegen::traits::CodeGenerator, template::{traits::TemplateUnit, error::TemplateResult, types::RenderedFile}};


pub struct CodegenUnit<G>
where
    G: CodeGenerator,
{
    pub destination: PathBuf,
    pub generator: G,
}

impl<G> CodegenUnit<G>
where
    G: CodeGenerator,
{
    pub fn format_token_stream(&self, tokens: TokenStream) -> String {
        unparse(
            &parse2(tokens)
                .expect("Failed to parse TokenStream into syn::File")
        )
        .replace("\r\n", "\n") // Normalize line endings to Unix style
        .replace("\t", "    ") // Replace tabs with spaces for consistency
    }
}

impl<G> TemplateUnit for CodegenUnit<G>
where
    G: CodeGenerator,
{
    fn render(&self) -> TemplateResult<Vec<RenderedFile>> {
        Ok(vec![
            RenderedFile {
                path: self.destination.clone(),
                content: self.format_token_stream(self.generator.generate()),
            }
        ])
    }
}