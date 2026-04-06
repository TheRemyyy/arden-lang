#[allow(unused_imports)]
use super::*;
use crate::formatter::{self};
use std::fs;
#[cfg(not(windows))]
use std::os::unix::ffi::OsStringExt;

#[test]
fn compile_source_runs_unique_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("interface-method-dispatch-runtime");
    let source_path = temp_root.join("interface_method_dispatch_runtime.apex");
    let output_path = temp_root.join("interface_method_dispatch_runtime");
    let source = r#"
            interface Named {
                function get(): String;
            }

            class Boxed implements Named {
                value: String;
                constructor(value: String) { this.value = value; }
                function get(): String { return this.value; }
            }

            function main(): Integer {
                n: Named = Boxed("abc");
                return if (n.get().length() == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("single-implementation interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unique_interface_bound_method_value_runtime() {
    let temp_root = make_temp_project_root("interface-bound-method-value-runtime");
    let source_path = temp_root.join("interface_bound_method_value_runtime.apex");
    let output_path = temp_root.join("interface_bound_method_value_runtime");
    let source = r#"
            interface Named {
                function get(): String;
            }

            class Boxed implements Named {
                value: String;
                constructor(value: String) { this.value = value; }
                function get(): String { return this.value; }
            }

            function main(): Integer {
                n: Named = Boxed("abc");
                f: () -> String = n.get;
                return if (f().length() == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("single-implementation interface bound method value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled interface bound method value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_method_wrong_arity_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-method-wrong-arity");
    let source_path = temp_root.join("no_check_interface_method_wrong_arity.apex");
    let output_path = temp_root.join("no_check_interface_method_wrong_arity");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.read(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("interface method wrong arity should fail in codegen");
    assert!(
        err.contains("Reader.read() expects 0 argument(s), got 1"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_missing_interface_method_before_runtime() {
    let temp_root = make_temp_project_root("no-check-missing-interface-method");
    let source_path = temp_root.join("no_check_missing_interface_method.apex");
    let output_path = temp_root.join("no_check_missing_interface_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            class Other {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function missing(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing interface method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for interface 'Reader'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_missing_interface_bound_method_before_runtime() {
    let temp_root = make_temp_project_root("no-check-missing-interface-bound-method");
    let source_path = temp_root.join("no_check_missing_interface_bound_method.apex");
    let output_path = temp_root.join("no_check_missing_interface_bound_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            class Other {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function missing(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                f: () -> Integer = reader.missing;
                return f();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing interface bound method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for interface 'Reader'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_dispatch_to_non_implementor_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-non-implementor-dispatch");
    let source_path = temp_root.join("no_check_interface_non_implementor_dispatch.apex");
    let output_path = temp_root.join("no_check_interface_non_implementor_dispatch");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Other {
                function read(): Integer { return 9; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.read();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-implementor interface dispatch should fail in codegen");
    assert!(
        err.contains("Unknown interface method implementation: read"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_bound_method_from_non_implementor_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-non-implementor-bound-method");
    let source_path = temp_root.join("no_check_interface_non_implementor_bound_method.apex");
    let output_path = temp_root.join("no_check_interface_non_implementor_bound_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Other {
                function read(): Integer { return 9; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                f: () -> Integer = reader.read;
                return f();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-implementor interface bound method should fail in codegen");
    assert!(
        err.contains("Unknown interface method implementation: read"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_bound_method_function_value_signature_mismatch() {
    let temp_root = make_temp_project_root("no-check-interface-bound-method-signature-mismatch");
    let source_path = temp_root.join("no_check_interface_bound_method_signature_mismatch.apex");
    let output_path = temp_root.join("no_check_interface_bound_method_signature_mismatch");
    let source = r#"
            interface Named {
                function get(): Integer;
            }

            class Boxed implements Named {
                constructor() {}
                function get(): Integer { return 1; }
            }

            function main(): Integer {
                value: Named = Boxed();
                f: (Integer) -> Integer = value.get;
                return f(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("interface bound method signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_bound_constructor_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("generic-bound-constructor-dispatch-runtime");
    let source_path = temp_root.join("generic_bound_constructor_dispatch_runtime.apex");
    let output_path = temp_root.join("generic_bound_constructor_dispatch_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Person implements Named {
                constructor() {}
                function name(): Integer { return 1; }
            }

            class Holder<T extends Named> {
                value: T;

                constructor(value: T) {
                    require(value.name() == 1);
                    this.value = value;
                }
            }

            function main(): Integer {
                Holder<Person>(Person());
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("generic bound constructor dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run generic bound constructor dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-method-dispatch-runtime");
    let source_path = temp_root.join("multi_bound_interface_method_dispatch_runtime.apex");
    let output_path = temp_root.join("multi_bound_interface_method_dispatch_runtime");
    let source = r#"
            interface A { function a(): Integer; }
            interface B { function b(): Integer; }

            class C implements A, B {
                constructor() {}
                function a(): Integer { return 1; }
                function b(): Integer { return 2; }
            }

            function read_b<T extends A, B>(value: T): Integer {
                return value.b();
            }

            function main(): Integer {
                return if (read_b(C()) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("multi-bound interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run multi-bound interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_bound_method_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-bound-method-runtime");
    let source_path = temp_root.join("multi_bound_interface_bound_method_runtime.apex");
    let output_path = temp_root.join("multi_bound_interface_bound_method_runtime");
    let source = r#"
            interface A { function a(): Integer; }
            interface B { function b(): Integer; }

            class C implements A, B {
                constructor() {}
                function a(): Integer { return 1; }
                function b(): Integer { return 2; }
            }

            function read_b<T extends A, B>(value: T): Integer {
                f: () -> Integer = value.b;
                return f();
            }

            function main(): Integer {
                return if (read_b(C()) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("multi-bound interface bound method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run multi-bound interface bound method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_interface_implements_runtime() {
    let temp_root = make_temp_project_root("generic-interface-implements-runtime");
    let source_path = temp_root.join("generic_interface_implements_runtime.apex");
    let output_path = temp_root.join("generic_interface_implements_runtime");
    let source = r#"
            interface I<T> {
                function get(): T;
            }

            class C implements I<String> {
                function get(): String { return "ok"; }
            }

            function main(): Integer {
                i: I<String> = C();
                return if (i.get().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic interface implements clause should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic interface implements binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_specialized_parent_interface_method_runtime() {
    let temp_root = make_temp_project_root("specialized-parent-interface-runtime");
    let source_path = temp_root.join("specialized_parent_interface_runtime.apex");
    let output_path = temp_root.join("specialized_parent_interface_runtime");
    let source = r#"
            interface Reader<T> {
                function read(): T;
            }

            interface StringReader extends Reader<String> {}

            class FileReader implements StringReader {
                function read(): String { return "ok"; }
            }

            function main(): Integer {
                reader: StringReader = FileReader();
                f: () -> String = reader.read;
                return if (reader.read().length() == 2 && f().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("specialized parent interface methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled specialized parent interface binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-tail-expression-runtime");
    let source_path = temp_root.join("async_block_tail_expression_runtime.apex");
    let output_path = temp_root.join("async_block_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { 7 };
                return await(task);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_negative_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-negative-tail-expression-runtime");
    let source_path = temp_root.join("async_block_negative_tail_expression_runtime.apex");
    let output_path = temp_root.join("async_block_negative_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { -7 };
                return 10 + await(task);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block negative tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block negative tail-expression binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_binary_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-binary-tail-expression-runtime");
    let source_path = temp_root.join("async_block_binary_tail_expression_runtime.apex");
    let output_path = temp_root.join("async_block_binary_tail_expression_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async { 2 + 5 };
                return await(task);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block binary tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block binary tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_function_value_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-function-value-tail-runtime");
    let source_path = temp_root.join("async_block_function_value_tail_runtime.apex");
    let output_path = temp_root.join("async_block_function_value_tail_runtime");
    let source = r#"
            function inc(x: Integer): Integer { return x + 1; }

            function main(): Integer {
                task: Task<(Integer) -> Integer> = async { inc };
                f: (Integer) -> Integer = await(task);
                return f(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block function-value tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block function-value tail-expression binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_unit_enum_value_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-unit-enum-tail-runtime");
    let source_path = temp_root.join("async_block_unit_enum_tail_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block unit-enum tail-expression path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block unit-enum tail-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_and_reference_async_block_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-builtin-tail-runtime");
    let source_path = temp_root.join("async_block_builtin_tail_runtime.apex");
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
                lambda_task: Task<(Integer) -> Integer> = async { |x: Integer| x + 1 };
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("builtin and reference async block tail-expression paths should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin and reference async block tail-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_implicit_default_class_constructor() {
    let temp_root = make_temp_project_root("implicit-default-ctor");
    let source_path = temp_root.join("implicit_ctor.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("implicit default constructor codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_explicit_generic_method_calls() {
    let temp_root = make_temp_project_root("generic-method-codegen");
    let source_path = temp_root.join("generic_method.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("explicit generic method codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_generic_class_instance_method_calls() {
    let temp_root = make_temp_project_root("generic-class-method-codegen");
    let source_path = temp_root.join("generic_class_method.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("generic class instance method codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_class_instance_methods() {
    let temp_root = make_temp_project_root("generic-class-method-runtime");
    let source_path = temp_root.join("generic_class_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic class runtime codegen should succeed");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic class binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_function_returned_objects() {
    let temp_root = make_temp_project_root("function-return-method-runtime");
    let source_path = temp_root.join("function_return_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method call on function return value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled function-return method binary");
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_try_unwrapped_objects() {
    let temp_root = make_temp_project_root("try-object-method-runtime");
    let source_path = temp_root.join("try_object_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method call on try-unwrapped object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled try-object method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_awaited_objects_without_extra_parentheses() {
    let temp_root = make_temp_project_root("await-object-method-runtime");
    let source_path = temp_root.join("await_object_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("awaited object method chain should parse and codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled awaited-object method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_await_timeout() {
    let temp_root = make_temp_project_root("await-timeout-negative-runtime");
    let source_path = temp_root.join("await_timeout_negative_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("negative await_timeout should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled negative await_timeout binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_option_unwrap_panic_message() {
    let temp_root = make_temp_project_root("option-unwrap-panic-message-runtime");
    let source_path = temp_root.join("option_unwrap_panic_message_runtime.apex");
    let output_path = temp_root.join("option_unwrap_panic_message_runtime");
    let source = r#"
            function main(): Integer {
                return Option.none().unwrap();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Option.none unwrap panic path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Option.none unwrap binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Option.unwrap() called on None\n"));
    assert!(!stdout.contains("\\n"));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_result_unwrap_panic_message() {
    let temp_root = make_temp_project_root("result-unwrap-panic-message-runtime");
    let source_path = temp_root.join("result_unwrap_panic_message_runtime.apex");
    let output_path = temp_root.join("result_unwrap_panic_message_runtime");
    let source = r#"
            function main(): Integer {
                return Result.error("boom").unwrap();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Result.error unwrap panic path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Result.error unwrap binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Result.unwrap() called on Error\n"));
    assert!(!stdout.contains("\\n"));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_integer_division_by_zero_runtime_error() {
    let temp_root = make_temp_project_root("integer-division-by-zero-runtime");
    let source_path = temp_root.join("integer_division_by_zero_runtime.apex");
    let output_path = temp_root.join("integer_division_by_zero_runtime");
    let source = r#"
            function main(): Integer {
                denominator: Integer = 0;
                return 6 / denominator;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("dynamic integer division by zero path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled integer division by zero binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Integer division by zero\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_clean_integer_modulo_by_zero_runtime_error() {
    let temp_root = make_temp_project_root("integer-modulo-by-zero-runtime");
    let source_path = temp_root.join("integer_modulo_by_zero_runtime.apex");
    let output_path = temp_root.join("integer_modulo_by_zero_runtime");
    let source = r#"
            function main(): Integer {
                denominator: Integer = 0;
                return 6 % denominator;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("dynamic integer modulo by zero path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled integer modulo by zero binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("Integer modulo by zero\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_time_sleep() {
    let temp_root = make_temp_project_root("time-sleep-negative-runtime");
    let source_path = temp_root.join("time_sleep_negative_runtime.apex");
    let output_path = temp_root.join("time_sleep_negative_runtime");
    let source = r#"
            import std.time.*;

            function main(): Integer {
                delay_ms: Integer = -1;
                Time.sleep(delay_ms);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("dynamic negative Time.sleep should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled negative Time.sleep binary");
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
    let source_path = temp_root.join("args_get_negative_runtime.apex");
    let output_path = temp_root.join("args_get_negative_runtime");
    let source = r#"
            import std.args.*;

            function main(): Integer {
                idx: Integer = 0 - 1;
                value: String = Args.get(idx);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("dynamic negative Args.get should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled negative Args.get binary");
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
    let source_path = temp_root.join("args_get_oob_runtime.apex");
    let output_path = temp_root.join("args_get_oob_runtime");
    let source = r#"
            import std.args.*;

            function main(): Integer {
                idx: Integer = Args.count() + 5;
                value: String = Args.get(idx);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("dynamic out-of-bounds Args.get should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled out-of-bounds Args.get binary");
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
    let source_path = temp_root.join("file_read_nul_byte_runtime.apex");
    let output_path = temp_root.join("file_read_nul_byte_runtime");
    let input_path = temp_root.join("payload.bin");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("payload.bin");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    fs::write(&input_path, [b'A', 0, b'B']).expect("write binary payload");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.read with NUL byte payload should still codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .expect("run compiled File.read NUL-byte binary");
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
    let source_path = temp_root.join("file_read_invalid_utf8_runtime.apex");
    let output_path = temp_root.join("file_read_invalid_utf8_runtime");
    let input_path = temp_root.join("payload.bin");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("payload.bin");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    fs::write(&input_path, [b'A', 0xFF, b'B']).expect("write invalid utf8 payload");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.read with invalid UTF-8 payload should still codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .expect("run compiled File.read invalid UTF-8 binary");
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
    let source_path = temp_root.join("file_write_dev_full_runtime.apex");
    let output_path = temp_root.join("file_write_dev_full_runtime");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                ok: Boolean = File.write("/dev/full", "hello world");
                return if (ok) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.write /dev/full failure path should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled File.write /dev/full binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_file_exists_false_for_directories() {
    let temp_root = make_temp_project_root("file-exists-directory-runtime");
    let source_path = temp_root.join("file_exists_directory_runtime.apex");
    let output_path = temp_root.join("file_exists_directory_runtime");
    let directory_path = temp_root.join("dir");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                return if (File.exists("dir")) { 0; } else { 1; };
            }
        "#;

    fs::create_dir_all(&directory_path).expect("create directory");
    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.exists directory path should codegen");

    let status = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .status()
        .expect("run compiled File.exists directory binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_file_delete_false_for_directories() {
    let temp_root = make_temp_project_root("file-delete-directory-runtime");
    let source_path = temp_root.join("file_delete_directory_runtime.apex");
    let output_path = temp_root.join("file_delete_directory_runtime");
    let directory_path = temp_root.join("dir");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                return if (File.delete("dir")) { 0; } else { 1; };
            }
        "#;

    fs::create_dir_all(&directory_path).expect("create directory");
    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.delete directory path should codegen");

    let status = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .status()
        .expect("run compiled File.delete directory binary");
    assert_eq!(status.code(), Some(1));
    assert!(directory_path.exists(), "directory should not be removed");

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_file_read_from_fifo() {
    let temp_root = make_temp_project_root("file-read-fifo-runtime");
    let source_path = temp_root.join("file_read_fifo_runtime.apex");
    let output_path = temp_root.join("file_read_fifo_runtime");
    let fifo_path = temp_root.join("pipe");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("pipe");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let mkfifo_status = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("spawn mkfifo");
    assert!(mkfifo_status.success(), "mkfifo should succeed");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.read FIFO failure path should codegen");

    let writer_fifo = fifo_path.clone();
    let writer = std::thread::spawn(move || {
        let mut handle = std::fs::OpenOptions::new()
            .write(true)
            .open(&writer_fifo)
            .expect("open fifo for writing");
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
        .expect("run compiled File.read FIFO binary");
    writer.join().expect("join fifo writer");
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
    let source_path = temp_root.join("ifexpr_generic_ctor_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if-expression generic constructors should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if-expression generic constructor binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_if_expression_objects() {
    let temp_root = make_temp_project_root("ifexpr-object-method-runtime");
    let source_path = temp_root.join("ifexpr_object_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method call on if-expression object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if-expression object binary");
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_match_expression_objects() {
    let temp_root = make_temp_project_root("match-object-field-runtime");
    let source_path = temp_root.join("match_object_field_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field access on match-expression object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled match-expression object binary");
    assert_eq!(status.code(), Some(19));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_calls_on_indexed_objects() {
    let temp_root = make_temp_project_root("index-object-method-runtime");
    let source_path = temp_root.join("index_object_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method call on indexed object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled indexed-object method binary");
    assert_eq!(status.code(), Some(30));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_indexed_objects() {
    let temp_root = make_temp_project_root("index-object-field-runtime");
    let source_path = temp_root.join("index_object_field_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field access on indexed object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled indexed-object field binary");
    assert_eq!(status.code(), Some(31));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_read_accesses_runtime() {
    let temp_root = make_temp_project_root("borrowed-read-access-runtime");
    let source_path = temp_root.join("borrowed_read_access_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed read accesses should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_class_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-class-read-access-runtime");
    let source_path = temp_root.join("borrowed_class_read_access_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed class reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed class read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_list_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-list-read-access-runtime");
    let source_path = temp_root.join("borrowed_list_read_access_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed list reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed list read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_string_index_runtime() {
    let temp_root = make_temp_project_root("borrowed-string-index-runtime");
    let source_path = temp_root.join("borrowed_string_index_runtime.apex");
    let output_path = temp_root.join("borrowed_string_index_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "ab";
                rs: &String = &s;
                return if (rs[1] == 'b') { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed string index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_map_read_access_runtime() {
    let temp_root = make_temp_project_root("borrowed-map-read-access-runtime");
    let source_path = temp_root.join("borrowed_map_read_access_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed map reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed map read access binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_map_object_field_reads_runtime() {
    let temp_root = make_temp_project_root("borrowed-map-object-field-reads-runtime");
    let source_path = temp_root.join("borrowed_map_object_field_reads_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed map object field reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed map object field reads binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_builtin_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-builtin-methods-runtime");
    let source_path = temp_root.join("mutable_borrowed_builtin_methods_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed builtin methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed builtin methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_nested_builtin_field_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-nested-builtin-field-runtime");
    let source_path = temp_root.join("mutable_borrowed_nested_builtin_field_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed nested builtin field methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed nested builtin field methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_map_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                rm: &mut Map<String, Integer> = &mut m;
                rm.set("k", 7);
                return if (rm["k"] == 7 && rm.contains("k")) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed map methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed map binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_set_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-set-runtime");
    let source_path = temp_root.join("mutable_borrowed_set_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed set methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed set binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_methods_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_range_runtime");
    let source = r#"
            function main(): Integer {
                mut r: Range<Integer> = range(0, 3);
                rr: &mut Range<Integer> = &mut r;
                first: Integer = rr.next();
                return if (first == 0 && rr.has_next()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed range methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed range binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_next_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-next-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_next_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed range next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed range next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_range_has_next_after_next_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-range-has-next-runtime");
    let source_path = temp_root.join("mutable_borrowed_range_has_next_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_range_has_next_runtime() {
    let temp_root = make_temp_project_root("borrowed-range-has-next-runtime");
    let source_path = temp_root.join("borrowed_range_has_next_runtime.apex");
    let output_path = temp_root.join("borrowed_range_has_next_runtime");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(0, 2);
                rr: &Range<Integer> = &r;
                return if (rr.has_next()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_task_methods_runtime() {
    let temp_root = make_temp_project_root("borrowed-task-methods-runtime");
    let source_path = temp_root.join("borrowed_task_methods_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed task methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed task methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_task_cancel_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-task-cancel-runtime");
    let source_path = temp_root.join("mutable_borrowed_task_cancel_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed task cancel should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed task cancel binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_string_length_runtime() {
    let temp_root = make_temp_project_root("canceled-task-string-length-runtime");
    let source_path = temp_root.join("canceled_task_string_length_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task string length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task string length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_string_equality_runtime() {
    let temp_root = make_temp_project_root("canceled-task-string-equality-runtime");
    let source_path = temp_root.join("canceled_task_string_equality_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task string equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task string equality binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_field_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-field-runtime");
    let source_path = temp_root.join("canceled_task_object_field_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task object field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task object field binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_string_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-string-method-runtime");
    let source_path = temp_root.join("canceled_task_object_string_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_nested_object_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-nested-object-method-runtime");
    let source_path = temp_root.join("canceled_task_nested_object_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task nested object method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task nested object method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_recursive_object_string_method_runtime() {
    let temp_root = make_temp_project_root("canceled-task-recursive-object-string-runtime");
    let source_path = temp_root.join("canceled_task_recursive_object_string_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task recursive object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task recursive object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_mutually_recursive_object_string_method_runtime() {
    let temp_root =
        make_temp_project_root("canceled-task-mutually-recursive-object-string-runtime");
    let source_path = temp_root.join("canceled_task_mutually_recursive_object_string_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task mutually recursive object string method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task mutually recursive object string method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_result_error_string_match_runtime() {
    let temp_root = make_temp_project_root("canceled-task-result-error-string-match-runtime");
    let source_path = temp_root.join("canceled_task_result_error_string_match_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task result error string match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task result error string match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_result_error_string_match_runtime() {
    let temp_root =
        make_temp_project_root("canceled-task-object-result-error-string-match-runtime");
    let source_path = temp_root.join("canceled_task_object_result_error_string_match_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task object result error string match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task object result error string match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_object_result_error_class_match_runtime() {
    let temp_root = make_temp_project_root("canceled-task-object-result-error-class-match-runtime");
    let source_path = temp_root.join("canceled_task_object_result_error_class_match_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task object result error class match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task object result error class match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_assignment_runtime() {
    let temp_root = make_temp_project_root("block-expression-assignment-runtime");
    let source_path = temp_root.join("block_expression_assignment_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("block expression assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled block expression assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_method_receiver_runtime() {
    let temp_root = make_temp_project_root("block-expression-method-receiver-runtime");
    let source_path = temp_root.join("block_expression_method_receiver_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("block expression method receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled block expression method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_block_expression_match_binding_receiver_runtime() {
    let temp_root = make_temp_project_root("block-expression-match-binding-receiver-runtime");
    let source_path = temp_root.join("block_expression_match_binding_receiver_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("block expression match binding receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled block expression match binding receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_canceled_task_range_has_next_runtime() {
    let temp_root = make_temp_project_root("canceled-task-range-has-next-runtime");
    let source_path = temp_root.join("canceled_task_range_has_next_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("canceled task range has_next should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled canceled task range has_next binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_field_reference_runtime() {
    let temp_root = make_temp_project_root("borrowed-field-reference-runtime");
    let source_path = temp_root.join("borrowed_field_reference_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed field reference should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed field reference binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_field_reference_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-field-reference-runtime");
    let source_path = temp_root.join("mutable_borrowed_field_reference_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed field reference should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed field reference binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_float_list_index_arithmetic_runtime() {
    let temp_root = make_temp_project_root("borrowed-float-list-index-arithmetic-runtime");
    let source_path = temp_root.join("borrowed_float_list_index_arithmetic_runtime.apex");
    let output_path = temp_root.join("borrowed_float_list_index_arithmetic_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1.5);
                rxs: &List<Float> = &xs;
                sum: Float = rxs[0] + 1.25;
                return if (sum == 2.75) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed float list index arithmetic should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed float list arithmetic binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_float_list_index_interpolation_runtime() {
    let temp_root = make_temp_project_root("borrowed-float-list-index-interp-runtime");
    let source_path = temp_root.join("borrowed_float_list_index_interp_runtime.apex");
    let output_path = temp_root.join("borrowed_float_list_index_interp_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1.5);
                rxs: &List<Float> = &xs;
                text: String = "{rxs[0]}";
                return if (text == "1.500000") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed float list index interpolation should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed float list interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_list_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-list-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_list_index_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_list_index_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                rxs: &mut List<Integer> = &mut xs;
                rxs[0] = 2;
                return if (rxs[0] == 2 && xs[0] == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed list index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed list index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_index_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_map_index_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                rm: &mut Map<String, Integer> = &mut m;
                rm["k"] = 7;
                return if (rm["k"] == 7 && m["k"] == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed map index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed map index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_nested_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-nested-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_nested_index_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_nested_index_assignment_runtime");
    let source = r#"
            class Bag {
                mut xs: List<Integer>;
                mut m: Map<String, Integer>;

                constructor() {
                    this.xs = List<Integer>();
                    this.xs.push(1);
                    this.m = Map<String, Integer>();
                }
            }

            function main(): Integer {
                mut bag: Bag = Bag();
                rb: &mut Bag = &mut bag;
                rb.xs[0] = 3;
                rb.m["k"] = 4;
                if (rb.xs[0] != 3) { return 1; }
                if (rb.m["k"] != 4) { return 2; }
                if (bag.xs[0] != 3) { return 3; }
                if (bag.m["k"] != 4) { return 4; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed nested index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed nested index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_with_mutating_builtin_field_runtime() {
    let temp_root = make_temp_project_root("method-with-mutating-builtin-field-runtime");
    let source_path = temp_root.join("method_with_mutating_builtin_field_runtime.apex");
    let output_path = temp_root.join("method_with_mutating_builtin_field_runtime");
    let source = r#"
            class Bag {
                mut xs: List<Integer>;
                constructor() { this.xs = List<Integer>(); }
                function add_one(): None {
                    this.xs.push(1);
                    return None;
                }
            }

            function main(): Integer {
                mut bag: Bag = Bag();
                bag.add_one();
                return if (bag.xs[0] == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method with mutating builtin field should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutating builtin field method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_local_deref_assignment_runtime() {
    let temp_root = make_temp_project_root("local-deref-assignment-runtime");
    let source_path = temp_root.join("local_deref_assignment_runtime.apex");
    let output_path = temp_root.join("local_deref_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut x: Integer = 5;
                rx: &mut Integer = &mut x;
                *rx = 19;
                return if (*rx == 19) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("local deref assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled local deref assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_mutable_reference_assignment_runtime() {
    let temp_root = make_temp_project_root("direct-mutable-reference-assignment-runtime");
    let source_path = temp_root.join("direct_mutable_reference_assignment_runtime.apex");
    let output_path = temp_root.join("direct_mutable_reference_assignment_runtime");
    let source = r#"
            function write_ref(r: &mut Integer): None {
                *r = 13;
                return None;
            }

            function main(): Integer {
                mut x: Integer = 5;
                rx: &mut Integer = &mut x;
                write_ref(rx);
                return if (*rx == 13) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct mutable reference assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct mutable reference assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mutable_reference_assignment_runtime() {
    let temp_root = make_temp_project_root("inline-mutable-reference-assignment-runtime");
    let source_path = temp_root.join("inline_mutable_reference_assignment_runtime.apex");
    let output_path = temp_root.join("inline_mutable_reference_assignment_runtime");
    let source = r#"
            function write_ref(r: &mut Integer): None {
                *r = 17;
                return None;
            }

            function main(): Integer {
                mut x: Integer = 5;
                write_ref(&mut x);
                rx: &mut Integer = &mut x;
                return if (*rx == 17) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("inline mutable reference assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled inline mutable reference assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_unwrap_method_chains_on_call_results() {
    let temp_root = make_temp_project_root("option-call-unwrap-method-runtime");
    let source_path = temp_root.join("option_call_unwrap_method_runtime.apex");
    let output_path = temp_root.join("option_call_unwrap_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function choose(): Option<Boxed<Integer>> {
                return Option.some(Boxed<Integer>(32));
            }

            function main(): Integer {
                return choose().unwrap().get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("option unwrap method chain on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled option-unwrap method chain binary");
    assert_eq!(status.code(), Some(32));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_methods_on_call_results() {
    let temp_root = make_temp_project_root("list-call-method-runtime");
    let source_path = temp_root.join("list_call_method_runtime.apex");
    let output_path = temp_root.join("list_call_method_runtime");
    let source = r#"
            function make(): List<Integer> {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                return xs;
            }

            function main(): Integer {
                return make().length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list-call method binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_range_methods_on_call_results() {
    let temp_root = make_temp_project_root("range-call-method-runtime");
    let source_path = temp_root.join("range_call_method_runtime.apex");
    let output_path = temp_root.join("range_call_method_runtime");
    let source = r#"
            function mk(): Range<Integer> {
                return range(0, 10);
            }

            function main(): Integer {
                return if (mk().has_next()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("range method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled range-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_methods_on_call_results() {
    let temp_root = make_temp_project_root("set-call-method-runtime");
    let source_path = temp_root.join("set_call_method_runtime.apex");
    let output_path = temp_root.join("set_call_method_runtime");
    let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().contains(7)) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_remove_on_call_results() {
    let temp_root = make_temp_project_root("set-remove-call-method-runtime");
    let source_path = temp_root.join("set_remove_call_method_runtime.apex");
    let output_path = temp_root.join("set_remove_call_method_runtime");
    let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().remove(7)) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set remove on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-remove call binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_contains_on_option_values() {
    let temp_root = make_temp_project_root("set-option-contains-runtime");
    let source_path = temp_root.join("set_option_contains_runtime.apex");
    let output_path = temp_root.join("set_option_contains_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                s.add(Option.some(7));
                return if (s.contains(Option.some(7))) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set option contains should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-option contains binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_contains_on_result_values() {
    let temp_root = make_temp_project_root("set-result-contains-runtime");
    let source_path = temp_root.join("set_result_contains_runtime.apex");
    let output_path = temp_root.join("set_result_contains_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Result<Integer, Integer>> = Set<Result<Integer, Integer>>();
                s.add(Result.ok(7));
                return if (s.contains(Result.ok(7))) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set result contains should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-result contains binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_methods_on_call_results() {
    let temp_root = make_temp_project_root("map-call-method-runtime");
    let source_path = temp_root.join("map_call_method_runtime.apex");
    let output_path = temp_root.join("map_call_method_runtime");
    let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(1, 7);
                return m;
            }

            function main(): Integer {
                return if (build().contains(1)) { build().length(); } else { 9; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_growth_past_initial_capacity() {
    let temp_root = make_temp_project_root("map-growth-runtime");
    let source_path = temp_root.join("map_growth_runtime.apex");
    let output_path = temp_root.join("map_growth_runtime");
    let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(0, 10);
                m.set(1, 11);
                m.set(2, 12);
                m.set(3, 13);
                m.set(4, 14);
                m.set(5, 15);
                m.set(6, 16);
                m.set(7, 17);
                m.set(8, 18);
                return m;
            }

            function main(): Integer {
                m: Map<Integer, Integer> = build();
                return if (m.contains(8)) { m.get(8); } else { 99; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-growth binary");
    assert_eq!(status.code(), Some(18));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_option_growth_for_earlier_keys() {
    let temp_root = make_temp_project_root("map-option-growth-earlier-runtime");
    let source_path = temp_root.join("map_option_growth_earlier_runtime.apex");
    let output_path = temp_root.join("map_option_growth_earlier_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Option.some(0)) && m.get(Option.some(0)) == 10 && m.contains(Option.some(8)) && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map option growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-option growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_option_updates_after_growth() {
    let temp_root = make_temp_project_root("map-option-update-runtime");
    let source_path = temp_root.join("map_option_update_runtime.apex");
    let output_path = temp_root.join("map_option_update_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                m.set(Option.some(4), 99);
                return if (m.length() == 9 && m.get(Option.some(4)) == 99 && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map option update should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-option update binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_option_remove_after_growth() {
    let temp_root = make_temp_project_root("set-option-remove-runtime");
    let source_path = temp_root.join("set_option_remove_runtime.apex");
    let output_path = temp_root.join("set_option_remove_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    s.add(Option.some(i));
                    i = i + 1;
                }
                removed: Boolean = s.remove(Option.some(4));
                return if (removed && !s.contains(Option.some(4)) && s.contains(Option.some(8)) && s.length() == 8) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set option remove should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-option remove binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_result_growth_with_integer_error_keys() {
    let temp_root = make_temp_project_root("map-result-growth-runtime");
    let source_path = temp_root.join("map_result_growth_runtime.apex");
    let output_path = temp_root.join("map_result_growth_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Result<Integer, Integer>, Integer> = Map<Result<Integer, Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Result.error(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Result.error(0)) && m.get(Result.error(0)) == 10 && m.contains(Result.error(8)) && m.get(Result.error(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map result growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-result growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_result_error_contains_after_growth() {
    let temp_root = make_temp_project_root("set-result-error-growth-runtime");
    let source_path = temp_root.join("set_result_error_growth_runtime.apex");
    let output_path = temp_root.join("set_result_error_growth_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Result<Integer, Integer>> = Set<Result<Integer, Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    s.add(Result.error(i));
                    i = i + 1;
                }
                return if (s.contains(Result.error(0)) && s.contains(Result.error(8)) && !s.contains(Result.error(9))) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set result growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-result growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_nested_result_option_growth_and_updates() {
    let temp_root = make_temp_project_root("map-nested-result-option-growth-runtime");
    let source_path = temp_root.join("map_nested_result_option_growth_runtime.apex");
    let output_path = temp_root.join("map_nested_result_option_growth_runtime");
    let source = r#"
            function key(i: Integer): Result<Option<Integer>, Integer> {
                if (i % 2 == 0) {
                    return Result.ok(Option.some(i));
                }
                return Result.error(i);
            }

            function main(): Integer {
                m: Map<Result<Option<Integer>, Integer>, Result<Integer, Integer>> = Map<Result<Option<Integer>, Integer>, Result<Integer, Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(key(i), Result.ok(i + 10));
                    i = i + 1;
                }
                m.set(key(4), Result.error(99));
                a: Result<Integer, Integer> = m.get(key(0));
                b: Result<Integer, Integer> = m.get(key(4));
                c: Result<Integer, Integer> = m.get(key(7));
                return if (a == Result.ok(10) && b == Result.error(99) && c == Result.ok(17) && m.length() == 9) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested result-option map growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested result-option map binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_error_with_non_integer_ok_type() {
    let temp_root = make_temp_project_root("result-error-layout-runtime");
    let source_path = temp_root.join("result_error_layout_runtime.apex");
    let output_path = temp_root.join("result_error_layout_runtime");
    let source = r#"
            function bad(): Result<Float, String> {
                return Result.error("x");
            }

            function main(): Integer {
                r: Result<Float, String> = bad();
                return if (r.is_ok()) { 1; } else { 0; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("result error layout should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled result-error layout binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_class_pointer_keys() {
    let temp_root = make_temp_project_root("map-class-key-runtime");
    let source_path = temp_root.join("map_class_key_runtime.apex");
    let output_path = temp_root.join("map_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Boxed, Integer> = Map<Boxed, Integer>();
                m.set(a, 11);
                m.set(b, 12);
                return if (m.contains(a) && m.get(a) == 11 && m.get(b) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-class-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_with_class_pointer_keys() {
    let temp_root = make_temp_project_root("set-class-key-runtime");
    let source_path = temp_root.join("set_class_key_runtime.apex");
    let output_path = temp_root.join("set_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                s: Set<Boxed> = Set<Boxed>();
                s.add(a);
                s.add(b);
                return if (s.contains(a) && s.contains(b)) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-class-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_nested_option_class_keys() {
    let temp_root = make_temp_project_root("map-option-class-key-runtime");
    let source_path = temp_root.join("map_option_class_key_runtime.apex");
    let output_path = temp_root.join("map_option_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Option<Boxed>, Integer> = Map<Option<Boxed>, Integer>();
                m.set(Option.some(a), 11);
                m.set(Option.some(b), 12);
                return if (m.contains(Option.some(a)) && m.get(Option.some(b)) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested option class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested option class key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_multi_variant_enum_keys() {
    let temp_root = make_temp_project_root("map-enum-key-runtime");
    let source_path = temp_root.join("map_enum_key_runtime.apex");
    let output_path = temp_root.join("map_enum_key_runtime");
    let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                m: Map<E, Integer> = Map<E, Integer>();
                m.set(E.A(1), 11);
                m.set(E.B(2), 12);
                return if (m.contains(E.A(1)) && m.get(E.A(1)) == 11 && m.get(E.B(2)) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map enum key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-enum-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_with_multi_variant_enum_keys() {
    let temp_root = make_temp_project_root("set-enum-key-runtime");
    let source_path = temp_root.join("set_enum_key_runtime.apex");
    let output_path = temp_root.join("set_enum_key_runtime");
    let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                s: Set<E> = Set<E>();
                s.add(E.A(1));
                s.add(E.B(2));
                return if (s.contains(E.A(1)) && s.contains(E.B(2))) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("set enum key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled set-enum-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_is_some_in_condition() {
    let temp_root = make_temp_project_root("option-is-some-condition-runtime");
    let source_path = temp_root.join("option_is_some_condition_runtime.apex");
    let output_path = temp_root.join("option_is_some_condition_runtime");
    let source = r#"
            function choose(): Option<Integer> {
                return Option.some(1);
            }

            function main(): Integer {
                return if (choose().is_some()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("option is_some condition should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled option-is-some binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_is_ok_in_condition() {
    let temp_root = make_temp_project_root("result-is-ok-condition-runtime");
    let source_path = temp_root.join("result_is_ok_condition_runtime.apex");
    let output_path = temp_root.join("result_is_ok_condition_runtime");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function main(): Integer {
                return if (choose().is_ok()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("result is_ok condition should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled result-is-ok binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_literal_receiver() {
    let temp_root = make_temp_project_root("string-length-literal-runtime");
    let source_path = temp_root.join("string_length_literal_runtime.apex");
    let output_path = temp_root.join("string_length_literal_runtime");
    let source = r#"
            function main(): Integer {
                return "abc".length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string length on literal receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string-length literal binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_concatenation_receiver() {
    let temp_root = make_temp_project_root("string-length-concat-runtime");
    let source_path = temp_root.join("string_length_concat_runtime.apex");
    let output_path = temp_root.join("string_length_concat_runtime");
    let source = r#"
            function main(): Integer {
                return ("a" + "bc").length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string length on concatenation receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string-length concat binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_interpolation_receiver() {
    let temp_root = make_temp_project_root("string-length-interp-runtime");
    let source_path = temp_root.join("string_length_interp_runtime.apex");
    let output_path = temp_root.join("string_length_interp_runtime");
    let source = r#"
            function main(): Integer {
                return ("a{1}c").length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string length on interpolation receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string-length interpolation binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_list_get_object_results() {
    let temp_root = make_temp_project_root("list-get-object-field-runtime");
    let source_path = temp_root.join("list_get_object_field_runtime.apex");
    let output_path = temp_root.join("list_get_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                xs.push(Boxed(5));
                return xs.get(0).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field access on list.get object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list-get object field binary");
    assert_eq!(status.code(), Some(5));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_map_get_object_results() {
    let temp_root = make_temp_project_root("map-get-object-field-runtime");
    let source_path = temp_root.join("map_get_object_field_runtime.apex");
    let output_path = temp_root.join("map_get_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(6));
                return m.get(1).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field access on map.get object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map-get object field binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_missing_map_get_object_results() {
    let temp_root = make_temp_project_root("map-get-missing-object-runtime");
    let source_path = temp_root.join("map_get_missing_object_runtime.apex");
    let output_path = temp_root.join("map_get_missing_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m.get(1).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("missing map.get object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled missing map.get object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_map_index_object_results() {
    let temp_root = make_temp_project_root("map-index-object-field-runtime");
    let source_path = temp_root.join("map_index_object_field_runtime.apex");
    let output_path = temp_root.join("map_index_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(8));
                return m[1].value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field access on map index object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map index object field binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_index_assignment_with_string_keys() {
    let temp_root = make_temp_project_root("map-index-assign-runtime");
    let source_path = temp_root.join("map_index_assign_runtime.apex");
    let output_path = temp_root.join("map_index_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["x"] = 21;
                return m["x"];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map index assignment binary");
    assert_eq!(status.code(), Some(21));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_index_assignment_on_function_returned_list() {
    let temp_root = make_temp_project_root("list-index-assign-call-runtime");
    let source_path = temp_root.join("list_index_assign_call_runtime.apex");
    let output_path = temp_root.join("list_index_assign_call_runtime");
    let source = r#"
            function make(): List<Integer> {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                return xs;
            }

            function main(): Integer {
                make()[0] = 7;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list index assignment on function-returned list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_index_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("list-index-compound-assign-call-runtime");
    let source_path = temp_root.join("list_index_compound_assign_call_runtime.apex");
    let output_path = temp_root.join("list_index_compound_assign_call_runtime");
    let source = r#"
            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make(): List<Integer> {
                    this.calls += 1;
                    xs: List<Integer> = List<Integer>();
                    xs.push(1);
                    return xs;
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make()[0] += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list index compound assignment on function-returned list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("field-compound-assign-call-runtime");
    let source_path = temp_root.join("field_compound_assign_call_runtime.apex");
    let output_path = temp_root.join("field_compound_assign_call_runtime");
    let source = r#"
            class Boxed {
                mut value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make_box(): Boxed {
                    this.calls += 1;
                    return Boxed(1);
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make_box().value += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("field compound assignment on function-returned object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled field compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_index_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("map-index-compound-assign-call-runtime");
    let source_path = temp_root.join("map_index_compound_assign_call_runtime.apex");
    let output_path = temp_root.join("map_index_compound_assign_call_runtime");
    let source = r#"
            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make_map(): Map<String, Integer> {
                    this.calls += 1;
                    mut m: Map<String, Integer> = Map<String, Integer>();
                    m["k"] = 1;
                    return m;
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make_map()["k"] += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map index compound assignment on function-returned map should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_map_index_assignment_on_function_value_call_result() {
    let temp_root = make_temp_project_root("field-map-index-assign-function-value-runtime");
    let source_path = temp_root.join("field_map_index_assign_function_value_runtime.apex");
    let output_path = temp_root.join("field_map_index_assign_function_value_runtime");
    let source = r#"
            class Box {
                mut m: Map<String, Integer>;
                constructor() { this.m = Map<String, Integer>(); }
            }

            class Holder {
                make: (Integer) -> Box;
                constructor(make: (Integer) -> Box) { this.make = make; }
            }

            function build(x: Integer): Box { return Box(); }

            function main(): Integer {
                holder: Holder = Holder(build);
                holder.make(1).m["k"] = 7;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map index assignment on function-valued field call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled field map assignment function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_map_index_compound_assignment_on_function_value_call_result() {
    let temp_root =
        make_temp_project_root("field-map-index-compound-assign-function-value-runtime");
    let source_path = temp_root.join("field_map_index_compound_assign_function_value_runtime.apex");
    let output_path = temp_root.join("field_map_index_compound_assign_function_value_runtime");
    let source = r#"
            class Box {
                mut m: Map<String, Integer>;
                constructor() {
                    this.m = Map<String, Integer>();
                    this.m.set("k", 1);
                }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box { return Box(); }

            function main(): Integer {
                holder: Holder = Holder(build);
                holder.make().m["k"] += 2;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None).expect(
        "map index compound assignment on function-valued field call result should codegen",
    );

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled field map compound assignment function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_index_compound_assignment_without_double_key_evaluation() {
    let temp_root = make_temp_project_root("map-index-compound-assign-key-runtime");
    let source_path = temp_root.join("map_index_compound_assign_key_runtime.apex");
    let output_path = temp_root.join("map_index_compound_assign_key_runtime");
    let source = r#"
            class Counter {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function key(): String {
                    this.calls += 1;
                    return "k";
                }
            }

            function main(): Integer {
                mut counter: Counter = Counter();
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 1;
                m[counter.key()] += 2;
                return if (counter.calls == 1) { 0 } else { counter.calls };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map index compound assignment with key side effects should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map compound assignment key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_list_index_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-list-index-compound-runtime");
    let source_path = temp_root.join("mutable_borrowed_list_index_compound_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_list_index_compound_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                rxs: &mut List<Integer> = &mut xs;
                rxs[0] += 2;
                return if (rxs[0] == 3 && xs[0] == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed list compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed list compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_index_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-index-compound-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_index_compound_runtime.apex");
    let output_path = temp_root.join("mutable_borrowed_map_index_compound_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 1;
                rm: &mut Map<String, Integer> = &mut m;
                rm["k"] += 2;
                return if (rm["k"] == 3 && m["k"] == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable borrowed map compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable borrowed map compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mod_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mod-compound-assign-runtime");
    let source_path = temp_root.join("mod_compound_assign_runtime.apex");
    let output_path = temp_root.join("mod_compound_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut x: Integer = 17;
                x %= 5;
                return if (x == 2) { 0 } else { x };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mod compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mod compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_mod_runtime() {
    let temp_root = make_temp_project_root("float-mod-runtime");
    let source_path = temp_root.join("float_mod_runtime.apex");
    let output_path = temp_root.join("float_mod_runtime");
    let source = r#"
            function main(): Integer {
                value: Float = 5.5 % 2.0;
                return if (value == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("float modulo should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled float modulo binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_mod_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("float-mod-compound-assign-runtime");
    let source_path = temp_root.join("float_mod_compound_assign_runtime.apex");
    let output_path = temp_root.join("float_mod_compound_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut value: Float = 5.5;
                value %= 2.0;
                return if (value == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("float modulo compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled float modulo compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mod_compound_assignment_without_double_key_evaluation() {
    let temp_root = make_temp_project_root("mod-compound-assign-key-runtime");
    let source_path = temp_root.join("mod_compound_assign_key_runtime.apex");
    let output_path = temp_root.join("mod_compound_assign_key_runtime");
    let source = r#"
            class Counter {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function key(): String {
                    this.calls += 1;
                    return "k";
                }
            }

            function main(): Integer {
                mut counter: Counter = Counter();
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 9;
                m[counter.key()] %= 4;
                return if (counter.calls == 1 && m["k"] == 1) { 0 } else { counter.calls };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mod compound assignment with key side effects should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mod compound assignment key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_arithmetic_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-arithmetic-runtime");
    let source_path = temp_root.join("mixed_numeric_arithmetic_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_arithmetic_runtime");
    let source = r#"
            function main(): Integer {
                sum: Float = 1 + 2.5;
                product: Float = 3.0 * 2;
                less: Boolean = 1 < 1.5;
                greater_or_equal: Boolean = 6.0 >= 6;
                return if (sum == 3.5 && product == 6.0 && less && greater_or_equal) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric arithmetic should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric arithmetic binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_equality_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-equality-runtime");
    let source_path = temp_root.join("mixed_numeric_equality_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_equality_runtime");
    let source = r#"
            function main(): Integer {
                left_to_right: Boolean = 1 == 1.0;
                right_to_left: Boolean = 1.0 == 1;
                neq_left_to_right: Boolean = 1 != 2.0;
                neq_right_to_left: Boolean = 2.0 != 1;
                return if (left_to_right && right_to_left && neq_left_to_right && neq_right_to_left) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric equality binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_branch_and_math_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-branch-math-runtime");
    let source_path = temp_root.join("mixed_numeric_branch_math_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_branch_math_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                branch: Float = if (true) { 1 } else { 2.5 };
                min_value: Float = Math.min(1, 2.5);
                max_value: Float = Math.max(2, 1.5);
                return if (branch == 1.0 && min_value == 1.0 && max_value == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric branch and math should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric branch and math binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_match_expression_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-match-runtime");
    let source_path = temp_root.join("mixed_numeric_match_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_match_runtime");
    let source = r#"
            enum Kind {
                IntCase,
                FloatCase
            }

            function main(): Integer {
                kind: Kind = Kind.IntCase;
                value: Float = match (kind) {
                    Kind.IntCase => 1,
                    Kind.FloatCase => 2.5,
                };
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric match expression should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_assert_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-assert-runtime");
    let source_path = temp_root.join("mixed_numeric_assert_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_assert_runtime");
    let source = r#"
            function main(): Integer {
                assert_eq(1, 1.0);
                assert_eq(1.0, 1);
                assert_ne(1, 2.0);
                assert_ne(2.0, 1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric assert helpers should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric assert binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_match_literal_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-match-literal-runtime");
    let source_path = temp_root.join("mixed_numeric_match_literal_runtime.apex");
    let output_path = temp_root.join("mixed_numeric_match_literal_runtime");
    let source = r#"
            function main(): Integer {
                first: Integer = match (1.0) {
                    1 => 0,
                    _ => 1,
                };
                second: Integer = match (1) {
                    1.0 => 0,
                    _ => 2,
                };
                return if (first == 0 && second == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric match literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric match literal binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_parameter_runtime() {
    let temp_root = make_temp_project_root("int-to-float-param-runtime");
    let source_path = temp_root.join("int_to_float_param_runtime.apex");
    let output_path = temp_root.join("int_to_float_param_runtime");
    let source = r#"
            function echo(value: Float): Float {
                return value;
            }

            function main(): Integer {
                value: Float = echo(1);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer argument to float parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_method_and_constructor_runtime() {
    let temp_root = make_temp_project_root("int-to-float-method-ctor-runtime");
    let source_path = temp_root.join("int_to_float_method_ctor_runtime.apex");
    let output_path = temp_root.join("int_to_float_method_ctor_runtime");
    let source = r#"
            class Boxed {
                value: Float;
                constructor(value: Float) {
                    this.value = value;
                }
                function scale(factor: Float): Float {
                    return this.value * factor;
                }
            }

            function main(): Integer {
                box: Boxed = Boxed(2);
                scaled: Float = box.scale(3);
                return if (box.value == 2.0 && scaled == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer argument to float method and constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float method/constructor binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_function_value_runtime() {
    let temp_root = make_temp_project_root("int-to-float-function-value-runtime");
    let source_path = temp_root.join("int_to_float_function_value_runtime.apex");
    let output_path = temp_root.join("int_to_float_function_value_runtime");
    let source = r#"
            function main(): Integer {
                scale: (Float) -> Float = (value: Float) => value * 2.0;
                result: Float = scale(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer argument to float function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_container_methods_runtime() {
    let temp_root = make_temp_project_root("int-to-float-container-methods-runtime");
    let source_path = temp_root.join("int_to_float_container_methods_runtime.apex");
    let output_path = temp_root.join("int_to_float_container_methods_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1);
                xs.set(0, 4);

                m: Map<String, Float> = Map<String, Float>();
                m.set("k", 2);

                s: Set<Float> = Set<Float>();
                s.add(3);

                return if (xs[0] == 4.0 && m["k"] == 2.0 && s.contains(3.0)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer argument to float container methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float container methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_assignment_into_float_containers_runtime() {
    let temp_root = make_temp_project_root("int-to-float-container-assign-runtime");
    let source_path = temp_root.join("int_to_float_container_assign_runtime.apex");
    let output_path = temp_root.join("int_to_float_container_assign_runtime");
    let source = r#"
            class Boxed {
                mut items: List<Float>;
                constructor() {
                    this.items = List<Float>();
                    this.items.push(1.0);
                }
            }

            function main(): Integer {
                mut xs: List<Float> = List<Float>();
                xs.push(1.0);
                xs[0] = 5;

                mut m: Map<String, Float> = Map<String, Float>();
                m["k"] = 6;

                mut box: Boxed = Boxed();
                box.items[0] = 7;

                return if (xs[0] == 5.0 && m["k"] == 6.0 && box.items[0] == 7.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer assignment into float containers should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float container assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_assignment_into_float_fields_runtime() {
    let temp_root = make_temp_project_root("int-to-float-field-assign-runtime");
    let source_path = temp_root.join("int_to_float_field_assign_runtime.apex");
    let output_path = temp_root.join("int_to_float_field_assign_runtime");
    let source = r#"
            class Boxed {
                mut value: Float;
                constructor() {
                    this.value = 1;
                }
            }

            function main(): Integer {
                mut box: Boxed = Boxed();
                box.value = 2;
                return if (box.value == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer assignment into float fields should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float field assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_arguments_to_float_math_unary_runtime() {
    let temp_root = make_temp_project_root("int-to-float-math-unary-runtime");
    let source_path = temp_root.join("int_to_float_math_unary_runtime.apex");
    let output_path = temp_root.join("int_to_float_math_unary_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                floorValue: Float = Math.floor(2);
                ceilValue: Float = Math.ceil(2);
                roundValue: Float = Math.round(2);
                return if (floorValue == 2.0 && ceilValue == 2.0 && roundValue == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer arguments to float math unary functions should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled int-to-float math unary binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_integer_payloads_for_float_option_and_result() {
    let temp_root = make_temp_project_root("reject-int-to-float-option-result");
    let source_path = temp_root.join("reject_int_to_float_option_result.apex");
    let output_path = temp_root.join("reject_int_to_float_option_result");
    let source = r#"
            function main(): Integer {
                maybe: Option<Float> = Option.some(1);
                okv: Result<Float, String> = Result.ok(2);
                errv: Result<String, Float> = Result.error(3);
                errValue: Float = match (errv) {
                    Result.Error(v) => v,
                    _ => 0.0,
                };

                if (!maybe.is_some() || maybe.unwrap() != 1.0) { return 1; }
                if (!okv.is_ok() || okv.unwrap() != 2.0) { return 2; }
                if (!errv.is_error() || errValue != 3.0) { return 3; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Option/Result payloads should stay invariant across Integer/Float");
    assert!(err.contains("Type mismatch"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_invalid_to_int_and_to_float_argument_types() {
    let temp_root = make_temp_project_root("invalid-to-int-to-float-types");
    let source_path = temp_root.join("invalid_to_int_to_float_types.apex");
    let output_path = temp_root.join("invalid_to_int_to_float_types");
    let source = r#"
            function main(): Integer {
                a: Integer = to_int(true);
                b: Float = to_float("8");
                return a + to_int(b);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("invalid to_int/to_float argument types should fail");
    assert!(err.contains("to_int") || err.contains("to_float"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_to_int_conversion_runtime() {
    let temp_root = make_temp_project_root("string-to-int-runtime");
    let source_path = temp_root.join("string_to_int_runtime.apex");
    let output_path = temp_root.join("string_to_int_runtime");
    let source = r#"
            function main(): Integer {
                input: String = "100";
                value: Integer = to_int(input);
                return if (value == 100) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string to int conversion should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string to int binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_option_runtime() {
    let temp_root = make_temp_project_root("to-string-option-runtime");
    let source_path = temp_root.join("to_string_option_runtime.apex");
    let output_path = temp_root.join("to_string_option_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.some(1));
                return if (Str.compare(value, "Some(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-option-none-runtime");
    let source_path = temp_root.join("to_string_direct_option_none_runtime.apex");
    let output_path = temp_root.join("to_string_direct_option_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.none());
                return if (Str.compare(value, "None") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on direct Option.none should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string direct Option.none binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_nested_direct_option_runtime() {
    let temp_root = make_temp_project_root("to-string-nested-direct-option-runtime");
    let source_path = temp_root.join("to_string_nested_direct_option_runtime.apex");
    let output_path = temp_root.join("to_string_nested_direct_option_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.some(Option.none()));
                return if (Str.compare(value, "Some(None)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on nested direct Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string nested direct Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_result_runtime() {
    let temp_root = make_temp_project_root("to-string-result-runtime");
    let source_path = temp_root.join("to_string_result_runtime.apex");
    let output_path = temp_root.join("to_string_result_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                result: Result<Integer, String> = Result.ok(1);
                value: String = to_string(result);
                return if (Str.compare(value, "Ok(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on Result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_result_ok_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-result-ok-runtime");
    let source_path = temp_root.join("to_string_direct_result_ok_runtime.apex");
    let output_path = temp_root.join("to_string_direct_result_ok_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Result.ok(1));
                return if (Str.compare(value, "Ok(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on direct Result.ok should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string direct Result.ok binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_result_error_with_option_none_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-result-error-option-none-runtime");
    let source_path = temp_root.join("to_string_direct_result_error_option_none_runtime.apex");
    let output_path = temp_root.join("to_string_direct_result_error_option_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Result.error(Option.none()));
                return if (Str.compare(value, "Error(None)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on direct Result.error(Option.none()) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string direct Result.error(Option.none()) binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_char_runtime() {
    let temp_root = make_temp_project_root("to-string-char-runtime");
    let source_path = temp_root.join("to_string_char_runtime.apex");
    let output_path = temp_root.join("to_string_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                c: Char = 'b';
                value: String = to_string(c);
                return if (Str.compare(value, "b") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_unicode_char_runtime() {
    let temp_root = make_temp_project_root("to-string-unicode-char-runtime");
    let source_path = temp_root.join("to_string_unicode_char_runtime.apex");
    let output_path = temp_root.join("to_string_unicode_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                c: Char = '🚀';
                value: String = to_string(c);
                return if (Str.compare(value, "🚀") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string on Unicode Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string Unicode Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_str_ends_with_false_for_longer_suffix_runtime() {
    let temp_root = make_temp_project_root("str-ends-with-longer-suffix-runtime");
    let source_path = temp_root.join("str_ends_with_longer_suffix_runtime.apex");
    let output_path = temp_root.join("str_ends_with_longer_suffix_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                if (Str.endsWith("a", "abc")) { return 1; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Str.endsWith longer suffix should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Str.endsWith longer suffix binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_option_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-option-runtime");
    let source_path = temp_root.join("string_interpolation_option_runtime.apex");
    let output_path = temp_root.join("string_interpolation_option_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Option.some(1)}";
                return if (value == "Some(1)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-direct-option-none-runtime");
    let source_path = temp_root.join("string_interpolation_direct_option_none_runtime.apex");
    let output_path = temp_root.join("string_interpolation_direct_option_none_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Option.none()}";
                return if (value == "None") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on direct Option.none should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation direct Option.none binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_result_error_with_option_none_runtime() {
    let temp_root =
        make_temp_project_root("string-interpolation-direct-result-error-option-none-runtime");
    let source_path =
        temp_root.join("string_interpolation_direct_result_error_option_none_runtime.apex");
    let output_path =
        temp_root.join("string_interpolation_direct_result_error_option_none_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Result.error(Option.none())}";
                return if (value == "Error(None)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on direct Result.error(Option.none()) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation direct Result.error(Option.none()) binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_result_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-result-runtime");
    let source_path = temp_root.join("string_interpolation_result_runtime.apex");
    let output_path = temp_root.join("string_interpolation_result_runtime");
    let source = r#"
            function main(): Integer {
                result: Result<Integer, String> = Result.error("boom");
                value: String = "{result}";
                return if (value == "Error(boom)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on Result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_result_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-direct-result-runtime");
    let source_path = temp_root.join("string_interpolation_direct_result_runtime.apex");
    let output_path = temp_root.join("string_interpolation_direct_result_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Result.ok(1)}";
                return if (value == "Ok(1)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on direct Result.ok should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation direct Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_invalid_interpolation_expr() {
    let temp_root = make_temp_project_root("string-interpolation-primary-error-runtime");
    let source_path = temp_root.join("string_interpolation_primary_error_runtime.apex");
    let output_path = temp_root.join("string_interpolation_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: String = "{1 + true}";
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("invalid interpolation expression should fail typecheck");
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
    let source_path = temp_root.join("unknown_method_receiver_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_method_receiver_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope.missing();
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown method receiver should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot call method on type unknown"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_to_string_arg() {
    let temp_root = make_temp_project_root("unknown-to-string-arg-primary-error-runtime");
    let source_path = temp_root.join("unknown_to_string_arg_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_to_string_arg_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: String = to_string(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown to_string arg should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("to_string() currently supports"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_print_arg() {
    let temp_root = make_temp_project_root("unknown-print-arg-primary-error-runtime");
    let source_path = temp_root.join("unknown_print_arg_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_print_arg_primary_error_runtime");
    let source = r#"
            import std.io.print;

            function main(): None {
                print(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown print arg should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("print() currently supports"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_arithmetic_operand() {
    let temp_root = make_temp_project_root("unknown-arithmetic-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_arithmetic_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_arithmetic_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope + 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown arithmetic operand should fail typecheck");
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
    let source_path = temp_root.join("unknown_comparison_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_comparison_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = nope < 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown comparison operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Comparison requires numeric types"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_logical_operand() {
    let temp_root = make_temp_project_root("unknown-logical-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_logical_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_logical_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = nope && true;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown logical operand should fail typecheck");
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
    let source_path = temp_root.join("unknown_indexed_object_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_indexed_object_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = nope[0];
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown indexed object should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot index type unknown"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_string_index() {
    let temp_root = make_temp_project_root("unknown-string-index-primary-error-runtime");
    let source_path = temp_root.join("unknown_string_index_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_string_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Char = "hi"[nope];
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown string index should fail typecheck");
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
    let source_path = temp_root.join("unknown_list_get_index_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_list_get_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                value: Integer = xs.get(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown list get index should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("List.get() index must be Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_list_set_index() {
    let temp_root = make_temp_project_root("unknown-list-set-index-primary-error-runtime");
    let source_path = temp_root.join("unknown_list_set_index_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_list_set_index_primary_error_runtime");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                xs.set(nope, 1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown list set index should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("List.set() index must be Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_await_operand() {
    let temp_root = make_temp_project_root("unknown-await-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_await_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_await_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = await(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown await operand should fail typecheck");
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
    let source_path = temp_root.join("unknown_require_condition_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_require_condition_primary_error_runtime");
    let source = r#"
            function main(): None {
                require(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown require condition should fail typecheck");
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
    let source_path = temp_root.join("unknown_if_condition_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_if_condition_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = if (nope) { 1 } else { 2 };
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown if condition should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("If condition must be Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_range_argument() {
    let temp_root = make_temp_project_root("unknown-range-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_range_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_range_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Range<Integer> = range(nope, 3);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown range argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_exit_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_exit_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                exit(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown exit argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("exit() requires Integer code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_fail_argument() {
    let temp_root = make_temp_project_root("unknown-fail-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_fail_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_fail_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                fail(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown fail argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("fail() requires String message"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_assert_argument() {
    let temp_root = make_temp_project_root("unknown-assert-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_assert_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_assert_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown assert argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_assert_true_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_assert_true_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert_true(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown assert_true argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("assert_true() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_assert_false_argument() {
    let temp_root = make_temp_project_root("unknown-assert-false-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_assert_false_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_assert_false_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                assert_false(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown assert_false argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("assert_false() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_len_argument() {
    let temp_root = make_temp_project_root("unknown-str-len-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_len_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_len_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Integer = Str.len(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.len argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.len() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_contains_argument() {
    let temp_root = make_temp_project_root("unknown-str-contains-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_contains_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_contains_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.contains(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.contains argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_str_starts_with_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_starts_with_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.startsWith(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.startsWith argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_str_ends_with_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_ends_with_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Boolean = Str.endsWith(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.endsWith argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_str_compare_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_compare_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: Integer = Str.compare(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.compare argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_str_concat_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_concat_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.concat(nope, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.concat argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_str_upper_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_upper_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.upper(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.upper argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.upper() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_str_trim_argument() {
    let temp_root = make_temp_project_root("unknown-str-trim-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_str_trim_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_str_trim_argument_primary_error_runtime");
    let source = r#"
            import std.string.*;

            function main(): None {
                value: String = Str.trim(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Str.trim argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Str.trim() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_read_argument() {
    let temp_root = make_temp_project_root("unknown-file-read-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_read_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_file_read_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                value: String = File.read(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown File.read argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.read() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_exists_argument() {
    let temp_root = make_temp_project_root("unknown-file-exists-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_exists_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_file_exists_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                value: Boolean = File.exists(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown File.exists argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.exists() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_delete_argument() {
    let temp_root = make_temp_project_root("unknown-file-delete-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_delete_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_file_delete_argument_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                File.delete(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown File.delete argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.delete() requires String path"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_file_write_path() {
    let temp_root = make_temp_project_root("unknown-file-write-path-primary-error-runtime");
    let source_path = temp_root.join("unknown_file_write_path_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_file_write_path_primary_error_runtime");
    let source = r#"
            import std.fs.*;

            function main(): None {
                File.write(nope, "x");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown File.write path should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("File.write() path must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_await_timeout_argument() {
    let temp_root = make_temp_project_root("unknown-await-timeout-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_await_timeout_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_await_timeout_argument_primary_error_runtime");
    let source = r#"
            async function work(): Task<Integer> { return 1; }

            function main(): None {
                maybe: Option<Integer> = work().await_timeout(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown await_timeout argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_args_get_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_args_get_argument_primary_error_runtime");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Args.get argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Args.get() requires Integer index"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_time_now_argument() {
    let temp_root = make_temp_project_root("unknown-time-now-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_time_now_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_time_now_argument_primary_error_runtime");
    let source = r#"
            import std.time.*;

            function main(): None {
                value: String = Time.now(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Time.now argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Time.now() requires String format"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_time_sleep_argument() {
    let temp_root = make_temp_project_root("unknown-time-sleep-argument-primary-error-runtime");
    let source_path = temp_root.join("unknown_time_sleep_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_time_sleep_argument_primary_error_runtime");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Time.sleep argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_system_getenv_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_system_getenv_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.getenv(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown System.getenv argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_system_shell_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_system_shell_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: Integer = System.shell(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown System.shell argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_system_exec_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_system_exec_argument_primary_error_runtime");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.exec(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown System.exec argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_unary_neg_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_unary_neg_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = -nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown unary neg operand should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Cannot negate non-numeric type"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_only_primary_error_for_unknown_unary_not_operand() {
    let temp_root = make_temp_project_root("unknown-unary-not-operand-primary-error-runtime");
    let source_path = temp_root.join("unknown_unary_not_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_unary_not_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Boolean = !nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown unary not operand should fail typecheck");
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
    let source_path = temp_root.join("unknown_try_operand_primary_error_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown try operand should fail typecheck");
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
    let source_path = temp_root.join("unknown_deref_operand_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_deref_operand_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Integer = *nope;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown deref operand should fail typecheck");
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
        temp_root.join("unknown_list_constructor_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_list_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: List<Integer> = List<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown List constructor argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_map_constructor_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_map_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: Map<String, Integer> = Map<String, Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Map constructor argument should fail typecheck");
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
    let source_path = temp_root.join("unknown_set_constructor_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_set_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                items: Set<Integer> = Set<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Set constructor argument should fail typecheck");
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
        temp_root.join("unknown_option_constructor_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_option_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Option<Integer> = Option<Integer>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Option constructor argument should fail typecheck");
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
        temp_root.join("unknown_result_constructor_argument_primary_error_runtime.apex");
    let output_path = temp_root.join("unknown_result_constructor_argument_primary_error_runtime");
    let source = r#"
            function main(): None {
                value: Result<Integer, String> = Result<Integer, String>(nope);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("unknown Result constructor argument should fail typecheck");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(
        !err.contains("Constructor Result<Integer, String> expects 0 arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_list_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_list_ctor_arity.apex");
    let output_path = temp_root.join("no_check_invalid_list_ctor_arity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(1, 2);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid list constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects 0 or 1 arguments, got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_capacity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-capacity-type");
    let source_path = temp_root.join("no_check_invalid_list_capacity_type.apex");
    let output_path = temp_root.join("no_check_invalid_list_capacity_type");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>("bad");
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-integer list capacity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects optional Integer capacity, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_map_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-map-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_map_ctor_arity.apex");
    let output_path = temp_root.join("no_check_invalid_map_ctor_arity");
    let source = r#"
            function main(): Integer {
                items: Map<String, Integer> = Map<String, Integer>(1);
                return items.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid map constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor Map<String, Integer> expects 0 arguments, got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_method_receiver() {
    let temp_root = make_temp_project_root("no-check-unknown-method-receiver-primary-error");
    let source_path = temp_root.join("no_check_unknown_method_receiver_primary_error.apex");
    let output_path = temp_root.join("no_check_unknown_method_receiver_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("unknown method receiver should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");
    assert!(
        !err.contains("Cannot determine object type for method call"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_field_root() {
    let temp_root = make_temp_project_root("no-check-unknown-field-root-primary-error");
    let source_path = temp_root.join("no_check_unknown_field_root_primary_error.apex");
    let output_path = temp_root.join("no_check_unknown_field_root_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("unknown field root should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_undefined_root_before_read_or_method_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-undefined-root-read-method");
    let read_source_path = temp_root.join("no_check_nested_undefined_root_read.apex");
    let read_output_path = temp_root.join("no_check_nested_undefined_root_read");
    let read_source = r#"
            function main(): None {
                println(missing.inner.items[0]);
                return None;
            }
        "#;

    fs::write(&read_source_path, read_source).expect("write read source");
    let read_err = compile_source(
        read_source,
        &read_source_path,
        &read_output_path,
        false,
        false,
        None,
        None,
    )
    .expect_err("nested undefined-root read should fail in codegen");
    assert!(
        read_err.contains("Undefined variable: missing"),
        "{read_err}"
    );

    let method_source_path = temp_root.join("no_check_nested_undefined_root_method.apex");
    let method_output_path = temp_root.join("no_check_nested_undefined_root_method");
    let method_source = r#"
            function main(): None {
                missing.inner.items.push(1);
                return None;
            }
        "#;

    fs::write(&method_source_path, method_source).expect("write method source");
    let method_err = compile_source(
        method_source,
        &method_source_path,
        &method_output_path,
        false,
        false,
        None,
        None,
    )
    .expect_err("nested undefined-root method should fail in codegen");
    assert!(
        method_err.contains("Undefined variable: missing"),
        "{method_err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_function_for_unknown_direct_call() {
    let temp_root = make_temp_project_root("no-check-unknown-direct-call-primary-error");
    let source_path = temp_root.join("no_check_unknown_direct_call_primary_error.apex");
    let output_path = temp_root.join("no_check_unknown_direct_call_primary_error");
    let source = r#"
            function main(): Integer {
                return missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("unknown direct call should fail in codegen without checks");
    assert!(err.contains("Undefined function: missing"), "{err}");
    assert!(!err.contains("Unknown function: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_function_value() {
    let temp_root = make_temp_project_root("no-check-unknown-function-value-primary-error");
    let source_path = temp_root.join("no_check_unknown_function_value_primary_error.apex");
    let output_path = temp_root.join("no_check_unknown_function_value_primary_error");
    let source = r#"
            function main(): None {
                callback: (Integer) -> Integer = missing;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("unknown function value should fail in codegen without checks");
    assert!(err.contains("Undefined variable: missing"), "{err}");
    assert!(!err.contains("Unknown variable: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_literal_call_with_non_function_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-literal-call-non-function-type");
    let source_path = temp_root.join("no_check_literal_call_non_function_type.apex");
    let output_path = temp_root.join("no_check_literal_call_non_function_type");
    let source = r#"
            function main(): Integer {
                return 1(2);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("literal call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );
    assert!(!err.contains("Invalid callee"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_local_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-local-call-non-function-type");
    let source_path = temp_root.join("no_check_local_call_non_function_type.apex");
    let output_path = temp_root.join("no_check_local_call_non_function_type");
    let source = r#"
            function main(): Integer {
                s: String = "hi";
                return s(2);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("local non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(!err.contains("Undefined function: s"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_function_call_with_user_facing_type_diagnostic()
{
    let temp_root = make_temp_project_root("no-check-module-local-call-non-function-type");
    let source_path = temp_root.join("no_check_module_local_call_non_function_type.apex");
    let output_path = temp_root.join("no_check_module_local_call_non_function_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return M.Box(1)(2);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local non-function call should fail in codegen without checks");
    assert!(err.contains("Cannot call non-function type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_module_local_constructor_in_single_file_mode() {
    let temp_root = make_temp_project_root("no-check-module-local-constructor-runtime");
    let source_path = temp_root.join("no_check_module_local_constructor_runtime.apex");
    let output_path = temp_root.join("no_check_module_local_constructor_runtime");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            function make(): M.Box {
                return M.Box(7);
            }

            function main(): Integer {
                value: M.Box = make();
                return value.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("module-local constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_namespace_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-namespace-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_namespace_alias_ctor.apex");
    let output_path = temp_root.join("no_check_current_package_namespace_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            import app as root;

            function main(): Integer {
                value: root.M.Box = root.M.Box(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("current-package namespace alias constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled current-package namespace alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_exact_import_class_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-exact-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_exact_alias_ctor.apex");
    let output_path = temp_root.join("no_check_current_package_exact_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                value: BoxType = BoxType(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("current-package exact imported class alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled current-package exact class alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_exact_import_generic_class_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_ctor.apex");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                value: BoxType<Integer> = BoxType<Integer>(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("current-package exact imported generic class alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled current-package exact generic class alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_non_function_call_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-generic-alias-non-function");
    let source_path =
        temp_root.join("no_check_current_package_exact_generic_alias_non_function.apex");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_non_function");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Integer>(7)(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic class alias non-function call should fail");
    assert!(
        err.contains("Cannot call non-function type M.Box<Integer>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-index");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_index.apex");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Integer>(7)[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic class alias indexing should fail");
    assert!(err.contains("Cannot index type M.Box<Integer>"), "{err}");
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-println");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_println.apex");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Integer>(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic class alias println should fail");
    assert!(err.contains("got M.Box<Integer>"), "{err}");
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_list_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-list-generic-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_list_generic_alias_index.apex");
    let output_path = temp_root.join("no_check_current_package_exact_list_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<List<Integer>>(List<Integer>())[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<List<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.ListI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_option_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-option-generic-alias-println");
    let source_path =
        temp_root.join("no_check_current_package_exact_option_generic_alias_println.apex");
    let output_path = temp_root.join("no_check_current_package_exact_option_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Option<Integer>>(Option.none<Integer>()));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("option generic class alias println should fail");
    assert!(err.contains("got M.Box<Option<Integer>>"), "{err}");
    assert!(!err.contains("M.Box.spec.OptI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_map_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-map-generic-alias-index");
    let source_path = temp_root.join("no_check_current_package_exact_map_generic_alias_index.apex");
    let output_path = temp_root.join("no_check_current_package_exact_map_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<String, Integer>>(Map<String, Integer>())[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("map generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Map<String, Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.MapStr_I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_result_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-result-generic-alias-println");
    let source_path =
        temp_root.join("no_check_current_package_exact_result_generic_alias_println.apex");
    let output_path = temp_root.join("no_check_current_package_exact_result_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Result<Integer, String>>(Result.ok(7)));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("result generic class alias println should fail");
    assert!(err.contains("got M.Box<Result<Integer, String>>"), "{err}");
    assert!(!err.contains("M.Box.spec.ResI64_Str"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_function_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-function-generic-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_function_generic_alias_index.apex");
    let output_path = temp_root.join("no_check_current_package_exact_function_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function id(x: Integer): Integer {
                return x;
            }

            function main(): Integer {
                return BoxType<(Integer) -> Integer>(id)[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("function generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<(Integer) -> Integer>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.FnI64ToI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_nested_map_result_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-nested-map-result-generic-alias-index",
    );
    let source_path =
        temp_root.join("no_check_current_package_exact_nested_map_result_generic_alias_index.apex");
    let output_path =
        temp_root.join("no_check_current_package_exact_nested_map_result_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<Map<String, Integer>, Result<Integer, String>>>(
                    Map<Map<String, Integer>, Result<Integer, String>>()
                )[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("nested map/result generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Map<Map<String, Integer>, Result<Integer, String>>>"),
        "{err}"
    );
    assert!(
        !err.contains("M.Box.spec.MapMapStr_I64_ResI64_Str"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_list_function_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-list-function-generic-alias-println",
    );
    let source_path =
        temp_root.join("no_check_current_package_exact_list_function_generic_alias_println.apex");
    let output_path =
        temp_root.join("no_check_current_package_exact_list_function_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<List<(Integer) -> Integer>>(List<(Integer) -> Integer>()));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list function generic class alias println should fail");
    assert!(
        err.contains("got M.Box<List<(Integer) -> Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.ListFnI64ToI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_nested_named_generic_map_result_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-nested-named-generic-map-result-class-alias-index",
    );
    let source_path = temp_root.join(
        "no_check_current_package_exact_nested_named_generic_map_result_class_alias_index.apex",
    );
    let output_path = temp_root
        .join("no_check_current_package_exact_nested_named_generic_map_result_class_alias_index");
    let source = r#"
            package app;

            module N {
                class Inner<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<N.Inner<Integer>, Result<N.Inner<String>, String>>>(
                    Map<N.Inner<Integer>, Result<N.Inner<String>, String>>()
                )[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("nested named generic map/result class alias indexing should fail");
    assert!(
        err.contains(
            "Cannot index type M.Box<Map<N.Inner<Integer>, Result<N.Inner<String>, String>>>"
        ),
        "{err}"
    );
    assert!(!err.contains("N.InnerI64.ResGN.InnerStr"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-named-generic-payload-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_named_generic_payload_alias_index.apex");
    let output_path =
        temp_root.join("no_check_current_package_exact_named_generic_payload_alias_index");
    let source = r#"
            package app;

            module Payload {
                class Item<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Payload.Item<Integer>>(Payload.Item<Integer>(7))[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Payload.Item<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.GPayload.ItemI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_underscored_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-underscored-named-generic-payload-alias-index",
    );
    let source_path = temp_root
        .join("no_check_current_package_exact_underscored_named_generic_payload_alias_index.apex");
    let output_path = temp_root
        .join("no_check_current_package_exact_underscored_named_generic_payload_alias_index");
    let source = r#"
            package app;

            module N {
                class Inner_Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<N.Inner_Box<Integer>>(N.Inner_Box<Integer>(7))[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("underscored named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<N.Inner_Box<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("N.Inner_<Box<Integer>>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_underscored_two_arg_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-underscored-two-arg-named-generic-payload-alias-index",
    );
    let source_path = temp_root.join(
        "no_check_current_package_exact_underscored_two_arg_named_generic_payload_alias_index.apex",
    );
    let output_path = temp_root.join(
        "no_check_current_package_exact_underscored_two_arg_named_generic_payload_alias_index",
    );
    let source = r#"
            package app;

            module N {
                class Inner_Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module O {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) { this.first = first; this.second = second; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<O.Pair_Box<N.Inner_Box<Integer>, String>>(O.Pair_Box<N.Inner_Box<Integer>, String>(N.Inner_Box<Integer>(7), "x"))[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("underscored two-arg named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<O.Pair_Box<N.Inner_Box<Integer>, String>>"),
        "{err}"
    );
    assert!(
        !err.contains("O.Pair_Box<N.Inner_<Box<Integer>>, String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_field_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-field-call-non-function-type");
    let source_path = temp_root.join("no_check_field_call_non_function_type.apex");
    let output_path = temp_root.join("no_check_field_call_non_function_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): Integer {
                b: Box = Box(1);
                return b.value();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("field non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );
    assert!(
        !err.contains("Unknown method 'value' for class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_indexing_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-index-type");
    let source_path = temp_root.join("no_check_integer_index_type.apex");
    let output_path = temp_root.join("no_check_integer_index_type");
    let source = r#"
            function main(): Integer {
                value: Integer = 7;
                return value[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("integer indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type Integer"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_class_indexing_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-class-index-type");
    let source_path = temp_root.join("no_check_class_index_type.apex");
    let output_path = temp_root.join("no_check_class_index_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): Integer {
                b: Box = Box(1);
                return b[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("class indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type Box"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_class_indexing_with_user_facing_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-module-local-class-index-type");
    let source_path = temp_root.join("no_check_module_local_class_index_type.apex");
    let output_path = temp_root.join("no_check_module_local_class_index_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            function render(): Integer {
                return M.Box(1)[0];
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local class indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_index_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-index-assign-type");
    let source_path = temp_root.join("no_check_integer_index_assign_type.apex");
    let output_path = temp_root.join("no_check_integer_index_assign_type");
    let source = r#"
            function main(): None {
                mut value: Integer = 7;
                value[0] = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("integer index assignment should fail in codegen without checks");
    assert!(err.contains("Cannot index type Integer"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_class_index_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-class-index-assign-type");
    let source_path = temp_root.join("no_check_class_index_assign_type.apex");
    let output_path = temp_root.join("no_check_class_index_assign_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): None {
                mut b: Box = Box(1);
                b[0] = 2;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("class index assignment should fail in codegen without checks");
    assert!(err.contains("Cannot index type Box"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_class_index_assignment_with_user_facing_type_diagnostic(
) {
    let temp_root = make_temp_project_root("no-check-module-local-class-index-assign-type");
    let source_path = temp_root.join("no_check_module_local_class_index_assign_type.apex");
    let output_path = temp_root.join("no_check_module_local_class_index_assign_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                mut value: M.Box = M.Box(1);
                value[0] = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local class index assignment should fail in codegen");
    assert!(err.contains("Cannot index type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_deref_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-deref-type");
    let source_path = temp_root.join("no_check_integer_deref_type.apex");
    let output_path = temp_root.join("no_check_integer_deref_type");
    let source = r#"
            function main(): Integer {
                value: Integer = 7;
                return *value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("integer deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_literal_deref_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-literal-deref-type");
    let source_path = temp_root.join("no_check_literal_deref_type.apex");
    let output_path = temp_root.join("no_check_literal_deref_type");
    let source = r#"
            function main(): Integer {
                return *1;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("literal deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_deref_with_user_facing_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-module-local-deref-type");
    let source_path = temp_root.join("no_check_module_local_deref_type.apex");
    let output_path = temp_root.join("no_check_module_local_deref_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return *M.Box(1);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_deref_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-deref-assign-type");
    let source_path = temp_root.join("no_check_integer_deref_assign_type.apex");
    let output_path = temp_root.join("no_check_integer_deref_assign_type");
    let source = r#"
            function main(): None {
                mut value: Integer = 7;
                *value = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("integer deref assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_try_on_non_option_result_type() {
    let temp_root = make_temp_project_root("no-check-invalid-try-non-result-type");
    let source_path = temp_root.join("no_check_invalid_try_non_result_type.apex");
    let output_path = temp_root.join("no_check_invalid_try_non_result_type");
    let source = r#"
            function main(): None {
                value: Integer = 7;
                out: Integer = value?;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("? on Integer should fail in codegen without checks");
    assert!(
        err.contains("'?' operator can only be used on Option or Result, got Integer"),
        "{err}"
    );
    assert!(!err.contains("expected the StructValue variant"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_try_on_module_local_non_option_result_type_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-try-module-local-non-result");
    let source_path = temp_root.join("no_check_invalid_try_module_local_non_result.apex");
    let output_path = temp_root.join("no_check_invalid_try_module_local_non_result");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return M.Box(7)?;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("? on module-local Box should fail in codegen without checks");
    assert!(
        err.contains("'?' operator can only be used on Option or Result, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_box_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-box-payload-runtime");
    let source_path = temp_root.join("no_check_box_payload_runtime.apex");
    let output_path = temp_root.join("no_check_box_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<Integer>(41);
                return *value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("Box payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled no-check box payload binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_rc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-rc-payload-runtime");
    let source_path = temp_root.join("no_check_rc_payload_runtime.apex");
    let output_path = temp_root.join("no_check_rc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Rc<Integer> = Rc<Integer>(42);
                return *value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("Rc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled no-check rc payload binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_arc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-arc-payload-runtime");
    let source_path = temp_root.join("no_check_arc_payload_runtime.apex");
    let output_path = temp_root.join("no_check_arc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Arc<Integer> = Arc<Integer>(43);
                return *value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("Arc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled no-check arc payload binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assignment_to_immutable_variable_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-local-assign");
    let source_path = temp_root.join("no_check_immutable_local_assign.apex");
    let output_path = temp_root.join("no_check_immutable_local_assign");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                value = 9;
                return value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("immutable local assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign to immutable variable 'value'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assignment_through_immutable_reference_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-ref-assign");
    let source_path = temp_root.join("no_check_immutable_ref_assign.apex");
    let output_path = temp_root.join("no_check_immutable_ref_assign");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                view: &List<Integer> = &xs;
                view[0] = 7;
                return xs[0];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("immutable reference assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign through immutable reference 'view'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_deref_assignment_through_immutable_reference_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-deref-assign");
    let source_path = temp_root.join("no_check_immutable_deref_assign.apex");
    let output_path = temp_root.join("no_check_immutable_deref_assign");
    let source = r#"
            function main(): Integer {
                mut value: Integer = 1;
                r: &Integer = &value;
                *r = 9;
                return value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("immutable deref assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign through immutable reference 'r'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_function_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-function-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_function_parameter_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_function_parameter_assignment_runtime");
    let source = r#"
            function bump(mut value: Integer): Integer {
                value = value + 3;
                return value;
            }

            function main(): Integer {
                return if (bump(4) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable function parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable function parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_method_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-method-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_method_parameter_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_method_parameter_assignment_runtime");
    let source = r#"
            class Counter {
                function bump(mut value: Integer): Integer {
                    value += 5;
                    return value;
                }
            }

            function main(): Integer {
                counter: Counter = Counter();
                return if (counter.bump(2) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable method parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable method parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_constructor_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-constructor-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_constructor_parameter_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_constructor_parameter_assignment_runtime");
    let source = r#"
            class Boxed {
                value: Integer;

                constructor(mut value: Integer) {
                    value += 2;
                    this.value = value;
                }
            }

            function main(): Integer {
                boxed: Boxed = Boxed(5);
                return if (boxed.value == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable constructor parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable constructor parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_async_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-async-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_async_parameter_assignment_runtime.apex");
    let output_path = temp_root.join("mutable_async_parameter_assignment_runtime");
    let source = r#"
            async function bump(mut value: Integer): Integer {
                value = value * 2;
                return value;
            }

            function main(): Integer {
                return if (await(bump(6)) == 12) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutable async parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutable async parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_integer_boolean_addition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-bool-add");
    let source_path = temp_root.join("no_check_invalid_int_bool_add.apex");
    let output_path = temp_root.join("no_check_invalid_int_bool_add");
    let source = r#"
            function main(): Integer {
                return 1 + true;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid integer + boolean should fail in codegen without checks");
    assert!(
        err.contains("Arithmetic operator requires numeric types, got Integer and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_integer_boolean_equality_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-bool-eq");
    let source_path = temp_root.join("no_check_invalid_int_bool_eq.apex");
    let output_path = temp_root.join("no_check_invalid_int_bool_eq");
    let source = r#"
            function main(): Integer {
                return if (1 == true) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid integer == boolean should fail in codegen without checks");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_boolean_comparison_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-bool-comparison");
    let source_path = temp_root.join("no_check_invalid_bool_comparison.apex");
    let output_path = temp_root.join("no_check_invalid_bool_comparison");
    let source = r#"
            function main(): Integer {
                return if (true < false) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid boolean comparison should fail in codegen without checks");
    assert!(
        err.contains("Comparison requires numeric types, got Boolean and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_integer_logical_and_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-int-logical-and");
    let source_path = temp_root.join("no_check_invalid_int_logical_and.apex");
    let output_path = temp_root.join("no_check_invalid_int_logical_and");
    let source = r#"
            function main(): Integer {
                return if (1 && 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid integer logical and should fail in codegen without checks");
    assert!(
        err.contains("Logical operator requires Boolean types, got Integer and Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_interpolated_integer_boolean_addition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-interp-int-bool-add");
    let source_path = temp_root.join("no_check_invalid_interp_int_bool_add.apex");
    let output_path = temp_root.join("no_check_invalid_interp_int_bool_add");
    let source = r#"
            function main(): None {
                println("{1 + true}");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid interpolated integer + boolean should fail in codegen");
    assert!(
        err.contains("Arithmetic operator requires numeric types, got Integer and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_unary_negation_on_boolean_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-neg-bool");
    let source_path = temp_root.join("no_check_invalid_unary_neg_bool.apex");
    let output_path = temp_root.join("no_check_invalid_unary_neg_bool");
    let source = r#"
            function main(): Integer {
                return -true;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid unary negation on boolean should fail in codegen");
    assert!(
        err.contains("Cannot negate non-numeric type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_unary_not_on_integer_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-not-int");
    let source_path = temp_root.join("no_check_invalid_unary_not_int.apex");
    let output_path = temp_root.join("no_check_invalid_unary_not_int");
    let source = r#"
            function main(): Integer {
                return if (!1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid unary not on integer should fail in codegen");
    assert!(
        err.contains("Cannot apply '!' to non-boolean type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_invalid_unary_neg_with_user_facing_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-unary-neg-module-local-type");
    let source_path = temp_root.join("no_check_invalid_unary_neg_module_local_type.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid unary negation on module-local Box should fail in codegen");
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
    let source_path = temp_root.join("no_check_invalid_unary_not_module_local_type.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid unary not on module-local Box should fail in codegen");
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
    let source_path = temp_root.join("no_check_invalid_if_stmt_condition.apex");
    let output_path = temp_root.join("no_check_invalid_if_stmt_condition");
    let source = r#"
            function main(): Integer {
                if (1) { return 0; }
                return 1;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean if statement condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_if_expression_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-if-expr-condition");
    let source_path = temp_root.join("no_check_invalid_if_expr_condition.apex");
    let output_path = temp_root.join("no_check_invalid_if_expr_condition");
    let source = r#"
            function main(): Integer {
                return if (1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean if expression condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_while_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-while-condition");
    let source_path = temp_root.join("no_check_invalid_while_condition.apex");
    let output_path = temp_root.join("no_check_invalid_while_condition");
    let source = r#"
            function main(): Integer {
                while (1) { return 0; }
                return 1;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean while condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_require_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-condition");
    let source_path = temp_root.join("no_check_invalid_require_condition.apex");
    let output_path = temp_root.join("no_check_invalid_require_condition");
    let source = r#"
            function main(): None {
                require(1, "boom");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean require condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_require_without_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-no-msg-condition");
    let source_path = temp_root.join("no_check_invalid_require_no_msg_condition.apex");
    let output_path = temp_root.join("no_check_invalid_require_no_msg_condition");
    let source = r#"
            function main(): None {
                require(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean require condition without message should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-condition");
    let source_path = temp_root.join("no_check_invalid_assert_condition.apex");
    let output_path = temp_root.join("no_check_invalid_assert_condition");
    let source = r#"
            function main(): None {
                assert(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean assert condition should fail in codegen");
    assert!(
        err.contains("Condition must be Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_checked_rejects_non_boolean_assert_condition() {
    let temp_root = make_temp_project_root("checked-invalid-assert-condition");
    let source_path = temp_root.join("checked_invalid_assert_condition.apex");
    let output_path = temp_root.join("checked_invalid_assert_condition");
    let source = r#"
            function main(): None {
                assert(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("checked assert(Integer) should fail");
    assert!(err.contains("assert() requires boolean condition"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_true_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-true-condition");
    let source_path = temp_root.join("no_check_invalid_assert_true_condition.apex");
    let output_path = temp_root.join("no_check_invalid_assert_true_condition");
    let source = r#"
            function main(): None {
                assert_true(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean assert_true condition should fail in codegen");
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
    let source_path = temp_root.join("no_check_invalid_assert_true_module_local_type.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("assert_true on module-local Box should fail in codegen");
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
    let source_path = temp_root.join("checked_invalid_assert_true_condition.apex");
    let output_path = temp_root.join("checked_invalid_assert_true_condition");
    let source = r#"
            function main(): None {
                assert_true(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("checked assert_true(Integer) should fail");
    assert!(err.contains("assert_true() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_boolean_assert_false_condition_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-false-condition");
    let source_path = temp_root.join("no_check_invalid_assert_false_condition.apex");
    let output_path = temp_root.join("no_check_invalid_assert_false_condition");
    let source = r#"
            function main(): None {
                assert_false(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("non-boolean assert_false condition should fail in codegen");
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
    let source_path = temp_root.join("no_check_invalid_assert_false_module_local_type.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("assert_false on module-local Box should fail in codegen");
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
    let source_path = temp_root.join("checked_invalid_assert_false_condition.apex");
    let output_path = temp_root.join("checked_invalid_assert_false_condition");
    let source = r#"
            function main(): None {
                assert_false(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("checked assert_false(Integer) should fail");
    assert!(err.contains("assert_false() requires boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_eq_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-eq-types");
    let source_path = temp_root.join("no_check_invalid_assert_eq_types.apex");
    let output_path = temp_root.join("no_check_invalid_assert_eq_types");
    let source = r#"
            function main(): None {
                assert_eq(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("assert_eq on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_ne_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-ne-types");
    let source_path = temp_root.join("no_check_invalid_assert_ne_types.apex");
    let output_path = temp_root.join("no_check_invalid_assert_ne_types");
    let source = r#"
            function main(): None {
                assert_ne(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("assert_ne on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_string_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-string-index-type");
    let source_path = temp_root.join("no_check_invalid_string_index_type.apex");
    let output_path = temp_root.join("no_check_invalid_string_index_type");
    let source = r#"
            function main(): Integer {
                ch: Char = "hi"[true];
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("string[Boolean] should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-type");
    let source_path = temp_root.join("no_check_invalid_list_index_type.apex");
    let output_path = temp_root.join("no_check_invalid_list_index_type");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs[true];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list[Boolean] should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_integer_list_index_with_user_facing_type_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-list-index-module-local-type");
    let source_path = temp_root.join("no_check_invalid_list_index_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_list_index_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                return xs[M.Box(7)];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list index with module-local Box should fail in codegen");
    assert!(err.contains("Index must be Integer, found M.Box"), "{err}");
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_index_assignment_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-assignment-type");
    let source_path = temp_root.join("no_check_invalid_list_index_assignment_type.apex");
    let output_path = temp_root.join("no_check_invalid_list_index_assignment_type");
    let source = r#"
            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs[true] = 20;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list[Boolean] assignment should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_for_loop_sugar_iterable_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-for-loop-sugar-iterable");
    let source_path = temp_root.join("no_check_invalid_for_loop_sugar_iterable.apex");
    let output_path = temp_root.join("no_check_invalid_for_loop_sugar_iterable");
    let source = r#"
            function main(): Integer {
                mut total: Integer = 0;
                for (i in true) {
                    total = total + i;
                }
                return total;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("for-loop sugar over Boolean should fail in codegen");
    assert!(err.contains("Cannot iterate over Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_integer_for_loop_sugar_iterable_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-for-loop-sugar-module-local-iterable");
    let source_path = temp_root.join("no_check_invalid_for_loop_sugar_module_local_iterable.apex");
    let output_path = temp_root.join("no_check_invalid_for_loop_sugar_module_local_iterable");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                for (i in M.Box(7)) {
                }
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("for-loop sugar over module-local Box should fail in codegen");
    assert!(err.contains("Cannot iterate over M.Box"), "{err}");
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_range_argument_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-argument-types");
    let source_path = temp_root.join("no_check_invalid_range_argument_types.apex");
    let output_path = temp_root.join("no_check_invalid_range_argument_types");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(true, 3);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("range(Boolean, Integer) should fail in codegen");
    assert!(
        err.contains("range() arguments must be all Integer or all Float"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_exit_code_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-exit-code-type");
    let source_path = temp_root.join("no_check_invalid_exit_code_type.apex");
    let output_path = temp_root.join("no_check_invalid_exit_code_type");
    let source = r#"
            function main(): None {
                exit(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("exit(Boolean) should fail in codegen");
    assert!(err.contains("exit() requires Integer code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_time_sleep_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-sleep-type");
    let source_path = temp_root.join("no_check_invalid_time_sleep_type.apex");
    let output_path = temp_root.join("no_check_invalid_time_sleep_type");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Time.sleep(Boolean) should fail in codegen");
    assert!(
        err.contains("Time.sleep(ms) requires Integer milliseconds"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_args_get_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-args-get-index-type");
    let source_path = temp_root.join("no_check_invalid_args_get_index_type.apex");
    let output_path = temp_root.join("no_check_invalid_args_get_index_type");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Args.get(Boolean) should fail in codegen");
    assert!(err.contains("Args.get() requires Integer index"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_shell_command_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-shell-command-type");
    let source_path = temp_root.join("no_check_invalid_system_shell_command_type.apex");
    let output_path = temp_root.join("no_check_invalid_system_shell_command_type");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                return System.shell(true);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("System.shell(Boolean) should fail in codegen");
    assert!(
        err.contains("System.shell() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_exists_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-exists-path-type");
    let source_path = temp_root.join("no_check_invalid_file_exists_path_type.apex");
    let output_path = temp_root.join("no_check_invalid_file_exists_path_type");
    let source = r#"
            import std.file.*;

            function main(): Integer {
                return if (File.exists(true)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.exists(Boolean) should fail in codegen");
    assert!(
        err.contains("File.exists() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_exists_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-exists-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_exists_module_local_path.apex");
    let output_path = temp_root.join("no_check_invalid_file_exists_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return if (File.exists(M.Box(7))) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.exists(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.exists() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_read_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-read-path-type");
    let source_path = temp_root.join("no_check_invalid_file_read_path_type.apex");
    let output_path = temp_root.join("no_check_invalid_file_read_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                value: String = File.read(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.read(Boolean) should fail in codegen");
    assert!(
        err.contains("File.read() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_read_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-read-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_read_module_local_path.apex");
    let output_path = temp_root.join("no_check_invalid_file_read_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return File.read(M.Box(7));
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.read(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.read() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_delete_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-delete-path-type");
    let source_path = temp_root.join("no_check_invalid_file_delete_path_type.apex");
    let output_path = temp_root.join("no_check_invalid_file_delete_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.delete(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.delete(Boolean) should fail in codegen");
    assert!(
        err.contains("File.delete() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_delete_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-delete-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_delete_module_local_path.apex");
    let output_path = temp_root.join("no_check_invalid_file_delete_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                File.delete(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.delete(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.delete() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_write_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-write-path-type");
    let source_path = temp_root.join("no_check_invalid_file_write_path_type.apex");
    let output_path = temp_root.join("no_check_invalid_file_write_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.write(true, "ok");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.write(Boolean, String) should fail in codegen");
    assert!(err.contains("File.write() path must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_write_content_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-write-content-type");
    let source_path = temp_root.join("no_check_invalid_file_write_content_type.apex");
    let output_path = temp_root.join("no_check_invalid_file_write_content_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.write("ok.txt", true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("File.write(String, Boolean) should fail in codegen");
    assert!(err.contains("File.write() content must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_time_now_format_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-now-format-type");
    let source_path = temp_root.join("no_check_invalid_time_now_format_type.apex");
    let output_path = temp_root.join("no_check_invalid_time_now_format_type");
    let source = r#"
            import std.time.*;

            function main(): None {
                value: String = Time.now(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Time.now(Boolean) should fail in codegen");
    assert!(err.contains("Time.now() requires String format"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_getenv_name_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-getenv-name-type");
    let source_path = temp_root.join("no_check_invalid_system_getenv_name_type.apex");
    let output_path = temp_root.join("no_check_invalid_system_getenv_name_type");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.getenv(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("System.getenv(Boolean) should fail in codegen");
    assert!(
        err.contains("System.getenv() requires String name"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_exec_command_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-exec-command-type");
    let source_path = temp_root.join("no_check_invalid_system_exec_command_type.apex");
    let output_path = temp_root.join("no_check_invalid_system_exec_command_type");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.exec(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("System.exec(Boolean) should fail in codegen");
    assert!(
        err.contains("System.exec() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_fail_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-fail-message-type");
    let source_path = temp_root.join("no_check_invalid_fail_message_type.apex");
    let output_path = temp_root.join("no_check_invalid_fail_message_type");
    let source = r#"
            function main(): None {
                fail(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("fail(Boolean) should fail in codegen");
    assert!(err.contains("fail() requires String message"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_require_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-message-type");
    let source_path = temp_root.join("no_check_invalid_require_message_type.apex");
    let output_path = temp_root.join("no_check_invalid_require_message_type");
    let source = r#"
            function main(): None {
                require(false, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("require(Boolean, Boolean) should fail in codegen");
    assert!(
        err.contains("require() message must be String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_require_message_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-require-module-local-message");
    let source_path = temp_root.join("no_check_invalid_require_module_local_message.apex");
    let output_path = temp_root.join("no_check_invalid_require_module_local_message");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                require(true, M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("require(Boolean, module-local Box) should fail in codegen");
    assert!(
        err.contains("require() message must be String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_len_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-len-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_len_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_len_argument_type");
    let source = r#"
            import std.str.*;

            function main(): Integer {
                return Str.len(true);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.len(Boolean) should fail in codegen");
    assert!(
        err.contains("Str.len() requires String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_str_len_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-str-len-module-local-type");
    let source_path = temp_root.join("no_check_invalid_str_len_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_len_module_local_type");
    let source = r#"
            import std.str.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return Str.len(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.len(module-local Box) should fail in codegen");
    assert!(
        err.contains("Str.len() requires String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_compare_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-compare-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_compare_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_compare_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Integer = Str.compare(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.compare(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.compare() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_concat_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-concat-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_concat_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_concat_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.concat(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.concat(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.concat() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_upper_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-upper-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_upper_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_upper_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.upper(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.upper(Boolean) should fail in codegen");
    assert!(err.contains("Str.upper() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_lower_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-lower-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_lower_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_lower_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.lower(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.lower(Boolean) should fail in codegen");
    assert!(err.contains("Str.lower() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_trim_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-trim-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_trim_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_trim_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.trim(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.trim(Boolean) should fail in codegen");
    assert!(err.contains("Str.trim() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_contains_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-contains-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_contains_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_contains_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.contains(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.contains(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.contains() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_starts_with_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-starts-with-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_starts_with_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_starts_with_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.startsWith(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.startsWith(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.startsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_ends_with_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-ends-with-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_ends_with_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_str_ends_with_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.endsWith(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Str.endsWith(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.endsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_numeric_to_float_string_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-string-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_float_string_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_to_float_string_argument_type");
    let source = r#"
            function main(): Float {
                return to_float("8");
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_float(String) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_numeric_to_float_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-boolean-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_float_boolean_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_to_float_boolean_argument_type");
    let source = r#"
            function main(): Float {
                return to_float(true);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_float(Boolean) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_numeric_to_float_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-module-local-type");
    let source_path = temp_root.join("no_check_invalid_to_float_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_to_float_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Float {
                return to_float(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_float(module-local Box) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_supported_to_int_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-int-boolean-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_int_boolean_argument_type.apex");
    let output_path = temp_root.join("no_check_invalid_to_int_boolean_argument_type");
    let source = r#"
            function main(): Integer {
                return to_int(true);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_int(Boolean) should fail in codegen");
    assert!(
        err.contains("to_int() requires Integer, Float, or String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_supported_to_int_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-int-module-local-type");
    let source_path = temp_root.join("no_check_invalid_to_int_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_to_int_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return to_int(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_int(module-local Box) should fail in codegen");
    assert!(
        err.contains("to_int() requires Integer, Float, or String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_zero_range_step_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-zero-step");
    let source_path = temp_root.join("no_check_invalid_range_zero_step.apex");
    let output_path = temp_root.join("no_check_invalid_range_zero_step");
    let source = r#"
            function main(): Integer {
                value: Range<Integer> = range(0, 10, 0);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("range(..., 0) should fail in codegen");
    assert!(err.contains("range() step cannot be 0"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_time_sleep_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-sleep-negative-constant");
    let source_path = temp_root.join("no_check_invalid_time_sleep_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_time_sleep_negative_constant");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Time.sleep(-1) should fail in codegen");
    assert!(
        err.contains("Time.sleep() milliseconds must be non-negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_args_get_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-args-get-negative-constant");
    let source_path = temp_root.join("no_check_invalid_args_get_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_args_get_negative_constant");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Args.get(-1) should fail in codegen");
    assert!(err.contains("Args.get() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_await_timeout_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-timeout-negative-constant");
    let source_path = temp_root.join("no_check_invalid_await_timeout_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_await_timeout_negative_constant");
    let source = r#"
            async function work(): Task<Integer> {
                return 1;
            }

            function main(): None {
                value: Option<Integer> = work().await_timeout(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("await_timeout(-1) should fail in codegen");
    assert!(
        err.contains("Task.await_timeout() timeout must be non-negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_string_literal_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-string-literal");
    let source_path = temp_root.join("no_check_invalid_await_string_literal.apex");
    let output_path = temp_root.join("no_check_invalid_await_string_literal");
    let source = r#"
            function main(): String {
                return await "hi";
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("await on String literal should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_string_local_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-string-local");
    let source_path = temp_root.join("no_check_invalid_await_string_local.apex");
    let output_path = temp_root.join("no_check_invalid_await_string_local");
    let source = r#"
            function main(): String {
                value: String = "hi";
                return await value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("await on String local should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_box_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-box");
    let source_path = temp_root.join("no_check_invalid_await_box.apex");
    let output_path = temp_root.join("no_check_invalid_await_box");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<Integer>(7);
                return await value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("await on Box<Integer> should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got Box<Integer>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_module_local_box_with_user_facing_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-await-module-local-box");
    let source_path = temp_root.join("no_check_invalid_await_module_local_box.apex");
    let output_path = temp_root.join("no_check_invalid_await_module_local_box");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return await M.Box(7);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("await on module-local Box should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_println_on_unsupported_display_type_with_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-println-unsupported-display");
    let source_path = temp_root.join("no_check_invalid_println_unsupported_display.apex");
    let output_path = temp_root.join("no_check_invalid_println_unsupported_display");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): None {
                b: Box = Box(7);
                println(b);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("println(Box) should fail in codegen");
    assert!(
            err.contains(
                "println() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got Box"
            ),
            "{err}"
        );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interpolation_on_unsupported_display_type_with_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-interpolation-unsupported-display");
    let source_path = temp_root.join("no_check_invalid_interpolation_unsupported_display.apex");
    let output_path = temp_root.join("no_check_invalid_interpolation_unsupported_display");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function render(): String {
                b: Box = Box(7);
                return "box={b}";
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("string interpolation on Box should fail in codegen");
    assert!(
            err.contains(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got Box"
            ),
            "{err}"
        );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interpolation_on_module_local_unsupported_display_type_with_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-interpolation-module-local-display");
    let source_path = temp_root.join("no_check_invalid_interpolation_module_local_display.apex");
    let output_path = temp_root.join("no_check_invalid_interpolation_module_local_display");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return "box={M.Box(7)}";
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("string interpolation on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_println_on_module_local_unsupported_display_type_with_type_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-println-module-local-display");
    let source_path = temp_root.join("no_check_invalid_println_module_local_display.apex");
    let output_path = temp_root.join("no_check_invalid_println_module_local_display");
    let source = r#"
            import std.io.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                println(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("println on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "println() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_to_string_on_module_local_unsupported_display_type_with_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-string-module-local-display");
    let source_path = temp_root.join("no_check_invalid_to_string_module_local_display.apex");
    let output_path = temp_root.join("no_check_invalid_to_string_module_local_display");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return to_string(M.Box(7));
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("to_string on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "to_string() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_index_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_index_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_list_index_negative_constant");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs[-1];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("xs[-1] should fail in codegen");
    assert!(err.contains("List index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_string_index_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-string-index-negative-constant");
    let source_path = temp_root.join("no_check_invalid_string_index_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_string_index_negative_constant");
    let source = r#"
            function main(): Char {
                s: String = "abc";
                return s[-1];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("s[-1] should fail in codegen");
    assert!(err.contains("String index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_get_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-get-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_get_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_list_get_negative_constant");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs.get(-1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("List.get(-1) should fail in codegen");
    assert!(err.contains("List.get() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_set_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-set-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_set_negative_constant.apex");
    let output_path = temp_root.join("no_check_invalid_list_set_negative_constant");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                xs.set(-1, 99);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("List.set(-1, 99) should fail in codegen");
    assert!(err.contains("List.set() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_constructor_capacity_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-constructor-negative-capacity");
    let source_path = temp_root.join("no_check_invalid_list_constructor_negative_capacity.apex");
    let output_path = temp_root.join("no_check_invalid_list_constructor_negative_capacity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(-1);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("List<Integer>(-1) should fail in codegen");
    assert!(
        err.contains("List constructor capacity cannot be negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constant_ascii_string_index_out_of_bounds_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-ascii-string-index-oob");
    let source_path = temp_root.join("no_check_invalid_ascii_string_index_oob.apex");
    let output_path = temp_root.join("no_check_invalid_ascii_string_index_oob");
    let source = r#"
            function main(): Char {
                return "abc"[5];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("constant ASCII string index OOB should fail in codegen");
    assert!(err.contains("String index out of bounds"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constant_unicode_string_index_out_of_bounds_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unicode-string-index-oob");
    let source_path = temp_root.join("no_check_invalid_unicode_string_index_oob.apex");
    let output_path = temp_root.join("no_check_invalid_unicode_string_index_oob");
    let source = r#"
            function main(): Char {
                return "🚀"[1];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("constant Unicode string index OOB should fail in codegen");
    assert!(err.contains("String index out of bounds"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_boolean_method_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-method-call");
    let source_path = temp_root.join("no_check_invalid_boolean_method_call.apex");
    let output_path = temp_root.join("no_check_invalid_boolean_method_call");
    let source = r#"
            function main(): Integer {
                flag: Boolean = true;
                return flag.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Boolean.length() should fail in codegen");
    assert!(err.contains("Cannot call method on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_method_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-method-call");
    let source_path = temp_root.join("no_check_invalid_integer_method_call.apex");
    let output_path = temp_root.join("no_check_invalid_integer_method_call");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                return value.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Integer.length() should fail in codegen");
    assert!(err.contains("Cannot call method on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_boolean_field_access_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-field-access");
    let source_path = temp_root.join("no_check_invalid_boolean_field_access.apex");
    let output_path = temp_root.join("no_check_invalid_boolean_field_access");
    let source = r#"
            function main(): Integer {
                flag: Boolean = true;
                return flag.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Boolean field access should fail in codegen");
    assert!(err.contains("Cannot access field on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_field_access_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-field-access");
    let source_path = temp_root.join("no_check_invalid_integer_field_access.apex");
    let output_path = temp_root.join("no_check_invalid_integer_field_access");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                return value.value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Integer field access should fail in codegen");
    assert!(err.contains("Cannot access field on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_boolean_field_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-field-assignment");
    let source_path = temp_root.join("no_check_invalid_boolean_field_assignment.apex");
    let output_path = temp_root.join("no_check_invalid_boolean_field_assignment");
    let source = r#"
            function main(): Integer {
                mut flag: Boolean = true;
                flag.value = false;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Boolean field assignment should fail in codegen");
    assert!(err.contains("Cannot access field on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_field_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-field-assignment");
    let source_path = temp_root.join("no_check_invalid_integer_field_assignment.apex");
    let output_path = temp_root.join("no_check_invalid_integer_field_assignment");
    let source = r#"
            function main(): Integer {
                mut value: Integer = 1;
                value.value = 2;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Integer field assignment should fail in codegen");
    assert!(err.contains("Cannot access field on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_class_field_access_with_class_diagnostic() {
    let temp_root = make_temp_project_root("no-check-unknown-class-field-access");
    let source_path = temp_root.join("no_check_unknown_class_field_access.apex");
    let output_path = temp_root.join("no_check_unknown_class_field_access");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Box = Box(7);
                return b.missing;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing Box field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_class_field_assignment_with_class_diagnostic() {
    let temp_root = make_temp_project_root("no-check-unknown-class-field-assignment");
    let source_path = temp_root.join("no_check_unknown_class_field_assignment.apex");
    let output_path = temp_root.join("no_check_unknown_class_field_assignment");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): None {
                mut b: Box = Box(7);
                b.missing = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing Box field assignment should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_unknown_field_before_index_assignment_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-unknown-field-index-assignment");
    let source_path = temp_root.join("no_check_nested_unknown_field_index_assignment.apex");
    let output_path = temp_root.join("no_check_nested_unknown_field_index_assignment");
    let source = r#"
            class Inner {
                mut items: List<Integer>;
                constructor() { this.items = List<Integer>(); }
            }

            class Box {
                mut inner: Inner;
                constructor() { this.inner = Inner(); }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box {
                return Box();
            }

            function main(): None {
                holder: Holder = Holder(build);
                holder.make().inner.missing[0] = 9;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("nested missing field index assignment should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Inner'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_deref_root_cause_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-deref-root-cause");
    let undef_source_path = temp_root.join("no_check_nested_deref_undefined_root.apex");
    let undef_output_path = temp_root.join("no_check_nested_deref_undefined_root");
    let undef_source = r#"
            function main(): None {
                println(*missing.inner.ptr);
                return None;
            }
        "#;

    fs::write(&undef_source_path, undef_source).expect("write undefined-root source");
    let undef_err = compile_source(
        undef_source,
        &undef_source_path,
        &undef_output_path,
        false,
        false,
        None,
        None,
    )
    .expect_err("nested undefined-root deref should fail in codegen");
    assert!(
        undef_err.contains("Undefined variable: missing"),
        "{undef_err}"
    );

    let missing_source_path = temp_root.join("no_check_nested_deref_missing_field.apex");
    let missing_output_path = temp_root.join("no_check_nested_deref_missing_field");
    let missing_source = r#"
            class Inner {
                value: Integer;
                constructor() { this.value = 1; }
            }

            class Box {
                inner: Inner;
                constructor() { this.inner = Inner(); }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box { return Box(); }

            function main(): None {
                holder: Holder = Holder(build);
                println(*holder.make().inner.missing);
                return None;
            }
        "#;

    fs::write(&missing_source_path, missing_source).expect("write missing-field source");
    let missing_err = compile_source(
        missing_source,
        &missing_source_path,
        &missing_output_path,
        false,
        false,
        None,
        None,
    )
    .expect_err("nested missing-field deref should fail in codegen");
    assert!(
        missing_err.contains("Unknown field 'missing' on class 'Inner'"),
        "{missing_err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_unknown_class_method_with_class_diagnostic() {
    let temp_root = make_temp_project_root("unknown-class-method-diagnostic");
    let source_path = temp_root.join("unknown_class_method_diagnostic.apex");
    let output_path = temp_root.join("unknown_class_method_diagnostic");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Box = Box(7);
                return b.missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("missing Box method should fail");
    assert!(
        err.contains("Unknown method 'missing' for class 'Box'"),
        "{err}"
    );
    assert!(!err.contains("Unknown class: Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_class_root_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-class-root");
    let source_path = temp_root.join("no_check_generic_missing_class_root.apex");
    let output_path = temp_root.join("no_check_generic_missing_class_root");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): None {
                b: Box<Integer> = Box<Integer>.missing(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic missing class root should fail in codegen");
    assert!(err.contains("Undefined variable: Box"), "{err}");
    assert!(
        !err.contains(
            "Explicit generic function value should be specialized before code generation"
        ),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_function_call_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-function-call");
    let source_path = temp_root.join("no_check_generic_missing_function_call.apex");
    let output_path = temp_root.join("no_check_generic_missing_function_call");
    let source = r#"
            function main(): Integer {
                return missing<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic missing function call should fail in codegen");
    assert!(err.contains("Undefined function: missing"), "{err}");
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_method_call_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-method-call");
    let source_path = temp_root.join("no_check_generic_missing_method_call.apex");
    let output_path = temp_root.join("no_check_generic_missing_method_call");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Box(1).missing<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic missing method call should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-enum-variant-call-type-args");
    let source_path = temp_root.join("no_check_enum_variant_call_type_args.apex");
    let output_path = temp_root.join("no_check_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                return Boxed.Wrap<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("enum variant call type args should fail in codegen");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_missing_method_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-enum-missing-method-diagnostic");
    let source_path = temp_root.join("no_check_enum_missing_method_diagnostic.apex");
    let output_path = temp_root.join("no_check_enum_missing_method_diagnostic");
    let source = r#"
            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                value: Boxed = Boxed.Wrap(1);
                return value.missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing enum method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'Boxed'"),
        "{err}"
    );
    assert!(
        !err.contains("Unknown interface method implementation"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-imported-enum-variant-call-type-args");
    let source_path = temp_root.join("no_check_imported_enum_variant_call_type_args.apex");
    let output_path = temp_root.join("no_check_imported_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;

            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("imported enum variant call type args should fail in codegen");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "{err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_enum_variant_alias_constructor_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-enum-variant-alias-runtime");
    let source_path = temp_root.join("no_check_imported_enum_variant_alias_runtime.apex");
    let output_path = temp_root.join("no_check_imported_enum_variant_alias_runtime");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;

            function main(): Integer {
                value: Boxed = WrapCtor(7);
                return match (value) { Boxed.Wrap(v) => { if (v == 7) { 0 } else { 1 } } };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked imported enum variant alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked imported enum variant alias constructor binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_option_some_alias_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-runtime");
    let source_path = temp_root.join("no_check_imported_option_some_alias_runtime.apex");
    let output_path = temp_root.join("no_check_imported_option_some_alias_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                value: Option<Integer> = Present(4);
                return if (value.unwrap() == 4) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked imported Option.Some alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked imported Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_option_some_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-fn-value-runtime");
    let source_path = temp_root.join("no_check_imported_option_some_alias_fn_value_runtime.apex");
    let output_path = temp_root.join("no_check_imported_option_some_alias_fn_value_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Present;
                value: Option<Integer> = wrap(6);
                return if (value.unwrap() == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked imported Option.Some alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked imported Option.Some alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("no_check_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box<Integer>;
                return ctor(3).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked generic class constructor function value binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_imported_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("no_check_imported_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            import Box as B;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = B<Integer>;
                return ctor(4).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked imported generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked imported generic class constructor function value binary");
    assert_eq!(status.code(), Some(4));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_nested_generic_class_constructor_function_value_runtime() {
    let temp_root =
        make_temp_project_root("no-check-imported-nested-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("no_check_imported_nested_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("no_check_imported_nested_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B<Integer>;
                return ctor(6).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).expect(
        "unchecked imported nested generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().expect(
        "run compiled unchecked imported nested generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("no_check_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_inferred_generic_class_constructor_function_value_runtime()
{
    let temp_root =
        make_temp_project_root("no-check-imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("no_check_imported_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("no_check_imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).expect(
        "unchecked imported inferred generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().expect(
        "run compiled unchecked imported inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_namespace_alias_inferred_generic_class_constructor_function_value_runtime(
) {
    let temp_root = make_temp_project_root(
        "no-check-namespace-alias-inferred-generic-class-ctor-fn-value-runtime",
    );
    let source_path = temp_root
        .join("no_check_namespace_alias_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("no_check_namespace_alias_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module U {
                module M {
                    class Box<T> {
                        value: T;
                        constructor(value: T) { this.value = value; }
                    }
                }
            }

            import U as u;

            function main(): Integer {
                ctor: (Integer) -> u.M.Box<Integer> = u.M.Box;
                return ctor(9).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
            .expect("unchecked namespace alias inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path).status().expect(
            "run compiled unchecked namespace alias inferred generic class constructor function value binary",
        );
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_inferred_generic_class_constructor_function_value_runtime(
) {
    let temp_root = make_temp_project_root(
        "no-check-wildcard-imported-inferred-generic-class-ctor-fn-value-runtime",
    );
    let source_path = temp_root
        .join("no_check_wildcard_imported_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("no_check_wildcard_imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(17).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).expect(
            "unchecked wildcard imported inferred generic class constructor function value should codegen",
        );

    let status = std::process::Command::new(&output_path).status().expect(
            "run compiled unchecked wildcard imported inferred generic class constructor function value binary",
        );
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("no-check-nested-generic-class-field-runtime");
    let source_path = temp_root.join("no_check_nested_generic_class_field_runtime.apex");
    let output_path = temp_root.join("no_check_nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked nested generic class field access binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("no-check-nested-generic-class-method-runtime");
    let source_path = temp_root.join("no_check_nested_generic_class_method_runtime.apex");
    let output_path = temp_root.join("no_check_nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked nested generic class method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked nested generic class method call binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_nested_generic_class_field_access_runtime() {
    let temp_root =
        make_temp_project_root("no-check-wildcard-imported-nested-generic-class-field-runtime");
    let source_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_field_runtime.apex");
    let output_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked wildcard imported nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path).status().expect(
        "run compiled unchecked wildcard imported nested generic class field access binary",
    );
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_nested_generic_class_method_runtime() {
    let temp_root =
        make_temp_project_root("no-check-wildcard-imported-nested-generic-class-method-runtime");
    let source_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_method_runtime.apex");
    let output_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect("unchecked wildcard imported nested generic class method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unchecked wildcard imported nested generic class method binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_static_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-option-static-call-type-args");
    let source_path = temp_root.join("no_check_option_static_call_type_args.apex");
    let output_path = temp_root.join("no_check_option_static_call_type_args");
    let source = r#"
            function main(): Option<Integer> {
                return Option.some<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Option.some explicit type args should fail in codegen");
    assert!(
        err.contains("Option static methods do not accept explicit type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_imported_option_some_alias_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-type-args");
    let source_path = temp_root.join("no_check_imported_option_some_alias_type_args.apex");
    let output_path = temp_root.join("no_check_imported_option_some_alias_type_args");
    let source = r#"
            import Option.Some as Present;

            function main(): Option<Integer> {
                return Present<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("imported Option.Some alias type args should fail in codegen");
    assert!(
        err.contains("Built-in function 'Option.some' does not accept type arguments"),
        "{err}"
    );
    assert!(!err.contains("Unknown variant 'Some'"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_static_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-result-static-call-type-args");
    let source_path = temp_root.join("no_check_result_static_call_type_args.apex");
    let output_path = temp_root.join("no_check_result_static_call_type_args");
    let source = r#"
            function main(): Result<Integer, String> {
                return Result.ok<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Result.ok explicit type args should fail in codegen");
    assert!(
        err.contains("Result static methods do not accept explicit type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_explicit_type_args_on_non_function_field_call_cleanly() {
    let temp_root = make_temp_project_root("no-check-non-function-field-generic-call");
    let source_path = temp_root.join("no_check_non_function_field_generic_call.apex");
    let output_path = temp_root.join("no_check_non_function_field_generic_call");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Box(1).value<Integer>();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic call on non-function field should fail in codegen");
    assert!(
        err.contains("Unknown method 'value' for class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains("Cannot call non-function type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_explicit_type_args_on_non_function_field_value_cleanly() {
    let temp_root = make_temp_project_root("no-check-non-function-field-generic-value");
    let source_path = temp_root.join("no_check_non_function_field_generic_value.apex");
    let output_path = temp_root.join("no_check_non_function_field_generic_value");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                f: Integer = Box(1).value<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic function value on non-function field should fail in codegen");
    assert!(
        err.contains("Unknown field 'value' on class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains(
            "Explicit generic function value should be specialized before code generation"
        ),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_namespaced_non_function_field_call_with_demangled_class_name() {
    let temp_root = make_temp_project_root("no-check-namespaced-non-function-field-call");
    let source_path = temp_root.join("no_check_namespaced_non_function_field_call.apex");
    let output_path = temp_root.join("no_check_namespaced_non_function_field_call");
    let source = r#"
            module U {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return U.Box(1).value<Integer>();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic call on namespaced non-function field should fail in codegen");
    assert!(
        err.contains("Unknown method 'value' for class 'U.Box'"),
        "{err}"
    );
    assert!(!err.contains("U__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_namespaced_non_function_field_value_with_demangled_class_name() {
    let temp_root = make_temp_project_root("no-check-namespaced-non-function-field-value");
    let source_path = temp_root.join("no_check_namespaced_non_function_field_value.apex");
    let output_path = temp_root.join("no_check_namespaced_non_function_field_value");
    let source = r#"
            module U {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                f: Integer = U.Box(1).value<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic value on namespaced non-function field should fail in codegen");
    assert!(
        err.contains("Unknown field 'value' on class 'U.Box'"),
        "{err}"
    );
    assert!(!err.contains("U__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_string_method_with_string_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-string-method-name");
    let source_path = temp_root.join("no_check_invalid_string_method_name.apex");
    let output_path = temp_root.join("no_check_invalid_string_method_name");
    let source = r#"
            function main(): Integer {
                s: String = "abc";
                return s.missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("unknown String method should fail in codegen");
    assert!(err.contains("Unknown String method: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-method-arity");
    let source_path = temp_root.join("no_check_invalid_list_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_list_method_arity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                return xs.length(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("list method arity mismatch should fail in codegen");
    assert!(
        err.contains("List.length() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_map_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-map-method-arity");
    let source_path = temp_root.join("no_check_invalid_map_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_map_method_arity");
    let source = r#"
            function main(): Integer {
                values: Map<Integer, Integer> = Map<Integer, Integer>();
                return values.get(1, 2);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("map method arity mismatch should fail in codegen");
    assert!(
        err.contains("Map.get() expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_set_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-set-method-arity");
    let source_path = temp_root.join("no_check_invalid_set_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_set_method_arity");
    let source = r#"
            function main(): Integer {
                values: Set<Integer> = Set<Integer>();
                return if (values.contains(1, 2)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("set method arity mismatch should fail in codegen");
    assert!(
        err.contains("Set.contains() expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-option-method-arity");
    let source_path = temp_root.join("no_check_invalid_option_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_option_method_arity");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = Option.some(1);
                return value.unwrap(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("option method arity mismatch should fail in codegen");
    assert!(
        err.contains("Option.unwrap() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-result-method-arity");
    let source_path = temp_root.join("no_check_invalid_result_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_result_method_arity");
    let source = r#"
            function main(): Integer {
                value: Result<Integer, String> = Result.ok(1);
                return value.unwrap(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("result method arity mismatch should fail in codegen");
    assert!(
        err.contains("Result.unwrap() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_range_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-method-arity");
    let source_path = temp_root.join("no_check_invalid_range_method_arity.apex");
    let output_path = temp_root.join("no_check_invalid_range_method_arity");
    let source = r#"
            function main(): Integer {
                values: Range<Integer> = range(0, 3);
                return values.next(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("range method arity mismatch should fail in codegen");
    assert!(
        err.contains("Range.next() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_none_constructor_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-option-none-arity");
    let source_path = temp_root.join("no_check_invalid_option_none_arity.apex");
    let output_path = temp_root.join("no_check_invalid_option_none_arity");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = Option.none(1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Option.none arity mismatch should fail in codegen");
    assert!(
        err.contains("Option.none() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_stdlib_math_abs_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-abs-arity");
    let source_path = temp_root.join("no_check_invalid_math_abs_arity.apex");
    let output_path = temp_root.join("no_check_invalid_math_abs_arity");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                return Math.abs();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.abs arity mismatch should fail in codegen");
    assert!(
        err.contains("Math__abs() expects 1 argument(s), got 0"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_stdlib_math_pi_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-pi-arity");
    let source_path = temp_root.join("no_check_invalid_math_pi_arity.apex");
    let output_path = temp_root.join("no_check_invalid_math_pi_arity");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                return if (Math.pi(1) > 0.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.pi arity mismatch should fail in codegen");
    assert!(
        err.contains("Math__pi() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_exit_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-exit-arity");
    let source_path = temp_root.join("no_check_invalid_exit_arity.apex");
    let output_path = temp_root.join("no_check_invalid_exit_arity");
    let source = r#"
            function main(): Integer {
                exit();
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("exit arity mismatch should fail in codegen");
    assert!(err.contains("exit() expects 1 argument(s), got 0"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_abs_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-abs-boolean");
    let source_path = temp_root.join("no_check_invalid_math_abs_boolean.apex");
    let output_path = temp_root.join("no_check_invalid_math_abs_boolean");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Boolean = true;
                return Math.abs(value);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.abs(Boolean) should fail in codegen");
    assert!(
        err.contains("Math.abs() requires numeric type, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_min_boolean_arguments_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-min-boolean");
    let source_path = temp_root.join("no_check_invalid_math_min_boolean.apex");
    let output_path = temp_root.join("no_check_invalid_math_min_boolean");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Boolean = Math.min(true, false);
                return if (value) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.min(Boolean, Boolean) should fail in codegen");
    assert!(
        err.contains("Math.min() arguments must be numeric types, got Boolean and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_min_on_module_local_non_numeric_type_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-math-min-module-local-type");
    let source_path = temp_root.join("no_check_invalid_math_min_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_math_min_module_local_type");
    let source = r#"
            import std.math.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Float {
                return Math.min(M.Box(7), 1.0);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.min(module-local Box, Float) should fail in codegen");
    assert!(
        err.contains("Math.min() arguments must be numeric types, got M.Box and Float"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_max_on_module_local_non_numeric_type_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-math-max-module-local-type");
    let source_path = temp_root.join("no_check_invalid_math_max_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_math_max_module_local_type");
    let source = r#"
            import std.math.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Float {
                return Math.max(M.Box(7), 1.0);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("Math.max(module-local Box, Float) should fail in codegen");
    assert!(
        err.contains("Math.max() arguments must be numeric types, got M.Box and Float"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_logical_operator_on_module_local_non_boolean_type_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-logical-module-local-type");
    let source_path = temp_root.join("no_check_invalid_logical_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_logical_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Boolean {
                return M.Box(7) && true;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("logical operator on module-local Box should fail in codegen");
    assert!(
        err.contains("Logical operator requires Boolean types, got M.Box and Boolean"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_to_float_function_value_signature() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-fn-value-signature");
    let source_path = temp_root.join("no_check_invalid_to_float_fn_value_signature.apex");
    let output_path = temp_root.join("no_check_invalid_to_float_fn_value_signature");
    let source = r#"
            function main(): Integer {
                f: (Boolean) -> Float = to_float;
                value: Float = f(true);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid to_float function value signature should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected (Boolean) -> Float, got (unknown) -> Float"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_assert_true_function_value_signature() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-true-fn-value-signature");
    let source_path = temp_root.join("no_check_invalid_assert_true_fn_value_signature.apex");
    let output_path = temp_root.join("no_check_invalid_assert_true_fn_value_signature");
    let source = r#"
            function main(): Integer {
                ensure_true: (Integer) -> None = assert_true;
                ensure_true(1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("invalid assert_true function value signature should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected (Integer) -> None, got (unknown) -> None"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_literal_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-literal-type");
    let source_path = temp_root.join("no_check_invalid_match_literal_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_literal_type");
    let source = r#"
            function main(): Integer {
                return match (true) {
                    1 => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("match literal type mismatch should fail in codegen");
    assert!(
        err.contains("Pattern type mismatch: expected Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_literal_type_mismatch_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-match-literal-module-local-type");
    let source_path = temp_root.join("no_check_invalid_match_literal_module_local_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_literal_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return match (M.Box(1)) {
                    1 => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local match literal type mismatch should fail in codegen");
    assert!(
        err.contains("Pattern type mismatch: expected M.Box, found Integer"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_expr_variant_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-expr-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_expr_variant_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_expr_variant_type");
    let source = r#"
            function main(): Integer {
                return match (true) {
                    Some(v) => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("match expression variant mismatch should fail in codegen");
    assert!(
        err.contains("Cannot match variant Some on type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_expr_variant_type_mismatch_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-match-expr-module-local-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_expr_module_local_variant_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_expr_module_local_variant_type");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): Integer {
                value: M.Token = M.Token.Int(1);
                return match (value) {
                    Some(v) => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local match expression variant mismatch should fail");
    assert!(
        err.contains("Cannot match variant Some on type M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_stmt_variant_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-stmt-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_stmt_variant_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_stmt_variant_type");
    let source = r#"
            function main(): Integer {
                match (true) {
                    Some(v) => { return 0; }
                    _ => { return 1; }
                }
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("match statement variant mismatch should fail in codegen");
    assert!(
        err.contains("Cannot match variant Some on type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_stmt_variant_type_mismatch_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-match-stmt-module-local-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_stmt_module_local_variant_type.apex");
    let output_path = temp_root.join("no_check_invalid_match_stmt_module_local_variant_type");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): Integer {
                value: M.Token = M.Token.Int(1);
                match (value) {
                    Some(v) => { return 0; }
                    _ => { return 1; }
                }
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local match statement variant mismatch should fail");
    assert!(
        err.contains("Cannot match variant Some on type M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_match_expr_with_user_enum_some_string_payload_runtime() {
    let temp_root = make_temp_project_root("match-expr-user-enum-some-string-runtime");
    let source_path = temp_root.join("match_expr_user_enum_some_string_runtime.apex");
    let output_path = temp_root.join("match_expr_user_enum_some_string_runtime");
    let source = r#"
            enum E {
                Some(String),
                Missing
            }

            function main(): Integer {
                value: E = E.Some("hello");
                return match (value) {
                    E.Some(v) => v.length(),
                    E.Missing => 0,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("match expression with user enum Some(String) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled user enum Some(String) match expression binary");
    assert_eq!(status.code(), Some(5));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_boolean_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-bool-runtime");
    let source_path = temp_root.join("string_interpolation_bool_runtime.apex");
    let output_path = temp_root.join("string_interpolation_bool_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{true}";
                return if (Str.compare(value, "true") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on Boolean should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation Boolean binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_char_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-char-runtime");
    let source_path = temp_root.join("string_interpolation_char_runtime.apex");
    let output_path = temp_root.join("string_interpolation_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{'b'}";
                return if (Str.compare(value, "b") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_none_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-none-runtime");
    let source_path = temp_root.join("string_interpolation_none_runtime.apex");
    let output_path = temp_root.join("string_interpolation_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{None}";
                return if (Str.compare(value, "None") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation on None should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation None binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_math_abs_min_integer_runtime() {
    let temp_root = make_temp_project_root("math-abs-min-integer-runtime");
    let source_path = temp_root.join("math_abs_min_integer_runtime.apex");
    let output_path = temp_root.join("math_abs_min_integer_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Integer = 0 - 9223372036854775807 - 1;
                result: Integer = Math.abs(value);
                return if (result < 0) { 1 } else { 0 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.abs minimum integer should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Math.abs minimum integer binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Math.abs() overflow on minimum Integer\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_boolean_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-bool-runtime");
    let source_path = temp_root.join("print_bool_runtime.apex");
    let output_path = temp_root.join("print_bool_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print(Boolean) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print Boolean binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "true");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_unicode_char_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-char-runtime");
    let source_path = temp_root.join("print_char_runtime.apex");
    let output_path = temp_root.join("print_char_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print('🚀');
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print(Char) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print Char binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "🚀");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_none_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-none-runtime");
    let source_path = temp_root.join("print_none_runtime.apex");
    let output_path = temp_root.join("print_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(None);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print(None) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print None binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "None");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_option_runtime() {
    let temp_root = make_temp_project_root("print-option-runtime");
    let source_path = temp_root.join("print_option_runtime.apex");
    let output_path = temp_root.join("print_option_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Option.some(1));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print on Option should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print Option binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Some(1)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("print-direct-option-none-runtime");
    let source_path = temp_root.join("print_direct_option_none_runtime.apex");
    let output_path = temp_root.join("print_direct_option_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Option.none());
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print on direct Option.none should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print direct Option.none binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "None");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_result_error_with_option_none_runtime() {
    let temp_root = make_temp_project_root("print-direct-result-error-option-none-runtime");
    let source_path = temp_root.join("print_direct_result_error_option_none_runtime.apex");
    let output_path = temp_root.join("print_direct_result_error_option_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Result.error(Option.none()));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print on direct Result.error(Option.none()) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print direct Result.error(Option.none()) binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(None)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_result_runtime() {
    let temp_root = make_temp_project_root("print-result-runtime");
    let source_path = temp_root.join("print_result_runtime.apex");
    let output_path = temp_root.join("print_result_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                result: Result<Integer, String> = Result.error("boom");
                print(result);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print on Result should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print Result binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(boom)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_result_error_runtime() {
    let temp_root = make_temp_project_root("print-direct-result-error-runtime");
    let source_path = temp_root.join("print_direct_result_error_runtime.apex");
    let output_path = temp_root.join("print_direct_result_error_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Result.error("boom"));
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("print on direct Result.error should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled print direct Result.error binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(boom)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_with_string_literal_index_key_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-map-string-key-runtime");
    let source_path = temp_root.join("string_interpolation_map_string_key_runtime.apex");
    let output_path = temp_root.join("string_interpolation_map_string_key_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["x"] = 7;
                s: String = "{m["x"]}";
                return if (s == "7") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation with string literal key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string interpolation with string key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_long_string_interpolation_runtime() {
    let temp_root = make_temp_project_root("long-string-interpolation-runtime");
    let source_path = temp_root.join("long_string_interpolation_runtime.apex");
    let output_path = temp_root.join("long_string_interpolation_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                mut s: String = "";
                mut i: Integer = 0;
                while (i < 60000) {
                    s = Str.concat(s, "a");
                    i = i + 1;
                }
                out: String = "x{s}y";
                return if (out.length() == 60002) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("long string interpolation should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled long string interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_with_nested_braces_string_literal_runtime() {
    let temp_root = make_temp_project_root("string-interp-nested-braces-string-runtime");
    let source_path = temp_root.join("string_interp_nested_braces_string_runtime.apex");
    let output_path = temp_root.join("string_interp_nested_braces_string_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                s: String = "{Str.contains("\{x\}", "{")}";
                return if (s == "true") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation with nested braces in string literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested braces string interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_with_char_brace_literal_runtime() {
    let temp_root = make_temp_project_root("string-interp-char-brace-runtime");
    let source_path = temp_root.join("string_interp_char_brace_runtime.apex");
    let output_path = temp_root.join("string_interp_char_brace_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "{'}'}";
                return if (s == "}") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string interpolation with char brace literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled char brace interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_interpolation_from_nested_module_runtime() {
    let temp_root = make_temp_project_root("float-interpolation-nested-module-runtime");
    let source_path = temp_root.join("float_interpolation_nested_module_runtime.apex");
    let output_path = temp_root.join("float_interpolation_nested_module_runtime");
    let source = r#"
            import std.io.*;

            module Metrics {
                module Api {
                    function ratio(value: Integer): Float {
                        return to_float(value) / 2.0;
                    }
                }
            }

            function main(): Integer {
                println("ratio={Metrics.Api.ratio(3)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested module float interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled nested module float interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("ratio=1.500000"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mixed_numeric_if_interpolation_runtime() {
    let temp_root = make_temp_project_root("inline-mixed-if-interpolation-runtime");
    let source_path = temp_root.join("inline_mixed_if_interpolation_runtime.apex");
    let output_path = temp_root.join("inline_mixed_if_interpolation_runtime");
    let source = r#"
            import std.io.*;
            function main(): Integer {
                println("value={if (true) { 1 } else { 2.5 }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("inline mixed numeric if interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled inline mixed numeric if interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mixed_numeric_match_interpolation_runtime() {
    let temp_root = make_temp_project_root("inline-mixed-match-interpolation-runtime");
    let source_path = temp_root.join("inline_mixed_match_interpolation_runtime.apex");
    let output_path = temp_root.join("inline_mixed_match_interpolation_runtime");
    let source = r#"
            import std.io.*;
            enum Kind { A, B }
            function main(): Integer {
                println("value={match (Kind.A) { Kind.A => { 1 } Kind.B => { 2.5 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("inline mixed numeric match interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled inline mixed numeric match interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unit_enum_match_expression_runtime() {
    let temp_root = make_temp_project_root("unit-enum-match-expression-runtime");
    let source_path = temp_root.join("unit_enum_match_expression_runtime.apex");
    let output_path = temp_root.join("unit_enum_match_expression_runtime");
    let source = r#"
            enum Kind { A, B }
            function main(): Integer {
                value: Integer = match (Kind.A) { Kind.A => { 1 } Kind.B => { 2 } };
                require(value == 1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unit enum match expression should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled unit enum match expression binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_unit_enum_variant_alias_match_expression_runtime() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-alias-match-runtime");
    let source_path = temp_root.join("imported_unit_enum_variant_alias_match_runtime.apex");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_match_runtime");
    let source = r#"
            import std.io.*;
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                println("value={match (A) { A => { 1 } E.B => { 2.5 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported unit enum variant alias match interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled imported unit enum variant alias match binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_unit_enum_variant_alias_patterns_runtime() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-alias-pattern-runtime");
    let source_path = temp_root.join("imported_unit_enum_variant_alias_pattern_runtime.apex");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_pattern_runtime");
    let source = r#"
            import std.io.*;
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                println("value={match (E.B) { A => { 1 } E.B => { 2 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported unit enum variant alias pattern should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled imported unit enum variant alias pattern binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=2\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn check_source_rejects_non_exhaustive_imported_unit_enum_variant_alias_pattern() {
    let temp_root =
        make_temp_project_root("imported-unit-enum-variant-alias-pattern-non-exhaustive");
    let source_path =
        temp_root.join("imported_unit_enum_variant_alias_pattern_non_exhaustive.apex");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_pattern_non_exhaustive");
    let source = r#"
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                return match (E.B) { A => { 1 } };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("alias unit variant pattern should not act as catch-all");
    assert!(
        err.contains("Non-exhaustive match expression"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_payload_enum_variant_alias_patterns_runtime() {
    let temp_root = make_temp_project_root("imported-payload-enum-variant-alias-pattern-runtime");
    let source_path = temp_root.join("imported_payload_enum_variant_alias_pattern_runtime.apex");
    let output_path = temp_root.join("imported_payload_enum_variant_alias_pattern_runtime");
    let source = r#"
            enum E { A(Integer), B(Integer) }
            import E.A as First;
            function main(): Integer {
                value: Integer = match (E.B(2)) { First(v) => { v } E.B(v) => { v + 1 } };
                require(value == 3);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported payload enum variant alias patterns should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled imported payload enum variant alias pattern binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_top_level_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-top-level-type-alias-runtime");
    let source_path = temp_root.join("imported_top_level_type_alias_runtime.apex");
    let output_path = temp_root.join("imported_top_level_type_alias_runtime");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }
            import Box as B;
            function main(): Integer {
                return B(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported top-level type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported top-level type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_nested_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-nested-type-alias-runtime");
    let source_path = temp_root.join("imported_nested_type_alias_runtime.apex");
    let output_path = temp_root.join("imported_nested_type_alias_runtime");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                    function get(): Integer { return this.value; }
                }
            }
            import M.Box as B;
            function main(): Integer {
                return B(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported nested type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported nested type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_top_level_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-top-level-type-alias-runtime");
    let source_path = temp_root.join("imported_generic_top_level_type_alias_runtime.apex");
    let output_path = temp_root.join("imported_generic_top_level_type_alias_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }
            import Box as B;
            function main(): Integer {
                return B<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported generic top-level type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported generic top-level type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_nested_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-nested-type-alias-runtime");
    let source_path = temp_root.join("imported_generic_nested_type_alias_runtime.apex");
    let output_path = temp_root.join("imported_generic_nested_type_alias_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }
            import M.Box as B;
            function main(): Integer {
                return B<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported generic nested type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported generic nested type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box<Integer>;
                return ctor(3).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic class constructor function value binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("imported_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            import Box as B;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = B<Integer>;
                return ctor(4).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported generic class constructor function value binary");
    assert_eq!(status.code(), Some(4));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_nested_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-nested-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_nested_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("imported_nested_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B<Integer>;
                return ctor(6).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported nested generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported nested generic class constructor function value binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path = temp_root.join("imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("namespace_alias_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module U {
                module M {
                    class Box<T> {
                        value: T;
                        constructor(value: T) { this.value = value; }
                    }
                }
            }

            import U as u;

            function main(): Integer {
                ctor: (Integer) -> u.M.Box<Integer> = u.M.Box;
                return ctor(9).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path).status().expect(
        "run compiled namespace alias inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_inferred_generic_class_constructor_function_value_runtime()
{
    let temp_root =
        make_temp_project_root("wildcard-imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("wildcard_imported_inferred_generic_class_ctor_fn_value_runtime.apex");
    let output_path =
        temp_root.join("wildcard_imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(17).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None).expect(
        "wildcard imported inferred generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().expect(
        "run compiled wildcard imported inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("nested-generic-class-field-runtime");
    let source_path = temp_root.join("nested_generic_class_field_runtime.apex");
    let output_path = temp_root.join("nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested generic class field access binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("nested-generic-class-method-runtime");
    let source_path = temp_root.join("nested_generic_class_method_runtime.apex");
    let output_path = temp_root.join("nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested generic class method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested generic class method call binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-nested-generic-class-field-runtime");
    let source_path = temp_root.join("wildcard_imported_nested_generic_class_field_runtime.apex");
    let output_path = temp_root.join("wildcard_imported_nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("wildcard imported nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled wildcard imported nested generic class field access binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-nested-generic-class-method-runtime");
    let source_path = temp_root.join("wildcard_imported_nested_generic_class_method_runtime.apex");
    let output_path = temp_root.join("wildcard_imported_nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("wildcard imported nested generic class method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled wildcard imported nested generic class method binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_enum_type_alias_variant_runtime() {
    let temp_root = make_temp_project_root("imported-enum-type-alias-variant-runtime");
    let source_path = temp_root.join("imported_enum_type_alias_variant_runtime.apex");
    let output_path = temp_root.join("imported_enum_type_alias_variant_runtime");
    let source = r#"
            enum E { A(Integer) }
            import E as Alias;
            function main(): Integer {
                value: Alias = Alias.A(2);
                return match (value) {
                    Alias.A(v) => { v }
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported enum type alias variant should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported enum type alias variant binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_nested_generic_class_constructor_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-nested-generic-class-constructor-runtime");
    let source_path =
        temp_root.join("namespace_alias_nested_generic_class_constructor_runtime.apex");
    let output_path = temp_root.join("namespace_alias_nested_generic_class_constructor_runtime");
    let source = r#"
            module U {
                module M {
                    class Box<T> {
                        value: T;
                        constructor(value: T) { this.value = value; }
                        function get(): T { return this.value; }
                    }
                }
            }
            import U as u;
            function main(): Integer {
                return u.M.Box<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias nested generic class constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias nested generic class constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_enum_variant_constructor_runtime() {
    let temp_root = make_temp_project_root("namespace-alias-enum-variant-constructor-runtime");
    let source_path = temp_root.join("namespace_alias_enum_variant_constructor_runtime.apex");
    let output_path = temp_root.join("namespace_alias_enum_variant_constructor_runtime");
    let source = r#"
            module U {
                enum E { A(Integer), B }
            }
            import U as u;
            function main(): Integer {
                value: u.E = u.E.A(2);
                return match (value) { u.E.A(v) => { v } u.E.B => { 0 } };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias enum variant constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias enum variant constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_nested_enum_variant_constructor_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-nested-enum-variant-constructor-runtime");
    let source_path =
        temp_root.join("namespace_alias_nested_enum_variant_constructor_runtime.apex");
    let output_path = temp_root.join("namespace_alias_nested_enum_variant_constructor_runtime");
    let source = r#"
            module U {
                module M {
                    enum E { A(Integer), B }
                }
            }
            import U as u;
            function main(): Integer {
                value: u.M.E = u.M.E.A(2);
                return match (value) { u.M.E.A(v) => { v } u.M.E.B => { 0 } };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias nested enum variant constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias nested enum variant constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_function_alias_returning_generic_class_runtime() {
    let temp_root =
        make_temp_project_root("imported-generic-function-alias-returning-generic-class-runtime");
    let source_path =
        temp_root.join("imported_generic_function_alias_returning_generic_class_runtime.apex");
    let output_path =
        temp_root.join("imported_generic_function_alias_returning_generic_class_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
                function mk<T>(value: T): Box<T> { return Box<T>(value); }
            }
            import M.mk as mk;
            function main(): Integer {
                return mk<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported generic function alias returning generic class should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported generic function alias returning generic class binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_function_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-function-alias-runtime");
    let source_path = temp_root.join("imported_generic_function_alias_runtime.apex");
    let output_path = temp_root.join("imported_generic_function_alias_runtime");
    let source = r#"
            module M {
                function id<T>(value: T): T { return value; }
            }
            import M.id as id;
            function main(): Integer {
                return id<Integer>(2);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported generic function alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported generic function alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_exact_imported_module_named_main_runtime() {
    let temp_root = make_temp_project_root("exact-imported-module-main-runtime");
    let source_path = temp_root.join("exact_imported_module_main_runtime.apex");
    let output_path = temp_root.join("exact_imported_module_main_runtime");
    let source = r#"
            module M {
                module main {
                    function ping(): Integer { return 22; }
                }
            }

            import M.main as Main;

            function main(): Integer {
                return Main.ping();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("exact-imported module named main should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled exact-imported module named main binary");
    assert_eq!(status.code(), Some(22));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_module_named_main_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-module-main-runtime");
    let source_path = temp_root.join("wildcard_imported_module_main_runtime.apex");
    let output_path = temp_root.join("wildcard_imported_module_main_runtime");
    let source = r#"
            module M {
                module main {
                    function ping(): Integer { return 22; }
                }
            }

            import M.*;

            function main(): Integer {
                return main.ping();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("wildcard-imported module named main should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled wildcard-imported module named main binary");
    assert_eq!(status.code(), Some(22));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_expression_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("if-expression-builtin-function-value-runtime");
    let source_path = temp_root.join("if_expression_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("if_expression_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(flag: Boolean): (Integer) -> Float {
                return if (flag) { to_float } else { to_float };
            }
            function main(): Integer {
                println("value={choose(true)(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if-expression builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled if-expression builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_match_expression_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("match-expression-builtin-function-value-runtime");
    let source_path = temp_root.join("match_expression_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("match_expression_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            enum Mode { A, B }
            function choose(mode: Mode): (Integer) -> Float {
                return match (mode) { Mode.A => { to_float } Mode.B => { to_float } };
            }
            function main(): Integer {
                println("value={choose(Mode.A)(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("match-expression builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled match-expression builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_some_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("option-some-builtin-function-value-runtime");
    let source_path = temp_root.join("option_some_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Option<(Integer) -> Float> {
                return Option.some(to_float);
            }
            function main(): Integer {
                println("value={choose().unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Option.some builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_expression_option_some_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("if-expression-option-some-builtin-function-value-runtime");
    let source_path =
        temp_root.join("if_expression_option_some_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("if_expression_option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(flag: Boolean): Option<(Integer) -> Float> {
                return if (flag) { Option.some(to_float) } else { Option.some(to_float) };
            }
            function main(): Integer {
                println("value={choose(true).unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if-expression Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled if-expression Option.some builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_match_expression_option_some_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("match-expression-option-some-builtin-function-value-runtime");
    let source_path =
        temp_root.join("match_expression_option_some_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("match_expression_option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            enum Mode { A, B }
            function choose(mode: Mode): Option<(Integer) -> Float> {
                return match (mode) {
                    Mode.A => { Option.some(to_float) }
                    Mode.B => { Option.some(to_float) }
                };
            }
            function main(): Integer {
                println("value={choose(Mode.A).unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("match-expression Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled match-expression Option.some builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_ok_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("result-ok-builtin-function-value-runtime");
    let source_path = temp_root.join("result_ok_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("result_ok_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Result<(Integer) -> Float, String> {
                return Result.ok(to_float);
            }
            function main(): Integer {
                println("value={choose().unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Result.ok builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Result.ok builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_error_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("result-error-builtin-function-value-runtime");
    let source_path = temp_root.join("result_error_builtin_function_value_runtime.apex");
    let output_path = temp_root.join("result_error_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Result<String, (Integer) -> Float> {
                return Result.error(to_float);
            }
            function main(): Integer {
                errf: (Integer) -> Float = match (choose()) {
                    Result.Error(f) => f,
                    _ => to_float,
                };
                println("value={errf(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Result.error builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled Result.error builtin function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_some_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-option-some-function-value-runtime");
    let source_path = temp_root.join("direct_option_some_function_value_runtime.apex");
    let output_path = temp_root.join("direct_option_some_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Option.some;
                value: Option<Integer> = wrap(7);
                return if (value == Option.some(7)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Option.some function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Option.some function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_none_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-option-none-function-value-runtime");
    let source_path = temp_root.join("direct_option_none_function_value_runtime.apex");
    let output_path = temp_root.join("direct_option_none_function_value_runtime");
    let source = r#"
            function main(): Integer {
                empty: () -> Option<Integer> = Option.none;
                value: Option<Integer> = empty();
                return if (value == Option.none()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Option.none function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Option.none function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-result-ok-function-value-runtime");
    let source_path = temp_root.join("direct_result_ok_function_value_runtime.apex");
    let output_path = temp_root.join("direct_result_ok_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Result<Integer, String> = Result.ok;
                value: Result<Integer, String> = wrap(7);
                return if (value == Result.ok(7)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.ok function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.ok function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-result-error-function-value-runtime");
    let source_path = temp_root.join("direct_result_error_function_value_runtime.apex");
    let output_path = temp_root.join("direct_result_error_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (String) -> Result<Integer, String> = Result.error;
                value: Result<Integer, String> = wrap("boom");
                return if (value == Result.error("boom")) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.error function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.error function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_direct_option_some_function_value_type_mismatch() {
    let temp_root = make_temp_project_root("direct-option-some-function-value-type-mismatch");
    let source_path = temp_root.join("direct_option_some_function_value_type_mismatch.apex");
    let output_path = temp_root.join("direct_option_some_function_value_type_mismatch");
    let source = r#"
            function main(): Integer {
                wrap: (String) -> Option<Integer> = Option.some;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("direct Option.some mismatch should fail");
    assert!(
        err.contains(
            "Type mismatch: expected (String) -> Option<Integer>, got (unknown) -> Option<unknown>"
        ),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_enum_payload_variant_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-enum-payload-variant-function-value");
    let source_path = temp_root.join("direct_enum_payload_variant_function_value.apex");
    let output_path = temp_root.join("direct_enum_payload_variant_function_value");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (Integer) -> Boxed = Boxed.Wrap;
                value: Boxed = wrap(7);
                return match (value) {
                    Boxed.Wrap(v) => { if (v == 7) { 0 } else { 1 } }
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct enum payload variant function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct enum payload variant function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_enum_unit_variant_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-enum-unit-variant-function-value");
    let source_path = temp_root.join("direct_enum_unit_variant_function_value.apex");
    let output_path = temp_root.join("direct_enum_unit_variant_function_value");
    let source = r#"
            enum Mode { A, B }
            function main(): Integer {
                pick: () -> Mode = Mode.A;
                return if (pick() == Mode.A) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct enum unit variant function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct enum unit variant function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_direct_enum_variant_function_value_type_mismatch() {
    let temp_root = make_temp_project_root("direct-enum-variant-function-value-type-mismatch");
    let source_path = temp_root.join("direct_enum_variant_function_value_type_mismatch.apex");
    let output_path = temp_root.join("direct_enum_variant_function_value_type_mismatch");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (String) -> Boxed = Boxed.Wrap;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("direct enum variant mismatch should fail");
    assert!(
        err.contains("Type mismatch: expected (String) -> Boxed, got (Integer) -> Boxed"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_enum_variant_function_value_type_mismatch_with_user_facing_name(
) {
    let temp_root =
        make_temp_project_root("no-check-module-local-enum-variant-fn-value-type-mismatch");
    let source_path =
        temp_root.join("no_check_module_local_enum_variant_fn_value_type_mismatch.apex");
    let output_path = temp_root.join("no_check_module_local_enum_variant_fn_value_type_mismatch");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): None {
                f: () -> M.Token = M.Token.Int;
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local enum variant function value mismatch should fail");
    assert!(
        err.contains("Type mismatch: expected () -> M.Token, got (Integer) -> M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_specialized_constructor_wrong_arity_with_user_facing_diagnostic()
{
    let temp_root = make_temp_project_root("no-check-specialized-constructor-wrong-arity");
    let source_path = temp_root.join("no_check_specialized_constructor_wrong_arity.apex");
    let output_path = temp_root.join("no_check_specialized_constructor_wrong_arity");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }
                }
            }

            function main(): Integer {
                return M.Pair_Box<Integer, String>(7);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("specialized constructor wrong arity should fail in codegen");
    assert!(
        err.contains("Constructor M.Pair_Box<Integer, String> expects 2 argument(s), got 1"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_specialized_method_wrong_arity_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-specialized-method-wrong-arity");
    let source_path = temp_root.join("no_check_specialized_method_wrong_arity.apex");
    let output_path = temp_root.join("no_check_specialized_method_wrong_arity");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }

                    function first_value(): T {
                        return this.first;
                    }
                }
            }

            function main(): Integer {
                value: M.Pair_Box<Integer, String> = M.Pair_Box<Integer, String>(7, "x");
                return value.first_value(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("specialized method wrong arity should fail in codegen");
    assert!(
        err.contains("M.Pair_Box<Integer, String>.first_value() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_specialized_class_field_access_with_user_facing_class_diagnostic(
) {
    let temp_root = make_temp_project_root("no-check-unknown-specialized-class-field-access");
    let source_path = temp_root.join("no_check_unknown_specialized_class_field_access.apex");
    let output_path = temp_root.join("no_check_unknown_specialized_class_field_access");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }
                }
            }

            function main(): Integer {
                value: M.Pair_Box<Integer, String> = M.Pair_Box<Integer, String>(7, "x");
                return value.missing;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing specialized field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'M.Pair_Box<Integer, String>'"),
        "{err}"
    );
    assert!(!err.contains("M.Pair_Box.spec.I64_Str"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_missing_method_with_user_facing_class_name() {
    let temp_root = make_temp_project_root("no-check-module-local-missing-method-call");
    let source_path = temp_root.join("no_check_module_local_missing_method_call.apex");
    let output_path = temp_root.join("no_check_module_local_missing_method_call");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return M.Box(1).missing();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module-local missing method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'M.Box'"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_bound_method_function_value_wrong_arity() {
    let temp_root = make_temp_project_root("no-check-bound-method-function-value-wrong-arity");
    let source_path = temp_root.join("no_check_bound_method_function_value_wrong_arity.apex");
    let output_path = temp_root.join("no_check_bound_method_function_value_wrong_arity");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }

                function get(): Integer {
                    return this.value;
                }
            }

            function main(): Integer {
                b: Box = Box(7);
                f: () -> Integer = b.get;
                return f(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("bound method function value wrong arity should fail in codegen");
    assert!(
        err.contains("Function value () -> Integer expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_bound_method_function_value_signature_mismatch() {
    let temp_root =
        make_temp_project_root("no-check-generic-bound-method-function-signature-mismatch");
    let source_path =
        temp_root.join("no_check_generic_bound_method_function_signature_mismatch.apex");
    let output_path =
        temp_root.join("no_check_generic_bound_method_function_signature_mismatch");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Person implements Named {
                constructor() {}
                function name(): Integer { return 1; }
            }

            function read_name<T extends Named>(value: T): Integer {
                f: (Integer) -> Integer = value.name;
                return f(1);
            }

            function main(): Integer {
                return read_name(Person());
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("generic bound method signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_missing_bound_method_value_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-enum-missing-bound-method-value");
    let source_path = temp_root.join("no_check_enum_missing_bound_method_value.apex");
    let output_path = temp_root.join("no_check_enum_missing_bound_method_value");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }

                function missing(): Integer {
                    return this.value;
                }
            }

            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                value: Boxed = Boxed.Wrap(1);
                f: () -> Integer = value.missing;
                return f();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("missing enum bound method value should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Boxed'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_variant_function_value_field_access_without_panicking() {
    let temp_root = make_temp_project_root("no-check-enum-variant-function-value-field-access");
    let source_path = temp_root.join("no_check_enum_variant_function_value_field_access.apex");
    let output_path = temp_root.join("no_check_enum_variant_function_value_field_access");
    let source = r#"
            enum Boxed {
                Wrap(Integer)
            }

            function main(): Integer {
                f: (Integer) -> Boxed = Boxed.Wrap;
                return f(1).Wrap;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("enum variant function value field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'Wrap' on class 'Boxed'"),
        "{err}"
    );
    assert!(!err.contains("panicked at"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_function_wrong_arity_instead_of_ignoring_extra_args() {
    let temp_root = make_temp_project_root("no-check-module-function-wrong-arity");
    let source_path = temp_root.join("no_check_module_function_wrong_arity.apex");
    let output_path = temp_root.join("no_check_module_function_wrong_arity");
    let source = r#"
            module M {
                function f(x: Integer): Integer {
                    return x;
                }
            }

            function main(): Integer {
                return M.f(7, 8);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("module function wrong arity should fail in codegen");
    assert!(
        err.contains("Function value (Integer) -> Integer expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("enum-variant-function-value-type-args");
    let source_path = temp_root.join("enum_variant_function_value_type_args.apex");
    let output_path = temp_root.join("enum_variant_function_value_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (Integer) -> Boxed = Boxed.Wrap<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_constructor_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("builtin-constructor-function-value-type-args");
    let source_path = temp_root.join("builtin_constructor_function_value_type_args.apex");
    let output_path = temp_root.join("builtin_constructor_function_value_type_args");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Option.some<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("builtin constructor function value type args should fail");
    assert!(
        err.contains("Built-in function 'Option.some' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_imported_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("imported-enum-variant-function-value-type-args");
    let source_path = temp_root.join("imported_enum_variant_function_value_type_args.apex");
    let output_path = temp_root.join("imported_enum_variant_function_value_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;
            function main(): Integer {
                wrap: (Integer) -> Boxed = WrapCtor<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("imported enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("imported-enum-variant-call-type-args");
    let source_path = temp_root.join("imported_enum_variant_call_type_args.apex");
    let output_path = temp_root.join("imported_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;
            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("imported enum variant call type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_nested_imported_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("nested-imported-enum-variant-function-value-type-args");
    let source_path = temp_root.join("nested_imported_enum_variant_function_value_type_args.apex");
    let output_path = temp_root.join("nested_imported_enum_variant_function_value_type_args");
    let source = r#"
            module U {
                module V {
                    enum E { Wrap(Integer) }
                }
            }
            import U.V.E.Wrap as WrapCtor;
            function main(): Integer {
                wrap: (Integer) -> U.V.E = WrapCtor<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("nested imported enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'U.V.E.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_nested_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("nested-imported-enum-variant-call-type-args");
    let source_path = temp_root.join("nested_imported_enum_variant_call_type_args.apex");
    let output_path = temp_root.join("nested_imported_enum_variant_call_type_args");
    let source = r#"
            module U {
                module V {
                    enum E { Wrap(Integer) }
                }
            }
            import U.V.E.Wrap as WrapCtor;
            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("nested imported enum variant call type args should fail");
    assert!(
        err.contains("Enum variant 'U.V.E.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_constructor_function_value_type_args_nocheck() {
    let temp_root = make_temp_project_root("builtin-constructor-function-value-type-args-nocheck");
    let source_path = temp_root.join("builtin_constructor_function_value_type_args_nocheck.apex");
    let output_path = temp_root.join("builtin_constructor_function_value_type_args_nocheck");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Result<Integer, String> = Result.ok<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .expect_err("builtin constructor function value type args should fail in codegen");
    assert!(
        err.contains("Built-in function 'Result.ok' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_constructor_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("constructor-builtin-function-value-runtime");
    let source_path = temp_root.join("constructor_builtin_function_value_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("constructor builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled constructor builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_constructor_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-constructor-builtin-function-value-runtime");
    let source_path = temp_root.join("generic_constructor_builtin_function_value_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic constructor builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic constructor builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-method-builtin-function-value-runtime");
    let source_path = temp_root.join("generic_method_builtin_function_value_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_user_defined_result_generic_method_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("user-defined-result-generic-method-builtin-function-value");
    let source_path =
        temp_root.join("user_defined_result_generic_method_builtin_function_value.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("user-defined Result generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled user-defined Result generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_receiver_result_generic_method_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("if-receiver-result-generic-method-builtin-function-value");
    let source_path =
        temp_root.join("if_receiver_result_generic_method_builtin_function_value.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if receiver Result generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if receiver Result generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_receiver_map_generic_method_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("if-receiver-map-generic-method-builtin-function-value");
    let source_path = temp_root.join("if_receiver_map_generic_method_builtin_function_value.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if receiver Map generic method builtin function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if receiver Map generic method builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_lambda_capturing_this_runtime() {
    let temp_root = make_temp_project_root("method-lambda-capturing-this-runtime");
    let source_path = temp_root.join("method_lambda_capturing_this_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("method lambda capturing this should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled method lambda capturing this binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_lambda_capturing_this_and_builtin_callback_runtime() {
    let temp_root = make_temp_project_root("generic-method-lambda-capturing-this-builtin-callback");
    let source_path = temp_root.join("generic_method_lambda_capturing_this_builtin_callback.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic method lambda capturing this with builtin callback should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic method lambda capturing this builtin callback binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_lambda_if_expression_capture_runtime() {
    let temp_root = make_temp_project_root("lambda-if-expression-capture-runtime");
    let source_path = temp_root.join("lambda_if_expression_capture_runtime.apex");
    let output_path = temp_root.join("lambda_if_expression_capture_runtime");
    let source = r#"
            function main(): Integer {
                x: Integer = 7;
                f: () -> Integer = () => if (true) { x } else { 0 };
                return if (f() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("lambda if-expression capture should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled lambda if-expression capture binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_if_expression_capture_runtime() {
    let temp_root = make_temp_project_root("async-if-expression-capture-runtime");
    let source_path = temp_root.join("async_if_expression_capture_runtime.apex");
    let output_path = temp_root.join("async_if_expression_capture_runtime");
    let source = r#"
            function main(): Integer {
                x: Integer = 7;
                t: Task<Integer> = async { if (true) { x } else { 0 } };
                return if (await(t) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async if-expression capture should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async if-expression capture binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_shadowed_borrow_name_without_false_capture_runtime() {
    let temp_root = make_temp_project_root("async-shadowed-borrow-name-runtime");
    let source_path = temp_root.join("async_shadowed_borrow_name_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async shadowed borrow name should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async shadowed borrow name binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_match_pattern_shadowed_borrow_name_runtime() {
    let temp_root = make_temp_project_root("async-match-pattern-shadowed-borrow-name-runtime");
    let source_path = temp_root.join("async_match_pattern_shadowed_borrow_name_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async match pattern shadowed borrow name should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async match pattern shadowed borrow name binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_integer_tail_for_float_task_runtime() {
    let temp_root = make_temp_project_root("async-int-tail-float-task-runtime");
    let source_path = temp_root.join("async_int_tail_float_task_runtime.apex");
    let output_path = temp_root.join("async_int_tail_float_task_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async { 1 };
                value: Float = await(task);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block Integer tail for Task<Float> should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async Integer tail Float task binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_tail_lambda_for_float_return_runtime() {
    let temp_root = make_temp_project_root("lambda-int-tail-float-return-runtime");
    let source_path = temp_root.join("lambda_int_tail_float_return_runtime.apex");
    let output_path = temp_root.join("lambda_int_tail_float_return_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = () => 1;
                value: Float = f();
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("lambda Integer tail for Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled lambda Integer tail Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_integer_function_value_for_float_return_runtime() {
    let temp_root = make_temp_project_root("named-fn-int-to-float-runtime");
    let source_path = temp_root.join("named_fn_int_to_float_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("named Integer function value for Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled named Integer function value Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_explicit_generic_function_value_runtime() {
    let temp_root = make_temp_project_root("explicit-generic-function-value-runtime");
    let source_path = temp_root.join("explicit_generic_function_value_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("explicit generic function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_alias_explicit_generic_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-alias-explicit-generic-fn-value-runtime");
    let source_path = temp_root.join("imported_alias_explicit_generic_fn_value_runtime.apex");
    let output_path = temp_root.join("imported_alias_explicit_generic_fn_value_runtime");
    let source = r#"
            function id<T>(x: T): T {
                return x;
            }

            import id as ident;

            function main(): Integer {
                f: (Integer) -> Integer = ident<Integer>;
                return if (f(7) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported alias explicit generic function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_option_some_alias_runtime() {
    let temp_root = make_temp_project_root("imported-option-some-alias-runtime");
    let source_path = temp_root.join("imported_option_some_alias_runtime.apex");
    let output_path = temp_root.join("imported_option_some_alias_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                value: Option<Integer> = Present(7);
                return if (value.unwrap() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported Option.Some alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_option_some_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-option-some-alias-fn-value-runtime");
    let source_path = temp_root.join("imported_option_some_alias_fn_value_runtime.apex");
    let output_path = temp_root.join("imported_option_some_alias_fn_value_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Present;
                value: Option<Integer> = wrap(9);
                return if (value.unwrap() == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported Option.Some alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported Option.Some alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_result_ok_alias_runtime() {
    let temp_root = make_temp_project_root("imported-result-ok-alias-runtime");
    let source_path = temp_root.join("imported_result_ok_alias_runtime.apex");
    let output_path = temp_root.join("imported_result_ok_alias_runtime");
    let source = r#"
            import Result.Ok as Success;

            function main(): Integer {
                value: Result<Integer, String> = Success(5);
                return if (value.unwrap() == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("imported Result.Ok alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled imported Result.Ok alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_function_variable_retyped_to_float_return_runtime() {
    let temp_root = make_temp_project_root("fn-var-retype-float-runtime");
    let source_path = temp_root.join("fn_var_retype_float_runtime.apex");
    let output_path = temp_root.join("fn_var_retype_float_runtime");
    let source = r#"
            function one(): Integer {
                return 1;
            }

            function main(): Integer {
                g: () -> Integer = one;
                f: () -> Float = g;
                value: Float = f();
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("function variable retyped to Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled function variable retyped Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_retyped_to_integer_parameter_runtime() {
    let temp_root = make_temp_project_root("named-fn-retype-int-param-runtime");
    let source_path = temp_root.join("named_fn_retype_int_param_runtime.apex");
    let output_path = temp_root.join("named_fn_retype_int_param_runtime");
    let source = r#"
            function scale(value: Float): Float {
                return value * 2.0;
            }

            function main(): Integer {
                f: (Integer) -> Float = scale;
                result: Float = f(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("named function value retyped Integer parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled named function value retyped Integer parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_with_interface_return_runtime() {
    let temp_root = make_temp_project_root("named-fn-interface-return-runtime");
    let source_path = temp_root.join("named_fn_interface_return_runtime.apex");
    let output_path = temp_root.join("named_fn_interface_return_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Book implements Named {
                constructor() {}
                function name(): Integer { return 7; }
            }

            function build_book(): Book {
                return Book();
            }

            function main(): Integer {
                f: () -> Named = build_book;
                return if (f().name() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("named function value with interface return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled named function value interface return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_with_interface_parameter_runtime() {
    let temp_root = make_temp_project_root("named-fn-interface-param-runtime");
    let source_path = temp_root.join("named_fn_interface_param_runtime.apex");
    let output_path = temp_root.join("named_fn_interface_param_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Book implements Named {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function name(): Integer { return this.value; }
            }

            function read_name(value: Named): Integer {
                return value.name();
            }

            function main(): Integer {
                f: (Book) -> Integer = read_name;
                return if (f(Book(9)) == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("named function value with interface parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled named function value interface parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_function_variable_retyped_to_integer_parameter_runtime() {
    let temp_root = make_temp_project_root("fn-var-retype-int-param-runtime");
    let source_path = temp_root.join("fn_var_retype_int_param_runtime.apex");
    let output_path = temp_root.join("fn_var_retype_int_param_runtime");
    let source = r#"
            function scale(value: Float): Float {
                return value * 2.0;
            }

            function main(): Integer {
                g: (Float) -> Float = scale;
                f: (Integer) -> Float = g;
                result: Float = f(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("function variable retyped Integer parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled function variable retyped Integer parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_function_value_retyped_to_narrower_integer_parameter() {
    let temp_root = make_temp_project_root("fn-retype-narrower-int-param");
    let source_path = temp_root.join("fn_retype_narrower_int_param.apex");
    let output_path = temp_root.join("fn_retype_narrower_int_param");
    let source = r#"
            function truncate(value: Integer): Integer {
                return value;
            }

            function main(): Integer {
                f: (Float) -> Integer = truncate;
                return f(1.5);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("retyping function value to narrower Integer parameter should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("cannot assign"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_nested_integer_return_for_float_task_runtime() {
    let temp_root = make_temp_project_root("async-nested-int-return-float-task-runtime");
    let source_path = temp_root.join("async_nested_int_return_float_task_runtime.apex");
    let output_path = temp_root.join("async_nested_int_return_float_task_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async {
                    if (true) {
                        return 1;
                    }
                    return 2.5;
                };
                value: Float = await(task);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested async Integer return for Task<Float> should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested async Integer return Float task binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_nested_mixed_numeric_returns_runtime() {
    let temp_root = make_temp_project_root("async-nested-mixed-numeric-returns-runtime");
    let source_path = temp_root.join("async_nested_mixed_numeric_returns_runtime.apex");
    let output_path = temp_root.join("async_nested_mixed_numeric_returns_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async {
                    if (false) {
                        return 1;
                    } else {
                        return 2.5;
                    }
                };
                value: Float = await(task);
                return if (value == 2.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested async mixed numeric returns should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested async mixed numeric returns binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_loop_variable_over_integer_range_runtime() {
    let temp_root = make_temp_project_root("float-loop-var-integer-range-runtime");
    let source_path = temp_root.join("float_loop_var_integer_range_runtime.apex");
    let output_path = temp_root.join("float_loop_var_integer_range_runtime");
    let source = r#"
            function main(): Integer {
                mut total: Float = 0.0;
                for (x: Float in range(1, 4)) {
                    total = total + x;
                }
                return if (total == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Float loop variable over Integer range should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Float loop variable Integer range binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_loop_variable_over_integer_list_runtime() {
    let temp_root = make_temp_project_root("float-loop-var-integer-list-runtime");
    let source_path = temp_root.join("float_loop_var_integer_list_runtime.apex");
    let output_path = temp_root.join("float_loop_var_integer_list_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                xs.push(3);

                mut total: Float = 0.0;
                for (x: Float in xs) {
                    total = total + x;
                }

                return if (total == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Float loop variable over Integer list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Float loop variable Integer list binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_function_alias_call_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-call-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_call_runtime.apex");
    let output_path = temp_root.join("stdlib_fn_alias_call_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                value: Integer = abs(-5);
                return if (value == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("stdlib function alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib function alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_args_count_alias_call_runtime() {
    let temp_root = make_temp_project_root("args-count-alias-call-runtime");
    let source_path = temp_root.join("args_count_alias_call_runtime.apex");
    let output_path = temp_root.join("args_count_alias_call_runtime");
    let source = r#"
            import std.args.count as count;

            function main(): Integer {
                value: Integer = count();
                return if (value >= 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Args.count alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Args.count alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_function_alias_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-value-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_value_runtime.apex");
    let output_path = temp_root.join("stdlib_fn_alias_value_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                f: (Integer) -> Integer = abs;
                return if (f(-5) == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("stdlib alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_namespace_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-namespace-alias-value-runtime");
    let source_path = temp_root.join("stdlib_namespace_alias_value_runtime.apex");
    let output_path = temp_root.join("stdlib_namespace_alias_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer) -> Integer = math.abs;
                return if (f(-9) == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("stdlib namespace alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib namespace alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_function_alias_callback_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-callback-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_callback_runtime.apex");
    let output_path = temp_root.join("stdlib_fn_alias_callback_runtime");
    let source = r#"
            import std.math.abs as abs;

            function apply_twice(f: (Integer) -> Integer, x: Integer): Integer {
                return f(f(x));
            }

            function main(): Integer {
                return if (apply_twice(abs, -2) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("stdlib alias callback should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib alias callback binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_min_alias_call_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-alias-call-runtime");
    let source_path = temp_root.join("stdlib_math_min_alias_call_runtime.apex");
    let output_path = temp_root.join("stdlib_math_min_alias_call_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                return if (min(3, 1) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.min alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Math.min alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_min_alias_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-alias-value-runtime");
    let source_path = temp_root.join("stdlib_math_min_alias_value_runtime.apex");
    let output_path = temp_root.join("stdlib_math_min_alias_value_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                pick: (Integer, Integer) -> Integer = min;
                return if (pick(3, 1) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.min alias value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Math.min alias value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_min_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-mixed-fn-value-runtime");
    let source_path = temp_root.join("stdlib_math_min_mixed_fn_value_runtime.apex");
    let output_path = temp_root.join("stdlib_math_min_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pick: (Integer, Float) -> Float = math.min;
                return if (pick(3, 1.5) == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric Math.min function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric Math.min function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_max_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-max-mixed-fn-value-runtime");
    let source_path = temp_root.join("stdlib_math_max_mixed_fn_value_runtime.apex");
    let output_path = temp_root.join("stdlib_math_max_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pick: (Float, Integer) -> Float = math.max;
                return if (pick(1.5, 3) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric Math.max function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric Math.max function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_abs_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-abs-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_abs_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer) -> Float = Math.abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_abs_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-abs-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_abs_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("alias_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                f: (Integer) -> Float = abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("alias Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled alias Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_abs_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-abs-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_abs_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("namespace_alias_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer) -> Float = math.abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_min_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-min-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_min_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer, Integer) -> Float = Math.min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_min_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-min-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_min_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("alias_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                f: (Integer, Integer) -> Float = min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("alias Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled alias Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_min_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-min-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_min_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("namespace_alias_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer, Integer) -> Float = math.min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_max_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-max-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_max_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer, Integer) -> Float = Math.max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_max_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-max-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_max_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("alias_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.max as max;

            function main(): Integer {
                f: (Integer, Integer) -> Float = max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("alias Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled alias Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_max_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-max-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_max_widened_return_fn_value_runtime.apex");
    let output_path = temp_root.join("namespace_alias_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer, Integer) -> Float = math.max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("namespace alias Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled namespace alias Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_math_pow_integer_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-math-pow-int-fn-value-runtime");
    let source_path = temp_root.join("builtin_math_pow_int_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_math_pow_int_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pow_ints: (Integer, Integer) -> Float = math.pow;
                return if (pow_ints(2, 3) == 8.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer Math.pow function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled integer Math.pow function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_math_pow_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-math-pow-mixed-fn-value-runtime");
    let source_path = temp_root.join("builtin_math_pow_mixed_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_math_pow_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pow_mixed: (Integer, Float) -> Float = math.pow;
                return if (pow_mixed(9, 0.5) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric Math.pow function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric Math.pow function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_random_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-random-fn-value-runtime");
    let source_path = temp_root.join("direct_math_random_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_random_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = Math.random;
                return if (f() >= 0.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Math.random function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Math.random function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_pi_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-pi-fn-value-runtime");
    let source_path = temp_root.join("direct_math_pi_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_pi_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = Math.pi;
                return if (f() > 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Math.pi function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Math.pi function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_sqrt_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-sqrt-fn-value-runtime");
    let source_path = temp_root.join("direct_math_sqrt_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_math_sqrt_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: (Integer) -> Float = Math.sqrt;
                return if (f(9) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Math.sqrt function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Math.sqrt function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_system_cwd_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-system-cwd-fn-value-runtime");
    let source_path = temp_root.join("direct_system_cwd_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_system_cwd_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> String = System.cwd;
                return if (f() != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct System.cwd function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct System.cwd function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_system_os_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-system-os-fn-value-runtime");
    let source_path = temp_root.join("direct_system_os_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_system_os_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> String = System.os;
                return if (f() != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct System.os function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct System.os function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_time_unix_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-time-unix-fn-value-runtime");
    let source_path = temp_root.join("direct_time_unix_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_time_unix_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Integer = Time.unix;
                return if (f() >= 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Time.unix function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Time.unix function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_time_sleep_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-time-sleep-fn-value-runtime");
    let source_path = temp_root.join("direct_time_sleep_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_time_sleep_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: (Integer) -> None = Time.sleep;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Time.sleep function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Time.sleep function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_args_count_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-args-count-fn-value-runtime");
    let source_path = temp_root.join("direct_args_count_fn_value_runtime.apex");
    let output_path = temp_root.join("direct_args_count_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Integer = Args.count;
                return if (f() >= 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Args.count function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Args.count function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_to_float_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-float-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_float_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_to_float_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                conv: (Integer) -> Float = to_float;
                return if (conv(3) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_float function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_float function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_to_int_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-int-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_int_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_to_int_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                conv: (Float) -> Integer = to_int;
                return if (conv(3.9) == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_int function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_int function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_to_string_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-string-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_string_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_to_string_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                render: (Boolean) -> String = to_string;
                return if (render(true) == "true") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("to_string function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled to_string function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_mixed_numeric_assert_eq_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-mixed-assert-eq-fn-value-runtime");
    let source_path = temp_root.join("builtin_mixed_assert_eq_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_mixed_assert_eq_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Float) -> None = assert_eq;
                check(4, 4.0);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric assert_eq function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric assert_eq function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_mixed_numeric_assert_ne_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-mixed-assert-ne-fn-value-runtime");
    let source_path = temp_root.join("builtin_mixed_assert_ne_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_mixed_assert_ne_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Float, Integer) -> None = assert_ne;
                check(4.5, 4);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mixed numeric assert_ne function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mixed numeric assert_ne function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_eq_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-eq-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_eq_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_assert_eq_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Integer) -> None = assert_eq;
                check(4, 4);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("assert_eq function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled assert_eq function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_assert_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure: (Boolean) -> None = assert;
                ensure(true);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("assert function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled assert function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_ne_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-ne-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_ne_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_assert_ne_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Integer) -> None = assert_ne;
                check(4, 5);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("assert_ne function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled assert_ne function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_true_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-true-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_true_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_assert_true_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure_true: (Boolean) -> None = assert_true;
                ensure_true(true);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("assert_true function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled assert_true function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_false_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-false-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_false_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_assert_false_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure_false: (Boolean) -> None = assert_false;
                ensure_false(false);
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("assert_false function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled assert_false function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_fail_no_arg_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-fail-no-arg-fn-value-runtime");
    let source_path = temp_root.join("builtin_fail_no_arg_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_fail_no_arg_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                stop_now: () -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("fail() no-arg function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled fail() no-arg function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_fail_string_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-fail-string-fn-value-runtime");
    let source_path = temp_root.join("builtin_fail_string_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_fail_string_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                stop_with: (String) -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("fail(String) function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled fail(String) function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_exit_function_value_check_runtime() {
    let temp_root = make_temp_project_root("builtin-exit-fn-value-runtime");
    let source_path = temp_root.join("builtin_exit_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_exit_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                terminate: (Integer) -> None = exit;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("exit function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled exit function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_function_value_with_string_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-fn-string-param");
    let source_path = temp_root.join("reject_builtin_assert_fn_string_param.apex");
    let output_path = temp_root.join("reject_builtin_assert_fn_string_param");
    let source = r#"
            function main(): Integer {
                ensure: (String) -> None = assert;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("assert(String) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_fn_integer_param.apex");
    let output_path = temp_root.join("reject_builtin_assert_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure: (Integer) -> None = assert;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("assert(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_fail_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-fail-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_fail_fn_integer_param.apex");
    let output_path = temp_root.join("reject_builtin_fail_fn_integer_param");
    let source = r#"
            function main(): Integer {
                stop_with: (Integer) -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("fail(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("fail"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_true_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-true-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_true_fn_integer_param.apex");
    let output_path = temp_root.join("reject_builtin_assert_true_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure_true: (Integer) -> None = assert_true;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("assert_true(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert_true"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_false_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-false-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_false_fn_integer_param.apex");
    let output_path = temp_root.join("reject_builtin_assert_false_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure_false: (Integer) -> None = assert_false;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("assert_false(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert_false"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_integer_range_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-int-range-fn-value-runtime");
    let source_path = temp_root.join("builtin_int_range_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_int_range_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                build: (Integer, Integer) -> Range<Integer> = range;
                mut values: Range<Integer> = build(1, 4);
                mut total: Integer = 0;
                while (values.has_next()) {
                    total = total + values.next();
                }
                return if (total == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer range function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled integer range function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_float_range_step_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-float-range-step-fn-value-runtime");
    let source_path = temp_root.join("builtin_float_range_step_fn_value_runtime.apex");
    let output_path = temp_root.join("builtin_float_range_step_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                build: (Float, Float, Float) -> Range<Float> = range;
                mut values: Range<Float> = build(0.0, 1.0, 0.25);
                mut total: Float = 0.0;
                while (values.has_next()) {
                    total = total + values.next();
                }
                return if (total == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("float range function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled float range function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_read_line_alias_function_value_check_runtime() {
    let temp_root = make_temp_project_root("read-line-alias-fn-value-runtime");
    let source_path = temp_root.join("read_line_alias_fn_value_runtime.apex");
    let output_path = temp_root.join("read_line_alias_fn_value_runtime");
    let source = r#"
            import std.io.read_line as read_line;

            function main(): Integer {
                reader: () -> String = read_line;
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("read_line alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled read_line alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_args_get_alias_value_runtime() {
    let temp_root = make_temp_project_root("args-get-alias-value-runtime");
    let source_path = temp_root.join("args_get_alias_value_runtime.apex");
    let output_path = temp_root.join("args_get_alias_value_runtime");
    let source = r#"
            import std.args.get as get;

            function main(): Integer {
                fetch: (Integer) -> String = get;
                value: String = fetch(0);
                return if (value != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Args.get alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Args.get alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_list_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-list-iteration-runtime");
    let source_path = temp_root.join("borrowed_list_iteration_runtime.apex");
    let output_path = temp_root.join("borrowed_list_iteration_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                view: &List<Integer> = &xs;
                mut total: Integer = 0;
                for (x in view) {
                    total = total + x;
                }
                return if (total == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed list iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed list iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_range_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-range-iteration-runtime");
    let source_path = temp_root.join("borrowed_range_iteration_runtime.apex");
    let output_path = temp_root.join("borrowed_range_iteration_runtime");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(0, 3);
                rr: &Range<Integer> = &r;
                mut total: Integer = 0;
                for (x in rr) {
                    total = total + x;
                }
                return if (total == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed range iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed range iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_iteration_runtime() {
    let temp_root = make_temp_project_root("string-iteration-runtime");
    let source_path = temp_root.join("string_iteration_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_string_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-string-iteration-runtime");
    let source_path = temp_root.join("borrowed_string_iteration_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("borrowed string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled borrowed string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutably_borrowed_string_iteration_runtime() {
    let temp_root = make_temp_project_root("mut-borrowed-string-iteration-runtime");
    let source_path = temp_root.join("mut_borrowed_string_iteration_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("mutably borrowed string iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled mutably borrowed string iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_for_loop_sugar_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_typed_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("float-typed-integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("float_typed_integer_for_loop_sugar_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("typed integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled typed integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_zero_length_integer_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("zero-length-integer-for-loop-sugar-runtime");
    let source_path = temp_root.join("zero_length_integer_for_loop_sugar_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("zero-length integer for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-length integer for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_expression_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-expression-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_expression_for_loop_sugar_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer expression for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled integer expression for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_call_for_loop_sugar_runtime() {
    let temp_root = make_temp_project_root("integer-call-for-loop-sugar-runtime");
    let source_path = temp_root.join("integer_call_for_loop_sugar_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("integer call for-loop sugar should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled integer call for-loop sugar binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_range_integer_to_float_assignment() {
    let temp_root = make_temp_project_root("reject-range-int-float-assignment");
    let source_path = temp_root.join("reject_range_int_float_assignment.apex");
    let output_path = temp_root.join("reject_range_int_float_assignment");
    let source = r#"
            function main(): None {
                values: Range<Float> = range(1, 3);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Range<Integer> should not typecheck as Range<Float>");
    assert!(
        err.contains("Type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_option_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-option-int-float-argument");
    let source_path = temp_root.join("reject_option_int_float_argument.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Option<Integer> should not typecheck as Option<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_task_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-task-int-float-argument");
    let source_path = temp_root.join("reject_task_int_float_argument.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Task<Integer> should not typecheck as Task<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_map_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-map-int-float-argument");
    let source_path = temp_root.join("reject_map_int_float_argument.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Map<String, Integer> should not typecheck as Map<String, Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_list_integer_to_float_argument() {
    let temp_root = make_temp_project_root("reject-list-int-float-argument");
    let source_path = temp_root.join("reject_list_int_float_argument.apex");
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

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("List<Integer> should not typecheck as List<Float>");
    assert!(
        err.contains("Argument type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_option_integer_to_float_return() {
    let temp_root = make_temp_project_root("reject-option-int-float-return");
    let source_path = temp_root.join("reject_option_int_float_return.apex");
    let output_path = temp_root.join("reject_option_int_float_return");
    let source = r#"
            function produce(): Option<Float> {
                return Option.some(1);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Option<Integer> return should not typecheck as Option<Float>");
    assert!(
        err.contains("Return type mismatch"),
        "unexpected error output: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_if_expression_join_between_integer_and_float_ranges() {
    let temp_root = make_temp_project_root("reject-if-range-join-int-float");
    let source_path = temp_root.join("reject_if_range_join_int_float.apex");
    let output_path = temp_root.join("reject_if_range_join_int_float");
    let source = r#"
            function main(): None {
                cond: Boolean = true;
                values: Range<Float> = if (cond) { range(1, 3); } else { range(2.0, 4.0); };
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("Range<Integer> and Range<Float> branches should not join");
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
    let source_path = temp_root.join("list_index_assign_oob_runtime.apex");
    let output_path = temp_root.join("list_index_assign_oob_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs[10] = 24;
                return 24;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("out-of-bounds list assignment should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list assignment oob binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_index_assignment() {
    let temp_root = make_temp_project_root("list-index-assign-negative-runtime");
    let source_path = temp_root.join("list_index_assign_negative_runtime.apex");
    let output_path = temp_root.join("list_index_assign_negative_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                xs[index] = 25;
                return 25;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("negative list assignment should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled negative list assignment binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_missing_map_index_object_results() {
    let temp_root = make_temp_project_root("map-index-missing-object-runtime");
    let source_path = temp_root.join("map_index_missing_object_runtime.apex");
    let output_path = temp_root.join("map_index_missing_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m[1].value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("missing map index object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled missing map index object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_get_object_results() {
    let temp_root = make_temp_project_root("list-get-empty-object-runtime");
    let source_path = temp_root.join("list_get_empty_object_runtime.apex");
    let output_path = temp_root.join("list_get_empty_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.get(0).value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("empty list.get object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled empty list.get object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_pop_object_results() {
    let temp_root = make_temp_project_root("list-pop-empty-object-runtime");
    let source_path = temp_root.join("list_pop_empty_object_runtime.apex");
    let output_path = temp_root.join("list_pop_empty_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.pop().value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("empty list.pop object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled empty list.pop object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_get_index() {
    let temp_root = make_temp_project_root("list-get-negative-index-runtime");
    let source_path = temp_root.join("list_get_negative_index_runtime.apex");
    let output_path = temp_root.join("list_get_negative_index_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                return xs.get(index);
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("negative list.get index should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled negative list.get binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_index_operator() {
    let temp_root = make_temp_project_root("list-index-negative-runtime");
    let source_path = temp_root.join("list_index_negative_runtime.apex");
    let output_path = temp_root.join("list_index_negative_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                return xs[index];
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("negative list index operator should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled negative list index operator binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_index_operator() {
    let temp_root = make_temp_project_root("string-index-runtime");
    let source_path = temp_root.join("string_index_runtime.apex");
    let output_path = temp_root.join("string_index_runtime");
    let source = r#"
            function main(): Integer {
                c: Char = "abc"[1];
                if (c == 'b') { return 19; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string index operator should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string index binary");
    assert_eq!(status.code(), Some(19));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_main_with_string_return_type() {
    let temp_root = make_temp_project_root("main-string-return-type");
    let source_path = temp_root.join("main_string_return_type.apex");
    let output_path = temp_root.join("main_string_return_type");
    let source = r#"
            function main(): String {
                return "oops";
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("main string return type should fail before codegen");
    assert!(
        err.to_string()
            .contains("main() must return None or Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_file_no_check_rejects_main_with_string_return_type_cleanly() {
    let temp_root = make_temp_project_root("main-string-return-type-nocheck");
    let source_path = temp_root.join("main_string_return_type_nocheck.apex");
    let source = r#"
            function main(): String {
                return "oops";
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_file(&source_path, None, false, false, None, None)
        .expect_err("unchecked main string return type should fail before codegen");
    assert!(err.contains("main() must return None or Integer"), "{err}");
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_main_with_parameters() {
    let temp_root = make_temp_project_root("main-parameters");
    let source_path = temp_root.join("main_parameters.apex");
    let output_path = temp_root.join("main_parameters");
    let source = r#"
            function main(x: Integer): Integer {
                return x;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("main parameters should fail before codegen");
    assert!(
        err.to_string().contains("main() cannot declare parameters"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_try_outside_result_or_option_return_context() {
    let temp_root = make_temp_project_root("try-invalid-return-context");
    let source_path = temp_root.join("try_invalid_return_context.apex");
    let output_path = temp_root.join("try_invalid_return_context");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function helper(): Integer {
                value: Integer = choose()?;
                return value;
            }

            function main(): Integer {
                return helper();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("invalid try return context should fail before codegen");
    assert!(
        err.contains("'?' on Result requires the enclosing function to return Result"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_try_inside_lambda_even_with_outer_result_return() {
    let temp_root = make_temp_project_root("try-invalid-lambda-context");
    let source_path = temp_root.join("try_invalid_lambda_context.apex");
    let output_path = temp_root.join("try_invalid_lambda_context");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function wrap(): Result<Integer, String> {
                f: () -> Integer = () => choose()?;
                return Result.ok(f());
            }

            function main(): Integer {
                return wrap().unwrap();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect_err("invalid try inside lambda should fail before codegen");
    assert!(
        err.contains("'?' on Result requires the enclosing function to return Result"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_invalid_opt_level() {
    let temp_root = make_temp_project_root("compile-invalid-opt");
    let source_path = temp_root.join("invalid_opt.apex");
    let output_path = temp_root.join("invalid_opt");
    let source = "function main(): None { return None; }\n";

    let err = compile_source(
        source,
        &source_path,
        &output_path,
        true,
        true,
        Some("turbo"),
        None,
    )
    .expect_err("invalid opt level should be rejected");

    assert!(err.contains("Invalid optimization level"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_out_of_bounds_string_index_operator() {
    let temp_root = make_temp_project_root("string-index-oob-runtime");
    let source_path = temp_root.join("string_index_oob_runtime.apex");
    let output_path = temp_root.join("string_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 10;
                c: Char = "abc"[idx];
                return 20;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("out-of-bounds string index should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string index oob binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_literal_index_operator() {
    let temp_root = make_temp_project_root("unicode-string-index-runtime");
    let source_path = temp_root.join("unicode_string_index_runtime.apex");
    let output_path = temp_root.join("unicode_string_index_runtime");
    let source = r#"
            function main(): Integer {
                c: Char = "🚀"[0];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string literal index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_literal_index_operator_with_dynamic_index() {
    let temp_root = make_temp_project_root("unicode-string-dynamic-index-runtime");
    let source_path = temp_root.join("unicode_string_dynamic_index_runtime.apex");
    let output_path = temp_root.join("unicode_string_dynamic_index_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 0;
                c: Char = "🚀"[idx];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string literal dynamic index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode dynamic string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_unicode_string_literal_index_operator_past_char_len() {
    let temp_root = make_temp_project_root("unicode-string-index-oob-runtime");
    let source_path = temp_root.join("unicode_string_index_oob_runtime.apex");
    let output_path = temp_root.join("unicode_string_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 1;
                c: Char = "🚀"[idx];
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string literal oob dynamic index should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled unicode dynamic string index oob binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("String index out of bounds\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_variable_index_operator() {
    let temp_root = make_temp_project_root("unicode-string-variable-index-runtime");
    let source_path = temp_root.join("unicode_string_variable_index_runtime.apex");
    let output_path = temp_root.join("unicode_string_variable_index_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                idx: Integer = 0;
                c: Char = s[idx];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string variable index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode string variable index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_literal_length() {
    let temp_root = make_temp_project_root("unicode-string-literal-length-runtime");
    let source_path = temp_root.join("unicode_string_literal_length_runtime.apex");
    let output_path = temp_root.join("unicode_string_literal_length_runtime");
    let source = r#"
            function main(): Integer {
                return if ("🚀".length() == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string literal length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode string literal length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_variable_length() {
    let temp_root = make_temp_project_root("unicode-string-variable-length-runtime");
    let source_path = temp_root.join("unicode_string_variable_length_runtime.apex");
    let output_path = temp_root.join("unicode_string_variable_length_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                return if (s.length() == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string variable length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode string variable length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_system_exec_large_output() {
    let temp_root = make_temp_project_root("system-exec-large-output-runtime");
    let source_path = temp_root.join("system_exec_large_output_runtime.apex");
    let output_path = temp_root.join("system_exec_large_output_runtime");
    let command = if cfg!(windows) {
        r#"powershell -NoProfile -Command "$s='x'*5000; Write-Output $s""#
    } else {
        r#"python3 -c "print('x' * 5000)""#
    };
    let escaped_command = command.replace('\\', "\\\\").replace('"', "\\\"");
    let source = format!(
        r#"
                import std.system.*;

                function main(): Integer {{
                    out: String = System.exec("{escaped_command}");
                    return if (out.length() > 4500) {{ 0; }} else {{ 1; }};
                }}
            "#
    );

    fs::write(&source_path, source).expect("write source");
    compile_source(
        &fs::read_to_string(&source_path).expect("read source"),
        &source_path,
        &output_path,
        false,
        true,
        None,
        None,
    )
    .expect("large System.exec output should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled large System.exec binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_system_exec_nul_bytes() {
    let temp_root = make_temp_project_root("system-exec-nul-bytes-runtime");
    let source_path = temp_root.join("system_exec_nul_bytes_runtime.apex");
    let output_path = temp_root.join("system_exec_nul_bytes_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                out: String = System.exec("python3 -c \"import sys; sys.stdout.buffer.write(b'A\\x00B')\"");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("System.exec NUL-byte failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled System.exec NUL-byte binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("System.exec() cannot load NUL bytes\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_system_exec_invalid_utf8() {
    let temp_root = make_temp_project_root("system-exec-invalid-utf8-runtime");
    let source_path = temp_root.join("system_exec_invalid_utf8_runtime.apex");
    let output_path = temp_root.join("system_exec_invalid_utf8_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                out: String = System.exec("python3 -c \"import sys; sys.stdout.buffer.write(bytes([0xff]))\"");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("System.exec invalid UTF-8 failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled System.exec invalid UTF-8 binary");
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
fn compile_source_fails_fast_on_system_getenv_invalid_utf8() {
    let temp_root = make_temp_project_root("system-getenv-invalid-utf8-runtime");
    let source_path = temp_root.join("system_getenv_invalid_utf8_runtime.apex");
    let output_path = temp_root.join("system_getenv_invalid_utf8_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                value: String = System.getenv("APEX_BAD_UTF8_ENV");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("System.getenv invalid UTF-8 failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .env(
            "APEX_BAD_UTF8_ENV",
            std::ffi::OsString::from_vec(vec![0xff]),
        )
        .output()
        .expect("run compiled System.getenv invalid UTF-8 binary");
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
fn compile_source_runs_system_shell_with_decoded_exit_code() {
    let temp_root = make_temp_project_root("system-shell-decoded-exit-code-runtime");
    let source_path = temp_root.join("system_shell_decoded_exit_code_runtime.apex");
    let output_path = temp_root.join("system_shell_decoded_exit_code_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                code: Integer = System.shell("sh -c 'exit 7'");
                return if (code == 7) { 0; } else { code; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("System.shell decoded exit code should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled System.shell decoded exit code binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_file_read_missing_path() {
    let temp_root = make_temp_project_root("file-read-missing-path-runtime");
    let source_path = temp_root.join("file_read_missing_path_runtime.apex");
    let output_path = temp_root.join("file_read_missing_path_runtime");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("missing.txt");
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("File.read missing-path failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .expect("run compiled File.read missing-path binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("File.read() failed to open file\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_time_now_with_long_format() {
    let temp_root = make_temp_project_root("time-now-long-format-runtime");
    let source_path = temp_root.join("time_now_long_format_runtime.apex");
    let output_path = temp_root.join("time_now_long_format_runtime");
    let source = r#"
            import std.time.*;

            function main(): Integer {
                out: String = Time.now("%Y-%m-%d %H:%M:%S %A %B %Y-%m-%d %H:%M:%S %A %B %Y-%m-%d %H:%M:%S %A %B");
                return if (out.length() > 40) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Time.now long format should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Time.now long format binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(target_os = "linux")]
#[test]
fn compile_source_runs_system_cwd_with_long_working_directory() {
    let temp_root = make_temp_project_root("system-cwd-long-working-directory-runtime");
    let source_path = temp_root.join("system_cwd_long_working_directory_runtime.apex");
    let output_path = temp_root.join("system_cwd_long_working_directory_runtime");
    let source = r#"
            import std.string.*;
            import std.system.*;

            function main(): Integer {
                cwd: String = System.cwd();
                return if (Str.len(cwd) > 1100) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("System.cwd deep path runtime should codegen");

    let mut deep_dir = temp_root.join("cwd-depth-root");
    fs::create_dir_all(&deep_dir).expect("create deep root");
    let segment = "a".repeat(60);
    for index in 0..18 {
        deep_dir = deep_dir.join(format!("{segment}{index}"));
        fs::create_dir_all(&deep_dir).expect("create deep segment");
    }

    let status = std::process::Command::new(&output_path)
        .current_dir(&deep_dir)
        .status()
        .expect("run compiled System.cwd binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_read_line_with_long_input() {
    let temp_root = make_temp_project_root("read-line-long-input-runtime");
    let source_path = temp_root.join("read_line_long_input_runtime.apex");
    let output_path = temp_root.join("read_line_long_input_runtime");
    let source = r#"
            import std.io.*;
            import std.string.*;

            function main(): Integer {
                line: String = read_line();
                return if (Str.len(line) > 1500) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("read_line long input should codegen");

    let mut child = std::process::Command::new(&output_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn read_line binary");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().expect("child stdin");
        writeln!(stdin, "{}", "x".repeat(2000)).expect("write long stdin");
    }
    let status = child.wait().expect("wait for read_line binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_str_len() {
    let temp_root = make_temp_project_root("unicode-str-len-runtime");
    let source_path = temp_root.join("unicode_str_len_runtime.apex");
    let output_path = temp_root.join("unicode_str_len_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                s: String = "🚀";
                return if (Str.len(s) == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode Str.len should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled unicode Str.len binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_unicode_string_variable_index_operator_past_char_len() {
    let temp_root = make_temp_project_root("unicode-string-variable-index-oob-runtime");
    let source_path = temp_root.join("unicode_string_variable_index_oob_runtime.apex");
    let output_path = temp_root.join("unicode_string_variable_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                idx: Integer = 1;
                c: Char = s[idx];
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("unicode string variable oob index should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled unicode string variable index oob binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("String index out of bounds\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_equality_on_literals() {
    let temp_root = make_temp_project_root("string-eq-literal-runtime");
    let source_path = temp_root.join("string_eq_literal_runtime.apex");
    let output_path = temp_root.join("string_eq_literal_runtime");
    let source = r#"
            function main(): Integer {
                if ("b" == "b") { return 32; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string literal equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string equality literal binary");
    assert_eq!(status.code(), Some(32));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_equality_on_expression_results() {
    let temp_root = make_temp_project_root("string-eq-expr-runtime");
    let source_path = temp_root.join("string_eq_expr_runtime.apex");
    let output_path = temp_root.join("string_eq_expr_runtime");
    let source = r#"
            import std.string.*;
            function main(): Integer {
                if (Str.concat("a", "b") == "ab") { return 33; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("string expression equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled string equality expression binary");
    assert_eq!(status.code(), Some(33));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_identity_equality() {
    let temp_root = make_temp_project_root("list-eq-runtime");
    let source_path = temp_root.join("list_eq_runtime.apex");
    let output_path = temp_root.join("list_eq_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                if (xs == xs) { return 34; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list equality binary");
    assert_eq!(status.code(), Some(34));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_constructor_with_preallocated_integer_capacity() {
    let temp_root = make_temp_project_root("list-capacity-runtime");
    let source_path = temp_root.join("list_capacity_runtime.apex");
    let output_path = temp_root.join("list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(3);
                xs.push(10);
                xs.push(20);
                xs.push(30);
                if (xs.length() == 3 && xs.get(0) == 10 && xs.get(2) == 30) {
                    return 80;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list constructor with integer capacity should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled list capacity binary");
    assert_eq!(status.code(), Some(80));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_constructor_with_preallocated_option_capacity() {
    let temp_root = make_temp_project_root("option-list-capacity-runtime");
    let source_path = temp_root.join("option_list_capacity_runtime.apex");
    let output_path = temp_root.join("option_list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Option<Integer>> = List<Option<Integer>>(2);
                xs.push(Option<Integer>());
                xs.push(Option.some(9));
                if (xs.length() == 2 && xs.get(0).is_none() && xs.get(1).unwrap() == 9) {
                    return 81;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("list constructor with option capacity should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled option list capacity binary");
    assert_eq!(status.code(), Some(81));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_runtime_on_negative_list_constructor_capacity_expression() {
    let temp_root = make_temp_project_root("negative-list-capacity-runtime");
    let source_path = temp_root.join("negative_list_capacity_runtime.apex");
    let output_path = temp_root.join("negative_list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                cap: Integer = 1 - 2;
                xs: List<Integer> = List<Integer>(cap);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("negative runtime list capacity source should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled negative list capacity binary");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("List constructor capacity cannot be negative"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_typed_option_constructor_through_function_return() {
    let temp_root = make_temp_project_root("typed-option-constructor-runtime");
    let source_path = temp_root.join("typed_option_constructor_runtime.apex");
    let output_path = temp_root.join("typed_option_constructor_runtime");
    let source = r#"
            function build(): Option<List<Option<Integer>>> {
                return Option<List<Option<Integer>>>();
            }

            function main(): Integer {
                value: Option<List<Option<Integer>>> = build();
                if (value.is_none()) {
                    return 82;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("typed option constructor return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled typed option constructor binary");
    assert_eq!(status.code(), Some(82));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_typed_result_constructor_through_function_return() {
    let temp_root = make_temp_project_root("typed-result-constructor-runtime");
    let source_path = temp_root.join("typed_result_constructor_runtime.apex");
    let output_path = temp_root.join("typed_result_constructor_runtime");
    let source = r#"
            function build(): Result<List<Option<Integer>>, String> {
                return Result<List<Option<Integer>>, String>();
            }

            function main(): Integer {
                result: Result<List<Option<Integer>>, String> = build();
                return match (result) {
                    Result.Ok(xs) => xs.length(),
                    Result.Error(err) => 83,
                };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("typed result constructor return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled typed result constructor binary");
    assert_eq!(status.code(), Some(83));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_uses_typed_heap_sizes_for_builtin_smart_pointer_constructors() {
    let temp_root = make_temp_project_root("typed-smart-pointer-ir");
    let source_path = temp_root.join("typed_smart_pointer_ir.apex");
    let output_path = temp_root.join("typed_smart_pointer_ir");
    let source = r#"
            function main(): None {
                box_value: Box<List<Option<Integer>>> = Box<List<Option<Integer>>>();
                rc_value: Rc<List<Option<Integer>>> = Rc<List<Option<Integer>>>();
                arc_value: Arc<List<Option<Integer>>> = Arc<List<Option<Integer>>>();
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("typed smart pointer constructors should codegen");

    let ir_path = output_path.with_extension("ll");
    let ir = fs::read_to_string(&ir_path).expect("read generated llvm ir");
    let malloc_24_count = ir.matches("call ptr @malloc(i64 24)").count();
    assert!(
            malloc_24_count >= 3,
            "expected Box/Rc/Arc constructors to allocate 24-byte List<Option<Integer>> payloads, found {malloc_24_count} matching malloc calls in {}",
            ir_path.display()
        );
    assert!(
        !ir.contains("call ptr @malloc(i64 8)"),
        "builtin smart pointer constructors should not use hard-coded 8-byte payload allocations"
    );
    assert!(
        !ir.contains("call ptr @malloc(i64 16)"),
        "builtin smart pointer constructors should not use hard-coded 16-byte payload allocations"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_raw_ptr_deref_with_float_payloads() {
    let temp_root = make_temp_project_root("ptr-float-deref-codegen");
    let source_path = temp_root.join("ptr_float_deref_codegen.apex");
    let output_path = temp_root.join("ptr_float_deref_codegen");
    let source = r#"
            function load(slot: Ptr<Float>): Float {
                return *slot;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("raw Ptr<Float> deref should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_await_timeout_method_chain() {
    let temp_root = make_temp_project_root("async-block-await-timeout-runtime");
    let source_path = temp_root.join("async_block_await_timeout_runtime.apex");
    let output_path = temp_root.join("async_block_await_timeout_runtime");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = (async { 7 }).await_timeout(100);
                return if (value.unwrap() == 7) { 84 } else { 0 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("async block await_timeout chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled async block await_timeout binary");
    assert_eq!(status.code(), Some(84));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_await_timeout_zero_pending_runtime() {
    let temp_root = make_temp_project_root("await-timeout-zero-pending-runtime");
    let source_path = temp_root.join("await_timeout_zero_pending_runtime.apex");
    let output_path = temp_root.join("await_timeout_zero_pending_runtime");
    let source = r#"
            import std.time.*;

            function work(): Task<Integer> {
                return async {
                    Time.sleep(50);
                    return 7;
                };
            }

            function main(): Integer {
                maybe: Option<Integer> = work().await_timeout(0);
                return if (maybe.is_none()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("await_timeout(0) on pending task should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled await_timeout zero pending binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_async_block_is_done_method_chain() {
    let temp_root = make_temp_project_root("async-block-is-done-codegen");
    let source_path = temp_root.join("async_block_is_done_codegen.apex");
    let output_path = temp_root.join("async_block_is_done_codegen");
    let source = r#"
            function main(): None {
                done: Boolean = (async { 1 }).is_done();
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("async block is_done chain should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_identity_equality() {
    let temp_root = make_temp_project_root("map-eq-runtime");
    let source_path = temp_root.join("map_eq_runtime.apex");
    let output_path = temp_root.join("map_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(2));
                if (m == m) { return 35; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("map identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled map equality binary");
    assert_eq!(status.code(), Some(35));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_class_identity_equality() {
    let temp_root = make_temp_project_root("class-eq-runtime");
    let source_path = temp_root.join("class_eq_runtime.apex");
    let output_path = temp_root.join("class_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(2);
                if (b == b) { return 36; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("class identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled class equality binary");
    assert_eq!(status.code(), Some(36));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_unwrap_object_identity_equality() {
    let temp_root = make_temp_project_root("option-unwrap-object-eq-runtime");
    let source_path = temp_root.join("option_unwrap_object_eq_runtime.apex");
    let output_path = temp_root.join("option_unwrap_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(3);
                x: Option<Boxed> = Option.some(b);
                if (x.unwrap() == b) { return 37; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Option.unwrap object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Option.unwrap object equality binary");
    assert_eq!(status.code(), Some(37));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_get_object_identity_equality() {
    let temp_root = make_temp_project_root("map-get-object-eq-runtime");
    let source_path = temp_root.join("map_get_object_eq_runtime.apex");
    let output_path = temp_root.join("map_get_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(4);
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, b);
                if (m.get(1) == b) { return 38; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("Map.get object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled Map.get object equality binary");
    assert_eq!(status.code(), Some(38));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_await_timeout_unwrap_object_identity_equality() {
    let temp_root = make_temp_project_root("await-timeout-unwrap-object-eq-runtime");
    let source_path = temp_root.join("await_timeout_unwrap_object_eq_runtime.apex");
    let output_path = temp_root.join("await_timeout_unwrap_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            async function work(): Boxed {
                return Boxed(5);
            }

            function main(): Integer {
                b: Boxed = work().await_timeout(100).unwrap();
                if (b == b) { return 39; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("await_timeout unwrap object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled await_timeout unwrap object equality binary");
    assert_eq!(status.code(), Some(39));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_range_method_calls() {
    let temp_root = make_temp_project_root("direct-range-method-runtime");
    let source_path = temp_root.join("direct_range_method_runtime.apex");
    let output_path = temp_root.join("direct_range_method_runtime");
    let source = r#"
            function main(): Integer {
                if (range(0, 10).has_next()) { return 40; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct range method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct range method binary");
    assert_eq!(status.code(), Some(40));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_some_method_chains() {
    let temp_root = make_temp_project_root("direct-option-some-method-runtime");
    let source_path = temp_root.join("direct_option_some_method_runtime.apex");
    let output_path = temp_root.join("direct_option_some_method_runtime");
    let source = r#"
            function main(): Integer {
                if (Option.some(12).unwrap() == 12) { return 41; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Option.some method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Option.some method binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_some_object_method_chains() {
    let temp_root = make_temp_project_root("direct-option-some-object-method-runtime");
    let source_path = temp_root.join("direct_option_some_object_method_runtime.apex");
    let output_path = temp_root.join("direct_option_some_object_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Option.some(Boxed(14)).unwrap().value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Option.some object method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Option.some object method binary");
    assert_eq!(status.code(), Some(14));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_method_chains() {
    let temp_root = make_temp_project_root("direct-result-ok-method-runtime");
    let source_path = temp_root.join("direct_result_ok_method_runtime.apex");
    let output_path = temp_root.join("direct_result_ok_method_runtime");
    let source = r#"
            function main(): Integer {
                if (Result.ok(12).unwrap() == 12) { return 42; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.ok method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.ok method binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_object_method_chains() {
    let temp_root = make_temp_project_root("direct-result-ok-object-method-runtime");
    let source_path = temp_root.join("direct_result_ok_object_method_runtime.apex");
    let output_path = temp_root.join("direct_result_ok_object_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Result.ok(Boxed(15)).unwrap().value;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.ok object method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.ok object method binary");
    assert_eq!(status.code(), Some(15));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_constructor_method_calls() {
    let temp_root = make_temp_project_root("direct-ctor-method-runtime");
    let source_path = temp_root.join("direct_ctor_method_runtime.apex");
    let output_path = temp_root.join("direct_ctor_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): Integer {
                return Boxed(23).get();
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct constructor method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct constructor method binary");
    assert_eq!(status.code(), Some(23));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_integer_equality() {
    let temp_root = make_temp_project_root("direct-result-error-int-eq-runtime");
    let source_path = temp_root.join("direct_result_error_int_eq_runtime.apex");
    let output_path = temp_root.join("direct_result_error_int_eq_runtime");
    let source = r#"
            function main(): Integer {
                e: Integer = 7;
                if (Result.error(e) == Result.error(e)) { return 43; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.error integer equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.error integer equality binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_object_identity_equality() {
    let temp_root = make_temp_project_root("direct-result-error-object-eq-runtime");
    let source_path = temp_root.join("direct_result_error_object_eq_runtime.apex");
    let output_path = temp_root.join("direct_result_error_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                e: Boxed = Boxed(9);
                if (Result.error(e) == Result.error(e)) { return 44; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("direct Result.error object equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled direct Result.error object equality binary");
    assert_eq!(status.code(), Some(44));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_match_result_error_equality_against_static_constructor() {
    let temp_root = make_temp_project_root("match-result-error-static-eq-runtime");
    let source_path = temp_root.join("match_result_error_static_eq_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("match-result static equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled match-result static equality binary");
    assert_eq!(status.code(), Some(45));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_ultra_nested_option_result_static_equality() {
    let temp_root = make_temp_project_root("ultra-nested-option-result-static-eq-runtime");
    let source_path = temp_root.join("ultra_nested_option_result_static_eq_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("ultra-nested option/result static equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled ultra-nested option/result static equality binary");
    assert_eq!(status.code(), Some(46));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_if_merge_option_none_method_chain() {
    let temp_root = make_temp_project_root("if-merge-option-none-method-runtime");
    let source_path = temp_root.join("if_merge_option_none_method_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("if-merge option none method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if-merge option none method binary");
    assert_eq!(status.code(), Some(47));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_if_match_if_tagged_merge_method_chain() {
    let temp_root = make_temp_project_root("nested-if-match-if-tagged-merge-runtime");
    let source_path = temp_root.join("nested_if_match_if_tagged_merge_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("nested if-match-if tagged merge should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled nested if-match-if tagged merge binary");
    assert_eq!(status.code(), Some(48));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_chaotic_unreachable_tagged_branch_chain() {
    let temp_root = make_temp_project_root("chaotic-unreachable-tagged-branch-runtime");
    let source_path = temp_root.join("chaotic_unreachable_tagged_branch_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("chaotic unreachable tagged branch chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled chaotic unreachable tagged branch binary");
    assert_eq!(status.code(), Some(49));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_index_object_results() {
    let temp_root = make_temp_project_root("list-index-empty-object-runtime");
    let source_path = temp_root.join("list_index_empty_object_runtime.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("empty list index object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled empty list index object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_lambda_callee_calls() {
    let temp_root = make_temp_project_root("lambda-callee-codegen");
    let source_path = temp_root.join("lambda_callee.apex");
    let output_path = temp_root.join("lambda_callee");
    let source = r#"
            function main(): None {
                x: Integer = ((y: Integer) => y + 1)(2);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("lambda callee codegen should succeed");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_indexed_function_value_callees() {
    let temp_root = make_temp_project_root("indexed-function-callee-codegen");
    let source_path = temp_root.join("indexed_function_callee.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("indexed function-value callee should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_if_expression_function_value_callees() {
    let temp_root = make_temp_project_root("ifexpr-function-callee-codegen");
    let source_path = temp_root.join("ifexpr_function_callee.apex");
    let output_path = temp_root.join("ifexpr_function_callee");
    let source = r#"
            function inc(x: Integer): Integer { return x + 1; }
            function dec(x: Integer): Integer { return x - 1; }

            function main(): None {
                x: Integer = (if (true) { inc; } else { dec; })(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("if-expression function-value callee should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn formatted_async_block_tail_expression_preserves_runtime_behavior() {
    let temp_root = make_temp_project_root("formatted-async-block-tail-runtime");
    let source_path = temp_root.join("formatted_async_block_tail_runtime.apex");
    let output_path = temp_root.join("formatted_async_block_tail_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Integer> = async {
                    7
                };
                return await(task);
            }
        "#;

    let formatted = formatter::format_source(source).expect("format source");
    fs::write(&source_path, &formatted).expect("write formatted source");
    compile_source(
        &formatted,
        &source_path,
        &output_path,
        false,
        true,
        None,
        None,
    )
    .expect("formatted async tail-expression source should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run formatted async tail-expression binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_function_valued_field_calls() {
    let temp_root = make_temp_project_root("function-field-call-codegen");
    let source_path = temp_root.join("function_field_call.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("function-valued field calls should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_generic_method_returning_lambda() {
    let temp_root = make_temp_project_root("generic-method-lambda-codegen");
    let source_path = temp_root.join("generic_method_lambda.apex");
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

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .expect("generic method returning lambda should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_zero_arg_pipe_lambda_runtime() {
    let temp_root = make_temp_project_root("zero-arg-pipe-lambda-runtime");
    let source_path = temp_root.join("zero_arg_pipe_lambda_runtime.apex");
    let output_path = temp_root.join("zero_arg_pipe_lambda_runtime");
    let source = r#"
            function make(): () -> Integer { return || 7; }

            function main(): Integer {
                f: () -> Integer = make();
                return if (f() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("zero-arg pipe lambda should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg pipe lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_method_returning_zero_arg_pipe_lambda_runtime() {
    let temp_root = make_temp_project_root("generic-method-zero-arg-pipe-lambda-runtime");
    let source_path = temp_root.join("generic_method_zero_arg_pipe_lambda_runtime.apex");
    let output_path = temp_root.join("generic_method_zero_arg_pipe_lambda_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function lift(): () -> T { return || this.value; }
            }

            function main(): Integer {
                b: Box<String> = Box<String>("ok");
                f: () -> () -> String = b.lift;
                return if (f()().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("generic method returning zero-arg pipe lambda should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled generic method zero-arg pipe lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_entry_namespace_module_named_main_runtime() {
    let temp_root = make_temp_project_root("entry-namespace-module-main-runtime");
    let source_path = temp_root.join("entry_namespace_module_main_runtime.apex");
    let output_path = temp_root.join("entry_namespace_module_main_runtime");
    let source = r#"
package core;

module main {
    function ping(): Integer { return 22; }
}

function main(): Integer {
    return main.ping();
}
"#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("entry namespace module named main should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled entry namespace module named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_entry_namespace_class_named_main_runtime() {
    let temp_root = make_temp_project_root("entry-namespace-class-main-runtime");
    let source_path = temp_root.join("entry_namespace_class_main_runtime.apex");
    let output_path = temp_root.join("entry_namespace_class_main_runtime");
    let source = r#"
package core;

class main {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value; }
}

function main(): Integer {
    value: main = main(22);
    return value.get();
}
"#;

    fs::write(&source_path, source).expect("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .expect("entry namespace class named main should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled entry namespace class named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
