use super::*;
use std::fs;

#[test]
fn project_build_reports_demangled_generic_bound_errors() {
    let temp_root = make_temp_project_root("project-demangled-generic-bound-errors");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\ninterface Named { function name(): Integer; }\nclass Plain { constructor() {} }\nclass Box<T extends Named> {\n    value: Integer;\n    constructor() { this.value = 1; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    bad: u.Box<u.Plain> = u.Box<u.Plain>();\n    return bad.value;\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with invalid bound should fail");
        assert!(err.contains("lib.Plain"), "{err}");
        assert!(err.contains("lib.Named"), "{err}");
        assert!(!err.contains("lib__Plain"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_if_branch_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-if-branch-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass A { constructor() {} }\nclass B { constructor() {} }\nfunction pick(flag: Boolean): Integer {\n    value: Integer = if (flag) { A() } else { B() };\n    return value;\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.pick as pick;\nfunction main(): Integer { return pick(true); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with if branch mismatch should fail");
        assert!(err.contains("then is lib.A, else is lib.B"), "{err}");
        assert!(!err.contains("lib__A"), "{err}");
        assert!(!err.contains("lib__B"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_assignment_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-assignment-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Named { constructor() {} }\nclass Plain { constructor() {} }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    value: u.Named = u.Plain();\n    return 0;\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with assignment mismatch should fail");
        assert!(
            err.contains("cannot assign lib.Plain to variable of type lib.Named"),
            "{err}"
        );
        assert!(!err.contains("lib__Plain"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_field_class_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-field-class");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Named { constructor() {} }\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    return u.Box<u.Named>(u.Named()).missing;\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown field should fail");
        assert!(
            err.contains("Unknown field 'missing' on class 'lib.Box<lib.Named>'"),
            "{err}"
        );
        assert!(!err.contains("lib__Box"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_non_function_call_type() {
    let temp_root = make_temp_project_root("project-demangled-non-function-call-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    value: u.Box<Integer> = u.Box<Integer>(1);\n    return value(2);\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with non-function call should fail");
        assert!(
            err.contains("Cannot call non-function type lib.Box<Integer>"),
            "{err}"
        );
        assert!(!err.contains("lib__Box"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_if_condition_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-if-condition-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Flag { constructor() {} }\nfunction bad(): Integer {\n    if (Flag()) { return 1; }\n    return 0;\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with non-boolean if condition should fail");
        assert!(
            err.contains("Condition must be Boolean, found lib.Flag"),
            "{err}"
        );
        assert!(!err.contains("lib__Flag"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_index_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-index-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Key { constructor() {} }\nfunction bad(): Integer {\n    xs: List<Integer> = List<Integer>();\n    xs.push(1);\n    return xs[Key()];\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with bad index type should fail");
        assert!(
            err.contains("Index must be Integer, found lib.Key"),
            "{err}"
        );
        assert!(!err.contains("lib__Key"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_await_operand_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-await-operand-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Job { constructor() {} }\nfunction bad(): Integer {\n    return await(Job());\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with non-task await operand should fail");
        assert!(
            err.contains("'await' can only be used on Task types, got lib.Job"),
            "{err}"
        );
        assert!(!err.contains("lib__Job"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_match_arm_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-match-arm-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Left { constructor() {} }\nclass Right { constructor() {} }\nfunction bad(flag: Boolean): Integer {\n    value: Integer = match (flag) {\n        true => Left(),\n        false => Right(),\n    };\n    return value;\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(true); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with match arm mismatch should fail");
        assert!(
            err.contains("Match expression arm type mismatch: expected lib.Left, got lib.Right"),
            "{err}"
        );
        assert!(!err.contains("lib__Left"), "{err}");
        assert!(!err.contains("lib__Right"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_pattern_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-pattern-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Token { constructor() {} }\nfunction bad(value: Token): Integer {\n    return match (value) {\n        1 => 0,\n        _ => 1,\n    };\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib.bad as bad;\nimport lib.Token as Token;\nfunction main(): Integer { return bad(Token()); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with pattern type mismatch should fail");
        assert!(
            err.contains("Pattern type mismatch: expected lib.Token, found Integer"),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_option_some_argument_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-option-some-arg-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nclass Token { constructor() {} }\nfunction wrap(flag: Boolean): Option<Token> {\n    if (flag) {\n        return Option.some(1);\n    }\n    return Option.none();\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib.wrap as wrap;\nfunction main(): Integer { return if (wrap(true).is_some()) { 1 } else { 0 }; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with Option.some argument mismatch should fail");
        assert!(
            err.contains("Return type mismatch: expected Option<lib.Token>, found Option<Integer>"),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_type_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-type-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nmodule Api {\n    class Token { constructor() {} }\n}\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction read(value: u.Api.Missing): Integer {\n    return 0;\n}\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown type should fail");
        assert!(err.contains("Unknown type: u.Api.Missing"), "{err}");
        assert!(!err.contains("lib__Api__Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_invalid_nested_namespace_aliased_extern_signature() {
    let temp_root =
        make_temp_project_root("project-invalid-nested-namespace-aliased-extern-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nextern(c) function host(value: root.M.Api.Named): root.M.Api.Named;\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail for invalid namespace aliased extern signature");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(
            !err.contains("Extern function 'app__host' has non-FFI-safe parameter"),
            "{err}"
        );
        assert!(
            !err.contains("Extern function 'app__host' has non-FFI-safe return type"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_lambda_signature() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-lambda-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    f: (root.M.Api.Named) -> Integer = (value: root.M.Api.Named) => 0;\n    return 0;\n}\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased lambda signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased lambda signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
            "build should fail after namespace aliased lambda signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_invalid_nested_namespace_aliased_constructor_type_arg() {
    let temp_root =
        make_temp_project_root("project-invalid-nested-namespace-aliased-constructor-type-arg");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    root.M.Box<root.M.Api.Named>();\n    return 0;\n}\n",
        )
        .must("write main");
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M {\n    module Api { interface Labelled { function name(): Integer; } }\n    class Box<T> { constructor() {} }\n}\n",
        )
        .must("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail for invalid namespace aliased constructor type arg");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Built smoke"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_function_type_let_annotation(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-function-type-let-annotation",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    f: (root.M.Api.Named) -> Integer = (value: root.M.Api.Named) => 0;\n    return 0;\n}\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased function-type let annotation build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased function-type let annotation interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
                "build should fail after namespace aliased function-type let annotation interface removal",
            );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_nested_namespace_aliased_function_type_inside_generic_constructor() {
    let temp_root = make_temp_project_root(
        "project-nested-namespace-aliased-function-type-inside-generic-constructor",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    values: List<(root.M.Api.Named) -> Integer> = List<(root.M.Api.Named) -> Integer>();\n    return 0;\n}\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
                "project build should accept nested namespace aliased function types in generic constructors",
            );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_nested_module_local_function_type_inside_generic_constructor() {
    let temp_root = make_temp_project_root(
        "project-nested-module-local-function-type-inside-generic-constructor",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    function make(): Integer {\n        values: List<(Named) -> Integer> = List<(Named) -> Integer>();\n        return 0;\n    }\n}\nfunction main(): Integer { return M.make(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
                "project build should accept nested module-local function types in generic constructors",
            );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_local_module_function_call_type_args() {
    let temp_root = make_temp_project_root("project-local-module-function-call-type-args");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.*;\nmodule M {\n    class Box { constructor() {} }\n    function make<T>(): None { }\n}\nfunction main(): None {\n    M.make<M.Box>();\n}\n",
        )
        .must("write main");
    fs::write(
            src_dir.join("helper.arden"),
            "package util;\nmodule N {\n    module M {\n        class Box { constructor() {} }\n    }\n}\n",
        )
        .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should accept local module function call type args");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_generic_interface_references() {
    let temp_root = make_temp_project_root("project-module-local-generic-interface-references");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    class Payload { constructor() {} }\n    interface Named<T> { }\n    interface Child extends Named<Payload> { }\n    class Book implements Named<Payload> { constructor() {} }\n}\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should accept module-local generic interface references");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_generic_function_values_with_local_type_args() {
    let temp_root =
        make_temp_project_root("project-module-local-generic-function-values-local-type-args");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n    }\n    function id<T>(value: T): T { return value; }\n    function run(): Integer {\n        f: (Box) -> Box = id<Box>;\n        return f(Box(7)).value;\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept module-local generic function values with local type args",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local generic function value binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_lambda_parameter_types() {
    let temp_root = make_temp_project_root("project-module-local-lambda-parameter-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    interface Named { function value(): Integer; }\n    class Box implements Named {\n        inner: Integer;\n        constructor(inner: Integer) { this.inner = inner; }\n        function value(): Integer { return this.inner; }\n    }\n    function run(): Integer {\n        f: (Named) -> Integer = (value: Named) => value.value();\n        return f(Box(21));\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should accept module-local lambda parameter types");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local lambda parameter binary");
    assert_eq!(status.code(), Some(21));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_contextual_lambda_parameter_inference() {
    let temp_root = make_temp_project_root("project-contextual-lambda-parameter-inference");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction keep(): Integer { f: (Integer) -> Integer = (x: Integer) => x; return f(7) - 7; }\nfunction main(): Integer { return keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should accept contextual lambda parameter inference");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled contextual lambda parameter inference binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_contextual_lambda_parameter_inference_with_exact_import_shadowing() {
    let temp_root =
        make_temp_project_root("project-contextual-lambda-parameter-inference-exact-shadowing");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport Option.None as Empty;\nfunction keep(): Integer { f: (Integer) -> Integer = (Empty: Integer) => Empty; return f(7) - 7; }\nfunction main(): Integer { return keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept contextual lambda parameter inference with exact import shadowing",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled contextual lambda parameter inference exact import shadowing binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_contextual_lambda_parameter_inference_with_exact_import_shadowing(
) {
    let temp_root = make_temp_project_root(
        "project-module-local-contextual-lambda-parameter-inference-exact-shadowing",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { f: (Integer) -> Integer = (Empty: Integer) => Empty; return f(7) - 7; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept module-local contextual lambda parameter inference with exact import shadowing",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must(
            "run compiled module-local contextual lambda parameter inference exact import shadowing binary",
        );
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_contextual_lambda_parameter_inference_in_if_expression() {
    let temp_root = make_temp_project_root("project-contextual-lambda-parameter-inference-if-expr");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction choose(flag: Boolean): (Integer) -> Integer { return if (flag) { (x: Integer) => x } else { (x: Integer) => x + 1 }; }\nfunction main(): Integer { return choose(false)(6) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept contextual lambda parameter inference in if expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled contextual lambda parameter inference if-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_contextual_lambda_parameter_inference_in_match_expression() {
    let temp_root =
        make_temp_project_root("project-contextual-lambda-parameter-inference-match-expr");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction choose(flag: Boolean): (Integer) -> Integer { return match (flag) { true => { (x: Integer) => x }, false => { (x: Integer) => x + 1 }, }; }\nfunction main(): Integer { return choose(false)(6) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept contextual lambda parameter inference in match expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled contextual lambda parameter inference match-expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_contextual_lambda_parameter_inference_in_async_tail() {
    let temp_root = make_temp_project_root("project-contextual-lambda-parameter-inference-async");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction make(): Task<(Integer) -> Integer> { return async { (x: Integer) => x + 1 }; }\nfunction main(): Integer { return (await(make()))(6) - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept contextual lambda parameter inference in async tail expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled contextual lambda parameter inference async-tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_imported_generic_constructor_function_values_in_async_if_tail() {
    let temp_root = make_temp_project_root("project-async-if-imported-generic-constructor-fn");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Box as BoxCtor;\nfunction choose(flag: Boolean): Task<(Integer) -> Box<Integer>> { return async { if (flag) { BoxCtor<Integer> } else { BoxCtor<Integer> } }; }\nfunction main(): Integer { return (await(choose(true)))(7).value - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept imported generic constructor function values in async if tails",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic constructor async-if binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_exact_import_alias_generic_constructor_function_values() {
    let temp_root = make_temp_project_root("project-exact-import-generic-constructor-fn-value");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } }\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Box as BoxCtor;\nfunction main(): Integer { ctor: (Integer) -> Box<Integer> = BoxCtor<Integer>; return ctor(7).value - 7; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should accept exact-import alias generic constructor function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled exact-import generic constructor function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_nested_enum_variant_patterns() {
    let temp_root = make_temp_project_root("project-module-local-nested-enum-variant-patterns");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    module N {\n        enum E { A(Integer), B(Integer) }\n    }\n    function run(): Integer {\n        value: N.E = N.E.A(44);\n        return match (value) {\n            N.E.A(v) => v,\n            N.E.B(v) => v,\n        };\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should accept module-local nested enum variant patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local nested enum variant pattern binary");
    assert_eq!(status.code(), Some(44));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_avoids_cascading_errors_for_stale_nested_namespace_aliased_interface_type() {
    let temp_root = make_temp_project_root("project-stale-nested-namespace-aliased-interface-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { value: root.M.Api.Named = Book(); return value.name(); }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial nested namespace aliased interface build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without nested namespace aliased interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after nested namespace aliased interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains(
                "Type mismatch: cannot assign app.Book to variable of type root.M.Api.Named"
            ),
            "{err}"
        );
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_exact_imported_interface_alias_in_implements() {
    let temp_root =
        make_temp_project_root("project-stale-exact-imported-interface-alias-implements");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.M.Api.Named as Named;\nclass Book implements Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact imported interface implements build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without exact imported implemented interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after exact imported implemented interface removal");
        assert!(
            err.contains("Imported alias 'Named' no longer resolves"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains("Class 'app.Book' implements unknown interface 'Named'"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_in_implements() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-implements");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased implements build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased implemented interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after namespace aliased implemented interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains("Class 'app.Book' implements unknown interface 'root.M.Api.Named'"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_avoids_cascading_errors_for_stale_for_loop_namespace_aliased_interface_type() {
    let temp_root =
        make_temp_project_root("project-stale-for-loop-namespace-aliased-interface-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction books(): List<Book> { values: List<Book> = List<Book>(); values.push(Book()); return values; }\nfunction main(): Integer { for (value: root.M.Api.Named in books()) { return value.name(); } return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial for-loop namespace aliased interface build should succeed");
    });
    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled for-loop namespace aliased interface binary");
    assert_eq!(status.code(), Some(1));

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without for-loop namespace aliased interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after for-loop namespace aliased interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
                !err.contains("Loop variable type mismatch: declared root.M.Api.Named, but iterating over List<app.Book>"),
                "{err}"
            );
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_class_field() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-class-field");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book { value: root.M.Api.Named; constructor() {} }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased class field build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased class field interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after namespace aliased class field interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_enum_payload() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-enum-payload");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nenum Wrap { Named(root.M.Api.Named) }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased enum payload build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased enum payload interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after namespace aliased enum payload interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_method_signature(
) {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-method-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book { constructor() {} function take(value: root.M.Api.Named): Integer { return 0; } }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased method signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased method signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
            "build should fail after namespace aliased method signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_constructor_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-constructor-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\nclass Book { constructor(value: root.M.Api.Named) {} }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased constructor signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased constructor signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
            "build should fail after namespace aliased constructor signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_interface_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-interface-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\ninterface NamedConsumer { function take(value: root.M.Api.Named): Integer; }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased interface signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased interface signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
            "build should fail after namespace aliased interface signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_interface_return_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-interface-return-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app as root;\ninterface NamedFactory { function make(): root.M.Api.Named; }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace aliased interface return signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.arden"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("rewrite helper without namespace aliased interface return signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
                "build should fail after namespace aliased interface return signature interface removal",
            );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_list_constructor_capacity_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-list-constructor-capacity");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nclass Token { constructor() {} }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib.Token as Token;\nfunction main(): Integer {\n    xs: List<Integer> = List<Integer>(Token());\n    return xs.length();\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with non-integer list capacity should fail");
        assert!(
            err.contains(
                "Constructor List<Integer> expects optional Integer capacity, got lib.Token"
            ),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_interface_signature_types() {
    let temp_root = make_temp_project_root("project-unknown-interface-signature-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\ninterface Api {\n    function decode(value: Missing): Missing;\n}\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown interface signature types should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_extern_signature_types() {
    let temp_root = make_temp_project_root("project-unknown-extern-signature-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nextern(c) function host(value: Missing): Missing;\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown extern signature types should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_enum_payload_types() {
    let temp_root = make_temp_project_root("project-unknown-enum-payload-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\nenum Message { Value(Missing) }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown enum payload type should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_implemented_interface() {
    let temp_root = make_temp_project_root("project-demangled-unknown-implemented-interface");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nmodule Api {\n    class Report implements Missing {\n        constructor() {}\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown implemented interface should fail");
        assert!(
            err.contains("Class 'lib.Api.Report' implements unknown interface 'Missing'"),
            "{err}"
        );
        assert!(!err.contains("lib__Api__Report"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_variant_enum_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-variant-enum-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nenum Choice { Left, Right }\nfunction read(): Integer {\n    return match (Choice.Left) {\n        Choice.Missing => 0,\n        _ => 1,\n    };\n}\n",
        )
        .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib.read as read;\nfunction main(): Integer { return read(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("project build with unknown variant should fail");
        assert!(
            err.contains("Unknown variant 'Choice.Missing' for enum 'lib.Choice'"),
            "{err}"
        );
        assert!(!err.contains("lib__Choice"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}
