use pyo3::{
    prelude::*,
    exceptions::{PyValueError, PyTypeError, PyRuntimeError},
};

use crate::{
    commands::{
        BuildOptions, TranspileOptions,
        build_project, transpile_project,
    }, error::AppError,
};


impl From<AppError> for PyErr {
    fn from(err: AppError) -> PyErr {
        match err {
            AppError::CompilationError(e) => PyRuntimeError::new_err(e.to_string()),
            AppError::ParserError(e) => PyValueError::new_err(e.to_string()),
            AppError::TemplateError(e) => PyTypeError::new_err(e.to_string()),
            AppError::GeneratorError(msg) => PyRuntimeError::new_err(msg),
            AppError::IoError(e) => PyRuntimeError::new_err(e.to_string()),
            AppError::UnknownError(e) => PyRuntimeError::new_err(e.to_string()),
        }
    }
}


#[pyfunction(name = "transpile_command")]
#[pyo3(signature = (project_dir, out_dir=None, stdout=false))]
fn py_transpile_command(py: Python<'_>, project_dir: String, out_dir: Option<String>, stdout: bool) -> PyResult<Bound<'_, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        transpile_project(TranspileOptions {
            project_dir: project_dir,
            out_dir: out_dir,
            stdout: stdout,
        })
        .await?;

        Ok(())
    })
}

#[pyfunction(name = "build_command")]
#[pyo3(signature = (project_dir, out_dir=None, release=false))]
fn py_build_command(py: Python<'_>, project_dir: String, out_dir: Option<String>, release: bool) -> PyResult<Bound<'_, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        build_project(BuildOptions {
            project_dir: project_dir,
            out_dir: out_dir,
            release: release,
        })
        .await?;

        Ok(())
    })
}

/// Python bindings for py2binmod
#[pymodule]
#[pyo3(name = "_py2binmod")]
fn py_py2binmod_module(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_transpile_command, py)?)?;
    m.add_function(wrap_pyfunction!(py_build_command, py)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
