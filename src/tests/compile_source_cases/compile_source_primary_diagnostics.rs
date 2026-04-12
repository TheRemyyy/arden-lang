use super::*;
use std::fs;

#[test]
fn compile_source_reports_only_primary_error_for_invalid_interpolation_expr() {
    let temp_root = make_temp_project_root("string-interpolation-primary-error-runtime");
    let source_path = temp_root.join("string_interpolation_primary_error_runtime.arden");
    let output_path = temp_root.join("string_interpolation_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: String = "{1 + true}";
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid interpolation expression should fail typecheck");
    assert!(
        err.contains("Arithmetic operator requires numeric types, got Integer and Boolean"),
        "{err}"
    );
    assert!(
        !err.contains("String interpolation currently supports"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_method_receiver() {
    let temp_root = make_temp_project_root("unknown-method-receiver-primary-error-runtime");
    let source_path = temp_root.join("unknown_method_receiver_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_method_receiver_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope.missing();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown method receiver should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot call method on type unknown"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_to_string_arg() {
    let temp_root = make_temp_project_root("unknown-to-string-arg-primary-error-runtime");
    let source_path = temp_root.join("unknown_to_string_arg_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_to_string_arg_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: String = to_string(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown to_string arg should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("to_string() currently supports"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_print_arg() {
    let temp_root = make_temp_project_root("unknown-print-arg-primary-error-runtime");
    let source_path = temp_root.join("unknown_print_arg_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_print_arg_primary_error_runtime");
    let source = r#"
            import std.io.print;

            function main(): None {
                print(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown print arg should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("print() currently supports"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_arithmetic_operand() {
    let temp_root = make_temp_project_root("unknown-arithmetic-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_arithmetic_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_arithmetic_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope + 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown arithmetic operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Arithmetic operator requires numeric types"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_comparison_operand() {
    let temp_root = make_temp_project_root("unknown-comparison-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_comparison_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_comparison_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = nope < 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown comparison operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Comparison requires numeric types"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_accepts_ordered_char_comparisons() {
    let temp_root = make_temp_project_root("ordered-char-comparison-runtime");
    let source_path = temp_root.join("ordered_char_comparison_runtime.arden");
    let output_path = temp_root.join("ordered_char_comparison_runtime");
    let source = r#"
            function main(): Integer {
                letter: Char = 'm';
                return if ((letter >= 'a') && (letter <= 'z')) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("ordered char comparison should compile");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run ordered char comparison binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_logical_operand() {
    let temp_root = make_temp_project_root("unknown-logical-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_logical_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_logical_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = nope && true;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown logical operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Logical operator requires Boolean types"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_indexed_object() {
    let temp_root = make_temp_project_root("unknown-indexed-object-primary-error-runtime");
    let source_path = temp_root.join("unknown_indexed_object_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_indexed_object_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope[0];
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown indexed object should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot index type unknown"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_string_index() {
    let temp_root = make_temp_project_root("unknown-string-index-primary-error-runtime");
    let source_path = temp_root.join("unknown_string_index_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_string_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Char = "hi"[nope];
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown string index should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Index must be Integer, found unknown"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_list_get_index() {
    let temp_root = make_temp_project_root("unknown-list-get-index-primary-error-runtime");
    let source_path = temp_root.join("unknown_list_get_index_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_list_get_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                value: Integer = xs.get(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown list get index should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("List.get() index must be Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_list_set_index() {
    let temp_root = make_temp_project_root("unknown-list-set-index-primary-error-runtime");
    let source_path = temp_root.join("unknown_list_set_index_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_list_set_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                xs.set(nope, 1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown list set index should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("List.set() index must be Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_await_operand() {
    let temp_root = make_temp_project_root("unknown-await-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_await_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_await_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = await(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown await operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("'await' can only be used on Task types"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_require_condition() {
    let temp_root = make_temp_project_root("unknown-require-condition-primary-error-runtime");
    let source_path = temp_root.join("unknown_require_condition_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_require_condition_primary_error_runtime");
    let source = r#"
            function main(): None {
                require(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown require condition should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("require() condition must be Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_if_condition() {
    let temp_root = make_temp_project_root("unknown-if-condition-primary-error-runtime");
    let source_path = temp_root.join("unknown_if_condition_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_if_condition_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = if (nope) { 1 } else { 2 };
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown if condition should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("If condition must be Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_range_argument() {
    let temp_root = make_temp_project_root("unknown-range-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_range_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_range_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Range<Integer> = range(nope, 3);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown range argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("range() arguments must be all Integer or all Float"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_exit_argument() {
    let temp_root = make_temp_project_root("unknown-exit-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_exit_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_exit_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                exit(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown exit argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("exit() requires Integer code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_fail_argument() {
    let temp_root = make_temp_project_root("unknown-fail-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_fail_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_fail_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                fail(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown fail argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("fail() requires String message"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_assert_argument() {
    let temp_root = make_temp_project_root("unknown-assert-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_assert_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_assert_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown assert argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("assert() requires boolean condition"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_assert_true_argument() {
    let temp_root = make_temp_project_root("unknown-assert-true-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_assert_true_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_assert_true_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert_true(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown assert_true argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("assert_true() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_assert_false_argument() {
    let temp_root = make_temp_project_root("unknown-assert-false-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_assert_false_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_assert_false_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert_false(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown assert_false argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("assert_false() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_len_argument() {
    let temp_root = make_temp_project_root("unknown-str-len-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_len_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_len_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Integer = Str.len(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.len argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.len() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_contains_argument() {
    let temp_root = make_temp_project_root("unknown-str-contains-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_contains_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_contains_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.contains(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.contains argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Str.contains() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_starts_with_argument() {
    let temp_root =
        make_temp_project_root("unknown-str-starts-with-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_str_starts_with_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_starts_with_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.startsWith(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.startsWith argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Str.startsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_ends_with_argument() {
    let temp_root = make_temp_project_root("unknown-str-ends-with-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_ends_with_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_ends_with_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.endsWith(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.endsWith argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Str.endsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_compare_argument() {
    let temp_root = make_temp_project_root("unknown-str-compare-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_compare_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_compare_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Integer = Str.compare(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.compare argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Str.compare() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_concat_argument() {
    let temp_root = make_temp_project_root("unknown-str-concat-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_concat_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_concat_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.concat(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.concat argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Str.concat() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_upper_argument() {
    let temp_root = make_temp_project_root("unknown-str-upper-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_upper_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_upper_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.upper(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.upper argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.upper() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_trim_argument() {
    let temp_root = make_temp_project_root("unknown-str-trim-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_trim_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_str_trim_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.trim(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Str.trim argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.trim() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_read_argument() {
    let temp_root = make_temp_project_root("unknown-file-read-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_read_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_file_read_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                value: String = File.read(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown File.read argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.read() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_exists_argument() {
    let temp_root = make_temp_project_root("unknown-file-exists-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_exists_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_file_exists_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                value: Boolean = File.exists(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown File.exists argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.exists() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_delete_argument() {
    let temp_root = make_temp_project_root("unknown-file-delete-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_delete_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_file_delete_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                File.delete(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown File.delete argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.delete() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_write_path() {
    let temp_root = make_temp_project_root("unknown-file-write-path-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_write_path_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_file_write_path_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                File.write(nope, "x");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown File.write path should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.write() path must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_await_timeout_argument() {
    let temp_root = make_temp_project_root("unknown-await-timeout-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_await_timeout_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_await_timeout_argument_primary_error_runtime");
    let source = r#"
            async function work(): Task<Integer> { return 1; }

            function main(): None {
                maybe: Option<Integer> = work().await_timeout(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown await_timeout argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Task.await_timeout() expects Integer milliseconds"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_args_get_argument() {
    let temp_root = make_temp_project_root("unknown-args-get-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_args_get_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_args_get_argument_primary_error_runtime");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Args.get argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Args.get() requires Integer index"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_time_now_argument() {
    let temp_root = make_temp_project_root("unknown-time-now-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_time_now_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_time_now_argument_primary_error_runtime");
    let source = r#"
            import std.time.*;

            function main(): None {
                value: String = Time.now(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Time.now argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Time.now() requires String format"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_time_sleep_argument() {
    let temp_root = make_temp_project_root("unknown-time-sleep-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_time_sleep_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_time_sleep_argument_primary_error_runtime");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Time.sleep argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Time.sleep() requires Integer milliseconds"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_system_getenv_argument() {
    let temp_root = make_temp_project_root("unknown-system-getenv-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_system_getenv_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_system_getenv_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.getenv(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown System.getenv argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("System.getenv() requires String name"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_system_shell_argument() {
    let temp_root = make_temp_project_root("unknown-system-shell-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_system_shell_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_system_shell_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: Integer = System.shell(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown System.shell argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("System.shell() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_system_exec_argument() {
    let temp_root = make_temp_project_root("unknown-system-exec-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_system_exec_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_system_exec_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.exec(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown System.exec argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("System.exec() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_unary_neg_operand() {
    let temp_root = make_temp_project_root("unknown-unary-neg-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_unary_neg_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_unary_neg_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = -nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown unary neg operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot negate non-numeric type"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_unary_not_operand() {
    let temp_root = make_temp_project_root("unknown-unary-not-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_unary_not_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_unary_not_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = !nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown unary not operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Cannot apply '!' to non-boolean type"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_try_operand() {
    let temp_root = make_temp_project_root("unknown-try-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_try_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_try_operand_primary_error_runtime");
    let source = r#"
            function helper(): Option<Integer> {
                value: Integer = nope?;
                return Option.some(value);
            }

            function main(): None {
                helper();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown try operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("'?' operator can only be used on Option or Result"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_deref_operand() {
    let temp_root = make_temp_project_root("unknown-deref-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_deref_operand_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_deref_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = *nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown deref operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Cannot dereference non-pointer type"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_list_constructor_argument() {
    let temp_root =
        make_temp_project_root("unknown-list-constructor-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_list_constructor_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_list_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: List<Integer> = List<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown List constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor List<Integer> expects optional Integer capacity"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_map_constructor_argument() {
    let temp_root =
        make_temp_project_root("unknown-map-constructor-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_map_constructor_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_map_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: Map<String, Integer> = Map<String, Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Map constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor Map<String, Integer> expects 0 arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_set_constructor_argument() {
    let temp_root =
        make_temp_project_root("unknown-set-constructor-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_set_constructor_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_set_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: Set<Integer> = Set<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Set constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor Set<Integer> expects 0 arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_option_constructor_argument() {
    let temp_root =
        make_temp_project_root("unknown-option-constructor-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_option_constructor_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_option_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Option<Integer> = Option<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Option constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor Option<Integer> expects 0 arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_result_constructor_argument() {
    let temp_root =
        make_temp_project_root("unknown-result-constructor-argument-primary-error-runtime");
    let source_path =
        temp_root.join("unknown_result_constructor_argument_primary_error_runtime.arden");
    let output_path = temp_root.join("unknown_result_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Result<Integer, String> = Result<Integer, String>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("unknown Result constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor Result<Integer, String> expects 0 arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}
