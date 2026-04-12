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
