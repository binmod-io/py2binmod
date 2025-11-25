use anyhow::{anyhow, Error};
use ruff_python_ast::{self as ast};

use crate::types::{
    ModuleFunction,
    HostFunction,
    Parameter,
    ParameterType,
};

pub trait TryFromAst: Sized {
    type Expr;
    type Error;

    fn try_from_ast(expr: &Self::Expr) -> Result<Self, Self::Error>;
}

impl TryFromAst for ModuleFunction {
    type Expr = ast::StmtFunctionDef;
    type Error = Error;

    fn try_from_ast(expr: &Self::Expr) -> Result<Self, Self::Error> {
        let docstring = if let Some(ast::Stmt::Expr(expr)) = expr.body.first() {
            if let ast::Expr::StringLiteral(s) = &*expr.value {
                Some(s.value.to_str().to_string())
            } else {
                None
            }
        } else {
            None
        };

        Ok(ModuleFunction {
            name: expr.name.to_string(),
            docstring,
            parameters: expr.parameters
                .iter()
                .map(|arg| Parameter::try_from_ast(arg.as_parameter()))
                .collect::<Result<Vec<Parameter>, Error>>()?,
            return_type: ParameterType::try_from_ast(
                expr.returns
                    .as_deref()
                    .ok_or_else(|| anyhow!("Missing return type annotation for function {}", expr.name))?,
            )?,
        })
    }
}

impl TryFromAst for HostFunction {
    type Expr = ast::StmtFunctionDef;
    type Error = Error;

    fn try_from_ast(expr: &Self::Expr) -> Result<Self, Self::Error> {
        Ok(HostFunction {
            name: expr.name.to_string(),
            parameters: expr.parameters
                .iter()
                .map(|arg| Parameter::try_from_ast(arg.as_parameter()))
                .collect::<Result<Vec<Parameter>, Error>>()?,
            return_type: ParameterType::try_from_ast(
                expr.returns
                    .as_deref()
                    .ok_or_else(|| anyhow!("Missing return type annotation for host function {}", expr.name))?,
            )?,
        })
    }
}

impl TryFromAst for Parameter {
    type Expr = ast::Parameter;
    type Error = Error;

    fn try_from_ast(expr: &Self::Expr) -> Result<Self, Self::Error> {
        Ok(Parameter { 
            name: expr.name().to_string(),
            type_hint: ParameterType::try_from_ast(
                expr
                    .annotation()
                    .as_deref()
                    .ok_or_else(|| anyhow!("Missing type annotation for parameter {}", expr.name()))?,
            )?
        })
    }
}

impl TryFromAst for ParameterType {
    type Expr = ast::Expr;
    type Error = Error;

    fn try_from_ast(expr: &Self::Expr) -> Result<Self, Self::Error> {
        fn normalize_ident(name: &str) -> &str {
            match name {
                "int" | "builtins.int" => "int",
                "float" | "builtins.float" => "float",
                "str" | "builtins.str" => "str",
                "bool" | "builtins.bool" => "bool",
                "None" | "NoneType" => "None",
                other => other,
            }
        }

        fn parse_name(expr: &ast::Expr) -> Option<String> {
            match expr {
                ast::Expr::Name(n) => Some(n.id.to_string()),
                ast::Expr::Attribute(_) => {
                    let mut parts = Vec::new();
                    let mut current = expr;

                    while let ast::Expr::Attribute(attr) = current {
                        parts.push(attr.attr.to_string());
                        current = &*attr.value;
                    }

                    if let ast::Expr::Name(n) = current {
                        parts.push(n.id.to_string());
                        parts.reverse();
                        Some(parts.join("."))
                    } else {
                        None
                    }
                },
                _ => None,
            }
        }

        fn parse_subscript(expr: &ast::ExprSubscript) -> Result<(String, Vec<&ast::Expr>), Error> {
            Ok((
                parse_name(&expr.value)
                    .ok_or_else(|| anyhow!("Unsupported subscript base expression"))?,
                match &*expr.slice {
                    ast::Expr::Tuple(t) => t.elts.iter().collect(),
                    other => vec![other],
                }
            ))
        }

        fn parse_union(expr: &ast::ExprBinOp) -> Result<ParameterType, Error> {
            let left = ParameterType::try_from_ast(&*expr.left)?;
            let right = ParameterType::try_from_ast(&*expr.right)?;

            if right == ParameterType::None {
                return Ok(ParameterType::Optional(Box::new(left)));
            }
            if left == ParameterType::None {
                return Ok(ParameterType::Optional(Box::new(right)));
            }

            Err(anyhow!("Only Optional unions supported (T | None)"))
        }

        match expr {
            // Primitive literals
            ast::Expr::Name(n) => match normalize_ident(n.id.as_str()) {
                "int" => Ok(ParameterType::Integer),
                "float" => Ok(ParameterType::Float),
                "str" => Ok(ParameterType::String),
                "bool" => Ok(ParameterType::Boolean),
                "None" => Ok(ParameterType::None),
                _ => Ok(ParameterType::Any),
            },

            // Optional and Union types
            ast::Expr::BinOp(binop) => {
                if matches!(binop.op, ast::Operator::BitOr) {
                    return parse_union(binop);
                }

                Err(anyhow!("Unsupported binary operation in type annotation"))
            },

            // Subscripted types: list[T], dict[K, V], tuple[T1, T2, ...]
            ast::Expr::Subscript(sub) => {
                let (base, args) = parse_subscript(sub)?;
                let base_normalized = base
                    .replace("typing.", "")
                    .replace("collections.abc.", "");

                match base_normalized.as_str() {
                    // list[T]
                    "list" | "List" => Ok(ParameterType::List(
                        Box::new(ParameterType::try_from_ast(
                            args.first()
                                .ok_or_else(|| anyhow!("Missing type argument for List"))?,
                        )?)
                    )),

                    // dict[K, V]
                    "dict" | "Dict" | "Mapping" => {
                        if args.len() != 2 {
                            return Err(anyhow!("Dict type annotation requires two type arguments"));
                        }

                        Ok(ParameterType::Map {
                            key_type: Box::new(ParameterType::try_from_ast(args[0])?),
                            value_type: Box::new(ParameterType::try_from_ast(args[1])?),
                        })
                    },

                    // tuple[T1, T2, ...]
                    "tuple" | "Tuple" => Ok(ParameterType::Tuple(
                        args
                            .iter()
                            .map(|a| ParameterType::try_from_ast(a))
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter()
                            .map(|t| Box::new(t))
                            .collect(),
                    )),

                    "Optional" => Ok(ParameterType::Optional(
                        Box::new(ParameterType::try_from_ast(
                            args.first()
                                .ok_or_else(|| anyhow!("Missing type argument for Optional"))?,
                        )?)
                    )),

                    _ => Ok(ParameterType::Any)
                }
            },

            // None literal
            ast::Expr::NoneLiteral(_) => Ok(ParameterType::None),

            _ => Err(anyhow!("Unsupported type annotation expression: {:?}", expr)),
        }
    }
}
