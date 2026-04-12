use super::*;
use std::fs;

#[test]
fn project_build_supports_module_wildcard_import_calls() {
    let temp_root = make_temp_project_root("module-wildcard-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(7); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module wildcard import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module wildcard import call binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_wildcard_import_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("module-wildcard-import-generic-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.U.*;\nfunction main(): Integer { f: (Integer) -> Integer = id<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module wildcard import explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module wildcard import explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_wildcard_import_integer_to_float_calls() {
    let temp_root = make_temp_project_root("module-wildcard-import-int-to-float-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule U { function scale(value: Float): Float { return value * 2.0; } }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { value: Float = scale(3); return if (value == 6.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support wildcard imported int-to-float calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard-import int-to-float call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_wildcard_imported_nested_module_integer_to_float_calls() {
    let temp_root =
        make_temp_project_root("wildcard-import-nested-module-int-to-float-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule U { module Math { function scale(value: Float): Float { return value * 2.0; } } }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { value: Float = Math.scale(3); return if (value == 6.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support wildcard imported nested-module int-to-float calls",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard-import nested-module int-to-float call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_wildcard_import_calls() {
    let temp_root = make_temp_project_root("stdlib-wildcard-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { return if (abs(-7) == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib wildcard import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib wildcard import call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_wildcard_import_function_values() {
    let temp_root = make_temp_project_root("stdlib-wildcard-import-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { f: (Integer) -> Float = abs; return if (f(-7) == 7.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib wildcard import function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib wildcard import function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_match_expr_method_receivers() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-match-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if ((match (true) { true => CurrentDir, false => CurrentDir, }).length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in match-expression method receivers",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import match-expression method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_some_alias_calls() {
    let temp_root = make_temp_project_root("builtin-option-some-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.Some as Present;\nfunction main(): Integer { value: Option<Integer> = Present(7); return if (value.unwrap() == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.Some aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_ok_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-result-ok-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Result.Ok as Success;\nfunction main(): Integer { f: (Integer) -> Result<Integer, String> = Success; value: Result<Integer, String> = f(7); return if (value.unwrap() == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Result.Ok alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Result.Ok alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_error_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-result-error-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Result.Error as Failure;\nfunction main(): Integer { f: (String) -> Result<Integer, String> = Failure; value: Result<Integer, String> = f(\"boom\"); return if (value.is_error()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Result.Error alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Result.Error alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_alias_patterns() {
    let temp_root = make_temp_project_root("builtin-option-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.Some as Present;\nimport Option.None as Empty;\nfunction main(): Integer { value: Option<Integer> = Present(7); return match (value) { Present(inner) => if (inner == 7) { 0 } else { 1 }, Empty => 2, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option alias patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option alias pattern binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_alias_patterns() {
    let temp_root = make_temp_project_root("builtin-result-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Result.Ok as Success;\nimport Result.Error as Failure;\nfunction main(): Integer { value: Result<Integer, String> = Success(7); return match (value) { Success(inner) => if (inner == 7) { 0 } else { 1 }, Failure(err) => 2, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Result alias patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Result alias pattern binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { value: Option<Integer> = Empty; return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_builtin_option_none_alias_values() {
    let temp_root = make_temp_project_root("module-local-builtin-option-none-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { value: Option<Integer> = Empty; return if (value.is_none()) { 0 } else { 1 }; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local builtin Option.None aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local builtin Option.None alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { f: () -> Option<Integer> = Empty; value: Option<Integer> = f(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_return_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-return-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction make(): Option<Integer> { return Empty; }\nfunction main(): Integer { value: Option<Integer> = make(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None alias return values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias return value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_argument_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-arg-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction take(value: Option<Integer>): Integer { return if (value.is_none()) { 0 } else { 1 }; }\nfunction main(): Integer { return take(Empty); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None alias argument values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias argument value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_default_extern_link_names() {
    let temp_root = make_temp_project_root("project-extern-default-link-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nextern(c) function abs(value: Integer): Integer;\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.abs;\nfunction main(): Integer { return abs(-7); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should preserve default extern link names");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled project extern default-link-name binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_default_extern_link_names_through_exact_import_aliases() {
    let temp_root = make_temp_project_root("project-extern-default-link-name-alias");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nextern(c) function abs(value: Integer): Integer;\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.abs as absolute;\nfunction main(): Integer { return absolute(-7); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should preserve extern link names through exact import aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled project extern default-link-name alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_module_extern_link_names_through_exact_import_aliases() {
    let temp_root = make_temp_project_root("project-module-extern-default-link-name-alias");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nmodule C { extern(c) function abs(value: Integer): Integer; }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.C.abs as absolute;\nfunction main(): Integer { return absolute(-7); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should preserve module extern link names through exact import aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled project module extern default-link-name alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_prefers_shadowed_local_over_namespace_alias_for_nested_field_chain_calls() {
    let temp_root = make_temp_project_root("shadowed-local-over-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nclass Holder { function add1(x: Integer): Integer { return x + 5; } }\nfunction main(): Integer { u: Holder = Holder(); return u.add1(1); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should prefer shadowed local over namespace alias");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled shadowed-local-over-namespace-alias binary");
    assert_eq!(
        output.status.code(),
        Some(6),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_shadowed_local_nested_method_calls_without_alias_leakage() {
    let temp_root = make_temp_project_root("shadowed-local-nested-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule M { class Box { value: Integer; constructor(v: Integer) { this.value = v; } function get(): Integer { return this.value; } } }\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nclass Holder { inner: u.M.Box; constructor(v: Integer) { this.inner = u.M.Box(v); } function get(): Integer { return this.inner.get() + 10; } }\nfunction main(): Integer { u: Holder = Holder(2); return u.get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should keep shadowed local nested method calls local");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled shadowed-local-nested-method binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_class_constructors() {
    let temp_root = make_temp_project_root("class-constructor-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nclass Box { value: Integer; constructor(v: Integer) { this.value = v; } }\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util as u;\nfunction main(): None { u.Box(2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias class constructors");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_module_class_constructors() {
    let temp_root =
        make_temp_project_root("nested-class-constructor-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule Api {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util as u;\nfunction main(): None { u.Api.Box(2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias nested-module class constructors");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_namespace_aliases_without_functions() {
    let temp_root = make_temp_project_root("nested-module-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nmodule Api {\n    class Box {\n        constructor() {}\n    }\n}\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.Api as u;\nfunction main(): None { u.Box(); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested module namespace aliases without functions");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_namespace_aliases_without_functions() {
    let temp_root = make_temp_project_root("deep-nested-module-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule Api {\n    module V1 {\n        class Box {\n            constructor() {}\n        }\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util.Api.V1 as u;\nfunction main(): None { u.Box(); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support deep nested module namespace aliases without functions",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_interface_aliases() {
    let temp_root = make_temp_project_root("deep-nested-module-interface-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule Api {\n    module V1 {\n        interface Named { function name(): Integer; }\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.Api.V1 as u;\ninterface Printable extends u.Named { function print_me(): Integer; }\nclass Report implements Printable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support deep nested module interface aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_enum_alias_patterns() {
    let temp_root = make_temp_project_root("deep-nested-module-enum-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule Api {\n    module V1 {\n        enum Value { Ok(Integer) Error(Integer) }\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.Api.V1 as u;\nfunction main(): None { value: u.Value = u.Value.Ok(2); match (value) { u.Value.Ok(v) => { require(v == 2); } u.Value.Error(err) => { require(false); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support deep nested module enum alias patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_if_expression_function_value_callees() {
    let temp_root = make_temp_project_root("ifexpr-function-callee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction dec(x: Integer): Integer { return x - 1; }\nfunction main(): None { x: Integer = (if (true) { inc; } else { dec; })(1); require(x == 2); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support if-expression function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_unit_enum_variant_values() {
    let temp_root = make_temp_project_root("unit-enum-variant-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nenum E { A, B }\nfunction main(): None { e: E = E.A; match (e) { E.A => { } E.B => { } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support unit enum variant values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_enum_variant_aliases() {
    let temp_root = make_temp_project_root("exact-enum-variant-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nenum E { A(Integer) B(Integer) }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { E.A(v) => { require(false); } E.B(v) => { require(v == 2); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported enum variant aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_payload_enum_variant_function_value_aliases() {
    let temp_root = make_temp_project_root("imported-payload-enum-variant-fn-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nenum E { Wrap(Integer) }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.E.Wrap as WrapCtor;\nfunction main(): Integer { f: (Integer) -> E = WrapCtor; value: E = f(7); return match (value) { E.Wrap(v) => { if (v == 7) { 0 } else { 1 } } }; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support imported payload enum variant function value aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run imported payload enum variant function value alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_adapts_enum_variant_function_values_to_expected_signature() {
    let temp_root = make_temp_project_root("enum-variant-function-value-adapter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\ninterface Named { function value(): Integer; }\nclass Box implements Named { inner: Integer; constructor(inner: Integer) { this.inner = inner; } function value(): Integer { return this.inner; } }\nenum E { Wrap(Named) }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): Integer { ctor: (Box) -> E = E.Wrap; value: E = ctor(Box(7)); return match (value) { E.Wrap(named) => named.value() - 7, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should adapt enum variant function values to expected signatures");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run adapted enum variant function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_unit_enum_variant_function_value_aliases() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-fn-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nenum Mode { A, B }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.Mode.A as Pick;\nfunction main(): Integer { f: () -> Mode = Pick; return if (f() == Mode.A) { 0 } else { 1 }; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support imported unit enum variant function value aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run imported unit enum variant function value alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_colliding_top_level_enum_names_across_namespaces() {
    let temp_root = make_temp_project_root("colliding-enum-project");
    let src_dir = temp_root.join("src");
    let left_dir = src_dir.join("left");
    let right_dir = src_dir.join("right");
    fs::create_dir_all(&left_dir).must("create left namespace dir");
    fs::create_dir_all(&right_dir).must("create right namespace dir");
    write_test_project_config(
        &temp_root,
        &[
            "src/main.arden",
            "src/left/util.arden",
            "src/right/util.arden",
        ],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        left_dir.join("util.arden"),
        "package left;\nenum Shared { A }\n",
    )
    .must("write left enum");
    fs::write(
        right_dir.join("util.arden"),
        "package right;\nenum Shared { B }\n",
    )
    .must("write right enum");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build should reject colliding enum names");
        assert!(
            err.contains("colliding top-level enum names"),
            "unexpected error: {err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_enum_variant_alias_patterns() {
    let temp_root = make_temp_project_root("exact-enum-variant-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nenum E { A(Integer) B(Integer) }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { Variant(v) => { require(v == 2); } E.A(v) => { require(false); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported enum variant alias patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_enum_variant_patterns() {
    let temp_root = make_temp_project_root("namespace-alias-nested-enum-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package util;\nmodule Result {\n    enum Value { Ok(Integer) Error(Integer) }\n}\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): None { value: u.Result.Value = u.Result.Value.Ok(2); match (value) { u.Result.Value.Ok(v) => { require(v == 2); } u.Result.Value.Error(err) => { require(false); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias nested enum variant patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_enum_aliases() {
    let temp_root = make_temp_project_root("exact-nested-enum-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.M.E as Enum;\nfunction main(): None { e: Enum = Enum.B(2); match (e) { Enum.B(v) => { require(v == 2); } Enum.A(v) => { require(false); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported nested enum aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_enum_variant_aliases() {
    let temp_root = make_temp_project_root("exact-nested-enum-variant-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.M.E.B as Variant;\nfunction main(): None { e: M.E = Variant(2); match (e) { Variant(v) => { require(v == 2); } M.E.A(v) => { require(false); } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported nested enum variant aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}
