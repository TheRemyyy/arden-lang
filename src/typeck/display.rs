use super::*;

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

    let lines: Vec<&str> = source.lines().collect();
    let mut output = String::new();

    for error in errors {
        // Find line number
        let mut line_num: usize = 1;
        let mut col: usize = 1;
        for (i, ch) in source.char_indices() {
            if i >= error.span.start {
                break;
            }
            if ch == '\n' {
                line_num += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        output.push_str(&format!("{}: {}\n", "error".red().bold(), error.message));
        output.push_str(&format!(
            "  {} {}:{}:{}\n",
            "-->".blue().bold(),
            filename,
            line_num,
            col
        ));
        output.push_str(&format!("   {}\n", "|".blue().bold()));

        if line_num <= lines.len() {
            output.push_str(&format!(
                "{} {}\n",
                format!("{:3} |", line_num).blue().bold(),
                lines[line_num - 1]
            ));

            // Underline
            let underline_start = col.saturating_sub(1);
            let underline_len = error.span.end.saturating_sub(error.span.start).max(1);
            let available = lines[line_num - 1].len().saturating_sub(underline_start);
            let carets = "^".repeat(underline_len.min(available).max(1));
            output.push_str(&format!(
                "   {} {}{}\n",
                "|".blue().bold(),
                " ".repeat(underline_start),
                carets.red().bold()
            ));
        }

        if let Some(hint) = &error.hint {
            output.push_str(&format!(
                "   {} {}: {}\n",
                "=".blue().bold(),
                "help".blue().bold(),
                hint
            ));
        }

        output.push('\n');
    }

    output
}
