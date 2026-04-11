use super::TestExpectExt;
use crate::lsp::{
    find_nth_name_occurrence_in_span, offset_to_position_impl, position_to_offset_impl,
    word_at_position_impl,
};
use tower_lsp::lsp_types::Position;

#[test]
fn lsp_offsets_roundtrip_through_utf16_positions() {
    let text = "🙂value = 1;\n";
    let offset = text.find("value").must("identifier should exist");

    let position = offset_to_position_impl(text, offset);
    assert_eq!(position, Position::new(0, 2));
    assert_eq!(position_to_offset_impl(text, position), offset);
}

#[test]
fn lsp_word_lookup_handles_non_bmp_prefixes() {
    let text = "🙂value = 1;\n";
    let position = Position::new(0, 3);
    assert_eq!(
        word_at_position_impl(text, position).as_deref(),
        Some("value")
    );
}

#[test]
fn lsp_name_lookup_can_reach_parameter_after_function_name() {
    let text = "function value(value: Integer): Integer { return value; }\n";
    let span = 0..text.len();

    let function_name =
        find_nth_name_occurrence_in_span(text, "value", &span, 0).must("function name");
    let parameter_name =
        find_nth_name_occurrence_in_span(text, "value", &span, 1).must("parameter name");

    assert!(function_name.start < parameter_name.start);
    assert_eq!(&text[function_name], "value");
    assert_eq!(&text[parameter_name], "value");
}

#[test]
fn lsp_word_lookup_does_not_cross_whitespace_or_eof() {
    let text = "value\n";
    assert_eq!(word_at_position_impl(text, Position::new(0, 5)), None);
    assert_eq!(word_at_position_impl(text, Position::new(1, 0)), None);
}

#[test]
fn lsp_word_lookup_resolves_identifier_at_end_of_file() {
    let text = "value";
    assert_eq!(
        word_at_position_impl(text, Position::new(0, 5)).as_deref(),
        Some("value")
    );
}

#[test]
fn lsp_name_lookup_respects_identifier_boundaries() {
    let text = "value evaluate value_1 value";
    let span = 0..text.len();

    assert_eq!(
        find_nth_name_occurrence_in_span(text, "value", &span, 0),
        Some(0..5)
    );
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "value", &span, 1),
        Some(23..28)
    );
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "value", &span, 2),
        None
    );
}

#[test]
fn lsp_name_lookup_honors_requested_span_window() {
    let text = "function alpha(): None { alpha(); }\nfunction alpha_beta(): None {}\n";
    let first_line_end = text.find('\n').must("newline");
    let span = 0..first_line_end;

    assert_eq!(
        find_nth_name_occurrence_in_span(text, "alpha", &span, 0),
        Some(9..14)
    );
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "alpha", &span, 1),
        Some(25..30)
    );
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "alpha", &span, 2),
        None
    );
}
