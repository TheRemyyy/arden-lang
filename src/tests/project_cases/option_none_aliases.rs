use super::*;
use std::fs;

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_constructor() {
    let temp_root = make_temp_project_root("root-alias-builtin-option-none-constructor-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { value: Option<Integer> = root.Option.None(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None constructor",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None constructor binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_method_receivers() {
    let temp_root =
        make_temp_project_root("root-alias-builtin-option-none-method-receiver-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { return if (root.Option.None.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None method receivers",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None method receiver binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_function_values() {
    let temp_root = make_temp_project_root("root-alias-builtin-option-none-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { empty: () -> Option<Integer> = root.Option.None; value: Option<Integer> = empty(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_patterns() {
    let temp_root = make_temp_project_root("root-alias-builtin-patterns-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction classify(flag: Boolean): Integer { return match (if (flag) { root.Option.None() } else { root.Option.Some(7) }) { root.Option.None => 0, root.Option.Some(_) => 1, }; }\nfunction fail(flag: Boolean): Integer { return match (if (flag) { root.Result.Error(\"boom\") } else { root.Result.Ok(7) }) { root.Result.Error(_) => 0, root.Result.Ok(_) => 1, }; }\nfunction main(): Integer { return classify(true) + fail(true); }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support root namespace alias builtin patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin pattern binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_zero_arg_lambda_tail_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-lambda-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { empty: () -> Option<Integer> = () => Empty; return if (empty().is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support builtin Option.None alias zero-arg lambda tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias lambda tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_zero_arg_lambda_tail_values() {
    let temp_root = make_temp_project_root("root-alias-builtin-option-none-lambda-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { empty: () -> Option<Integer> = () => root.Option.None; return if (empty().is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None zero-arg lambda tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None lambda tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_match_scrutinee_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-match-scrutinee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { return match (Empty) { None => 0, Some(_) => 1, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None alias match scrutinee values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None alias match scrutinee binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_match_scrutinee_values() {
    let temp_root =
        make_temp_project_root("root-alias-builtin-option-none-match-scrutinee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { return match (root.Option.None) { None => 0, Some(_) => 1, }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None match scrutinee values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None match scrutinee binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_builtin_option_none_alias_zero_arg_lambda_tail_values() {
    let temp_root = make_temp_project_root("module-local-builtin-option-none-lambda-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { empty: () -> Option<Integer> = () => Empty; return if (empty().is_none()) { 0 } else { 1 }; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local builtin Option.None alias zero-arg lambda tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local builtin Option.None lambda tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_root_alias_builtin_option_none_zero_arg_lambda_tail_values()
{
    let temp_root =
        make_temp_project_root("module-local-root-alias-builtin-option-none-lambda-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import app as root; function keep(): Integer { empty: () -> Option<Integer> = () => root.Option.None; return if (empty().is_none()) { 0 } else { 1 }; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local root alias builtin Option.None zero-arg lambda tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local root alias builtin Option.None lambda tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_builtin_option_none_async_tail_values() {
    let temp_root = make_temp_project_root("module-local-builtin-option-none-async-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Task<Option<Integer>> { return async { Empty }; } }\nfunction main(): Integer { value: Option<Integer> = await(Inner.keep()); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local builtin Option.None async tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local builtin Option.None async tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_root_alias_builtin_option_none_async_tail_values() {
    let temp_root =
        make_temp_project_root("module-local-root-alias-builtin-option-none-async-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import app as root; function keep(): Task<Option<Integer>> { return async { root.Option.None }; } }\nfunction main(): Integer { value: Option<Integer> = await(Inner.keep()); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local root alias builtin Option.None async tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local root alias builtin Option.None async tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_async_if_tail_values() {
    let temp_root = make_temp_project_root("builtin-option-none-async-if-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction wrap(flag: Boolean): Option<Integer> { return await(async { if (flag) { Empty } else { Empty } }); }\nfunction main(): Integer { return if (wrap(true).is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support builtin Option.None async if-tail values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled builtin Option.None async if-tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_builtin_option_none_async_if_tail_values() {
    let temp_root = make_temp_project_root("root-alias-builtin-option-none-async-if-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app as root;\nfunction wrap(flag: Boolean): Option<Integer> { return await(async { if (flag) { root.Option.None } else { root.Option.None } }); }\nfunction main(): Integer { return if (wrap(true).is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 0; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support root namespace alias builtin Option.None async if-tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled root alias builtin Option.None async if-tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_builtin_option_none_async_if_tail_values() {
    let temp_root =
        make_temp_project_root("module-local-builtin-option-none-async-if-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function wrap(flag: Boolean): Task<Option<Integer>> { return async { if (flag) { Empty } else { Empty } }; } }\nfunction main(): Integer { value: Option<Integer> = await(Inner.wrap(true)); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local builtin Option.None async if-tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local builtin Option.None async if-tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_root_alias_builtin_option_none_async_if_tail_values() {
    let temp_root =
        make_temp_project_root("module-local-root-alias-builtin-option-none-async-if-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import app as root; function wrap(flag: Boolean): Task<Option<Integer>> { return async { if (flag) { root.Option.None } else { root.Option.None } }; } }\nfunction main(): Integer { value: Option<Integer> = await(Inner.wrap(true)); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local root alias builtin Option.None async if-tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local root alias builtin Option.None async if-tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_async_tail_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-async-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Task<Integer> { return async { Empty: Integer = 7; Empty }; } }\nfunction main(): Integer { return await(Inner.keep()) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None async tail values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None async tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_zero_arg_lambda_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { Empty: Integer = 7; f: () -> Integer = () => Empty; return f(); } }\nfunction main(): Integer { return Inner.keep() - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None zero-arg lambda values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_return_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Option<Integer> { Empty: Option<Integer> = Option.Some(7); return Empty; } }\nfunction main(): Integer { return match (Inner.keep()) { Some(v) => v - 7, None => 1, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None return values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_param_return_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-param-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(Empty: Option<Integer>): Option<Integer> { return Empty; } }\nfunction main(): Integer { return match (Inner.keep(Option.Some(7))) { Some(v) => v - 7, None => 1, }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None parameter return values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None parameter return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_param_zero_arg_lambda_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-param-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(Empty: Integer): Integer { f: () -> Integer = () => Empty; return f(); } }\nfunction main(): Integer { return Inner.keep(7) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None parameter zero-arg lambda values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None parameter lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_for_zero_arg_lambda_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-for-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { for (Empty in 7..8) { f: () -> Integer = () => Empty; return f() - 7; } return 1; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None for-loop zero-arg lambda values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None for-loop lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_shadowed_builtin_option_none_match_zero_arg_lambda_values() {
    let temp_root =
        make_temp_project_root("module-local-shadowed-builtin-option-none-match-lambda-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(value: Option<Integer>): Integer { return match (value) { Some(Empty) => { f: () -> Integer = () => Empty; f() - 7 }, None => 1, }; } }\nfunction main(): Integer { return Inner.keep(Option.Some(7)); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local shadowed builtin Option.None match-arm zero-arg lambda values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local shadowed builtin Option.None match-arm lambda binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
