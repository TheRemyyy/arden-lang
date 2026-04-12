use super::*;
use std::fs;

#[test]
fn project_build_runs_float_interpolation_from_aliased_async_module_runtime() {
    let temp_root = make_temp_project_root("float-interpolation-aliased-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        r#"
package util;
module Api {
    module V1 {
        function promote(value: Integer): Float {
            return to_float(value) / 2.0;
        }

        async function measure(value: Integer): Task<Float> {
            return promote(value) + 0.25;
        }
    }
}
"#,
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        r#"
package app;
import std.io.*;
import util as u;

function main(): Integer {
    println("async_value={await(u.Api.V1.measure(3))}");
    println("direct_value={u.Api.V1.promote(3)}");
    return 0;
}
"#,
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support aliased async float interpolation");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled aliased async float interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("async_value=1.750000"), "stdout={stdout}");
    assert!(stdout.contains("direct_value=1.500000"), "stdout={stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_dotted_package_async_float_await_runtime() {
    let temp_root = make_temp_project_root("dotted-package-async-float-await-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/lib.arden", "src/main.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        r#"
package demo.analytics;
module Api {
    module V2 {
        async function score(v: Integer): Task<Float> {
            return to_float(v) + 10.0;
        }
    }
}
"#,
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        r#"
package app;
import std.io.*;
import demo.analytics as analytics;

function main(): Integer {
    score: Float = await(analytics.Api.V2.score(10));
    println("score={score}");
    return if (score == 20.0) { 0 } else { 1 };
}
"#,
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should preserve dotted-package async Float await values");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled dotted-package async float await binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("score=20.000000"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_dotted_package_async_function_value_runtime() {
    let temp_root = make_temp_project_root("dotted-package-async-function-value-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/lib.arden", "src/main.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        r#"
package demo.analytics;
module Api {
    module V2 {
        async function score(v: Integer): Task<Float> {
            return to_float(v) + 10.0;
        }
    }
}
"#,
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        r#"
package app;
import std.io.*;
import demo.analytics as analytics;

function main(): Integer {
    f: (Integer) -> Task<Float> = analytics.Api.V2.score;
    score: Float = await(f(10));
    println("score={score}");
    return if (score == 20.0) { 0 } else { 1 };
}
"#,
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support dotted-package async function values");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled dotted-package async function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "score=20.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_namespace_alias_unit_enum_match_expressions() {
    let temp_root = make_temp_project_root("namespace-alias-unit-enum-match-expression-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nenum E { A, B }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): Integer { value: Integer = match (u.E.A) { u.E.A => { 1 } u.E.B => { 2 } }; require(value == 1); return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("namespace alias unit enum match expression should build");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled namespace alias unit enum match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_direct_constructor_method_calls() {
    let temp_root = make_temp_project_root("direct-ctor-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nclass Boxed { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } }\nfunction main(): Integer { return Boxed(23).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .must("project build should support direct constructor method calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run direct constructor method project binary");
    assert_eq!(status.code(), Some(23));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_no_check_rejects_main_with_string_return_type_cleanly() {
    let temp_root = make_temp_project_root("project-main-string-return-type-nocheck");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): String { return \"oops\"; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, false, false, false)
            .must_err("unchecked project build should reject invalid main signature");
        assert!(err.contains("main() must return None or Integer"), "{err}");
        assert!(!err.contains("Clang failed"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_rejects_non_binary_output_kind() {
    let temp_root = make_temp_project_root("run-non-binary-project");
    let src_dir = temp_root.join("src");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"smoke\"\noutput_kind = \"static\"\n",
        )
        .must("write arden.toml");
    fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = run_project(&[], false, true, false).must_err("run should reject library output");
        assert!(err.contains("requires `output_kind = \"bin\"`"), "{err}");
    });
    assert!(
        !temp_root.join("smoke").exists(),
        "run should fail before creating a library artifact"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_local_qualified_nested_enum_match_expressions() {
    let temp_root = make_temp_project_root("local-nested-enum-match-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M { enum E { A(Integer), B(Integer) } class Box { value: Integer; constructor(value: Integer) { this.value = value; } } }\nfunction main(): Integer { return (match (M.E.A(42)) { M.E.A(v) => M.Box(v), M.E.B(v) => M.Box(v) }).value; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .must("project build should support local qualified nested enum match expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run local nested enum match project binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_qualified_async_function_paths() {
    let temp_root = make_temp_project_root("module-local-qualified-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): M.Box { return M.Box(43); } }\nfunction main(): Integer { return await(M.mk()).value; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .must("project build should support module-local qualified async function paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run module-local qualified async project binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_deeper_local_nested_module_function_paths() {
    let temp_root = make_temp_project_root("deeper-local-nested-module-function-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } } function mk(): Box { return Box(51); } } }\nfunction main(): Integer { return M.N.mk().get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .must("project build should support deeper local nested module function paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run deeper local nested module function project binary");
    assert_eq!(status.code(), Some(51));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_deeper_local_nested_module_async_paths() {
    let temp_root = make_temp_project_root("deeper-local-nested-module-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): Box { return Box(53); } } }\nfunction main(): Integer { return await(M.N.mk()).value; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .must("project build should support deeper local nested module async paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run deeper local nested module async project binary");
    assert_eq!(status.code(), Some(53));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_module_destructors_with_import_alias_calls() {
    let temp_root = make_temp_project_root("nested-module-destructor-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.add1 as inc;\nmodule M { module N { class Box { constructor() {} destructor() { require(inc(1) == 2); } } } }\nfunction main(): Integer { b: M.N.Box = M.N.Box(); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested-module destructors with import alias calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run nested-module destructor import-alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_generic_bounds_through_file_scope_aliases() {
    let temp_root = make_temp_project_root("nested-module-generic-bound-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package app;\ninterface Named { function name(): Integer; }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Named as NamedAlias;\nmodule M { module N { class Box<T extends NamedAlias> { value: T; constructor(value: T) { this.value = value; } function get(): Integer { return this.value.name(); } } } }\nclass Item implements NamedAlias { function name(): Integer { return 7; } }\nfunction main(): Integer { return M.N.Box<Item>(Item()).get(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support nested-module generic bounds through file-scope aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run nested-module generic bound alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_interface_generic_bounds_through_file_scope_aliases() {
    let temp_root = make_temp_project_root("nested-module-interface-generic-bound-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package app;\ninterface Named { function name(): Integer; }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Named as NamedAlias;\nmodule M { module N { interface Reader<T extends NamedAlias> { function read(value: T): Integer; } class Box implements Reader<Item> { constructor() {} function read(value: Item): Integer { return value.name(); } } } }\nclass Item implements NamedAlias { function name(): Integer { return 7; } }\nfunction main(): Integer { reader: M.N.Reader<Item> = M.N.Box(); return reader.read(Item()); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support nested-module interface generic bounds through file-scope aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run nested-module interface generic bound alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_generic_base_classes() {
    let temp_root = make_temp_project_root("nested-module-generic-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M { module N { class Payload { constructor() {} } class Base<T> { constructor() {} } class Child extends Base<Payload> { constructor() {} } } }\nfunction main(): Integer { value: M.N.Child = M.N.Child(); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested-module generic base classes");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run nested-module generic base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_exact_import_alias_base_classes() {
    let temp_root = make_temp_project_root("generic-exact-alias-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Payload { constructor() {} }\nclass Base<T> { constructor() {} }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.Base as BaseAlias;\nimport lib.Payload as PayloadAlias;\nclass Child extends BaseAlias<PayloadAlias> { constructor() {} }\nfunction main(): Integer { value: Child = Child(); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic exact-import alias base classes");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic exact-import alias base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_namespace_alias_base_classes() {
    let temp_root = make_temp_project_root("generic-namespace-alias-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Payload { constructor() {} }\nclass Base<T> { constructor() {} }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib as u;\nclass Child extends u.Base<u.Payload> { constructor() {} }\nfunction main(): Integer { value: Child = Child(); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic namespace alias base classes");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic namespace alias base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_namespace_alias_imports() {
    let temp_root = make_temp_project_root("module-local-namespace-alias-import-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } function get(): T { return this.value; } }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M { import lib as u; function make(): Integer { f: (Integer) -> u.Box<Integer> = u.Box<Integer>; value: u.Box<Integer> = f(7); return value.get(); } }\nfunction main(): Integer { return M.make(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local namespace alias imports");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled module-local namespace alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_exact_import_aliases() {
    let temp_root = make_temp_project_root("module-local-exact-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } function get(): T { return this.value; } }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M { import lib.Box as Boxed; function make(): Integer { f: (Integer) -> Boxed<Integer> = Boxed<Integer>; value: Boxed<Integer> = f(7); return value.get(); } }\nfunction main(): Integer { return M.make(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local exact import aliases");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled module-local exact import alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_no_check_rejects_module_local_wildcard_import_leaking_to_top_level() {
    let temp_root = make_temp_project_root("module-local-wildcard-import-leak-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import std.math.*; function keep(): Float { return abs(-1.0); } }\nfunction main(): Float { return abs(-1.0); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build should reject top-level use of module-local wildcard import");
        assert!(
            err.contains("Function 'abs' is defined in 'std.math' but not imported in 'app'")
                || err.contains("Import check failed"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_wildcard_import_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-module-local-wildcard-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nmodule Inner { import lib.*; function run(): Integer { return add(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial module-local wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without module-local wildcard imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after module-local wildcard-imported symbol removal");
        assert!(
            err.contains("Wildcard import 'lib.*' no longer provides 'add'")
                || err.contains("Function 'add' is defined in 'lib' but not imported in 'app'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: add"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_exact_import_alias_dependents_after_symbol_removal() {
    let temp_root =
        make_temp_project_root("project-build-module-local-exact-import-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nmodule Inner { import lib.add as plus_one; function run(): Integer { return plus_one(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial module-local exact-import alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without module-local exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after module-local exact-import alias symbol removal");
        assert!(
            err.contains("Imported alias 'plus_one' no longer resolves")
                || err.contains("Function 'plus_one' is defined")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: plus_one"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_namespace_alias_dependents_after_symbol_removal() {
    let temp_root =
        make_temp_project_root("project-build-module-local-namespace-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nmodule Inner { import lib as l; function run(): Integer { return l.add(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial module-local namespace-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without module-local namespace-alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after module-local namespace-alias symbol removal");
        assert!(
            err.contains("Imported namespace alias 'l' has no member 'add'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: l"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_recovers_cleanly_after_invalid_files_list_fix() {
    let temp_root = make_temp_project_root("project-check-invalid-files-list-fix");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/helper.txt\", \"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write invalid arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("write main");
    fs::write(temp_root.join("src/helper.txt"), "not arden\n").must("write helper");

    with_current_dir(&temp_root, || {
        let err = check_file(None).must_err("check should reject invalid files list entry");
        assert!(
            err.contains("src/helper.txt") || err.contains("is not an .arden file"),
            "{err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .must("rewrite valid arden.toml");

    with_current_dir(&temp_root, || {
        check_file(None).must("check should recover cleanly after fixing files list");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_repeated_helper_validity_toggles() {
    let temp_root = make_temp_project_root("project-commands-helper-validity-toggles");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");

    let invalid_helper = "package lib;\nfunction add(: Integer { return 1; }\n";
    let valid_helper = "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n";

    fs::write(temp_root.join("src/helper.arden"), invalid_helper).must("write invalid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).must_err("check should fail on first invalid helper");
        build_project(false, false, true, false, false)
            .must_err("build should fail on first invalid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.arden"), valid_helper).must("write valid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).must("check should pass on first valid helper");
        build_project(false, false, true, false, false)
            .must("build should pass on first valid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.arden"), invalid_helper).must("rewrite invalid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).must_err("check should fail on second invalid helper");
        build_project(false, false, true, false, false)
            .must_err("build should fail on second invalid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.arden"), valid_helper).must("rewrite valid helper again");
    with_current_dir(&temp_root, || {
        check_command(None, false).must("check should pass after repeated validity toggles");
        build_project(false, false, true, false, false)
            .must("build should pass after repeated validity toggles");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_ignore_metadata_only_touch_after_recovery() {
    let temp_root = make_temp_project_root("project-commands-metadata-touch-after-recovery");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .must("write malformed helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must_err("project check should fail on malformed helper");
        build_project(false, false, true, false, false)
            .must_err("build should fail on malformed helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    let fixed_helper = "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n";
    fs::write(temp_root.join("src/helper.arden"), fixed_helper).must("rewrite valid helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .must("build should recover after helper fix");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.arden"), fixed_helper)
        .must("rewrite identical helper for metadata touch");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .must("project check should ignore metadata-only touch after recovery");
        build_project(false, false, true, false, false)
            .must("build should ignore metadata-only touch after recovery");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_metadata_only_config_edit() {
    let temp_root = make_temp_project_root("project-commands-metadata-config-edit");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should pass initially");
        build_project(false, false, true, false, false).must("build should pass initially");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.1\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke2\"\n",
        )
        .must("rewrite metadata-only arden.toml");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .must("project check should recover after metadata-only config edit");
        build_project(false, false, true, false, false)
            .must("build should recover after metadata-only config edit");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_output_only_config_edit() {
    let temp_root = make_temp_project_root("project-commands-output-only-config-edit");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should pass initially");
        build_project(false, false, true, false, false).must("build should pass initially");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-renamed\"\n",
        )
        .must("rewrite output-only arden.toml");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should ignore output-only config edit");
        build_project(false, false, true, false, false)
            .must("build should rebuild cleanly after output-only config edit");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn project_build_rebuilds_after_same_length_source_edit_with_preserved_mtime() {
    let temp_root = make_temp_project_root("project-build-same-length-preserved-mtime");
    write_test_project_config(
        &temp_root,
        &["src/main.arden"],
        "src/main.arden",
        "build/out",
    );
    let source_path = temp_root.join("src/main.arden");
    let output_path = temp_root.join("build/out");
    let mtime_reference = temp_root.join("src/main.mtime_ref.arden");

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 11; }\n",
    )
    .must("write initial main");
    fs::copy(&source_path, &mtime_reference).must("write mtime reference");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must("initial build should pass");
    });
    let first_status = std::process::Command::new(&output_path)
        .status()
        .must("run first built binary");
    assert_eq!(first_status.code(), Some(11));

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 22; }\n",
    )
    .must("rewrite main with same-length content");
    let touch_status = std::process::Command::new("touch")
        .arg("-r")
        .arg(&mtime_reference)
        .arg(&source_path)
        .status()
        .must("run touch to preserve main mtime");
    assert!(touch_status.success(), "touch should preserve source mtime");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should rebuild after same-length content change");
    });
    let second_status = std::process::Command::new(&output_path)
        .status()
        .must("run rebuilt binary after same-length content change");
    assert_eq!(second_status.code(), Some(22));

    let _ = fs::remove_file(mtime_reference);
    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn parse_project_unit_reparses_after_same_length_source_edit_with_preserved_mtime() {
    let temp_root = make_temp_project_root("parse-cache-same-length-preserved-mtime");
    let source_path = temp_root.join("src/main.arden");
    let mtime_reference = temp_root.join("src/main.mtime_ref.arden");

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 11; }\n",
    )
    .must("write initial source");
    fs::copy(&source_path, &mtime_reference).must("write mtime reference");

    let first = parse_project_unit(&temp_root, &source_path).must("first parse");
    assert!(!first.from_parse_cache);

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 22; }\n",
    )
    .must("rewrite source with same-length content");
    let touch_status = std::process::Command::new("touch")
        .arg("-r")
        .arg(&mtime_reference)
        .arg(&source_path)
        .status()
        .must("run touch to preserve source mtime");
    assert!(touch_status.success(), "touch should preserve source mtime");

    let second = parse_project_unit(&temp_root, &source_path)
        .must("second parse after same-length content change");
    assert!(!second.from_parse_cache);
    assert_ne!(first.semantic_fingerprint, second.semantic_fingerprint);

    let _ = fs::remove_file(mtime_reference);
    let _ = fs::remove_dir_all(temp_root);
}
