use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_invalid_integer_boolean_addition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-bool-add");
    let source_path = temp_root.join("no_check_invalid_int_bool_add.arden");
    let output_path = temp_root.join("no_check_invalid_int_bool_add");
    let source = r#"
            function main(): Integer {
                return 1 + true;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid integer + boolean should fail in codegen without checks");
    assert!(
        err.contains("Arithmetic operator requires numeric types, got Integer and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_integer_boolean_equality_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-bool-eq");
    let source_path = temp_root.join("no_check_invalid_int_bool_eq.arden");
    let output_path = temp_root.join("no_check_invalid_int_bool_eq");
    let source = r#"
            function main(): Integer {
                return if (1 == true) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid integer == boolean should fail in codegen without checks");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_boolean_comparison_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-bool-comparison");
    let source_path = temp_root.join("no_check_invalid_bool_comparison.arden");
    let output_path = temp_root.join("no_check_invalid_bool_comparison");
    let source = r#"
            function main(): Integer {
                return if (true < false) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid boolean comparison should fail in codegen without checks");
    assert!(
        err.contains("Comparison requires ordered types, got Boolean and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_accepts_ordered_char_comparisons_in_codegen() {
    let temp_root = make_temp_project_root("ordered-char-comparison-no-check-runtime");
    let source_path = temp_root.join("ordered_char_comparison_no_check_runtime.arden");
    let output_path = temp_root.join("ordered_char_comparison_no_check_runtime");
    let source = r#"
            function main(): Integer {
                letter: Char = 'M';
                return if ((letter >= 'A') && (letter <= 'Z')) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("ordered char comparison should compile in codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run ordered char comparison no-check binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_integer_logical_and_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-logical-and");
    let source_path = temp_root.join("no_check_invalid_int_logical_and.arden");
    let output_path = temp_root.join("no_check_invalid_int_logical_and");
    let source = r#"
            function main(): Integer {
                return if (1 && 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid integer logical and should fail in codegen without checks");
    assert!(
        err.contains("Logical operator requires Boolean types, got Integer and Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_interpolated_integer_boolean_addition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-interp-int-bool-add");
    let source_path = temp_root.join("no_check_invalid_interp_int_bool_add.arden");
    let output_path = temp_root.join("no_check_invalid_interp_int_bool_add");
    let source = r#"
            function main(): None {
                println("{1 + true}");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid interpolated integer + boolean should fail in codegen");
    assert!(
        err.contains("Arithmetic operator requires numeric types, got Integer and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_unary_negation_on_boolean_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-neg-bool");
    let source_path = temp_root.join("no_check_invalid_unary_neg_bool.arden");
    let output_path = temp_root.join("no_check_invalid_unary_neg_bool");
    let source = r#"
            function main(): Integer {
                return -true;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid unary negation on boolean should fail in codegen");
    assert!(
        err.contains("Cannot negate non-numeric type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_unary_not_on_integer_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-not-int");
    let source_path = temp_root.join("no_check_invalid_unary_not_int.arden");
    let output_path = temp_root.join("no_check_invalid_unary_not_int");
    let source = r#"
            function main(): Integer {
                return if (!1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid unary not on integer should fail in codegen");
    assert!(
        err.contains("Cannot apply '!' to non-boolean type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_invalid_unary_neg_with_user_facing_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-neg-module-local-type");
    let source_path = temp_root.join("no_check_invalid_unary_neg_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_unary_neg_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return -M.Box(7);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid unary negation on module-local Box should fail in codegen");
    assert!(
        err.contains("Cannot negate non-numeric type M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_invalid_unary_not_with_user_facing_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-not-module-local-type");
    let source_path = temp_root.join("no_check_invalid_unary_not_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_unary_not_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Boolean {
                return !M.Box(7);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid unary not on module-local Box should fail in codegen");
    assert!(
        err.contains("Cannot apply '!' to non-boolean type M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_if_statement_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-if-stmt-condition");
    let source_path = temp_root.join("no_check_invalid_if_stmt_condition.arden");
    let output_path = temp_root.join("no_check_invalid_if_stmt_condition");
    let source = r#"
            function main(): Integer {
                if (1) { return 0; }
                return 1;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean if statement condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_if_expression_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-if-expr-condition");
    let source_path = temp_root.join("no_check_invalid_if_expr_condition.arden");
    let output_path = temp_root.join("no_check_invalid_if_expr_condition");
    let source = r#"
            function main(): Integer {
                return if (1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean if expression condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_while_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-while-condition");
    let source_path = temp_root.join("no_check_invalid_while_condition.arden");
    let output_path = temp_root.join("no_check_invalid_while_condition");
    let source = r#"
            function main(): Integer {
                while (1) { return 0; }
                return 1;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean while condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_require_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-condition");
    let source_path = temp_root.join("no_check_invalid_require_condition.arden");
    let output_path = temp_root.join("no_check_invalid_require_condition");
    let source = r#"
            function main(): None {
                require(1, "boom");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean require condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_require_without_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-no-msg-condition");
    let source_path = temp_root.join("no_check_invalid_require_no_msg_condition.arden");
    let output_path = temp_root.join("no_check_invalid_require_no_msg_condition");
    let source = r#"
            function main(): None {
                require(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean require condition without message should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-condition");
    let source_path = temp_root.join("no_check_invalid_assert_condition.arden");
    let output_path = temp_root.join("no_check_invalid_assert_condition");
    let source = r#"
            function main(): None {
                assert(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean assert condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_checked_rejects_non_boolean_assert_condition() {
    let temp_root = make_temp_project_root("checked-invalid-assert-condition");
    let source_path = temp_root.join("checked_invalid_assert_condition.arden");
    let output_path = temp_root.join("checked_invalid_assert_condition");
    let source = r#"
            function main(): None {
                assert(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("checked assert(Integer) should fail");
    assert!(err.contains("assert() requires boolean condition"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_true_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-true-condition");
    let source_path = temp_root.join("no_check_invalid_assert_true_condition.arden");
    let output_path = temp_root.join("no_check_invalid_assert_true_condition");
    let source = r#"
            function main(): None {
                assert_true(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean assert_true condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_boolean_assert_true_condition_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-assert-true-module-local-type");
    let source_path = temp_root.join("no_check_invalid_assert_true_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_assert_true_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                assert_true(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_true on module-local Box should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_checked_rejects_non_boolean_assert_true_condition() {
    let temp_root = make_temp_project_root("checked-invalid-assert-true-condition");
    let source_path = temp_root.join("checked_invalid_assert_true_condition.arden");
    let output_path = temp_root.join("checked_invalid_assert_true_condition");
    let source = r#"
            function main(): None {
                assert_true(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("checked assert_true(Integer) should fail");
    assert!(err.contains("assert_true() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_false_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-false-condition");
    let source_path = temp_root.join("no_check_invalid_assert_false_condition.arden");
    let output_path = temp_root.join("no_check_invalid_assert_false_condition");
    let source = r#"
            function main(): None {
                assert_false(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-boolean assert_false condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_boolean_assert_false_condition_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-assert-false-module-local-type");
    let source_path = temp_root.join("no_check_invalid_assert_false_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_assert_false_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                assert_false(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_false on module-local Box should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_checked_rejects_non_boolean_assert_false_condition() {
    let temp_root = make_temp_project_root("checked-invalid-assert-false-condition");
    let source_path = temp_root.join("checked_invalid_assert_false_condition.arden");
    let output_path = temp_root.join("checked_invalid_assert_false_condition");
    let source = r#"
            function main(): None {
                assert_false(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("checked assert_false(Integer) should fail");
    assert!(err.contains("assert_false() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
