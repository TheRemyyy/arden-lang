use crate::diagnostics::{render_source_diagnostic, SourceDiagnostic};

#[test]
fn keeps_diagnostic_gutter_aligned_for_multi_digit_lines() {
    let source = (1..=12)
        .map(|line| format!("line {line};"))
        .collect::<Vec<_>>()
        .join("\n");
    let diagnostic = SourceDiagnostic {
        header: "error: sample".to_string(),
        filename: "sample.arden",
        span: 78..79,
        help: None,
        note: None,
    };

    let rendered = render_source_diagnostic(&source, &diagnostic);
    let pipe_lines = rendered
        .lines()
        .filter(|line| line.contains('|'))
        .collect::<Vec<_>>();

    assert!(pipe_lines.len() >= 3, "{rendered}");

    let column_positions = pipe_lines
        .iter()
        .map(|line| line.find('|').unwrap_or_default())
        .collect::<Vec<_>>();

    assert!(
        column_positions.windows(2).all(|pair| pair[0] == pair[1]),
        "{rendered}"
    );
}

#[test]
fn diagnostic_underline_width_tracks_unicode_scalar_count() {
    let source = "🙂🙂x\n";
    let second_emoji_start = "🙂".len();
    let second_emoji_end = second_emoji_start + "🙂".len();
    let diagnostic = SourceDiagnostic {
        header: "error: unicode width".to_string(),
        filename: "unicode.arden",
        span: second_emoji_start..second_emoji_end,
        help: None,
        note: None,
    };

    let rendered = render_source_diagnostic(source, &diagnostic);
    let caret_count = rendered.chars().filter(|ch| *ch == '^').count();
    assert_eq!(caret_count, 1, "{rendered}");
}

#[test]
fn diagnostic_renders_context_for_eof_after_trailing_newline() {
    let source = "value\n";
    let eof = source.len();
    let diagnostic = SourceDiagnostic {
        header: "error: eof".to_string(),
        filename: "eof.arden",
        span: eof..eof,
        help: None,
        note: None,
    };

    let rendered = render_source_diagnostic(source, &diagnostic);
    assert!(rendered.contains("eof.arden:2:1"), "{rendered}");
    assert!(rendered.contains("2 |"), "{rendered}");
    assert!(rendered.contains("^"), "{rendered}");
}

#[test]
fn diagnostic_output_normalizes_crlf_lines() {
    let source = "value\r\n";
    let diagnostic = SourceDiagnostic {
        header: "error: crlf".to_string(),
        filename: "crlf.arden",
        span: 0..5,
        help: None,
        note: None,
    };

    let rendered = render_source_diagnostic(source, &diagnostic);
    assert!(!rendered.contains('\r'), "{rendered:?}");
    assert!(rendered.contains("1 | value"), "{rendered}");
}

#[test]
fn diagnostic_location_and_context_handle_lone_cr_line_endings() {
    let source = "value\rother";
    let start = source.find("other").expect("other token");
    let diagnostic = SourceDiagnostic {
        header: "error: cr".to_string(),
        filename: "cr.arden",
        span: start..(start + "other".len()),
        help: None,
        note: None,
    };

    let rendered = render_source_diagnostic(source, &diagnostic);
    assert!(rendered.contains("cr.arden:2:1"), "{rendered}");
    assert!(rendered.contains("2 | other"), "{rendered}");
}
