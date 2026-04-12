use crate::ast::Span;
use crate::diagnostics::{render_source_diagnostic, SourceDiagnostic};
use colored::Colorize;

/// Error when using function without importing it.
#[derive(Debug, Clone)]
pub struct ImportError {
    pub function_name: String,
    pub defined_in: String,
    pub used_in: String,
    pub span: Span,
    pub suggestion: Option<String>,
}

impl ImportError {
    fn source_summary(&self) -> String {
        match self.defined_in.as_str() {
            "<unknown namespace alias>" => {
                format!("Unknown namespace alias usage '{}'", self.function_name)
            }
            "<unresolved import alias>" => {
                format!("Imported alias '{}' no longer resolves", self.function_name)
            }
            "<unresolved namespace alias member>" => {
                let (alias, member) = self
                    .function_name
                    .split_once('.')
                    .unwrap_or((self.function_name.as_str(), ""));
                format!("Imported namespace alias '{alias}' has no member '{member}'")
            }
            "<unresolved wildcard import>" => {
                format!(
                    "Wildcard import '{}.*' no longer provides '{}'",
                    self.suggestion.as_deref().unwrap_or(""),
                    self.function_name
                )
            }
            _ => format!(
                "Function '{}' is defined in '{}' but not imported here",
                self.function_name, self.defined_in
            ),
        }
    }

    fn import_hint(&self) -> String {
        if self.function_name.contains("__") {
            format!("import {}.*;", self.defined_in)
        } else {
            format!("import {}.{};", self.defined_in, self.function_name)
        }
    }

    fn source_help_message(&self) -> String {
        match self.defined_in.as_str() {
            "<unknown namespace alias>" => {
                "Import an existing namespace with 'import <namespace> as <alias>;'".to_string()
            }
            "<unresolved import alias>" => {
                format!(
                    "Update or remove the stale import for '{}'",
                    self.function_name
                )
            }
            "<unresolved namespace alias member>" => {
                "Update the import target or the member access".to_string()
            }
            "<unresolved wildcard import>" => {
                format!(
                    "Update the wildcard import target or import '{}' explicitly",
                    self.function_name
                )
            }
            _ => format!("Add '{}' to the top of your file", self.import_hint()),
        }
    }

    fn source_note_message(&self) -> String {
        let mut note = format!("current namespace: {}", self.used_in);
        if let Some(suggestion) = &self.suggestion {
            note.push_str(&format!("; did you mean '{}'?", suggestion));
        }
        note
    }

    pub fn format(&self) -> String {
        if self.defined_in == "<unknown namespace alias>" {
            return format!(
                "Unknown namespace alias usage '{}' in '{}'\n  \
                 Hint: Import an existing namespace with 'import <namespace> as <alias>;'",
                self.function_name, self.used_in
            );
        }

        if self.defined_in == "<unresolved import alias>" {
            return format!(
                "Imported alias '{}' no longer resolves in '{}'\n  \
                 Hint: Update or remove the stale import for '{}'",
                self.function_name, self.used_in, self.function_name
            );
        }

        if self.defined_in == "<unresolved namespace alias member>" {
            let (alias, member) = self
                .function_name
                .split_once('.')
                .unwrap_or((self.function_name.as_str(), ""));
            return format!(
                "Imported namespace alias '{}' has no member '{}' in '{}'\n  \
                 Hint: Update the import target or the member access",
                alias, member, self.used_in
            );
        }

        if self.defined_in == "<unresolved wildcard import>" {
            return format!(
                "Wildcard import '{}.*' no longer provides '{}' in '{}'\n  \
                 Hint: Update the wildcard import target or the referenced symbol",
                self.suggestion.as_deref().unwrap_or(""),
                self.function_name,
                self.used_in
            );
        }

        let mut result = format!(
            "Function '{}' is defined in '{}' but not imported in '{}'\n  \
             Hint: Add '{}' to the top of your file",
            self.function_name,
            self.defined_in,
            self.used_in,
            self.import_hint()
        );

        if let Some(suggestion) = &self.suggestion {
            result.push_str(&format!("\n  Or did you mean: '{}'?", suggestion));
        }

        result
    }

    pub fn format_with_source(&self, source: &str, filename: &str) -> String {
        render_source_diagnostic(
            source,
            &SourceDiagnostic {
                header: format!("{}: {}", "error".red().bold(), self.source_summary()),
                filename,
                span: self.span.clone(),
                help: Some(self.source_help_message()),
                note: Some(self.source_note_message()),
            },
        )
    }
}
