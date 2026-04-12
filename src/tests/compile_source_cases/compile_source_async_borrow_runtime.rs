use super::*;
use std::fs;

#[test]
fn compile_source_runs_async_block_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-tail-expression-runtime");
    let source_path = temp_root.join("async_block_tail_expression_runtime.arden");
    let output_path = temp_root.join("async_block_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { 7 };
                return await(task);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_negative_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-negative-tail-expression-runtime");
    let source_path = temp_root.join("async_block_negative_tail_expression_runtime.arden");
    let output_path = temp_root.join("async_block_negative_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { -7 };
                return 10 + await(task);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block negative tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block negative tail-expression binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_binary_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-binary-tail-expression-runtime");
    let source_path = temp_root.join("async_block_binary_tail_expression_runtime.arden");
    let output_path = temp_root.join("async_block_binary_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { 2 + 5 };
                return await(task);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block binary tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block binary tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_function_value_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-function-value-tail-runtime");
    let source_path = temp_root.join("async_block_function_value_tail_runtime.arden");
    let output_path = temp_root.join("async_block_function_value_tail_runtime");
    let source = r#"
            function inc(x: Integer): Integer { return x + 1; }

            function main(): Integer {
                task: Task<(Integer) -> Integer> = async { inc };
                f: (Integer) -> Integer = await(task);
                return f(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block function-value tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block function-value tail-expression binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_unit_enum_value_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-unit-enum-tail-runtime");
    let source_path = temp_root.join("async_block_unit_enum_tail_runtime.arden");
    let output_path = temp_root.join("async_block_unit_enum_tail_runtime");
    let source = r#"
            enum E { A, B }

            function main(): Integer {
                task: Task<E> = async { E.A };
                value: E = await(task);
                match (value) {
                    E.A => { return 0; }
                    E.B => { return 1; }
                }
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block unit-enum tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block unit-enum tail-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_and_reference_async_block_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-builtin-tail-runtime");
    let source_path = temp_root.join("async_block_builtin_tail_runtime.arden");
    let output_path = temp_root.join("async_block_builtin_tail_runtime");
    let source = r#"
            import std.string.*;
            import std.io.println;

            function main(): Integer {
                some_task: Task<Option<Integer>> = async { Option.some(7) };
                none_task: Task<Option<Integer>> = async { Option.none() };
                ok_task: Task<Result<Integer, String>> = async { Result.ok(7) };
                err_task: Task<Result<Integer, String>> = async { Result.error("boom") };
                len_task: Task<Integer> = async { Str.len("abc") };
                compare_task: Task<Integer> = async { Str.compare("a", "a") };
                concat_task: Task<String> = async { Str.concat("a", "b") };
                upper_task: Task<String> = async { Str.upper("ab") };
                lower_task: Task<String> = async { Str.lower("AB") };
                trim_task: Task<String> = async { Str.trim("  ok  ") };
                contains_task: Task<Boolean> = async { Str.contains("abc", "b") };
                starts_task: Task<Boolean> = async { Str.startsWith("abc", "a") };
                ends_task: Task<Boolean> = async { Str.endsWith("abc", "c") };
                string_task: Task<String> = async { to_string(7) };
                print_task: Task<None> = async { println("hi") };
                require_task: Task<None> = async { require(true) };
                range_task: Task<Range<Integer>> = async { range(0, 3) };
                lambda_task: Task<(Integer) -> Integer> = async { (x: Integer) => x + 1 };
                if_task: Task<Integer> = async { if (true) { Str.len("abc") } else { Str.len("ab") } };
                match_task: Task<String> = async {
                    match (1) {
                        1 => { to_string(7) }
                        _ => { to_string(8) }
                    }
                };

                await(print_task);
                await(require_task);

                if (await(some_task).unwrap() != 7) { return 1; }
                if (!await(none_task).is_none()) { return 2; }
                if (await(ok_task).unwrap() != 7) { return 3; }
                if (!await(err_task).is_error()) { return 4; }
                if (await(len_task) != 3) { return 5; }
                if (await(compare_task) != 0) { return 6; }
                if (await(concat_task) != "ab") { return 7; }
                if (await(upper_task) != "AB") { return 8; }
                if (await(lower_task) != "ab") { return 9; }
                if (await(trim_task) != "ok") { return 10; }
                if (!await(contains_task)) { return 11; }
                if (!await(starts_task)) { return 12; }
                if (!await(ends_task)) { return 13; }
                if (await(string_task) != "7") { return 14; }
                if (!await(range_task).has_next()) { return 15; }
                if ((await(lambda_task))(1) != 2) { return 16; }
                if (await(if_task) != 3) { return 17; }
                if (await(match_task) != "7") { return 18; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("builtin and reference async block tail-expression paths should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin and reference async block tail-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_implicit_default_class_constructor() {
    let temp_root = make_temp_project_root("implicit-default-ctor");
    let source_path = temp_root.join("implicit_ctor.arden");
    let output_path = temp_root.join("implicit_ctor");
    let source = r#"
            class C {
                function value(): Integer { return 7; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.value();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("implicit default constructor codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_explicit_generic_method_calls() {
    let temp_root = make_temp_project_root("generic-method-codegen");
    let source_path = temp_root.join("generic_method.arden");
    let output_path = temp_root.join("generic_method");
    let source = r#"
            class C {
                function id<T>(x: T): T { return x; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.id<Integer>(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("explicit generic method codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_generic_class_instance_method_calls() {
    let temp_root = make_temp_project_root("generic-class-method-codegen");
    let source_path = temp_root.join("generic_class_method.arden");
    let output_path = temp_root.join("generic_class_method");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): None {
                b: Boxed<Integer> = Boxed<Integer>(7);
                x: Integer = b.get();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("generic class instance method codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_class_instance_methods() {
    let temp_root = make_temp_project_root("generic-class-method-runtime");
    let source_path = temp_root.join("generic_class_runtime.arden");
    let output_path = temp_root.join("generic_class_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                b: Boxed<Integer> = Boxed<Integer>(7);
                return b.get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic class runtime codegen should succeed");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic class binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_function_returned_objects() {
    let temp_root = make_temp_project_root("function-return-method-runtime");
    let source_path = temp_root.join("function_return_method_runtime.arden");
    let output_path = temp_root.join("function_return_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function make_box(): Boxed<Integer> {
                return Boxed<Integer>(9);
            }

            function main(): Integer {
                return make_box().get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method call on function return value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled function-return method binary");
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_try_unwrapped_objects() {
    let temp_root = make_temp_project_root("try-object-method-runtime");
    let source_path = temp_root.join("try_object_method_runtime.arden");
    let output_path = temp_root.join("try_object_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function choose_box(): Result<Boxed<Integer>, String> {
                return Result.ok(Boxed<Integer>(21));
            }

            function use_box(): Result<Integer, String> {
                return Result.ok(choose_box()?.get());
            }

            function main(): Integer {
                result: Result<Integer, String> = use_box();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method call on try-unwrapped object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled try-object method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_awaited_objects_without_extra_parentheses() {
    let temp_root = make_temp_project_root("await-object-method-runtime");
    let source_path = temp_root.join("await_object_method_runtime.arden");
    let output_path = temp_root.join("await_object_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            async function make_box(): Boxed<Integer> {
                return Boxed<Integer>(3);
            }

            async function run(): Integer {
                return await(make_box()).get();
            }

            function main(): Integer {
                t: Task<Integer> = run();
                return await(t);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("awaited object method chain should parse and codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled awaited-object method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_await_timeout() {
    let temp_root = make_temp_project_root("await-timeout-negative-runtime");
    let source_path = temp_root.join("await_timeout_negative_runtime.arden");
    let output_path = temp_root.join("await_timeout_negative_runtime");
    let source = r#"
            async function work(): Integer {
                return 7;
            }

            function main(): Integer {
                timeout_ms: Integer = -1;
                maybe: Option<Integer> = work().await_timeout(timeout_ms);
                if (maybe.is_some()) { return 99; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("negative await_timeout should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled negative await_timeout binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_option_unwrap_panic_message() {
    let temp_root = make_temp_project_root("option-unwrap-panic-message-runtime");
    let source_path = temp_root.join("option_unwrap_panic_message_runtime.arden");
    let output_path = temp_root.join("option_unwrap_panic_message_runtime");
    let source = r#"
            function main(): Integer {
                return Option.none().unwrap();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Option.none unwrap panic path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Option.none unwrap binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Option.unwrap() called on None\n"));
    assert!(!stdout.contains("\\n"));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_result_unwrap_panic_message() {
    let temp_root = make_temp_project_root("result-unwrap-panic-message-runtime");
    let source_path = temp_root.join("result_unwrap_panic_message_runtime.arden");
    let output_path = temp_root.join("result_unwrap_panic_message_runtime");
    let source = r#"
            function main(): Integer {
                return Result.error("boom").unwrap();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Result.error unwrap panic path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Result.error unwrap binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Result.unwrap() called on Error\n"));
    assert!(!stdout.contains("\\n"));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_integer_division_by_zero_runtime_error() {
    let temp_root = make_temp_project_root("integer-division-by-zero-runtime");
    let source_path = temp_root.join("integer_division_by_zero_runtime.arden");
    let output_path = temp_root.join("integer_division_by_zero_runtime");
    let source = r#"
            function main(): Integer {
                denominator: Integer = 0;
                return 6 / denominator;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("dynamic integer division by zero path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled integer division by zero binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Integer division by zero\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_integer_modulo_by_zero_runtime_error() {
    let temp_root = make_temp_project_root("integer-modulo-by-zero-runtime");
    let source_path = temp_root.join("integer_modulo_by_zero_runtime.arden");
    let output_path = temp_root.join("integer_modulo_by_zero_runtime");
    let source = r#"
            function main(): Integer {
                denominator: Integer = 0;
                return 6 % denominator;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("dynamic integer modulo by zero path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled integer modulo by zero binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Integer modulo by zero\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_time_sleep() {
    let temp_root = make_temp_project_root("time-sleep-negative-runtime");
    let source_path = temp_root.join("time_sleep_negative_runtime.arden");
    let output_path = temp_root.join("time_sleep_negative_runtime");
    let source = r#"
            import std.time.*;

            function main(): Integer {
                delay_ms: Integer = -1;
                Time.sleep(delay_ms);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("dynamic negative Time.sleep should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled negative Time.sleep binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Time.sleep() milliseconds must be non-negative\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_args_get_index() {
    let temp_root = make_temp_project_root("args-get-negative-runtime");
    let source_path = temp_root.join("args_get_negative_runtime.arden");
    let output_path = temp_root.join("args_get_negative_runtime");
    let source = r#"
            import std.args.*;

            function main(): Integer {
                idx: Integer = 0 - 1;
                value: String = Args.get(idx);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("dynamic negative Args.get should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled negative Args.get binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Args.get() index cannot be negative\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_out_of_bounds_args_get_index() {
    let temp_root = make_temp_project_root("args-get-oob-runtime");
    let source_path = temp_root.join("args_get_oob_runtime.arden");
    let output_path = temp_root.join("args_get_oob_runtime");
    let source = r#"
            import std.args.*;

            function main(): Integer {
                idx: Integer = Args.count() + 5;
                value: String = Args.get(idx);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("dynamic out-of-bounds Args.get should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled out-of-bounds Args.get binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Args.get() index out of bounds\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_file_read_with_nul_bytes() {
    let temp_root = make_temp_project_root("file-read-nul-byte-runtime");
    let source_path = temp_root.join("file_read_nul_byte_runtime.arden");
    let output_path = temp_root.join("file_read_nul_byte_runtime");
    let input_path = temp_root.join("payload.bin");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("payload.bin");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    fs::write(&input_path, [b'A', 0, b'B']).must("write binary payload");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.read with NUL byte payload should still codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .must("run compiled File.read NUL-byte binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("File.read() cannot load NUL bytes\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_file_read_with_invalid_utf8() {
    let temp_root = make_temp_project_root("file-read-invalid-utf8-runtime");
    let source_path = temp_root.join("file_read_invalid_utf8_runtime.arden");
    let output_path = temp_root.join("file_read_invalid_utf8_runtime");
    let input_path = temp_root.join("payload.bin");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("payload.bin");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    fs::write(&input_path, [b'A', 0xFF, b'B']).must("write invalid utf8 payload");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.read with invalid UTF-8 payload should still codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .must("run compiled File.read invalid UTF-8 binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Invalid UTF-8 sequence in String\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_reports_file_write_failure_when_flush_fails() {
    let temp_root = make_temp_project_root("file-write-dev-full-runtime");
    let source_path = temp_root.join("file_write_dev_full_runtime.arden");
    let output_path = temp_root.join("file_write_dev_full_runtime");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                ok: Boolean = File.write("/dev/full", "hello world");
                return if (ok) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.write /dev/full failure path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled File.write /dev/full binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_file_exists_false_for_directories() {
    let temp_root = make_temp_project_root("file-exists-directory-runtime");
    let source_path = temp_root.join("file_exists_directory_runtime.arden");
    let output_path = temp_root.join("file_exists_directory_runtime");
    let directory_path = temp_root.join("dir");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                return if (File.exists("dir")) { 0; } else { 1; };
            }
        "#;

    fs::create_dir_all(&directory_path).must("create directory");
    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.exists directory path should codegen");

    let status = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .status()
        .must("run compiled File.exists directory binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_file_delete_false_for_directories() {
    let temp_root = make_temp_project_root("file-delete-directory-runtime");
    let source_path = temp_root.join("file_delete_directory_runtime.arden");
    let output_path = temp_root.join("file_delete_directory_runtime");
    let directory_path = temp_root.join("dir");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                return if (File.delete("dir")) { 0; } else { 1; };
            }
        "#;

    fs::create_dir_all(&directory_path).must("create directory");
    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.delete directory path should codegen");

    let status = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .status()
        .must("run compiled File.delete directory binary");
    assert_eq!(status.code(), Some(1));
    assert!(directory_path.exists(), "directory should not be removed");

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_file_read_from_fifo() {
    let temp_root = make_temp_project_root("file-read-fifo-runtime");
    let source_path = temp_root.join("file_read_fifo_runtime.arden");
    let output_path = temp_root.join("file_read_fifo_runtime");
    let fifo_path = temp_root.join("pipe");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("pipe");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let mkfifo_status = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .must("spawn mkfifo");
    assert!(mkfifo_status.success(), "mkfifo should succeed");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.read FIFO failure path should codegen");

    let writer_fifo = fifo_path.clone();
    let writer = std::thread::spawn(move || {
        let mut handle = std::fs::OpenOptions::new()
            .write(true)
            .open(&writer_fifo)
            .must("open fifo for writing");
        use std::io::Write as _;
        match handle.write_all(b"abc") {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(err) => panic!("write fifo payload: {err}"),
        }
    });

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .must("run compiled File.read FIFO binary");
    writer.join().must("join fifo writer");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("File.read() requires a seekable regular file\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_expression_generic_constructor_branches() {
    let temp_root = make_temp_project_root("ifexpr-generic-ctor-runtime");
    let source_path = temp_root.join("ifexpr_generic_ctor_runtime.arden");
    let output_path = temp_root.join("ifexpr_generic_ctor_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function make(flag: Boolean): Boxed<Integer> {
                return if (flag) { Boxed<Integer>(1); } else { Boxed<Integer>(2); };
            }

            function main(): Integer {
                return make(true).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if-expression generic constructors should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if-expression generic constructor binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_if_expression_objects() {
    let temp_root = make_temp_project_root("ifexpr-object-method-runtime");
    let source_path = temp_root.join("ifexpr_object_method_runtime.arden");
    let output_path = temp_root.join("ifexpr_object_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                return (if (true) { Boxed<Integer>(17); } else { Boxed<Integer>(18); }).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method call on if-expression object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if-expression object binary");
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_match_expression_objects() {
    let temp_root = make_temp_project_root("match-object-field-runtime");
    let source_path = temp_root.join("match_object_field_runtime.arden");
    let output_path = temp_root.join("match_object_field_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                return (match (0) { 0 => { Boxed<Integer>(19); }, _ => { Boxed<Integer>(20); }, }).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field access on match-expression object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled match-expression object binary");
    assert_eq!(status.code(), Some(19));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_indexed_objects() {
    let temp_root = make_temp_project_root("index-object-method-runtime");
    let source_path = temp_root.join("index_object_method_runtime.arden");
    let output_path = temp_root.join("index_object_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                xs: List<Boxed<Integer>> = List<Boxed<Integer>>();
                xs.push(Boxed<Integer>(30));
                return xs[0].get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method call on indexed object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled indexed-object method binary");
    assert_eq!(status.code(), Some(30));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_indexed_objects() {
    let temp_root = make_temp_project_root("index-object-field-runtime");
    let source_path = temp_root.join("index_object_field_runtime.arden");
    let output_path = temp_root.join("index_object_field_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed<Integer>> = List<Boxed<Integer>>();
                xs.push(Boxed<Integer>(31));
                return xs[0].value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field access on indexed object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled indexed-object field binary");
    assert_eq!(status.code(), Some(31));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_read_accesses_runtime() {
    let temp_root = make_temp_project_root("borrowed-read-access-runtime");
    let source_path = temp_root.join("borrowed_read_access_runtime.arden");
    let output_path = temp_root.join("borrowed_read_access_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): Integer {
                s: String = "ab";
                xs: List<Integer> = List<Integer>();
                xs.push(40);
                m: Map<String, Integer> = Map<String, Integer>();
                m.set("k", 41);
                b: Boxed = Boxed(42);

                rs: &String = &s;
                rxs: &List<Integer> = &xs;
                rm: &Map<String, Integer> = &m;
                rb: &Boxed = &b;

                if (rb.value != 42) { return 1; }
                if (rb.get() != 42) { return 2; }
                if (rs[1] != 'b') { return 3; }
                if (rxs[0] != 40) { return 4; }
                if (rxs.get(0) != 40) { return 5; }
                if (rxs.length() != 1) { return 6; }
                if (rm["k"] != 41) { return 7; }
                if (rm.get("k") != 41) { return 8; }
                if (!rm.contains("k")) { return 9; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed read accesses should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_class_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-class-read-access-runtime");
    let source_path = temp_root.join("borrowed_class_read_access_runtime.arden");
    let output_path = temp_root.join("borrowed_class_read_access_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(42);
                rb: &Boxed = &b;
                if (rb.value != 42) { return 1; }
                if (rb.get() != 42) { return 2; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed class reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed class read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_list_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-list-read-access-runtime");
    let source_path = temp_root.join("borrowed_list_read_access_runtime.arden");
    let output_path = temp_root.join("borrowed_list_read_access_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(40);
                rxs: &List<Integer> = &xs;
                if (rxs[0] != 40) { return 1; }
                if (rxs.get(0) != 40) { return 2; }
                if (rxs.length() != 1) { return 3; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed list reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed list read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_string_index_runtime() {
    let temp_root = make_temp_project_root("borrowed-string-index-runtime");
    let source_path = temp_root.join("borrowed_string_index_runtime.arden");
    let output_path = temp_root.join("borrowed_string_index_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "ab";
                rs: &String = &s;
                return if (rs[1] == 'b') { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed string index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_map_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-map-read-access-runtime");
    let source_path = temp_root.join("borrowed_map_read_access_runtime.arden");
    let output_path = temp_root.join("borrowed_map_read_access_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<String, Integer> = Map<String, Integer>();
                m.set("k", 41);
                rm: &Map<String, Integer> = &m;
                if (rm["k"] != 41) { return 1; }
                if (rm.get("k") != 41) { return 2; }
                if (!rm.contains("k")) { return 3; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed map reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed map read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_map_object_field_reads_runtime() {
    let temp_root = make_temp_project_root("borrowed-map-object-field-reads-runtime");
    let source_path = temp_root.join("borrowed_map_object_field_reads_runtime.arden");
    let output_path = temp_root.join("borrowed_map_object_field_reads_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(41));
                rm: &Map<Integer, Boxed> = &m;
                if (rm.get(1).value != 41) { return 1; }
                if (rm[1].value != 41) { return 2; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed map object field reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed map object field reads binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_builtin_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-builtin-methods-runtime");
    let source_path = temp_root.join("mutable_borrowed_builtin_methods_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_builtin_methods_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();

                rxs: &mut List<Integer> = &mut xs;

                rxs.push(1);
                rxs.set(0, 2);
                value: Integer = rxs.pop();

                if (value != 2) { return 1; }
                if (rxs.length() != 0) { return 2; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed builtin methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed builtin methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_nested_builtin_field_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-nested-builtin-field-runtime");
    let source_path = temp_root.join("mutable_borrowed_nested_builtin_field_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_nested_builtin_field_runtime");
    let source = r#"
            class Bag {
                mut xs: List<Integer>;
                mut m: Map<String, Integer>;
                mut s: Set<Integer>;
                mut r: Range<Integer>;

                constructor() {
                    this.xs = List<Integer>();
                    this.m = Map<String, Integer>();
                    this.s = Set<Integer>();
                    this.r = range(0, 3);
                }
            }

            function main(): Integer {
                mut bag: Bag = Bag();
                rb: &mut Bag = &mut bag;

                rb.xs.push(1);
                rb.xs.set(0, 3);
                value: Integer = rb.xs.pop();
                rb.m.set("k", value);
                rb.s.add(value);
                removed: Boolean = rb.s.remove(value);
                first: Integer = rb.r.next();

                if (value != 3) { return 1; }
                if (rb.m["k"] != 3) { return 2; }
                if (!removed) { return 3; }
                if (rb.s.contains(3)) { return 4; }
                if (first != 0) { return 5; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed nested builtin field methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed nested builtin field methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_map_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                rm: &mut Map<String, Integer> = &mut m;
                rm.set("k", 7);
                return if (rm["k"] == 7 && rm.contains("k")) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed map methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed map binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_set_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-set-runtime");
    let source_path = temp_root.join("mutable_borrowed_set_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_set_runtime");
    let source = r#"
            function main(): Integer {
                mut s: Set<Integer> = Set<Integer>();
                rs: &mut Set<Integer> = &mut s;
                rs.add(9);
                removed: Boolean = rs.remove(9);
                return if (removed && !rs.contains(9) && rs.length() == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed set methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed set binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_range_runtime");
    let source = r#"
            function main(): Integer {
                mut r: Range<Integer> = range(0, 3);
                rr: &mut Range<Integer> = &mut r;
                first: Integer = rr.next();
                return if (first == 0 && rr.has_next()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed range methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed range binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_next_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-next-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_next_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_range_next_runtime");
    let source = r#"
            function main(): Integer {
                mut r: Range<Integer> = range(0, 3);
                rr: &mut Range<Integer> = &mut r;
                first: Integer = rr.next();
                if (first != 0) { return 1; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed range next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed range next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_has_next_after_next_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-has-next-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_has_next_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_range_has_next_runtime");
    let source = r#"
            function main(): Integer {
                mut r: Range<Integer> = range(0, 3);
                rr: &mut Range<Integer> = &mut r;
                rr.next();
                if (!rr.has_next()) { return 1; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_range_has_next_runtime() {
    let temp_root = make_temp_project_root("borrowed-range-has-next-runtime");
    let source_path = temp_root.join("borrowed_range_has_next_runtime.arden");
    let output_path = temp_root.join("borrowed_range_has_next_runtime");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(0, 2);
                rr: &Range<Integer> = &r;
                return if (rr.has_next()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_task_methods_runtime() {
    let temp_root = make_temp_project_root("borrowed-task-methods-runtime");
    let source_path = temp_root.join("borrowed_task_methods_runtime.arden");
    let output_path = temp_root.join("borrowed_task_methods_runtime");
    let source = r#"
            async function work(): Integer {
                return 7;
            }

            function main(): Integer {
                t: Task<Integer> = work();
                rt: &Task<Integer> = &t;
                maybe: Option<Integer> = rt.await_timeout(100);
                if (maybe.unwrap() != 7) { return 1; }
                if (!rt.is_done()) { return 2; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed task methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed task methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_task_cancel_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-task-cancel-runtime");
    let source_path = temp_root.join("mutable_borrowed_task_cancel_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_task_cancel_runtime");
    let source = r#"
            async function work(): Integer {
                return 7;
            }

            function main(): Integer {
                mut t: Task<Integer> = work();
                rt: &mut Task<Integer> = &mut t;
                rt.cancel();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed task cancel should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed task cancel binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_string_length_runtime() {
    let temp_root = make_temp_project_root("canceled-task-string-length-runtime");
    let source_path = temp_root.join("canceled_task_string_length_runtime.arden");
    let output_path = temp_root.join("canceled_task_string_length_runtime");
    let source = r#"
            import std.time.*;

            function work(): Task<String> {
                return async {
                    Time.sleep(50);
                    "hi"
                };
            }

            function main(): Integer {
                mut t: Task<String> = work();
                t.cancel();
                s: String = await(t);
                return s.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task string length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task string length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_string_equality_runtime() {
    let temp_root = make_temp_project_root("canceled-task-string-equality-runtime");
    let source_path = temp_root.join("canceled_task_string_equality_runtime.arden");
    let output_path = temp_root.join("canceled_task_string_equality_runtime");
    let source = r#"
            import std.time.*;

            function work(): Task<String> {
                return async {
                    Time.sleep(50);
                    "hi"
                };
            }

            function main(): Integer {
                mut t: Task<String> = work();
                t.cancel();
                s: String = await(t);
                return if (s == "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task string equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task string equality binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_field_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-field-runtime");
    let source_path = temp_root.join("canceled_task_object_field_runtime.arden");
    let output_path = temp_root.join("canceled_task_object_field_runtime");
    let source = r#"
            import std.time.*;

            class Boxed {
                value: Integer;
                constructor() {
                    this.value = 7;
                }
            }

            function work(): Task<Boxed> {
                return async {
                    Time.sleep(50);
                    return Boxed();
                };
            }

            function main(): Integer {
                mut t: Task<Boxed> = work();
                t.cancel();
                b: Boxed = await(t);
                return if (b.value == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task object field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task object field binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_string_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-string-method-runtime");
    let source_path = temp_root.join("canceled_task_object_string_method_runtime.arden");
    let output_path = temp_root.join("canceled_task_object_string_method_runtime");
    let source = r#"
            import std.time.*;

            class Boxed {
                name: String;
                constructor() {
                    this.name = "hi";
                }
                function len(): Integer {
                    return this.name.length();
                }
            }

            function work(): Task<Boxed> {
                return async {
                    Time.sleep(50);
                    return Boxed();
                };
            }

            function main(): Integer {
                mut t: Task<Boxed> = work();
                t.cancel();
                b: Boxed = await(t);
                return b.len();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_nested_object_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-nested-object-method-runtime");
    let source_path = temp_root.join("canceled_task_nested_object_method_runtime.arden");
    let output_path = temp_root.join("canceled_task_nested_object_method_runtime");
    let source = r#"
            import std.time.*;

            class Inner {
                value: Integer;
                constructor() {
                    this.value = 7;
                }
            }

            class Outer {
                inner: Inner;
                constructor() {
                    this.inner = Inner();
                }
                function read(): Integer {
                    return this.inner.value;
                }
            }

            function work(): Task<Outer> {
                return async {
                    Time.sleep(50);
                    return Outer();
                };
            }

            function main(): Integer {
                mut t: Task<Outer> = work();
                t.cancel();
                o: Outer = await(t);
                return o.read();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task nested object method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task nested object method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_recursive_object_string_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-recursive-object-string-runtime");
    let source_path = temp_root.join("canceled_task_recursive_object_string_runtime.arden");
    let output_path = temp_root.join("canceled_task_recursive_object_string_runtime");
    let source = r#"
            import std.time.*;

            class Node {
                name: String;
                next: Node;
                constructor() {
                    this.name = "root";
                    this.next = this;
                }
                function nested_len(): Integer {
                    return this.next.name.length();
                }
            }

            function work(): Task<Node> {
                return async {
                    Time.sleep(50);
                    return Node();
                };
            }

            function main(): Integer {
                mut t: Task<Node> = work();
                t.cancel();
                n: Node = await(t);
                return n.nested_len();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task recursive object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task recursive object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_mutually_recursive_object_string_method_runtime() {
    let temp_root =
        make_temp_project_root("canceled-task-mutually-recursive-object-string-runtime");
    let source_path =
        temp_root.join("canceled_task_mutually_recursive_object_string_runtime.arden");
    let output_path = temp_root.join("canceled_task_mutually_recursive_object_string_runtime");
    let source = r#"
            import std.time.*;

            class Left {
                label: String;
                right: Right;
                constructor() {
                    this.label = "left";
                    this.right = Right();
                }
                function right_name_len(): Integer {
                    return this.right.name.length();
                }
            }

            class Right {
                name: String;
                left: Left;
                constructor() {
                    this.name = "right";
                    this.left = Left();
                }
            }

            function work(): Task<Left> {
                return async {
                    Time.sleep(50);
                    return Left();
                };
            }

            function main(): Integer {
                mut t: Task<Left> = work();
                t.cancel();
                l: Left = await(t);
                return l.right_name_len();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task mutually recursive object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task mutually recursive object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_result_error_string_match_runtime() {
    let temp_root = make_temp_project_root("canceled-task-result-error-string-match-runtime");
    let source_path = temp_root.join("canceled_task_result_error_string_match_runtime.arden");
    let output_path = temp_root.join("canceled_task_result_error_string_match_runtime");
    let source = r#"
            import std.time.*;

            function work(): Task<Result<Integer, String>> {
                return async {
                    Time.sleep(50);
                    return Result.ok(7);
                };
            }

            function main(): Integer {
                mut t: Task<Result<Integer, String>> = work();
                t.cancel();
                r: Result<Integer, String> = await(t);
                return match (r) {
                    Ok(value) => value,
                    Error(err) => err.length(),
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task result error string match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task result error string match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_result_error_string_match_runtime() {
    let temp_root =
        make_temp_project_root("canceled-task-object-result-error-string-match-runtime");
    let source_path =
        temp_root.join("canceled_task_object_result_error_string_match_runtime.arden");
    let output_path = temp_root.join("canceled_task_object_result_error_string_match_runtime");
    let source = r#"
            import std.time.*;

            class Boxed {
                state: Result<Integer, String>;
                constructor() {
                    this.state = Result.ok(7);
                }
                function read(): Integer {
                    return match (this.state) {
                        Ok(value) => value,
                        Error(err) => err.length(),
                    };
                }
            }

            function work(): Task<Boxed> {
                return async {
                    Time.sleep(50);
                    return Boxed();
                };
            }

            function main(): Integer {
                mut t: Task<Boxed> = work();
                t.cancel();
                b: Boxed = await(t);
                return b.read();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task object result error string match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task object result error string match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_result_error_class_match_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-result-error-class-match-runtime");
    let source_path = temp_root.join("canceled_task_object_result_error_class_match_runtime.arden");
    let output_path = temp_root.join("canceled_task_object_result_error_class_match_runtime");
    let source = r#"
            import std.time.*;

            class Problem {
                message: String;
                constructor() {
                    this.message = "boom";
                }
                function len(): Integer {
                    return this.message.length();
                }
            }

            class Holder {
                state: Result<Integer, Problem>;
                constructor() {
                    this.state = Result.ok(7);
                }
                function read(): Integer {
                    return match (this.state) {
                        Ok(value) => value,
                        Error(err) => err.len(),
                    };
                }
            }

            function work(): Task<Holder> {
                return async {
                    Time.sleep(50);
                    return Holder();
                };
            }

            function main(): Integer {
                mut t: Task<Holder> = work();
                t.cancel();
                h: Holder = await(t);
                return h.read();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task object result error class match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task object result error class match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_assignment_runtime() {
    let temp_root = make_temp_project_root("block-expression-assignment-runtime");
    let source_path = temp_root.join("block_expression_assignment_runtime.arden");
    let output_path = temp_root.join("block_expression_assignment_runtime");
    let source = r#"
            function main(): Integer {
                computed: Integer = {
                    value: Integer = 2;
                    value + 3
                };
                return if (computed == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("block expression assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled block expression assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_method_receiver_runtime() {
    let temp_root = make_temp_project_root("block-expression-method-receiver-runtime");
    let source_path = temp_root.join("block_expression_method_receiver_runtime.arden");
    let output_path = temp_root.join("block_expression_method_receiver_runtime");
    let source = r#"
            class Boxed {
                name: String;
                constructor() {
                    this.name = "hi";
                }
                function len(): Integer {
                    return this.name.length();
                }
            }

            function main(): Integer {
                return if ({
                    value: Boxed = Boxed();
                    value
                }.len() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("block expression method receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled block expression method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_match_binding_receiver_runtime() {
    let temp_root = make_temp_project_root("block-expression-match-binding-receiver-runtime");
    let source_path = temp_root.join("block_expression_match_binding_receiver_runtime.arden");
    let output_path = temp_root.join("block_expression_match_binding_receiver_runtime");
    let source = r#"
            class Boxed {
                name: String;
                constructor() {
                    this.name = "hi";
                }
                function len(): Integer {
                    return this.name.length();
                }
            }

            function main(): Integer {
                return if ({
                    current: Result<Integer, Boxed> = Result.error(Boxed());
                    match (current) {
                        Ok(value) => Boxed(),
                        Error(err) => err,
                    }
                }.len() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("block expression match binding receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled block expression match binding receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_range_has_next_runtime() {
    let temp_root = make_temp_project_root("canceled-task-range-has-next-runtime");
    let source_path = temp_root.join("canceled_task_range_has_next_runtime.arden");
    let output_path = temp_root.join("canceled_task_range_has_next_runtime");
    let source = r#"
            import std.time.*;

            function work(): Task<Range<Integer>> {
                return async {
                    Time.sleep(50);
                    return range(0, 3);
                };
            }

            function main(): Integer {
                mut t: Task<Range<Integer>> = work();
                t.cancel();
                r: Range<Integer> = await(t);
                return if (r.has_next() == false) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("canceled task range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled canceled task range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_field_reference_runtime() {
    let temp_root = make_temp_project_root("borrowed-field-reference-runtime");
    let source_path = temp_root.join("borrowed_field_reference_runtime.arden");
    let output_path = temp_root.join("borrowed_field_reference_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function read_ref(r: &Integer): Integer {
                return *r;
            }

            function main(): Integer {
                b: Boxed = Boxed(9);
                rb: &Boxed = &b;
                return if (read_ref(&rb.value) == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed field reference should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed field reference binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_field_reference_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-field-reference-runtime");
    let source_path = temp_root.join("mutable_borrowed_field_reference_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_field_reference_runtime");
    let source = r#"
            class Boxed {
                mut value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function write_ref(r: &mut Integer): None {
                *r = 11;
                return None;
            }

            function main(): Integer {
                mut b: Boxed = Boxed(9);
                rb: &mut Boxed = &mut b;
                write_ref(&mut rb.value);
                return if (rb.value == 11) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed field reference should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed field reference binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
