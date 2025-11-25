use std::{path::{Path, PathBuf}, ops::{Deref, DerefMut}, vec::IntoIter};
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectContext {
    pub venv_dir: PathBuf,
    pub site_packages_dir: PathBuf,
    pub project_dir: PathBuf,
    pub module_root: PathBuf,
    pub module_name: String,
    pub metadata: ProjectMetadata,
    pub modules: Vec<Module>,
}


#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub requires_python: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub py2binmod: Option<Py2BinmodConfig>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Py2BinmodConfig {
    pub venv: Option<PathBuf>,
    pub module_root: Option<PathBuf>,
    pub module: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Module {
    pub name: String,
    pub file_path: PathBuf,
    pub module_functions: ModuleFunctions,
    pub host_functions: Option<HostFunctions>,
}

impl Module {
    pub fn import_path(&self, module_root: &Path) -> Option<String> {
        let relative_path = self
            .file_path
            .strip_prefix(module_root)
            .unwrap();

        let mut components = relative_path
            .components()
            .map(|c| c.as_os_str().to_str().unwrap().to_string())
            .collect::<Vec<String>>();

        if let Some(last) = components.last_mut() {
            if last.ends_with(".py") {
                *last = last.trim_end_matches(".py").to_string();
            }
        }

        if components.last().map_or(false, |s| s == "__init__") {
            components.pop();
        }

        if components.is_empty() {
            return None;
        }

        Some(components.join("."))
    }
}


#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ModuleFunction {
    pub name: String,
    pub docstring: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: ParameterType,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ModuleFunctions(pub Vec<ModuleFunction>);

impl ModuleFunctions {
    pub fn new(functions: Vec<ModuleFunction>) -> Self {
        Self(functions)
    }

    pub fn as_slice(&self) -> &[ModuleFunction] {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut [ModuleFunction] {
        &mut self.0
    }

    pub fn as_vec(&self) -> &Vec<ModuleFunction> {
        &self.0
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<ModuleFunction> {
        &mut self.0
    }
}

impl Deref for ModuleFunctions {
    type Target = Vec<ModuleFunction>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ModuleFunctions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for ModuleFunctions {
    type Item = ModuleFunction;
    type IntoIter = IntoIter<ModuleFunction>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Default for ModuleFunctions {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl From<Vec<ModuleFunction>> for ModuleFunctions {
    fn from(functions: Vec<ModuleFunction>) -> Self {
        Self(functions)
    }
}


#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct HostFunction {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: ParameterType,
}


#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct HostFunctions {
    pub namespace: String,
    pub functions: Vec<HostFunction>,
}

impl HostFunctions {
    pub fn new(namespace: String, functions: Vec<HostFunction>) -> Self {
        Self { namespace, functions }
    }

    pub fn as_slice(&self) -> &[HostFunction] {
        &self.functions
    }

    pub fn as_mut_slice(&mut self) -> &mut [HostFunction] {
        &mut self.functions
    }

    pub fn as_vec(&self) -> &Vec<HostFunction> {
        &self.functions
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<HostFunction> {
        &mut self.functions
    }
}

impl Deref for HostFunctions {
    type Target = Vec<HostFunction>;

    fn deref(&self) -> &Self::Target {
        &self.functions
    }
}

impl DerefMut for HostFunctions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.functions
    }
}

impl IntoIterator for HostFunctions {
    type Item = HostFunction;
    type IntoIter = IntoIter<HostFunction>;

    fn into_iter(self) -> Self::IntoIter {
        self.functions.into_iter()
    }
}

impl Default for HostFunctions {
    fn default() -> Self {
        Self {
            namespace: "env".to_string(),
            functions: Vec::new(),
        }
    }
}


#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Parameter {
    pub name: String,
    pub type_hint: ParameterType,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    List(Box<ParameterType>),
    Tuple(Vec<Box<ParameterType>>),
    Map {
        key_type: Box<ParameterType>,
        value_type: Box<ParameterType>,
    },
    Optional(Box<ParameterType>),
    None,
    Any,
}
