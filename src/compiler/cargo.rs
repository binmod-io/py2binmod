use std::{path::{Path, PathBuf}, process::Stdio, sync::Arc, env::current_dir};
use async_trait::async_trait;
use tokio::{process::Command, io::{AsyncBufReadExt, BufReader}};

use crate::compiler::{
    error::{CompilerError, CompilerResult},
    traits::{Compiler, OutputSink, NullOutputSink},
    types::Artifact,
    utils::command_exists,
};


pub struct CargoCompiler {
    pub release: bool,
    pub target_dir: Option<PathBuf>,
    pub sink: Arc<dyn OutputSink + Send + Sync>,
}

impl CargoCompiler {
    pub fn new(release: bool, target_dir: Option<PathBuf>, sink: Arc<dyn OutputSink + Send + Sync>) -> Self {
        Self { release, target_dir, sink }
    }

    pub fn builder() -> CargoCompilerBuilder {
        CargoCompilerBuilder::builder()
    }

    pub async fn is_installed() -> bool {
        command_exists("cargo").await
    }

    pub async fn is_target_available() -> CompilerResult<bool> {
        Ok(
            String::from_utf8(
                Command::new("rustup")
                .arg("target")
                .arg("list")
                .arg("--installed")
                .output()
                .await?
                .stdout
            )
            .map_err(|_| CompilerError::CompilationFailed("Failed to read rustup output".into()))?
            .lines()
            .any(|line| line == "wasm32-wasip1")
        )
    }
}

#[async_trait]
impl Compiler for CargoCompiler {
    async fn compile(&self, project_dir: &Path) -> CompilerResult<Artifact> {
        Self::is_target_available().await?;

        let mut child = Command::new("cargo")
            .current_dir(project_dir)
            .arg("build")
            .args(self.release.then_some(vec!["--release"]).unwrap_or_default())
            .args(
                self.target_dir
                    .as_ref()
                    .map(|dir| vec!["--target-dir", dir.to_str().unwrap()])
                    .unwrap_or(
                        vec![
                            "--target-dir",
                            current_dir()
                                .map_err(|e| CompilerError::CompilationFailed(e.to_string()))?
                                .join("artifacts")
                                .to_str()
                                .ok_or(CompilerError::CompilationFailed(
                                    "Failed to convert target dir to string".into(),
                                ))?,
                        ]
                    )
            )
            .arg("--message-format=short")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout.next_line() => self.sink.stdout(&line).await,
                Ok(Some(line)) = stderr.next_line() => self.sink.stderr(&line).await,
                else => break,
            }
        }

        child.wait()
            .await
            .map_err(|e| CompilerError::CompilationFailed(e.to_string()))
            .and_then(|status| status
                .success()
                .then_some(())
                .ok_or_else(|| CompilerError::CompilationFailed(format!(
                    "cargo exited with status code {}",
                    status.code().unwrap_or(-1)
                )))
            )?;

        Ok(Artifact {
            target_dir: self.target_dir
                .clone()
                .unwrap_or_else(|| project_dir.join("artifacts")),
        })
    }
}

pub struct CargoCompilerBuilder {
    release: bool,
    target_dir: Option<PathBuf>,
    sink: Option<Arc<dyn OutputSink + Send + Sync>>,
}

impl CargoCompilerBuilder {
    pub fn builder() -> Self {
        Self {
            release: false,
            target_dir: None,
            sink: None,
        }
    }

    pub fn release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }

    pub fn target_dir<P: AsRef<Path>>(mut self, target_dir: P) -> Self {
        self.target_dir = Some(target_dir.as_ref().to_path_buf());
        self
    }

    pub fn output_sink<T: OutputSink + Send + Sync + 'static>(mut self, sink: T) -> Self {
        self.sink = Some(Arc::new(sink));
        self
    }

    pub fn output_sink_arc<T: OutputSink + Send + Sync + 'static>(mut self, sink: Arc<T>) -> Self {
        self.sink = Some(sink);
        self
    }

    pub fn build(self) -> CargoCompiler {
        CargoCompiler::new(self.release, self.target_dir, self.sink.unwrap_or(Arc::new(NullOutputSink)))
    }
}