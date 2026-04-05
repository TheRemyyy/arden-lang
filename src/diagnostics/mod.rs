use crate::borrowck;
use crate::typeck;
use colored::Colorize;
use std::path::PathBuf;

fn render_component_summary(
    header: &str,
    errors: impl IntoIterator<Item = String>,
    sources: &[(PathBuf, String)],
) -> String {
    let mut rendered = String::new();
    rendered.push_str(header);
    if !sources.is_empty() {
        rendered.push('\n');
        rendered.push_str("files:\n");
        for (path, _) in sources {
            rendered.push_str(&format!("  - {}\n", path.display()));
        }
    }
    for error in errors {
        rendered.push_str(&format!("{}: {}\n", "error".red().bold(), error));
    }
    rendered
}

pub(crate) fn render_type_errors(
    errors: Vec<typeck::TypeError>,
    sources: &[(PathBuf, String)],
) -> String {
    if sources.len() == 1 {
        let (path, source) = &sources[0];
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        return typeck::format_errors(&errors, source, filename);
    }

    render_component_summary(
        "semantic type-check errors across component",
        errors.into_iter().map(|error| error.message),
        sources,
    )
}

pub(crate) fn render_borrow_errors(
    errors: Vec<borrowck::BorrowError>,
    sources: &[(PathBuf, String)],
) -> String {
    if sources.len() == 1 {
        let (path, source) = &sources[0];
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        return borrowck::format_borrow_errors(&errors, source, filename);
    }

    render_component_summary(
        "semantic borrow-check errors across component",
        errors.into_iter().map(|error| error.message),
        sources,
    )
}
