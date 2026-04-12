use super::*;
use std::fs;

#[test]
fn compile_source_runs_unique_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("interface-method-dispatch-runtime");
    let source_path = temp_root.join("interface_method_dispatch_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("single-implementation interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_module_local_import_alias_leaking_to_top_level() {
    let temp_root = make_temp_project_root("module-local-import-alias-leak-top-level");
    let source_path = temp_root.join("module_local_import_alias_leak_top_level.arden");
    let output_path = temp_root.join("module_local_import_alias_leak_top_level");
    let source = r#"
            import std.io.*;

            module Inner {
                import std.math as math;

                function keep(): None {
                    println(to_string(math.abs(-1.0)));
                    return None;
                }
            }

            function main(): None {
                value: Float = math.abs(-1.0);
                println(to_string(value));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("module-local alias should not resolve at top level");
    assert!(
        err.contains("Unknown type: math")
            || err.contains("Undefined variable: math")
            || err.contains("Unknown namespace alias usage 'math.abs'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unique_interface_bound_method_value_runtime() {
    let temp_root = make_temp_project_root("interface-bound-method-value-runtime");
    let source_path = temp_root.join("interface_bound_method_value_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("single-implementation interface bound method value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled interface bound method value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_method_wrong_arity_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-method-wrong-arity");
    let source_path = temp_root.join("no_check_interface_method_wrong_arity.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("interface method wrong arity should fail in codegen");
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
    let source_path = temp_root.join("no_check_missing_interface_method.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing interface method should fail in codegen");
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
    let source_path = temp_root.join("no_check_missing_interface_bound_method.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing interface bound method should fail in codegen");
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
    let source_path = temp_root.join("no_check_interface_non_implementor_dispatch.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-implementor interface dispatch should fail in codegen");
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
    let source_path = temp_root.join("no_check_interface_non_implementor_bound_method.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-implementor interface bound method should fail in codegen");
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
    let source_path = temp_root.join("no_check_interface_bound_method_signature_mismatch.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("interface bound method signature mismatch should fail in codegen");
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
    let source_path = temp_root.join("generic_bound_constructor_dispatch_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("generic bound constructor dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run generic bound constructor dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-method-dispatch-runtime");
    let source_path = temp_root.join("multi_bound_interface_method_dispatch_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("multi-bound interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run multi-bound interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_bound_method_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-bound-method-runtime");
    let source_path = temp_root.join("multi_bound_interface_bound_method_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("multi-bound interface bound method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run multi-bound interface bound method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_interface_implements_runtime() {
    let temp_root = make_temp_project_root("generic-interface-implements-runtime");
    let source_path = temp_root.join("generic_interface_implements_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic interface implements clause should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic interface implements binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_specialized_parent_interface_method_runtime() {
    let temp_root = make_temp_project_root("specialized-parent-interface-runtime");
    let source_path = temp_root.join("specialized_parent_interface_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("specialized parent interface methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled specialized parent interface binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_integer_payloads_for_float_option_and_result() {
    let temp_root = make_temp_project_root("reject-int-to-float-option-result");
    let source_path = temp_root.join("reject_int_to_float_option_result.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Option/Result payloads should stay invariant across Integer/Float");
    assert!(err.contains("Type mismatch"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_invalid_to_int_and_to_float_argument_types() {
    let temp_root = make_temp_project_root("invalid-to-int-to-float-types");
    let source_path = temp_root.join("invalid_to_int_to_float_types.arden");
    let output_path = temp_root.join("invalid_to_int_to_float_types");
    let source = r#"
            function main(): Integer {
                a: Integer = to_int(true);
                b: Float = to_float("8");
                return a + to_int(b);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid to_int/to_float argument types should fail");
    assert!(err.contains("to_int") || err.contains("to_float"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_list_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_list_ctor_arity.arden");
    let output_path = temp_root.join("no_check_invalid_list_ctor_arity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(1, 2);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid list constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects 0 or 1 arguments, got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_capacity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-capacity-type");
    let source_path = temp_root.join("no_check_invalid_list_capacity_type.arden");
    let output_path = temp_root.join("no_check_invalid_list_capacity_type");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>("bad");
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-integer list capacity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects optional Integer capacity, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_map_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-map-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_map_ctor_arity.arden");
    let output_path = temp_root.join("no_check_invalid_map_ctor_arity");
    let source = r#"
            function main(): Integer {
                items: Map<String, Integer> = Map<String, Integer>(1);
                return items.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid map constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor Map<String, Integer> expects 0 arguments, got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_method_receiver() {
    let temp_root = make_temp_project_root("no-check-unknown-method-receiver-primary-error");
    let source_path = temp_root.join("no_check_unknown_method_receiver_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_method_receiver_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown method receiver should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");
    assert!(
        !err.contains("Cannot determine object type for method call"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_import_alias_leaking_to_top_level() {
    let temp_root = make_temp_project_root("no-check-module-local-import-alias-leak");
    let source_path = temp_root.join("no_check_module_local_import_alias_leak.arden");
    let output_path = temp_root.join("no_check_module_local_import_alias_leak");
    let source = r#"
            module Inner {
                import std.math as math;
                function keep(): Float {
                    return math.abs(-1.0);
                }
            }

            function main(): Float {
                return math.abs(-1.0);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local alias should not resolve at top level in no-check mode");
    assert!(
        err.contains("Undefined variable: math") || err.contains("Unknown type: math"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_wildcard_import_leaking_to_top_level() {
    let temp_root = make_temp_project_root("no-check-module-local-wildcard-import-leak");
    let source_path = temp_root.join("no_check_module_local_wildcard_import_leak.arden");
    let output_path = temp_root.join("no_check_module_local_wildcard_import_leak");
    let source = r#"
            module Inner {
                import std.math.*;
                function keep(): Float {
                    return abs(-1.0);
                }
            }

            function main(): Float {
                return abs(-1.0);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local wildcard import should not resolve at top level in no-check mode");
    assert!(err.contains("Undefined function: abs"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_field_root() {
    let temp_root = make_temp_project_root("no-check-unknown-field-root-primary-error");
    let source_path = temp_root.join("no_check_unknown_field_root_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_field_root_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown field root should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_undefined_root_before_read_or_method_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-undefined-root-read-method");
    let read_source_path = temp_root.join("no_check_nested_undefined_root_read.arden");
    let read_output_path = temp_root.join("no_check_nested_undefined_root_read");
    let read_source = r#"
            function main(): None {
                println(missing.inner.items[0]);
                return None;
            }
        "#;

    fs::write(&read_source_path, read_source).must("write read source");
    let read_err = compile_source(
        read_source,
        &read_source_path,
        &read_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested undefined-root read should fail in codegen");
    assert!(
        read_err.contains("Undefined variable: missing"),
        "{read_err}"
    );

    let method_source_path = temp_root.join("no_check_nested_undefined_root_method.arden");
    let method_output_path = temp_root.join("no_check_nested_undefined_root_method");
    let method_source = r#"
            function main(): None {
                missing.inner.items.push(1);
                return None;
            }
        "#;

    fs::write(&method_source_path, method_source).must("write method source");
    let method_err = compile_source(
        method_source,
        &method_source_path,
        &method_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested undefined-root method should fail in codegen");
    assert!(
        method_err.contains("Undefined variable: missing"),
        "{method_err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_function_for_unknown_direct_call() {
    let temp_root = make_temp_project_root("no-check-unknown-direct-call-primary-error");
    let source_path = temp_root.join("no_check_unknown_direct_call_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_direct_call_primary_error");
    let source = r#"
            function main(): Integer {
                return missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown direct call should fail in codegen without checks");
    assert!(err.contains("Undefined function: missing"), "{err}");
    assert!(!err.contains("Unknown function: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_function_value() {
    let temp_root = make_temp_project_root("no-check-unknown-function-value-primary-error");
    let source_path = temp_root.join("no_check_unknown_function_value_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_function_value_primary_error");
    let source = r#"
            function main(): None {
                callback: (Integer) -> Integer = missing;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown function value should fail in codegen without checks");
    assert!(err.contains("Undefined variable: missing"), "{err}");
    assert!(!err.contains("Unknown variable: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_literal_call_with_non_function_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-literal-call-non-function-type");
    let source_path = temp_root.join("no_check_literal_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_literal_call_non_function_type");
    let source = r#"
            function main(): Integer {
                return 1(2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("literal call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );
    assert!(!err.contains("Invalid callee"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_exact_import_alias_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("checked-exact-import-alias-call-non-function-type");
    let source_path = temp_root.join("checked_exact_import_alias_call_non_function_type.arden");
    let output_path = temp_root.join("checked_exact_import_alias_call_non_function_type");
    let source = r#"
            import std.system.cwd as CurrentDir;

            function main(): Integer {
                return CurrentDir();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("exact import alias non-function call should fail in checked build");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(
        !err.contains("Return type mismatch: expected Integer, found String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_exact_import_integer_alias_non_function_call_with_type_diagnostic() {
    let temp_root =
        make_temp_project_root("checked-exact-import-integer-alias-call-non-function-type");
    let source_path =
        temp_root.join("checked_exact_import_integer_alias_call_non_function_type.arden");
    let output_path = temp_root.join("checked_exact_import_integer_alias_call_non_function_type");
    let source = r#"
            import std.args.count as ArgCount;

            function main(): Integer {
                return ArgCount();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("integer exact import alias non-function call should fail in checked build");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_local_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-local-call-non-function-type");
    let source_path = temp_root.join("no_check_local_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_local_call_non_function_type");
    let source = r#"
            function main(): Integer {
                s: String = "hi";
                return s(2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("local non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(!err.contains("Undefined function: s"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_exact_import_alias_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-exact-import-alias-call-non-function-type");
    let source_path = temp_root.join("no_check_exact_import_alias_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_exact_import_alias_call_non_function_type");
    let source = r#"
            import std.system.cwd as CurrentDir;

            function main(): Integer {
                return CurrentDir();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("exact import alias non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(!err.contains("Unknown type: CurrentDir"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_function_call_with_user_facing_type_diagnostic()
{
    let temp_root = make_temp_project_root("no-check-module-local-call-non-function-type");
    let source_path = temp_root.join("no_check_module_local_call_non_function_type.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local non-function call should fail in codegen without checks");
    assert!(err.contains("Cannot call non-function type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_module_local_constructor_in_single_file_mode() {
    let temp_root = make_temp_project_root("no-check-module-local-constructor-runtime");
    let source_path = temp_root.join("no_check_module_local_constructor_runtime.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("module-local constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_namespace_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-namespace-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_namespace_alias_ctor.arden");
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

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("current-package namespace alias constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled current-package namespace alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_deref_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-deref-assign-type");
    let source_path = temp_root.join("no_check_integer_deref_assign_type.arden");
    let output_path = temp_root.join("no_check_integer_deref_assign_type");
    let source = r#"
            function main(): None {
                mut value: Integer = 7;
                *value = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer deref assignment should fail in codegen without checks");
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
    let source_path = temp_root.join("no_check_invalid_try_non_result_type.arden");
    let output_path = temp_root.join("no_check_invalid_try_non_result_type");
    let source = r#"
            function main(): None {
                value: Integer = 7;
                out: Integer = value?;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("? on Integer should fail in codegen without checks");
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
    let source_path = temp_root.join("no_check_invalid_try_module_local_non_result.arden");
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

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("? on module-local Box should fail in codegen without checks");
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
    let source_path = temp_root.join("no_check_box_payload_runtime.arden");
    let output_path = temp_root.join("no_check_box_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<Integer>(41);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Box payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check box payload binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_rc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-rc-payload-runtime");
    let source_path = temp_root.join("no_check_rc_payload_runtime.arden");
    let output_path = temp_root.join("no_check_rc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Rc<Integer> = Rc<Integer>(42);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Rc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check rc payload binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_arc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-arc-payload-runtime");
    let source_path = temp_root.join("no_check_arc_payload_runtime.arden");
    let output_path = temp_root.join("no_check_arc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Arc<Integer> = Arc<Integer>(43);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Arc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check arc payload binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_eq_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-eq-types");
    let source_path = temp_root.join("no_check_invalid_assert_eq_types.arden");
    let output_path = temp_root.join("no_check_invalid_assert_eq_types");
    let source = r#"
            function main(): None {
                assert_eq(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_eq on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_ne_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-ne-types");
    let source_path = temp_root.join("no_check_invalid_assert_ne_types.arden");
    let output_path = temp_root.join("no_check_invalid_assert_ne_types");
    let source = r#"
            function main(): None {
                assert_ne(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_ne on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
