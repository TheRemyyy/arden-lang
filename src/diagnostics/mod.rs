use crate::borrowck;
use crate::parser;
use crate::typeck;
use colored::Colorize;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct SourceDiagnostic<'a> {
    pub(crate) header: String,
    pub(crate) filename: &'a str,
    pub(crate) span: Range<usize>,
    pub(crate) help: Option<String>,
    pub(crate) note: Option<String>,
}

pub(crate) fn render_source_diagnostic(source: &str, diagnostic: &SourceDiagnostic<'_>) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let (line_num, col) = span_to_location(&diagnostic.span, source);
    let gutter_width = line_num.to_string().len().max(1);
    let empty_gutter = " ".repeat(gutter_width);

    let mut output = String::new();
    output.push_str(&diagnostic.header);
    output.push('\n');
    output.push_str(&format!(
        "  {} {}:{}:{}\n",
        "-->".blue().bold(),
        diagnostic.filename,
        line_num,
        col
    ));
    output.push_str(&format!("  {} {}\n", empty_gutter, "|".blue().bold()));

    if line_num <= lines.len() {
        let source_line = lines[line_num - 1];
        output.push_str(&format!(
            "  {} {}\n",
            format!("{line_num:>width$} |", width = gutter_width)
                .blue()
                .bold(),
            source_line
        ));

        let underline_start = visual_column_offset(source_line, col);
        let underline_len = diagnostic
            .span
            .end
            .saturating_sub(diagnostic.span.start)
            .max(1);
        let available = source_line.chars().count().saturating_sub(underline_start);
        let carets = "^".repeat(underline_len.min(available).max(1));
        output.push_str(&format!(
            "  {} {} {}{}\n",
            empty_gutter,
            "|".blue().bold(),
            " ".repeat(underline_start),
            carets.red().bold()
        ));
    }

    if let Some(help) = &diagnostic.help {
        output.push_str(&format!(
            "  {} {}: {}\n",
            "=".blue().bold(),
            "help".blue().bold(),
            help
        ));
    }

    if let Some(note) = &diagnostic.note {
        output.push_str(&format!(
            "  {} {}: {}\n",
            "=".blue().bold(),
            "note".blue().bold(),
            note
        ));
    }

    output
}

pub(crate) fn format_parse_error(
    error: &parser::ParseError,
    source: &str,
    filename: &str,
) -> String {
    render_source_diagnostic(
        source,
        &SourceDiagnostic {
            header: format!("{}: {}", "error".red().bold(), error.message),
            filename,
            span: error.span.clone(),
            help: None,
            note: Some(format!("while parsing {}", filename)),
        },
    )
}

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
        let filename = path.to_string_lossy();
        return typeck::format_errors(&errors, source, &filename);
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
        let filename = path.to_string_lossy();
        return borrowck::format_borrow_errors(&errors, source, &filename);
    }

    render_component_summary(
        "semantic borrow-check errors across component",
        errors.into_iter().map(|error| error.message),
        sources,
    )
}

pub(crate) fn span_to_location(span: &Range<usize>, source: &str) -> (usize, usize) {
    let mut line_num: usize = 1;
    let mut col: usize = 1;

    for (i, ch) in source.char_indices() {
        if i >= span.start {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line_num, col)
}

fn visual_column_offset(line: &str, col: usize) -> usize {
    line.chars().take(col.saturating_sub(1)).count()
}
