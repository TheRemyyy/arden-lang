use super::*;
use crate::formatter::{self};
use std::fs;

#[test]
fn compile_source_supports_lambda_callee_calls() {
    let temp_root = make_temp_project_root("lambda-callee-codegen");
    let source_path = temp_root.join("lambda_callee.arden");
    let output_path = temp_root.join("lambda_callee");
    let source = r#"
            function main(): None {
                x: Integer = ((y: Integer) => y + 1)(2);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("lambda callee codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_indexed_function_value_callees() {
    let temp_root = make_temp_project_root("indexed-function-callee-codegen");
    let source_path = temp_root.join("indexed_function_callee.arden");
    let output_path = temp_root.join("indexed_function_callee");
    let source = r#"
            function inc(x: Integer): Integer { return x + 1; }
            function dec(x: Integer): Integer { return x - 1; }

            function main(): None {
                fs: List<(Integer) -> Integer> = List<(Integer) -> Integer>();
                fs.push(inc);
                fs.push(dec);
                x: Integer = fs[0](1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("indexed function-value callee should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_if_expression_function_value_callees() {
    let temp_root = make_temp_project_root("ifexpr-function-callee-codegen");
    let source_path = temp_root.join("ifexpr_function_callee.arden");
    let output_path = temp_root.join("ifexpr_function_callee");
    let source = r#"
            function inc(x: Integer): Integer { return x + 1; }
            function dec(x: Integer): Integer { return x - 1; }

            function main(): None {
                x: Integer = (if (true) { inc; } else { dec; })(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("if-expression function-value callee should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn formatted_async_block_tail_expression_preserves_runtime_behavior() {
    let temp_root = make_temp_project_root("formatted-async-block-tail-runtime");
    let source_path = temp_root.join("formatted_async_block_tail_runtime.arden");
    let output_path = temp_root.join("formatted_async_block_tail_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async {
                    7
                };
                return await(task);
            }
        "#;

    let formatted = formatter::format_source(source).must("format source");
    fs::write(&source_path, &formatted).must("write formatted source");
    compile_source(
        &formatted,
        &source_path,
        &output_path,
        false,
        true,
        None,
        None,
    )
    .must("formatted async tail-expression source should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run formatted async tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_function_valued_field_calls() {
    let temp_root = make_temp_project_root("function-field-call-codegen");
    let source_path = temp_root.join("function_field_call.arden");
    let output_path = temp_root.join("function_field_call");
    let source = r#"
            class C {
                f: (Integer) -> Integer;
                constructor() { this.f = (n: Integer) => n + 1; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.f(2);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("function-valued field calls should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_generic_method_returning_lambda() {
    let temp_root = make_temp_project_root("generic-method-lambda-codegen");
    let source_path = temp_root.join("generic_method_lambda.arden");
    let output_path = temp_root.join("generic_method_lambda");
    let source = r#"
            class C {
                function mk<T>(x: T): () -> T { return () => x; }
            }

            function main(): None {
                c: C = C();
                f: () -> Integer = c.mk<Integer>(7);
                x: Integer = f();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("generic method returning lambda should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_zero_arg_pipe_lambda_runtime() {
    let temp_root = make_temp_project_root("zero-arg-pipe-lambda-runtime");
    let source_path = temp_root.join("zero_arg_pipe_lambda_runtime.arden");
    let output_path = temp_root.join("zero_arg_pipe_lambda_runtime");
    let source = r#"
            function make(): () -> Integer { return () => 7; }

            function main(): Integer {
                f: () -> Integer = make();
                return if (f() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("zero-arg pipe lambda should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg pipe lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_returning_zero_arg_pipe_lambda_runtime() {
    let temp_root = make_temp_project_root("generic-method-zero-arg-pipe-lambda-runtime");
    let source_path = temp_root.join("generic_method_zero_arg_pipe_lambda_runtime.arden");
    let output_path = temp_root.join("generic_method_zero_arg_pipe_lambda_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function lift(): () -> T { return () => this.value; }
            }

            function main(): Integer {
                b: Box<String> = Box<String>("ok");
                f: () -> () -> String = b.lift;
                return if (f()().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic method returning zero-arg pipe lambda should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic method zero-arg pipe lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
