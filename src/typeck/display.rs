use super::*;
use crate::diagnostics::{render_source_diagnostic, SourceDiagnostic};

impl TypeChecker {
    pub(crate) fn format_resolved_type_for_diagnostic(ty: &ResolvedType) -> String {
        match ty {
            ResolvedType::Integer => "Integer".to_string(),
            ResolvedType::Float => "Float".to_string(),
            ResolvedType::Boolean => "Boolean".to_string(),
            ResolvedType::String => "String".to_string(),
            ResolvedType::Char => "Char".to_string(),
            ResolvedType::None => "None".to_string(),
            ResolvedType::Class(name) => format_diagnostic_class_name(name),
            ResolvedType::Option(inner) => {
                format!(
                    "Option<{}>",
                    Self::format_resolved_type_for_diagnostic(inner)
                )
            }
            ResolvedType::Result(ok, err) => format!(
                "Result<{}, {}>",
                Self::format_resolved_type_for_diagnostic(ok),
                Self::format_resolved_type_for_diagnostic(err)
            ),
            ResolvedType::List(inner) => {
                format!("List<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Map(key, value) => format!(
                "Map<{}, {}>",
                Self::format_resolved_type_for_diagnostic(key),
                Self::format_resolved_type_for_diagnostic(value)
            ),
            ResolvedType::Set(inner) => {
                format!("Set<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Ref(inner) => {
                format!("&{}", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::MutRef(inner) => {
                format!("&mut {}", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Box(inner) => {
                format!("Box<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Rc(inner) => {
                format!("Rc<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Arc(inner) => {
                format!("Arc<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Ptr(inner) => {
                format!("Ptr<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Task(inner) => {
                format!("Task<{}>", Self::format_resolved_type_for_diagnostic(inner))
            }
            ResolvedType::Range(inner) => {
                format!(
                    "Range<{}>",
                    Self::format_resolved_type_for_diagnostic(inner)
                )
            }
            ResolvedType::Function(params, ret) => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(Self::format_resolved_type_for_diagnostic)
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::format_resolved_type_for_diagnostic(ret)
            ),
            ResolvedType::TypeVar(id) => format!("?T{}", id),
            ResolvedType::Unknown => "unknown".to_string(),
        }
    }

    pub(crate) fn format_ast_type_source(ty: &Type) -> String {
        match ty {
            Type::Integer => "Integer".to_string(),
            Type::Float => "Float".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => name.clone(),
            Type::Generic(name, args) => format!(
                "{}<{}>",
                name,
                args.iter()
                    .map(Self::format_ast_type_source)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Function(params, ret) => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(Self::format_ast_type_source)
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::format_ast_type_source(ret)
            ),
            Type::Option(inner) => format!("Option<{}>", Self::format_ast_type_source(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                Self::format_ast_type_source(ok),
                Self::format_ast_type_source(err)
            ),
            Type::List(inner) => format!("List<{}>", Self::format_ast_type_source(inner)),
            Type::Map(k, v) => format!(
                "Map<{}, {}>",
                Self::format_ast_type_source(k),
                Self::format_ast_type_source(v)
            ),
            Type::Set(inner) => format!("Set<{}>", Self::format_ast_type_source(inner)),
            Type::Ref(inner) => format!("&{}", Self::format_ast_type_source(inner)),
            Type::MutRef(inner) => format!("&mut {}", Self::format_ast_type_source(inner)),
            Type::Box(inner) => format!("Box<{}>", Self::format_ast_type_source(inner)),
            Type::Rc(inner) => format!("Rc<{}>", Self::format_ast_type_source(inner)),
            Type::Arc(inner) => format!("Arc<{}>", Self::format_ast_type_source(inner)),
            Type::Ptr(inner) => format!("Ptr<{}>", Self::format_ast_type_source(inner)),
            Type::Task(inner) => format!("Task<{}>", Self::format_ast_type_source(inner)),
            Type::Range(inner) => format!("Range<{}>", Self::format_ast_type_source(inner)),
        }
    }
}

/// Format type errors with source context
pub(crate) fn format_errors(errors: &[TypeError], source: &str, filename: &str) -> String {
    use colored::Colorize;

    let mut output = String::new();

    for error in errors {
        output.push_str(&render_source_diagnostic(
            source,
            &SourceDiagnostic {
                header: format!("{}: {}", "error".red().bold(), error.message),
                filename,
                span: error.span.clone(),
                help: error.hint.clone(),
                note: None,
            },
        ));
        output.push('\n');
    }

    output
}
