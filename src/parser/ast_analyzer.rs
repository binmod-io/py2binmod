use anyhow::{Context, anyhow};
use ruff_python_parser::parse_module;
use ruff_python_ast::{self as ast};
use std::path::Path;
use tokio::fs;

use crate::{
    types::{
        ModuleFunction, ModuleFunctions, HostFunction,
        HostFunctions, Module,
    },
    parser::{error::ParserResult, traits::TryFromAst},
};


pub struct AstAnalyzer;

impl AstAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub async fn analyze_file(&self, file_path: &Path) -> ParserResult<Option<Module>> {
        let content = fs::read_to_string(file_path).await?;
        let module_ast = parse_module(&content)
            .map(|m| m.into_suite())
            .context(format!("Failed to parse Python module: {:?}", file_path))?;

        let mut module_functions = Vec::new();
        let mut host_functions = None;

        for stmt in &module_ast {
            match stmt {
                ast::Stmt::FunctionDef(func) => {
                    if self.has_func_decorator(func, "mod_fn") {
                        module_functions.push(
                            ModuleFunction::try_from_ast(func)?
                        );
                    }
                }
                ast::Stmt::ClassDef(class) => {
                    if self.has_class_decorator(class, "host_fns") {
                        if let Some((namespace, host_fns)) = self.parse_host_fns_class(class)? {
                            host_functions = Some((namespace, host_fns))
                        }
                    }
                }
                _ => {}
            }
        }

        if module_functions.is_empty() && host_functions.is_none() {
            return Ok(None);
        }

        Ok(Some(Module {
            name: file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow!("Invalid file name"))?
                .to_string(),
            file_path: file_path.to_path_buf(),
            module_functions: ModuleFunctions::new(module_functions),
            host_functions: host_functions
                .map(|(namespace, fns)| HostFunctions::new(namespace, fns)),
        }))
    }

    fn has_func_decorator(&self, func: &ast::StmtFunctionDef, name: &str) -> bool {
        func
            .decorator_list
            .iter()
            .any(|decorator| {
                self.is_decorator_name(decorator, name)
            })
    }

    fn has_class_decorator(&self, class: &ast::StmtClassDef, name: &str) -> bool {
        class
            .decorator_list
            .iter()
            .any(|decorator| {
                self.is_decorator_name(decorator, name)
            })
    }

    fn is_decorator_name(&self, decorator: &ast::Decorator, name: &str) -> bool {
        match &decorator.expression {
            ast::Expr::Name(n) => n.id.as_str() == name,
            ast::Expr::Attribute(attr) => attr.attr.as_str() == name,
            ast::Expr::Call(call) => {
                match &*call.func {
                    ast::Expr::Name(n) => n.id.as_str() == name,
                    ast::Expr::Attribute(attr) => attr.attr.as_str() == name,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn get_decorator_args<'a>(&self, decorator: &'a ast::Decorator) -> Option<&'a ast::Arguments> {
        match &decorator.expression {
            ast::Expr::Call(call) => Some(&call.arguments),
            _ => None,
        }
    }

    fn parse_host_fns_class(&self, class: &ast::StmtClassDef) -> ParserResult<Option<(String, Vec<HostFunction>)>> {
        let namespace = self
            .get_decorator_args(
                class
                    .decorator_list
                    .iter()
                    .find(|d| self.is_decorator_name(d, "host_fns"))
                    .ok_or_else(|| anyhow!("Decorator not found"))?,
            )
            .and_then(|args| {
                args
                .find_argument_value("namespace", 0)
                .and_then(|expr| {
                    match expr {
                        ast::Expr::StringLiteral(s) => Some(s.value.to_string()),
                        _ => None,
                    }
                })
            })
            .ok_or(anyhow!("Missing 'namespace' argument in host_fns decorator"))?
            .to_string();

        let mut host_functions = Vec::new();

        for stmt in &class.body {
            if let ast::Stmt::FunctionDef(func) = stmt {
                if self.has_func_decorator(func, "host_fn") {
                    host_functions.push(
                        HostFunction::try_from_ast(func)?
                    );
                }
            }
        }

        if host_functions.is_empty() {
            Ok(None)
        } else {
            Ok(Some((namespace, host_functions)))
        }
    }
}