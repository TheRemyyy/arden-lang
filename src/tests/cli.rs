#[allow(unused_imports)]
use super::*;
#[cfg(unix)]
use crate::collect_arden_files;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_simple_project(root: &Path) {
    let src_dir = root.join("src");
    write_test_project_config(root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .expect("write main");
}

fn remove_incremental_build_fingerprints(root: &Path) {
    for fingerprint in ["build_fingerprint", "semantic_build_fingerprint"] {
        let path = root.join(".ardencache").join(fingerprint);
        if path.exists() {
            fs::remove_file(&path).expect("remove incremental build fingerprint");
        }
    }
}

#[test]
fn cli_check_command_succeeds_for_temp_project() {
    let temp_root = make_temp_project_root("cli-check");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { value: Integer = helper(); return None; }\n",
    )
    .expect("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): Integer { return 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should pass");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_parse_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-parse-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let parsed_cache_dir = temp_root.join(".ardencache").join("parsed");
        let parsed_cache_file = fs::read_dir(&parsed_cache_dir)
            .expect("read parse cache dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_file())
            .expect("parse cache file should exist");
        fs::write(&parsed_cache_file, b"not valid cache").expect("corrupt parse cache");

        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted parse cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_import_check_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-import-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let import_cache_file = fs::read_dir(temp_root.join(".ardencache").join("import_check"))
            .expect("read import cache dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_file())
            .expect("import cache file should exist");
        fs::write(&import_cache_file, b"not valid cache").expect("corrupt import cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted import cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_rewrite_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-rewrite-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let rewrite_cache_file = fs::read_dir(temp_root.join(".ardencache").join("rewritten"))
            .expect("read rewrite cache dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_file())
            .expect("rewrite cache file should exist");
        fs::write(&rewrite_cache_file, b"not valid cache").expect("corrupt rewrite cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted rewrite cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_object_cache_metadata_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-object-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let object_cache_file = fs::read_dir(temp_root.join(".ardencache").join("objects"))
            .expect("read object cache dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .expect("object cache metadata should exist");
        fs::write(&object_cache_file, b"not valid cache").expect("corrupt object cache meta");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted object cache metadata and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_dependency_graph_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-dependency-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let dependency_graph_cache = temp_root
            .join(".ardencache")
            .join("dependency_graph")
            .join("latest.json");
        fs::write(&dependency_graph_cache, b"not valid cache")
            .expect("corrupt dependency graph cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted dependency graph cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_semantic_summary_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-semantic-summary-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let semantic_summary_cache = temp_root
            .join(".ardencache")
            .join("semantic_summary")
            .join("latest.json");
        fs::write(&semantic_summary_cache, b"not valid cache")
            .expect("corrupt semantic summary cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted semantic summary cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_typecheck_summary_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-typecheck-summary-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let typecheck_summary_cache = temp_root
            .join(".ardencache")
            .join("typecheck_summary")
            .join("latest.json");
        fs::write(&typecheck_summary_cache, b"not valid cache")
            .expect("corrupt typecheck summary cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted typecheck summary cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_recovers_from_corrupted_link_manifest_cache_blob() {
    let temp_root = make_temp_project_root("cli-corrupt-link-manifest-cache");
    write_simple_project(&temp_root);

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");

        let link_manifest_cache = temp_root
            .join(".ardencache")
            .join("link")
            .join("latest.json");
        fs::write(&link_manifest_cache, b"not valid cache").expect("corrupt link manifest cache");
        remove_incremental_build_fingerprints(&temp_root);

        build_project(false, false, true, false, false)
            .expect("build should ignore corrupted link manifest cache and recover");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_format_targets_checks_and_formats_project_files() {
    let temp_root = make_temp_project_root("cli-fmt");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    let main_file = src_dir.join("main.arden");
    fs::write(
        &main_file,
        "function main(): None {println(\"hi\");return None;}\n",
    )
    .expect("write unformatted file");

    with_current_dir(&temp_root, || {
        let err = format_targets(None, true).expect_err("format check should fail before fmt");
        assert!(err.contains("format check failed"), "{err}");
        format_targets(None, false).expect("format should succeed");
        format_targets(None, true).expect("format check should pass after fmt");
    });

    let formatted = fs::read_to_string(&main_file).expect("read formatted file");
    assert!(
        formatted.contains("function main(): None {\n"),
        "{formatted}"
    );
    assert!(formatted.contains("    println(\"hi\");\n"), "{formatted}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_lists_filtered_tests_in_directory() {
    let temp_root = make_temp_project_root("cli-test-list");
    let test_file = temp_root.join("smoke_test.arden");
    fs::write(
        &test_file,
        r#"
                @Test
                function smokeAlpha(): None { return None; }

                @Test
                function otherBeta(): None { return None; }
            "#,
    )
    .expect("write test file");

    run_tests(Some(&temp_root), true, Some("smoke")).expect("test listing should succeed");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_recurses_into_nested_test_directories() {
    let temp_root = make_temp_project_root("cli-test-nested");
    let nested_dir = temp_root.join("tests").join("unit");
    fs::create_dir_all(&nested_dir).expect("create nested test dir");
    let nested_test = nested_dir.join("math_spec.arden");
    fs::write(
        &nested_test,
        r#"
                @Test
                function nestedSpec(): None { return None; }
            "#,
    )
    .expect("write nested test file");

    run_tests(Some(&temp_root.join("tests")), true, Some("nested"))
        .expect("nested directory test listing should succeed");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_discovers_mixed_case_test_filenames() {
    let temp_root = make_temp_project_root("cli-test-mixed-case");
    let test_file = temp_root.join("MathTest.arden");
    fs::write(
        &test_file,
        r#"
                @Test
                function mixedCase(): None { return None; }
            "#,
    )
    .expect("write mixed-case test file");

    run_tests(Some(&temp_root), true, Some("mixedCase"))
        .expect("mixed-case test discovery should succeed");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_errors_for_missing_directory() {
    let temp_root = make_temp_project_root("cli-test-missing-dir");
    let missing_dir = temp_root.join("missing-tests");

    let err =
        run_tests(Some(&missing_dir), true, None).expect_err("missing test directory should error");
    assert!(
        err.contains("does not exist"),
        "expected missing directory error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_rejects_non_arden_file_paths() {
    let temp_root = make_temp_project_root("cli-test-non-arden");
    let text_file = temp_root.join("notes.txt");
    fs::write(
        &text_file,
        "@Test\nfunction nope(): None { return None; }\n",
    )
    .expect("write non-arden file");

    let err = run_tests(Some(&text_file), true, None).expect_err("non-arden file path should fail");
    assert!(
        err.contains("is not an .arden file"),
        "expected non-arden file error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_reports_source_context_for_parse_errors() {
    let temp_root = make_temp_project_root("cli-test-parse-source-context");
    let test_file = temp_root.join("broken_test.arden");
    fs::write(
        &test_file,
        "@Test\nfunction broken(: None { return None; }\n",
    )
    .expect("write malformed test source");

    let err = run_tests(Some(&test_file), true, None)
        .expect_err("test command should report parse source context");
    assert!(err.contains("broken_test.arden:2:"), "{err}");
    assert!(err.contains("-->"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_without_path_uses_project_file_list_only() {
    let temp_root = make_temp_project_root("cli-test-project-default");
    let src_dir = temp_root.join("src");
    let examples_dir = temp_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("create examples dir");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/math_test.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .expect("write main");
    fs::write(
        src_dir.join("math_test.arden"),
        "@Test\nfunction listedTest(): None { return None; }\n",
    )
    .expect("write listed test");
    fs::write(
        examples_dir.join("broken_test.arden"),
        "@Test\nfunction broken(: None { return None; }\n",
    )
    .expect("write stray broken test");

    with_current_dir(&temp_root, || {
        run_tests(None, true, Some("listedTest"))
            .expect("default project test discovery should ignore non-project files");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_without_path_executes_tests_in_non_test_named_project_files() {
    let temp_root = make_temp_project_root("cli-test-project-non-test-filename");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "@Test\nfunction smokeFromMain(): None { assert_eq(2 + 2, 4); return None; }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main with test");

    with_current_dir(&temp_root, || {
        run_tests(None, false, Some("smokeFromMain")).expect(
            "project default test discovery should execute tests from non-test-named files",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_does_not_delete_existing_test_runner_neighbor_files() {
    let temp_root = make_temp_project_root("cli-test-runner-neighbor-files");
    let test_file = temp_root.join("smoke_test.arden");
    let existing_runner = temp_root.join("smoke_test.test_runner.arden");
    let existing_exe = temp_root.join("smoke_test.test_runner.exe");
    fs::write(
        &test_file,
        r#"
                @Test
                function smoke(): None { return None; }
            "#,
    )
    .expect("write test file");
    fs::write(&existing_runner, "keep me\n").expect("write neighboring runner file");
    fs::write(&existing_exe, "keep me too\n").expect("write neighboring exe file");

    run_tests(Some(&test_file), false, Some("smoke"))
        .expect("test execution should succeed without touching neighboring files");

    assert_eq!(
        fs::read_to_string(&existing_runner).expect("read neighboring runner"),
        "keep me\n"
    );
    assert_eq!(
        fs::read_to_string(&existing_exe).expect("read neighboring exe"),
        "keep me too\n"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_executes_project_local_alias_import_tests() {
    let temp_root = make_temp_project_root("cli-test-project-alias-imports");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden", "src/math_spec.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .expect("write main");
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nmodule Math {\n    class Box<T> { value: T; constructor(value: T) { this.value = value; } function get(): T { return this.value; } }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("math_spec.arden"),
            "package tests;\nimport lib as l;\n@Test\nfunction aliasImportTest(): None { value: Integer = l.Math.Box<Integer>(3).get(); assert_eq(value, 3); return None; }\n",
        )
        .expect("write test");

    with_current_dir(&temp_root, || {
        run_tests(None, false, Some("aliasImportTest"))
            .expect("project-local alias imports in tests should run successfully");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_build_reports_import_check_errors_only_once() {
    let _lock = cli_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_root = make_temp_project_root("cli-build-import-check-single-print");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer { value: root.M.Box = root.M.Box(7); return value.value; }\n",
        )
        .expect("write main");
    fs::write(
            temp_root.join("src/helper.arden"),
            "package app;\nmodule M { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } }\n",
        )
        .expect("write helper");

    let status = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(env!("CARGO_MANIFEST_DIR").to_string() + "/Cargo.toml")
        .arg("--")
        .arg("build")
        .current_dir(&temp_root)
        .status()
        .expect("run initial project build");
    assert!(status.success(), "initial build should succeed");

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(
            temp_root.join("src/helper.arden"),
            "package app;\nmodule M { class Other { value: Integer; constructor(value: Integer) { this.value = value; } } }\n",
        )
        .expect("rewrite helper without imported constructor symbol");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(env!("CARGO_MANIFEST_DIR").to_string() + "/Cargo.toml")
        .arg("--")
        .arg("build")
        .current_dir(&temp_root)
        .output()
        .expect("run stale-import build");
    assert!(
        !output.status.success(),
        "stale import build should fail: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Imported namespace alias 'root' has no member 'M.Box'"),
        "{stderr}"
    );
    assert_eq!(
        stderr
            .matches("Imported namespace alias 'root' has no member 'M.Box'")
            .count(),
        1,
        "{stderr}"
    );
    assert_eq!(stderr.matches("Import check failed").count(), 1, "{stderr}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_accepts_relative_project_file_path() {
    let temp_root = make_temp_project_root("cli-test-relative-project-file");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\n@Test\nfunction smoke(): None { assert_eq(1, 1); return None; }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main test file");

    with_current_dir(&temp_root, || {
        run_tests(Some(Path::new("src/main.arden")), false, Some("smoke"))
            .expect("relative project file path should execute tests");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_accepts_relative_single_file_path_in_current_directory() {
    let temp_root = make_temp_project_root("cli-test-relative-single-file");
    let test_file = temp_root.join("smoke_test.arden");
    fs::write(
        &test_file,
        "@Test\nfunction smoke(): None { return None; }\n",
    )
    .expect("write test file");

    with_current_dir(&temp_root, || {
        run_tests(Some(Path::new("smoke_test.arden")), false, Some("smoke"))
            .expect("relative single-file path should execute tests");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_skips_before_hooks_for_ignored_tests() {
    let temp_root = make_temp_project_root("cli-test-ignore-skips-before");
    let test_file = temp_root.join("ignored_before_test.arden");
    fs::write(
        &test_file,
        r#"
                @Before
                function setup(): None { fail("before hook should not run for ignored test"); }

                @Test
                @Ignore("later")
                function skipped(): None { return None; }
            "#,
    )
    .expect("write ignored before test");

    run_tests(Some(&test_file), false, Some("skipped"))
        .expect("ignored test should skip before hook execution");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_skips_after_hooks_for_ignored_tests() {
    let temp_root = make_temp_project_root("cli-test-ignore-skips-after");
    let test_file = temp_root.join("ignored_after_test.arden");
    fs::write(
        &test_file,
        r#"
                @After
                function teardown(): None { fail("after hook should not run for ignored test"); }

                @Test
                @Ignore("later")
                function skipped(): None { return None; }
            "#,
    )
    .expect("write ignored after test");

    run_tests(Some(&test_file), false, Some("skipped"))
        .expect("ignored test should skip after hook execution");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_run_tests_accepts_ignore_reasons_with_literal_braces() {
    let temp_root = make_temp_project_root("cli-test-ignore-reason-braces");
    let test_file = temp_root.join("ignored_braces_test.arden");
    fs::write(
        &test_file,
        "@Test\n@Ignore(\"\\{danger\\}\")\nfunction skipped(): None { return None; }\n",
    )
    .expect("write ignored braces test");

    run_tests(Some(&test_file), false, Some("skipped"))
        .expect("ignored test reason with literal braces should execute cleanly");

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(unix)]
#[test]
fn cli_run_tests_skips_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("cli-test-symlink-dir");
    let tests_dir = temp_root.join("tests");
    let outside_dir = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-tests-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&tests_dir).expect("create tests dir");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    fs::write(
        outside_dir.join("escape_test.arden"),
        "@Test\nfunction escaped(): None { return None; }\n",
    )
    .expect("write outside test");
    symlink(&outside_dir, tests_dir.join("linked-outside")).expect("create dir symlink");

    let files =
        find_test_files(&tests_dir).expect("test discovery should skip symlink directories");
    assert!(
        files.is_empty(),
        "symlinked outside directory should not be traversed: {files:?}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_dir_all(outside_dir);
}

#[cfg(unix)]
#[test]
fn cli_run_tests_rejects_symlinked_file_paths_outside_root() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("cli-test-symlink-file");
    let outside_file = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-test-file-{}-{}.arden",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let linked_file = temp_root.join("linked_test.arden");
    fs::write(
        &outside_file,
        "@Test\nfunction escaped(): None { return None; }\n",
    )
    .expect("write outside test file");
    symlink(&outside_file, &linked_file).expect("create file symlink");

    let err = run_tests(Some(&linked_file), true, None)
        .expect_err("test command should reject symlinked files escaping the root");
    assert!(
        err.contains("resolves outside the requested directory tree"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_file(outside_file);
}

#[cfg(unix)]
#[test]
fn cli_run_tests_accepts_symlinked_file_paths_inside_root() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("cli-test-safe-symlink-file");
    let real_file = temp_root.join("real_test.arden");
    let linked_file = temp_root.join("linked_test.arden");
    fs::write(
        &real_file,
        "@Test\nfunction smoke(): None { return None; }\n",
    )
    .expect("write real test file");
    symlink(&real_file, &linked_file).expect("create file symlink");

    run_tests(Some(&linked_file), true, Some("smoke"))
        .expect("safe in-tree symlinked test file should be accepted");

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(unix)]
#[test]
fn cli_run_tests_rejects_paths_through_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("cli-test-symlink-ancestor");
    let outside_dir = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-test-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let linked_dir = temp_root.join("linked-tests");
    let linked_file = linked_dir.join("escape_test.arden");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    fs::write(
        outside_dir.join("escape_test.arden"),
        "@Test\nfunction escaped(): None { return None; }\n",
    )
    .expect("write outside test");
    symlink(&outside_dir, &linked_dir).expect("create dir symlink");

    let err = run_tests(Some(&linked_file), true, None)
        .expect_err("test command should reject symlinked ancestor directories");
    assert!(
        err.contains("must not traverse symlinked directories"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_dir_all(outside_dir);
}

#[test]
fn cli_format_targets_rejects_project_files_outside_root() {
    let temp_root = make_temp_project_root("cli-fmt-outside-root");
    let outside_file = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-format-{}-{}.arden",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::write(&outside_file, "function stray(): None { return None; }\n")
        .expect("write outside file");
    let rel_outside = format!(
        "../{}",
        outside_file
            .file_name()
            .and_then(|name| name.to_str())
            .expect("outside file name")
    );
    write_test_project_config(
        &temp_root,
        &["src/main.arden", &rel_outside],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = format_targets(None, true)
            .expect_err("fmt should reject project files outside the root");
        assert!(
            err.contains("resolves outside the project root"),
            "expected outside-root validation error, got: {err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_file(outside_file);
}

#[cfg(unix)]
#[test]
fn collect_arden_files_skips_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("collect-arden-symlink-dir");
    let real_dir = temp_root.join("real");
    let outside_dir = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&real_dir).expect("create real dir");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    let inside_file = real_dir.join("inside.arden");
    let outside_file = outside_dir.join("outside.arden");
    fs::write(&inside_file, "function inside(): None { return None; }\n")
        .expect("write inside file");
    fs::write(&outside_file, "function outside(): None { return None; }\n")
        .expect("write outside file");
    symlink(&outside_dir, temp_root.join("linked-outside")).expect("create dir symlink");

    let files =
        collect_arden_files(&temp_root).expect("collect_arden_files should skip symlink dirs");
    assert!(
        files.contains(&inside_file),
        "expected real arden file to be discovered: {files:?}"
    );
    assert!(
        !files.contains(&outside_file),
        "symlinked outside directory should not be traversed: {files:?}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_dir_all(outside_dir);
}

#[cfg(unix)]
#[test]
fn collect_arden_files_rejects_symlinked_file_paths_outside_root() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("collect-arden-symlink-file");
    let outside_file = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-format-file-{}-{}.arden",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let linked_file = temp_root.join("linked_format.arden");
    fs::write(&outside_file, "function escaped(): None { return None; }\n")
        .expect("write outside file");
    symlink(&outside_file, &linked_file).expect("create file symlink");

    let err = collect_arden_files(&linked_file)
        .expect_err("fmt file collection should reject symlinked files escaping the root");
    assert!(
        err.contains("resolves outside the requested directory tree"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_file(outside_file);
}

#[cfg(unix)]
#[test]
fn collect_arden_files_accepts_symlinked_file_paths_inside_root() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("collect-arden-safe-symlink-file");
    let real_file = temp_root.join("real.arden");
    let linked_file = temp_root.join("linked.arden");
    fs::write(&real_file, "function smoke(): None { return None; }\n")
        .expect("write real arden file");
    symlink(&real_file, &linked_file).expect("create file symlink");

    let files = collect_arden_files(&linked_file)
        .expect("safe in-tree symlinked arden file should be accepted");
    assert_eq!(files, vec![linked_file]);

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(unix)]
#[test]
fn collect_arden_files_rejects_paths_through_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let temp_root = make_temp_project_root("collect-arden-symlink-ancestor");
    let outside_dir = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-format-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let linked_dir = temp_root.join("linked-format");
    let linked_file = linked_dir.join("escape.arden");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    fs::write(
        outside_dir.join("escape.arden"),
        "function escaped(): None { return None; }\n",
    )
    .expect("write outside file");
    symlink(&outside_dir, &linked_dir).expect("create dir symlink");

    let err = collect_arden_files(&linked_file)
        .expect_err("fmt file collection should reject symlinked ancestor directories");
    assert!(
        err.contains("must not traverse symlinked directories"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_dir_all(outside_dir);
}

#[test]
fn cli_lint_target_rejects_entry_outside_root() {
    let temp_root = make_temp_project_root("cli-lint-outside-root");
    let outside_file = temp_root.parent().expect("temp root parent").join(format!(
        "arden-outside-lint-{}-{}.arden",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::write(&outside_file, "function stray(): None { return None; }\n")
        .expect("write outside file");
    let rel_outside = format!(
        "../{}",
        outside_file
            .file_name()
            .and_then(|name| name.to_str())
            .expect("outside file name")
    );
    write_test_project_config(
        &temp_root,
        &["src/main.arden", &rel_outside],
        &rel_outside,
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nfunction helper(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = lint_target(None).expect_err("lint should reject entry outside the root");
        assert!(
            err.contains("resolves outside the project root"),
            "expected outside-root validation error, got: {err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_file(outside_file);
}

#[test]
fn cli_lint_target_rejects_directory_paths() {
    let temp_root = make_temp_project_root("cli-lint-dir-path");

    let err = lint_target(Some(&temp_root)).expect_err("lint should reject directory paths");
    assert!(
        err.contains("is not a file"),
        "expected directory path validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_info_rejects_invalid_project_opt_level() {
    let temp_root = make_temp_project_root("cli-info-invalid-opt-level");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"smoke\"\nopt_level = \"turbo\"\n",
        )
        .expect("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = show_project_info().expect_err("info should reject invalid opt level");
        assert!(err.contains("Invalid optimization level"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_info_rejects_non_arden_entry_path() {
    let temp_root = make_temp_project_root("cli-info-non-arden-entry");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.txt\"\nfiles = [\"src/main.txt\"]\noutput = \"smoke\"\n",
        )
        .expect("write arden.toml");
    fs::write(temp_root.join("src/main.txt"), "not arden\n").expect("write main");

    with_current_dir(&temp_root, || {
        let err = show_project_info().expect_err("info should reject non-arden entry");
        assert!(
            err.contains("must resolve to an .arden source file")
                || err.contains("is not an .arden file"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_info_rejects_directory_entry_path() {
    let temp_root = make_temp_project_root("cli-info-directory-entry");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src\"\nfiles = [\"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .expect("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = show_project_info().expect_err("info should reject directory entry path");
        assert!(
            err.contains("must resolve to a file") || err.contains("is not a file"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_info_rejects_non_arden_secondary_file_path() {
    let temp_root = make_temp_project_root("cli-info-non-arden-secondary-file");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.txt\"]\noutput = \"smoke\"\n",
        )
        .expect("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");
    fs::write(temp_root.join("src/helper.txt"), "not arden\n").expect("write helper");

    with_current_dir(&temp_root, || {
        let err = show_project_info().expect_err("info should reject non-arden secondary file");
        assert!(
            err.contains("must resolve to an .arden source file")
                || err.contains("is not an .arden file"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_fix_target_rejects_non_arden_file_paths() {
    let temp_root = make_temp_project_root("cli-fix-non-arden");
    let text_file = temp_root.join("notes.txt");
    fs::write(&text_file, "not arden\n").expect("write text file");

    let err = fix_target(Some(&text_file)).expect_err("fix should reject non-arden files");
    assert!(
        err.contains("is not an .arden file"),
        "expected non-arden validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_lex_file_rejects_directory_paths() {
    let temp_root = make_temp_project_root("cli-lex-dir-path");

    let err = lex_file(&temp_root).expect_err("lex should reject directory paths");
    assert!(
        err.contains("is not a file"),
        "expected directory path validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_parse_file_rejects_directory_paths() {
    let temp_root = make_temp_project_root("cli-parse-dir-path");

    let err = parse_file(&temp_root).expect_err("parse should reject directory paths");
    assert!(
        err.contains("is not a file"),
        "expected directory path validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_parse_file_reports_source_context_for_parse_errors() {
    let temp_root = make_temp_project_root("cli-parse-source-context");
    let source_file = temp_root.join("broken.arden");
    fs::write(&source_file, "function main(: None { return None; }\n")
        .expect("write malformed source");

    let err = parse_file(&source_file).expect_err("parse should report syntax error");
    assert!(err.contains("broken.arden:1:"), "{err}");
    assert!(err.contains("-->"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_compile_file_rejects_non_arden_paths() {
    let temp_root = make_temp_project_root("cli-compile-non-arden");
    let text_file = temp_root.join("notes.txt");
    fs::write(&text_file, "not arden\n").expect("write text file");

    let err = compile_file(&text_file, None, false, true, None, None)
        .expect_err("compile should reject non-arden files");
    assert!(
        err.contains("is not an .arden file"),
        "expected non-arden validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_commands_consistently_reject_invalid_files_before_output_processing() {
    let temp_root = make_temp_project_root("cli-invalid-file-before-output");
    let text_file = temp_root.join("notes.txt");
    let output_path = temp_root.join("nested/bin/app");
    fs::write(&text_file, "not arden\n").expect("write text file");

    let err = compile_file(&text_file, Some(&output_path), false, true, None, None)
        .expect_err("compile should reject invalid source before touching output path");
    assert!(err.contains("is not an .arden file"), "{err}");
    assert!(
        !output_path.exists(),
        "output path should not be created on source validation failure"
    );
    assert!(
        !output_path.parent().expect("parent").exists(),
        "output parent should not be created on source validation failure"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_project_commands_consistently_report_invalid_file_list_entries() {
    let temp_root = make_temp_project_root("cli-project-invalid-files-list-order");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/helper.txt\", \"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .expect("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");
    fs::write(temp_root.join("src/helper.txt"), "not arden\n").expect("write helper");

    with_current_dir(&temp_root, || {
        let info_err =
            show_project_info().expect_err("info should reject invalid files list entry");
        let check_err = check_file(None).expect_err("check should reject invalid files list entry");
        let lint_err = lint_target(None).expect_err("lint should reject invalid files list entry");
        let build_err = build_project(false, false, true, true, false)
            .expect_err("build should reject invalid files list entry");

        for err in [info_err, check_err, lint_err, build_err] {
            assert!(
                err.contains("src/helper.txt") || err.contains("is not an .arden file"),
                "{err}"
            );
        }
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_compile_creates_missing_output_parent_directory() {
    let temp_root = make_temp_project_root("cli-compile-create-parent");
    let source_file = temp_root.join("main.arden");
    let output_path = temp_root.join("nested/bin/app");
    fs::write(&source_file, "function main(): None { return None; }\n").expect("write source");

    compile_file(&source_file, Some(&output_path), true, true, None, None)
        .expect("compile should create missing output directories");

    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_check_file_rejects_directory_paths() {
    let temp_root = make_temp_project_root("cli-check-dir-path");

    let err = check_file(Some(&temp_root)).expect_err("check should reject directory paths");
    assert!(
        err.contains("is not a file"),
        "expected directory path validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_check_file_rejects_non_arden_paths() {
    let temp_root = make_temp_project_root("cli-check-non-arden");
    let text_file = temp_root.join("notes.txt");
    fs::write(&text_file, "not arden\n").expect("write text file");

    let err = check_file(Some(&text_file)).expect_err("check should reject non-arden files");
    assert!(
        err.contains("is not an .arden file"),
        "expected non-arden validation error, got: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_check_command_reports_cross_file_type_errors() {
    let temp_root = make_temp_project_root("cli-check-type-error");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nfunction main(): None { value: Integer = helper(); return None; }\n",
    )
    .expect("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction helper(): String { return \"oops\"; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false).expect_err("project check should fail");
        assert!(
            err.contains("Type mismatch")
                || err.contains("expected Integer")
                || err.contains("Expected Integer"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_info_rejects_output_path_matching_project_config() {
    let temp_root = make_temp_project_root("cli-info-output-config-collision");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"arden.toml\"\n",
        )
        .expect("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = show_project_info()
            .expect_err("info should reject output path matching project config");
        assert!(err.contains("project config"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn cli_check_command_reports_cross_file_borrow_errors() {
    let temp_root = make_temp_project_root("cli-check-borrow-error");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nfunction main(): None { s: String = \"hello\"; consume(s); t: String = s; return None; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.arden"),
        "package app;\nfunction consume(owned s: String): None { return None; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false).expect_err("project check should fail");
        assert!(
            err.contains("Use of moved value 's'") || err.contains("moved value 's'"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}
