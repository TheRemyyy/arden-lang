use super::*;
use std::fs;

#[test]
fn compile_source_runs_constructor_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("constructor-builtin-function-value-runtime");
    let source_path = temp_root.join("constructor_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("constructor_builtin_function_value_runtime");
    let source = r#"
            class Box {
                f: (Integer) -> Float;
                constructor(f: (Integer) -> Float) { this.f = f; }
            }
            function main(): Integer {
                return if (Box(to_float).f(1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("constructor builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled constructor builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_constructor_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-constructor-builtin-function-value-runtime");
    let source_path = temp_root.join("generic_constructor_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("generic_constructor_builtin_function_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }
            function main(): Integer {
                return if (Box<(Integer) -> Float>(to_float).value(1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic constructor builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic constructor builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-method-builtin-function-value-runtime");
    let source_path = temp_root.join("generic_method_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("generic_method_builtin_function_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }
                function get(): T { return this.value; }
            }
            function main(): Integer {
                mapped: Box<Float> = Box<Integer>(1).map<Float>(to_float);
                return if (mapped.get() == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_user_defined_result_generic_method_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("user-defined-result-generic-method-builtin-function-value");
    let source_path =
        temp_root.join("user_defined_result_generic_method_builtin_function_value.arden");
    let output_path = temp_root.join("user_defined_result_generic_method_builtin_function_value");
    let source = r#"
            class Result<T, E> {
                ok: T;
                err: E;
                constructor(ok: T, err: E) { this.ok = ok; this.err = err; }
                function map_ok<U>(f: (T) -> U): Result<U, E> { return Result<U, E>(f(this.ok), this.err); }
                function value(): T { return this.ok; }
            }
            function main(): Integer {
                mapped: Result<Float, String> = Result<Integer, String>(1, "ok").map_ok<Float>(to_float);
                return if (mapped.value() == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("user-defined Result generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled user-defined Result generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_receiver_result_generic_method_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("if-receiver-result-generic-method-builtin-function-value");
    let source_path =
        temp_root.join("if_receiver_result_generic_method_builtin_function_value.arden");
    let output_path = temp_root.join("if_receiver_result_generic_method_builtin_function_value");
    let source = r#"
            class Result<T, E> {
                ok: T;
                err: E;
                constructor(ok: T, err: E) { this.ok = ok; this.err = err; }
                function map_ok<U>(f: (T) -> U): Result<U, E> { return Result<U, E>(f(this.ok), this.err); }
                function value(): T { return this.ok; }
            }
            function main(): Integer {
                mapped: Result<Float, String> =
                    (if (true) { Result<Integer, String>(1, "ok"); } else { Result<Integer, String>(2, "ok"); })
                        .map_ok<Float>(to_float);
                return if (mapped.value() == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if receiver Result generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if receiver Result generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_receiver_map_generic_method_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("if-receiver-map-generic-method-builtin-function-value");
    let source_path = temp_root.join("if_receiver_map_generic_method_builtin_function_value.arden");
    let output_path = temp_root.join("if_receiver_map_generic_method_builtin_function_value");
    let source = r#"
            class Map<K, V> {
                key: K;
                value: V;
                constructor(key: K, value: V) { this.key = key; this.value = value; }
                function map_value<U>(f: (V) -> U): Map<K, U> { return Map<K, U>(this.key, f(this.value)); }
                function get(): V { return this.value; }
            }
            function main(): Integer {
                mapped: Map<String, Float> =
                    (if (true) { Map<String, Integer>("x", 1); } else { Map<String, Integer>("y", 2); })
                        .map_value<Float>(to_float);
                return if (mapped.get() == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if receiver Map generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if receiver Map generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_lambda_capturing_this_runtime() {
    let temp_root = make_temp_project_root("method-lambda-capturing-this-runtime");
    let source_path = temp_root.join("method_lambda_capturing_this_runtime.arden");
    let output_path = temp_root.join("method_lambda_capturing_this_runtime");
    let source = r#"
            class C {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function mk(): () -> Integer { return () => this.value; }
            }
            function main(): Integer {
                f: () -> Integer = C(7).mk();
                return if (f() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method lambda capturing this should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled method lambda capturing this binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_lambda_capturing_this_and_builtin_callback_runtime() {
    let temp_root = make_temp_project_root("generic-method-lambda-capturing-this-builtin-callback");
    let source_path = temp_root.join("generic_method_lambda_capturing_this_builtin_callback.arden");
    let output_path = temp_root.join("generic_method_lambda_capturing_this_builtin_callback");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function mk<U>(f: (T) -> U): () -> U { return () => f(this.value); }
            }
            function main(): Integer {
                thunk: () -> Float = Box<Integer>(7).mk<Float>(to_float);
                return if (thunk() == 7.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic method lambda capturing this with builtin callback should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic method lambda capturing this builtin callback binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_lambda_if_expression_capture_runtime() {
    let temp_root = make_temp_project_root("lambda-if-expression-capture-runtime");
    let source_path = temp_root.join("lambda_if_expression_capture_runtime.arden");
    let output_path = temp_root.join("lambda_if_expression_capture_runtime");
    let source = r#"
            function main(): Integer {
                x: Integer = 7;
                f: () -> Integer = () => if (true) { x } else { 0 };
                return if (f() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("lambda if-expression capture should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled lambda if-expression capture binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_if_expression_capture_runtime() {
    let temp_root = make_temp_project_root("async-if-expression-capture-runtime");
    let source_path = temp_root.join("async_if_expression_capture_runtime.arden");
    let output_path = temp_root.join("async_if_expression_capture_runtime");
    let source = r#"
            function main(): Integer {
                x: Integer = 7;
                t: Task<Integer> = async { if (true) { x } else { 0 } };
                return if (await(t) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async if-expression capture should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async if-expression capture binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_shadowed_borrow_name_without_false_capture_runtime() {
    let temp_root = make_temp_project_root("async-shadowed-borrow-name-runtime");
    let source_path = temp_root.join("async_shadowed_borrow_name_runtime.arden");
    let output_path = temp_root.join("async_shadowed_borrow_name_runtime");
    let source = r#"
            function main(): Integer {
                seed: Integer = 1;
                r: &Integer = &seed;
                task: Task<Integer> = async {
                    r: Integer = 2;
                    r
                };
                return if (await(task) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async shadowed borrow name should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async shadowed borrow name binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_match_pattern_shadowed_borrow_name_runtime() {
    let temp_root = make_temp_project_root("async-match-pattern-shadowed-borrow-name-runtime");
    let source_path = temp_root.join("async_match_pattern_shadowed_borrow_name_runtime.arden");
    let output_path = temp_root.join("async_match_pattern_shadowed_borrow_name_runtime");
    let source = r#"
            enum E { A(Integer) }
            function main(): Integer {
                seed: Integer = 1;
                v: &Integer = &seed;
                t: Task<Integer> = match (E.A(7)) {
                    E.A(v) => { async { v } }
                };
                return if (await(t) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async match pattern shadowed borrow name should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async match pattern shadowed borrow name binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_integer_tail_for_float_task_runtime() {
    let temp_root = make_temp_project_root("async-int-tail-float-task-runtime");
    let source_path = temp_root.join("async_int_tail_float_task_runtime.arden");
    let output_path = temp_root.join("async_int_tail_float_task_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async { 1 };
                value: Float = await(task);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block Integer tail for Task<Float> should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async Integer tail Float task binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_tail_lambda_for_float_return_runtime() {
    let temp_root = make_temp_project_root("lambda-int-tail-float-return-runtime");
    let source_path = temp_root.join("lambda_int_tail_float_return_runtime.arden");
    let output_path = temp_root.join("lambda_int_tail_float_return_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = () => 1;
                value: Float = f();
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("lambda Integer tail for Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled lambda Integer tail Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_integer_function_value_for_float_return_runtime() {
    let temp_root = make_temp_project_root("named-fn-int-to-float-runtime");
    let source_path = temp_root.join("named_fn_int_to_float_runtime.arden");
    let output_path = temp_root.join("named_fn_int_to_float_runtime");
    let source = r#"
            function one(): Integer {
                return 1;
            }

            function main(): Integer {
                f: () -> Float = one;
                value: Float = f();
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("named Integer function value for Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled named Integer function value Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_explicit_generic_function_value_runtime() {
    let temp_root = make_temp_project_root("explicit-generic-function-value-runtime");
    let source_path = temp_root.join("explicit_generic_function_value_runtime.arden");
    let output_path = temp_root.join("explicit_generic_function_value_runtime");
    let source = r#"
            function id<T>(x: T): T {
                return x;
            }

            function main(): Integer {
                f: (Integer) -> Integer = id<Integer>;
                return if (f(7) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("explicit generic function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
