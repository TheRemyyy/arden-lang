use super::TestExpectExt;
use crate::lsp::{
    find_nth_name_occurrence_in_span, lexer_error_range_impl, offset_in_span_impl,
    offset_to_position_impl, position_to_offset_impl, word_at_position_impl,
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

#[test]
fn lsp_name_lookup_treats_span_edges_as_identifier_boundaries() {
    let text = "foobar foo";

    let left_edge_span = 0..3;
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "foo", &left_edge_span, 0),
        Some(0..3)
    );

    let right_edge_span = 7..10;
    assert_eq!(
        find_nth_name_occurrence_in_span(text, "foo", &right_edge_span, 0),
        Some(7..10)
    );
}

#[test]
fn lsp_offset_in_span_uses_end_exclusive_boundary() {
    let span = 3..6;
    assert!(offset_in_span_impl(3, &span));
    assert!(offset_in_span_impl(5, &span));
    assert!(!offset_in_span_impl(6, &span));
}

#[test]
fn lsp_lexer_error_range_handles_extreme_offsets_without_overflow() {
    let text = "x";
    let msg = "Unknown token at 18446744073709551615: '?'";
    let range = lexer_error_range_impl(text, msg);
    assert_eq!(range.start, Position::new(0, 1));
    assert_eq!(range.end, Position::new(0, 1));
}

#[test]
fn lsp_lexer_error_range_saturates_for_offsets_larger_than_usize() {
    let text = "x";
    let msg = "Unknown token at 9999999999999999999999999999999999999999: '?'";
    let range = lexer_error_range_impl(text, msg);
    assert_eq!(range.start, Position::new(0, 1));
    assert_eq!(range.end, Position::new(0, 1));
}

#[test]
fn lsp_crlf_offsets_roundtrip_without_counting_carriage_return_column() {
    let text = "first\r\nsecond";
    let second_start = text.find("second").must("second line token should exist");
    let position = offset_to_position_impl(text, second_start);
    assert_eq!(position, Position::new(1, 0));
    assert_eq!(position_to_offset_impl(text, position), second_start);
}

#[test]
fn lsp_position_to_offset_uses_logical_columns_on_crlf_lines() {
    let text = "ab\r\nxy";
    let y_offset = text.find('y').must("expected y");
    assert_eq!(offset_to_position_impl(text, y_offset), Position::new(1, 1));
    assert_eq!(position_to_offset_impl(text, Position::new(1, 1)), y_offset);
}

#[test]
fn lsp_position_to_offset_clamps_past_eol_to_start_of_crlf_sequence() {
    let text = "ab\r\nxy";
    assert_eq!(position_to_offset_impl(text, Position::new(0, 99)), 2);
}
