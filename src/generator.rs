use std::path::{Path, PathBuf};
use tokio::fs;

use crate::{
    types::{ProjectContext, ProjectMetadata, Module},
    template::{
        traits::TemplateUnit,
        types::RenderedFile,
        units::{jinja::{JinjaTemplateUnit, context}, codegen::CodegenUnit},
        error::TemplateResult,
    },
    codegen::lib_rs::LibRsGenerator,
    error::AppResult,
};


pub struct ProjectGenerator {
    context: ProjectContext,
}


impl ProjectGenerator {
    pub fn new(context: ProjectContext) -> Self {
        Self { context }
    }

    pub fn builder() -> ProjectGeneratorBuilder {
        ProjectGeneratorBuilder::default()
    }

    fn units(&self) -> Vec<Box<dyn TemplateUnit>> {
        vec![
            Box::new(JinjaTemplateUnit {
                template_name: "README.md".into(),
                context: context! {
                    name => &self.context.metadata.name,
                    description => &self.context.metadata.description,
                }
            }),
            Box::new(JinjaTemplateUnit {
                template_name: "Cargo.toml".into(),
                context: context! {
                    name => &self.context.metadata.name,
                    version => &self.context.metadata.version,
                    description => &self.context.metadata.description,
                    authors => &self.context.metadata.authors,
                    license => &self.context.metadata.license,
                }
            }),
            Box::new(JinjaTemplateUnit {
                template_name: ".cargo/config.toml".into(),
                context: context! {}
            }),
            Box::new(JinjaTemplateUnit {
                template_name: "rust-toolchain.toml".into(),
                context: context! {}
            }),
            Box::new(CodegenUnit {
                destination: "src/lib.rs".into(),
                generator: LibRsGenerator::new(self.context.clone()),
            })
        ]
    }

    pub fn render(&self) -> AppResult<Vec<RenderedFile>> {
        Ok(
            self.units()
                .iter()
                .map(|unit| unit.render())
                .collect::<TemplateResult<Vec<Vec<RenderedFile>>>>()?
                .into_iter()
                .flatten()
                .collect()
        )
    }

    pub async fn generate(&self, output_dir: &Path) -> AppResult<()> {
        for file in self.render()? {
            let output_path = output_dir.join(&file.path);

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            fs::write(&output_path, file.content).await?;
        }

        Ok(())
    }
}


#[derive(Default)]
pub struct ProjectGeneratorBuilder {
    venv_dir: Option<PathBuf>,
    site_packages_dir: Option<PathBuf>,
    project_dir: Option<PathBuf>,
    module_root: Option<PathBuf>,
    import_root: Option<PathBuf>,
    module_name: Option<String>,
    metadata: Option<ProjectMetadata>,
    modules: Vec<Module>,
}

impl ProjectGeneratorBuilder {
    pub fn context(mut self, context: ProjectContext) -> Self {
        self.venv_dir = Some(context.venv_dir);
        self.site_packages_dir = Some(context.site_packages_dir);
        self.project_dir = Some(context.project_dir);
        self.module_root = Some(context.module_root);
        // self.import_root = Some(context.import_root);
        self.module_name = Some(context.module_name);
        self.metadata = Some(context.metadata);
        self.modules = context.modules;
        self
    }

    pub fn venv_dir(mut self, venv_dir: impl Into<PathBuf>) -> Self {
        self.venv_dir = Some(venv_dir.into());
        self
    }

    pub fn site_packages_dir(mut self, site_packages_dir: impl Into<PathBuf>) -> Self {
        self.site_packages_dir = Some(site_packages_dir.into());
        self
    }

    pub fn project_dir(mut self, project_dir: impl Into<PathBuf>) -> Self {
        self.project_dir = Some(project_dir.into());
        self
    }

    pub fn import_root(mut self, import_root: impl Into<PathBuf>) -> Self {
        self.import_root = Some(import_root.into());
        self
    }

    pub fn module_root(mut self, module_root: impl Into<PathBuf>) -> Self {
        self.module_root = Some(module_root.into());
        self
    }

    pub fn module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = Some(module_name.into());
        self
    }

    pub fn metadata(mut self, metadata: ProjectMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn module(mut self, module: Module) -> Self {
        self.modules.push(module);
        self
    }

    pub fn modules<I>(mut self, modules: I) -> Self
    where
        I: IntoIterator<Item = Module>,
    {
        self.modules.extend(modules);
        self
    }

    pub fn build(self) -> ProjectGenerator {
        ProjectGenerator::new(
            ProjectContext {
                venv_dir: self.venv_dir.expect("Virtual environment directory is required"),
                site_packages_dir: self.site_packages_dir.expect("Site-packages directory is required"),
                project_dir: self.project_dir.expect("Project directory is required"),
                // import_root: self.import_root.expect("Import root directory is required"),
                module_root: self.module_root.expect("Module root directory is required"),
                module_name: self.module_name.expect("Module name is required"),
                metadata: self.metadata.expect("Metadata is required"),
                modules: self.modules
            }
        )
    }
}
