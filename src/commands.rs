use std::path::PathBuf;
use tempfile::tempdir;

use crate::{
    error::{AppError, AppResult},
    parser::ProjectParser,
    generator::ProjectGenerator,
    compiler::{Compiler, cargo::CargoCompiler},
    ui::{Printer, Spinner, Style, Syntax, LogPanel},
};

#[derive(Debug, Clone)]
pub struct TranspileOptions {
    pub project_dir: String,
    pub out_dir: Option<String>,
    pub stdout: bool,
}

pub async fn transpile_project(options: TranspileOptions) -> AppResult<()> {
    if !options.out_dir.is_some() {
        Printer::warning("No output directory specified; defaulting to stdout.");
    }

    if options.out_dir.is_none() || options.stdout {
        let files = Spinner::step(
            Style::header("transpiling project"),
            None::<&str>,
            || async {
                ProjectGenerator::builder()
                    .context(
                        ProjectParser::builder()
                            .build()
                            .parse_project(&PathBuf::from(options.project_dir))
                            .await?,
                    )
                    .build()
                    .render()
            }
        )
        .await?;

        for file in &files {
            println!(
                "\n\n{}",
                Style::key(&file.path.display().to_string()),
            );
            println!("{}", "─".repeat(80));
            println!("{}", Syntax::code(&file.content, &file.path));
            println!("{}{}", "─".repeat(80), "\n");
        }
    } else if options.out_dir.is_some() {
        Spinner::step(
            Style::header("transpiling project"),
            None::<&str>,
            || async {
                ProjectGenerator::builder()
                    .context(
                        ProjectParser::builder()
                            .build()
                            .parse_project(&PathBuf::from(options.project_dir))
                            .await?,
                    )
                    .build()
                    .generate(&PathBuf::from(options.out_dir.unwrap()))
                    .await
            }
        )
        .await?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub project_dir: String,
    pub out_dir: Option<String>,
    pub release: bool,
}

pub async fn build_project(options: BuildOptions) -> AppResult<()> {
    let project_dir = PathBuf::from(&options.project_dir);
    let out_path = PathBuf::from(options.out_dir.unwrap_or(project_dir.join("artifacts").to_string_lossy().to_string()));

    if !CargoCompiler::is_installed().await {
        Printer::error("Cargo is not installed or not found in PATH.");
        Printer::info("Please install Rust and Cargo from https://www.rust-lang.org/tools/install");
        return Err(AppError::UnknownError(anyhow::anyhow!(
            "Cargo is not installed or not found in PATH."
        )));
    }

    if !CargoCompiler::is_target_available().await? {
        Printer::error("The target 'wasm32-wasip1' is not installed.");
        Printer::info("Please install the target by running: rustup target add wasm32-wasip1");
        return Err(AppError::UnknownError(anyhow::anyhow!(
            "The target 'wasm32-wasip1' is not installed."
        )));
    }

    {
        let temp_dir = tempdir()?;

        Spinner::step(
            Style::header("transpiling module"),
            Some(Printer::render_success("transpiled module")),
            || async {
                ProjectGenerator::builder()
                    .context(
                        ProjectParser::builder()
                            .build()
                            .parse_project(&project_dir.clone())
                            .await?,
                    )
                    .build()
                    .generate(temp_dir.path())
                    .await
            }
        )
        .await?;

        LogPanel::step(
            Style::header("compiling module"),
            10,
            Some(Printer::render_success("compiled module")),
            Some(Printer::render_error("compilation failed")),
            |panel| async {
                CargoCompiler::builder()
                    .release(options.release)
                    .target_dir(out_path.clone())
                    .output_sink_arc(panel)
                    .build()
                    .compile(temp_dir.path())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        )
        .await
        .map_err(|e| AppError::UnknownError(anyhow::anyhow!(e)))?;

        temp_dir.close()?;
    }

    Ok(())
}