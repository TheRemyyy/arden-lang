use super::*;
use std::fs;

#[test]
fn project_build_supports_stdlib_exact_import_calls() {
    let temp_root = make_temp_project_root("stdlib-exact-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.abs as absolute;\nfunction main(): Integer { return if (absolute(-7) == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib exact import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib exact import call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_exact_import_function_values() {
    let temp_root = make_temp_project_root("stdlib-exact-import-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.abs as absolute;\nfunction main(): Integer { f: (Integer) -> Float = absolute; return if (f(-7) == 7.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib exact import function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib exact import function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = Pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_string_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-string-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { value: String = CurrentDir; return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib string exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib string exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_integer_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-integer-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { value: Integer = ArgCount; return if (value >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib integer exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib integer exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_if_expressions() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-if-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = if (true) { Pi } else { 0.0 }; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import if expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import if expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_match_expressions() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-match-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = match (true) { true => Pi, false => 0.0, }; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import match expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_match_scrutinees() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-match-scrutinee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return match (CurrentDir) { \"\" => 1, _ => 0, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import match scrutinees");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import match scrutinee binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_match_statements() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-match-stmt-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { mut result: Integer = 1; match (CurrentDir) { \"\" => { result = 1; } _ => { result = 0; } } return result; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import match statements");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import match statement binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_stdlib_zero_arg_exact_import_values() {
    let temp_root =
        make_temp_project_root("module-local-stdlib-zero-arg-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner {\n    import std.system.cwd as CurrentDir;\n    function read(): String { value: String = CurrentDir; return value; }\n}\nfunction main(): Integer { value: String = Inner.read(); return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local zero-arg stdlib exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local zero-arg stdlib exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_return_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-return-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction read(): String { return CurrentDir; }\nfunction main(): Integer { value: String = read(); return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib exact import return values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib exact import return value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_wildcard_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-wildcard-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { value: Float = pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg stdlib wildcard values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg stdlib wildcard value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_namespace_zero_arg_values() {
    let temp_root = make_temp_project_root("stdlib-namespace-zero-arg-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math as math;\nfunction main(): Integer { value: Float = math.pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib namespace zero-arg values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib namespace zero-arg value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_wildcard_string_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-wildcard-string-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.*;\nfunction main(): Integer { value: String = cwd; return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support stdlib zero-arg wildcard string values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib zero-arg wildcard string value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_if_expression_builtin_function_values() {
    let temp_root = make_temp_project_root("if-expression-builtin-function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction choose(flag: Boolean): (Integer) -> Float { return if (flag) { to_float } else { to_float }; }\nfunction main(): Integer { return if (choose(true)(1) == 1.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support if-expression builtin function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if-expression builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_match_expression_builtin_function_values() {
    let temp_root = make_temp_project_root("match-expression-builtin-function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nenum Mode { A, B }\nfunction choose(mode: Mode): (Integer) -> Float { return match (mode) { Mode.A => { to_float } Mode.B => { to_float } }; }\nfunction main(): Integer { return if (choose(Mode.A)(1) == 1.0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support match-expression builtin function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled match-expression builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_typed_lists() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-typed-list-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { values: List<Float> = List<Float>(); values.push(Pi); return if (values[0] > 3.14 && values[0] < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in typed lists");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import typed list binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_builtin_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-builtin-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { text: String = to_string(Pi); return if (text.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in builtin calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import builtin call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_string_builtins() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-string-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nimport std.string.*;\nfunction main(): Integer { return if (Str.len(CurrentDir) >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in string builtins");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import string builtin binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_time_builtin_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-time-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nimport std.time.*;\nfunction main(): Integer { formatted: String = Time.now(CurrentDir); return if (formatted.length() >= 0) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in time builtins");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import time builtin binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_list_index_methods() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-list-index-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(); values.push(10); values.push(20); return values.get(ArgCount) - 20; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in list index methods",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import list index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_list_constructor_capacity() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-list-capacity-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(ArgCount); values.push(7); return values.get(0) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in list constructor capacity",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import list capacity binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_index_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-index-expression-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(); values.push(10); values.push(20); return values[ArgCount] - 20; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in index expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import index expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_string_index_expressions() {
    let temp_root =
        make_temp_project_root("zero-arg-exact-import-value-string-index-expression-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { text: String = \"ab\"; letter: Char = text[ArgCount]; return if (letter == 'b') { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in string index expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_as_indexed_objects() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-index-object-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        if cfg!(windows) {
            "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { letter: Char = CurrentDir[0]; return if (((letter >= 'A') && (letter <= 'Z')) || ((letter >= 'a') && (letter <= 'z')) || (letter == '\\\\') || (letter == '/')) { 0 } else { 1 }; }\n".to_string()
        } else {
            "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { letter: Char = CurrentDir[0]; return if (letter == '/') { 0 } else { 1 }; }\n".to_string()
        },
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values as indexed objects");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import indexed object binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_for_iterables() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-for-iterable-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { mut count: Integer = 0; for (ch in CurrentDir) { count += 1; } return if (count >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in for iterables");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import for iterable binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_async_returns() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-async-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { task: Task<Integer> = async { return ArgCount; }; return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in async returns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import async return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_async_tail_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-async-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { return if (await(async { ArgCount }) == 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in async tail expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import async tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_range_syntax() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-range-syntax-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { value: Range<Integer> = ArgCount..(ArgCount + 1); return if (value.has_next()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in range syntax");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import range syntax binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_range_syntax_for_loops() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-range-for-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { mut total: Integer = 0; for (value in ArgCount..(ArgCount + 1)) { total += value; } return if (total == 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in range syntax for loops",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import range for-loop binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_task_await_timeout() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-await-timeout-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nimport std.time.*;\nfunction work(): Task<Integer> { return async { Time.sleep(50); return 7; }; }\nfunction main(): Integer { value: Option<Integer> = work().await_timeout(ArgCount); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in Task.await_timeout",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import await_timeout binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_option_some() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-option-some-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { value: Option<Integer> = Option.some(ArgCount); return if (value.unwrap() == 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in Option.some");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import Option.some binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_result_ok() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-result-ok-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Result<Float, String> = Result.ok(Pi); return if (value.unwrap() > 3.14 && value.unwrap() < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in Result.ok");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import Result.ok binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_direct_result_ok_receivers() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-direct-result-ok-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { return if (Result.ok(Pi).unwrap() > 3.14) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in direct Result.ok receivers",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import direct Result.ok receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_require_messages() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-require-message-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { require(true, CurrentDir); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in require messages");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import require message binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_borrows() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-borrow-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { text: &String = &CurrentDir; return if ((*text).length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in borrows");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import borrow binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_direct_borrow_dereferences() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-direct-deref-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if ((*(&CurrentDir)).length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in direct borrow dereferences",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import direct deref binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_try_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-try-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction run(): Option<Integer> { value: Integer = Option.some(ArgCount)?; return Option.some(value); }\nfunction main(): Integer { return if (run().unwrap() == 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in try expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import try binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_zero_arg_lambdas() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { f: () -> Integer = () => ArgCount; return if (f() == 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in zero-arg lambdas");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_immediate_zero_arg_lambdas() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-immediate-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if (((() => CurrentDir))().length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in immediate zero-arg lambdas",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import immediate lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_println_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-println-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nimport std.io.println;\nfunction main(): Integer { println(Pi); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in println calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import println binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_string_interpolation() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-string-interp-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: String = \"{Pi}\"; return if (value.length() >= 4) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in string interpolation",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import string interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_variadic_ffi_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-variadic-ffi-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nextern(system, \"printf\") function sys_printf(fmt: String, ...): Integer;\nfunction main(): Integer { sys_printf(\"%f\\n\", Pi); return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in variadic FFI calls",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import variadic FFI binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_binary_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-binary-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { return if (Pi > 3.14 && Pi < 3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in binary expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import binary expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_unary_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-unary-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = -Pi; return if (value < -3.14 && value > -3.15) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values in unary expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import unary expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_as_method_receivers() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-method-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if (CurrentDir.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support zero-arg exact import values as method receivers");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_compound_assignments() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-compound-assign-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { mut value: String = \"\"; value += CurrentDir; return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in compound assignments",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_as_method_receiver() {
    let temp_root = make_temp_project_root("builtin-option-none-method-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { return if (Empty.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None alias method receivers");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_if_expr_method_receivers() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-if-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if ((if (true) { CurrentDir } else { CurrentDir }).length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in if-expression method receivers",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import if-expression method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_block_expr_method_receivers() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-block-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { return if (({ CurrentDir }).length() >= 1) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support zero-arg exact import values in block-expression method receivers",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled zero-arg exact import block-expression method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
