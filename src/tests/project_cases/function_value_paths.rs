use super::*;
use std::fs;

#[test]
fn project_check_supports_cross_file_function_value_references() {
    let temp_root = make_temp_project_root("function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package app;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { o: Option<(Integer) -> Integer> = Option.some(add1); r: Result<(Integer) -> Integer, String> = Result.ok(add1); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should support function value refs");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_function_value_alias_references() {
    let temp_root = make_temp_project_root("function-value-import-project");
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
        "package app;\nimport util.add1 as inc;\nfunction main(): None { f: (Integer) -> Integer = inc; o: Option<(Integer) -> Integer> = Option.some(inc); x: Integer = f(2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support imported function value aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_function_values() {
    let temp_root = make_temp_project_root("function-value-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\nfunction twice(f: (Integer) -> Integer, x: Integer): Integer { return f(f(x)); }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.add1; x: Integer = u.twice(f, 1); y: Integer = u.add1(2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias function values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_namespace_alias_function_values() {
    let temp_root = make_temp_project_root("function-value-nested-namespace-alias-project");
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
        "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.M.add1; x: Integer = u.M.add1(1); y: Integer = f(2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested namespace alias function values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_import_alias_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("generic-fn-value-exact-import-alias-project");
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
        "package app;\nimport app.U.id as ident;\nfunction main(): Integer { f: (Integer) -> Integer = ident<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support exact-import alias explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled exact-import alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("generic-fn-value-root-alias-project");
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
        "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root namespace alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_enums() {
    let temp_root = make_temp_project_root("namespace-alias-nested-enum-project");
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
        "package app;\nimport app as u;\nfunction main(): None { e: u.M.E = u.M.E.B(2); match (e) { u.M.E.B(v) => { require(v == 2); } u.M.E.A(v) => { require(false); } } return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias nested enums");
    });

    let _ = fs::remove_dir_all(temp_root);
}
