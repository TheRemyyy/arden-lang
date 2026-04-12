use super::*;
use std::fs;

#[test]
fn compile_source_runs_match_result_error_equality_against_static_constructor() {
    let temp_root = make_temp_project_root("match-result-error-static-eq-runtime");
    let source_path = temp_root.join("match_result_error_static_eq_runtime.arden");
    let output_path = temp_root.join("match_result_error_static_eq_runtime");
    let source = r#"
            function choose(flag: Boolean): Result<Integer, Integer> {
                if (flag) {
                    return Result.ok(1);
                }
                return Result.error(7);
            }

            function main(): Integer {
                value: Result<Integer, Integer> = match (false) {
                    true => choose(true),
                    false => choose(false),
                };
                if (value == Result.error(7)) { return 45; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("match-result static equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled match-result static equality binary");
    assert_eq!(status.code(), Some(45));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_ultra_nested_option_result_static_equality() {
    let temp_root = make_temp_project_root("ultra-nested-option-result-static-eq-runtime");
    let source_path = temp_root.join("ultra_nested_option_result_static_eq_runtime.arden");
    let output_path = temp_root.join("ultra_nested_option_result_static_eq_runtime");
    let source = r#"
            function wrap(flag: Boolean): Option<Result<Option<Result<Integer, Integer>>, Integer>> {
                if (flag) {
                    return Option.some(Result.ok(Option.some(Result.error(11))));
                }
                return Option.some(Result.error(9));
            }

            function main(): Integer {
                outer: Option<Result<Option<Result<Integer, Integer>>, Integer>> = if (true) {
                    wrap(true)
                } else {
                    wrap(false)
                };
                inner: Result<Option<Result<Integer, Integer>>, Integer> = outer.unwrap();
                payload: Option<Result<Integer, Integer>> = match (inner) {
                    Ok(value) => value,
                    Error(err) => Option.none(),
                };
                value: Result<Integer, Integer> = payload.unwrap();
                if (value == Result.error(11)) { return 46; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("ultra-nested option/result static equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled ultra-nested option/result static equality binary");
    assert_eq!(status.code(), Some(46));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_merge_option_none_method_chain() {
    let temp_root = make_temp_project_root("if-merge-option-none-method-runtime");
    let source_path = temp_root.join("if_merge_option_none_method_runtime.arden");
    let output_path = temp_root.join("if_merge_option_none_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function choose(flag: Boolean): Option<Boxed> {
                return if (flag) { Option.some(Boxed(47)) } else { Option.none() };
            }

            function main(): Integer {
                value: Option<Boxed> = choose(true);
                return value.unwrap().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if-merge option none method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if-merge option none method binary");
    assert_eq!(status.code(), Some(47));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_if_match_if_tagged_merge_method_chain() {
    let temp_root = make_temp_project_root("nested-if-match-if-tagged-merge-runtime");
    let source_path = temp_root.join("nested_if_match_if_tagged_merge_runtime.arden");
    let output_path = temp_root.join("nested_if_match_if_tagged_merge_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function choose(flag: Boolean): Result<Option<Boxed>, Integer> {
                return if (flag) {
                    match (true) {
                        true => if (true) { Result.ok(Option.some(Boxed(48))) } else { Result.error(1) },
                        false => Result.error(2),
                    }
                } else {
                    Result.error(3)
                };
            }

            function main(): Integer {
                value: Result<Option<Boxed>, Integer> = choose(true);
                return value.unwrap().unwrap().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested if-match-if tagged merge should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested if-match-if tagged merge binary");
    assert_eq!(status.code(), Some(48));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_chaotic_unreachable_tagged_branch_chain() {
    let temp_root = make_temp_project_root("chaotic-unreachable-tagged-branch-runtime");
    let source_path = temp_root.join("chaotic_unreachable_tagged_branch_runtime.arden");
    let output_path = temp_root.join("chaotic_unreachable_tagged_branch_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function choose(flag: Boolean): Option<Result<Boxed, Integer>> {
                return if (flag) {
                    Option.some(if (true) { Result.ok(Boxed(49)) } else { Result.error(1) })
                } else {
                    if (false) { Option.none() } else { Option.some(Result.error(2)) }
                };
            }

            function main(): Integer {
                picked: Option<Result<Boxed, Integer>> = match (true) {
                    true => choose(true),
                    false => choose(false),
                };
                inner: Result<Boxed, Integer> = picked.unwrap();
                if (inner == Result.error(2)) { return 1; }
                return inner.unwrap().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("chaotic unreachable tagged branch chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled chaotic unreachable tagged branch binary");
    assert_eq!(status.code(), Some(49));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_index_object_results() {
    let temp_root = make_temp_project_root("list-index-empty-object-runtime");
    let source_path = temp_root.join("list_index_empty_object_runtime.arden");
    let output_path = temp_root.join("list_index_empty_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs[0].value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("empty list index object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled empty list index object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}
