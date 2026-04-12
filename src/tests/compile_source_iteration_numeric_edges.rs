use super::*;
use std::fs;

#[test]
fn compile_source_runs_string_iteration_runtime() {
    let temp_root = make_temp_project_root("string-iteration-runtime");
    let source_path = temp_root.join("string_iteration_runtime.arden");
    let output_path = temp_root.join("string_iteration_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "abc";
                mut total: Integer = 0;
                for (ch in s) {
                    total = total + 1;
                }
                return if (total == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_string_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-string-iteration-runtime");
    let source_path = temp_root.join("borrowed_string_iteration_runtime.arden");
    let output_path = temp_root.join("borrowed_string_iteration_runtime");
    let source = r#"
            function main(): Integer {
                text: String = "Ahoj";
                view: &String = &text;
                mut total: Integer = 0;
                for (ch in view) {
                    total = total + 1;
                }
                return if (total == 4) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutably_borrowed_string_iteration_runtime() {
    let temp_root = make_temp_project_root("mut-borrowed-string-iteration-runtime");
    let source_path = temp_root.join("mut_borrowed_string_iteration_runtime.arden");
    let output_path = temp_root.join("mut_borrowed_string_iteration_runtime");
    let source = r#"
            function main(): Integer {
                mut text: String = "a";
                view: &mut String = &mut text;
                mut total: Integer = 0;
                for (ch in view) {
                    total = total + 1;
                }
                return if (total == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutably borrowed string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutably borrowed string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_for_loop_sugar_runtime.arden");
    let output_path = temp_root.join("integer_for_loop_sugar_runtime");
    let source = r#"
            function main(): Integer {
                mut total: Integer = 0;
                for (x in 4) {
                    total = total + x;
                }
                return if (total == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_typed_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("float-typed-integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("float_typed_integer_for_loop_sugar_runtime.arden");
    let output_path = temp_root.join("float_typed_integer_for_loop_sugar_runtime");
    let source = r#"
            function main(): Integer {
                mut total: Float = 0.0;
                for (x: Float in 4) {
                    total = total + x;
                }
                return if (total == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("typed integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled typed integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_zero_length_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("zero-length-integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("zero_length_integer_for_loop_sugar_runtime.arden");
    let output_path = temp_root.join("zero_length_integer_for_loop_sugar_runtime");
    let source = r#"
            function main(): Integer {
                mut total: Integer = 0;
                for (x in 0) {
                    total = total + 1;
                }
                return if (total == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("zero-length integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-length integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_expression_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-expression-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_expression_for_loop_sugar_runtime.arden");
    let output_path = temp_root.join("integer_expression_for_loop_sugar_runtime");
    let source = r#"
            function main(): Integer {
                end: Integer = 4;
                mut total: Integer = 0;
                for (x in end) {
                    total = total + x;
                }
                return if (total == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer expression for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled integer expression for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_call_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-call-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_call_for_loop_sugar_runtime.arden");
    let output_path = temp_root.join("integer_call_for_loop_sugar_runtime");
    let source = r#"
            function make_end(): Integer {
                return 4;
            }

            function main(): Integer {
                mut total: Integer = 0;
                for (x in make_end()) {
                    total = total + x;
                }
                return if (total == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer call for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled integer call for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_range_integer_to_float_assignment() {
    let temp_root = make_temp_project_root("reject-range-int-float-assignment");
    let source_path = temp_root.join("reject_range_int_float_assignment.arden");
    let output_path = temp_root.join("reject_range_int_float_assignment");
    let source = r#"
            function main(): None {
                values: Range<Float> = range(1, 3);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Range<Integer> should not typecheck as Range<Float>");
    assert!(
        err.contains("Type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_option_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-option-int-float-argument");
    let source_path = temp_root.join("reject_option_int_float_argument.arden");
    let output_path = temp_root.join("reject_option_int_float_argument");
    let source = r#"
            function take(value: Option<Float>): None {
                return None;
            }

            function main(): None {
                take(Option.some(1));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Option<Integer> should not typecheck as Option<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_task_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-task-int-float-argument");
    let source_path = temp_root.join("reject_task_int_float_argument.arden");
    let output_path = temp_root.join("reject_task_int_float_argument");
    let source = r#"
            async function take(value: Task<Float>): Task<None> {
                return None;
            }

            async function main_async(): Task<None> {
                pending: Task<Integer> = async { 1 };
                await take(pending);
                return None;
            }

            function main(): None {
                await main_async();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Task<Integer> should not typecheck as Task<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_map_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-map-int-float-argument");
    let source_path = temp_root.join("reject_map_int_float_argument.arden");
    let output_path = temp_root.join("reject_map_int_float_argument");
    let source = r#"
            function take(values: Map<String, Float>): None {
                return None;
            }

            function main(): None {
                ints: Map<String, Integer> = Map<String, Integer>();
                ints.insert("a", 1);
                take(ints);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Map<String, Integer> should not typecheck as Map<String, Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_list_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-list-int-float-argument");
    let source_path = temp_root.join("reject_list_int_float_argument.arden");
    let output_path = temp_root.join("reject_list_int_float_argument");
    let source = r#"
            function take(values: List<Float>): None {
                return None;
            }

            function main(): None {
                ints: List<Integer> = List<Integer>();
                ints.push(1);
                take(ints);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("List<Integer> should not typecheck as List<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_option_integer_to_float_return() {
    let temp_root = make_temp_project_root("reject-option-int-float-return");
    let source_path = temp_root.join("reject_option_int_float_return.arden");
    let output_path = temp_root.join("reject_option_int_float_return");
    let source = r#"
            function produce(): Option<Float> {
                return Option.some(1);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Option<Integer> return should not typecheck as Option<Float>");
    assert!(
        err.contains("Return type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_if_expression_join_between_integer_and_float_ranges() {
    let temp_root = make_temp_project_root("reject-if-range-join-int-float");
    let source_path = temp_root.join("reject_if_range_join_int_float.arden");
    let output_path = temp_root.join("reject_if_range_join_int_float");
    let source = r#"
            function main(): None {
                cond: Boolean = true;
                values: Range<Float> = if (cond) { range(1, 3); } else { range(2.0, 4.0); };
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Range<Integer> and Range<Float> branches should not join");
    assert!(
        err.contains("Type mismatch")
            || err.contains("Mismatched branch types")
            || err.contains("If expression branch type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_out_of_bounds_list_index_assignment() {
    let temp_root = make_temp_project_root("list-index-assign-oob-runtime");
    let source_path = temp_root.join("list_index_assign_oob_runtime.arden");
    let output_path = temp_root.join("list_index_assign_oob_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs[10] = 24;
                return 24;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("out-of-bounds list assignment should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list assignment oob binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}
