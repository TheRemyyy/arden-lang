use std::path::Path;

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use super::make_temp_project_root;
#[cfg(unix)]
use super::TestExpectErrExt;
#[cfg(unix)]
use super::TestExpectExt;

#[test]
fn is_test_like_file_matches_test_and_spec_suffixes() {
    assert!(crate::cli::test_discovery::is_test_like_file(Path::new(
        "demo_test.arden"
    )));
    assert!(crate::cli::test_discovery::is_test_like_file(Path::new(
        "demoSpec.arden"
    )));
    assert!(!crate::cli::test_discovery::is_test_like_file(Path::new(
        "demo.arden"
    )));
    assert!(!crate::cli::test_discovery::is_test_like_file(Path::new(
        "demo_test.txt"
    )));
    assert!(!crate::cli::test_discovery::is_test_like_file(Path::new(
        "latest.arden"
    )));
    assert!(!crate::cli::test_discovery::is_test_like_file(Path::new(
        "contest.arden"
    )));
}

#[cfg(unix)]
#[test]
fn find_test_files_ignores_symlink_entries() {
    let temp_root = make_temp_project_root("cli-test-discovery-symlink");
    let tests_dir = temp_root.join("tests");
    fs::create_dir_all(&tests_dir).must("create tests dir");
    fs::write(
        tests_dir.join("real_test.arden"),
        "function main(): None { return None; }\n",
    )
    .must("write real test");

    let linked_dir = temp_root.join("linked-tests");
    std::os::unix::fs::symlink(&tests_dir, &linked_dir).must("create symlink");

    let discovered = crate::cli::test_discovery::find_test_files(&temp_root)
        .must("discover test files should succeed");

    assert_eq!(discovered, vec![tests_dir.join("real_test.arden")]);

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(unix)]
#[test]
fn find_test_files_rejects_symlinked_root_directory() {
    let temp_root = make_temp_project_root("cli-test-discovery-root-symlink");
    let real_dir = temp_root.join("real-tests");
    fs::create_dir_all(&real_dir).must("create real tests dir");
    fs::write(
        real_dir.join("smoke_test.arden"),
        "@Test\nfunction smoke(): None { return None; }\n",
    )
    .must("write test file");
    let linked_dir = temp_root.join("linked-tests");
    std::os::unix::fs::symlink(&real_dir, &linked_dir).must("create root symlink dir");

    let err = crate::cli::test_discovery::find_test_files(&linked_dir)
        .must_err("symlink root should fail");
    assert!(
        err.contains("must not be a symlinked directory")
            || err.contains("must not traverse symlinked directories"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(unix)]
#[test]
fn find_test_files_rejects_symlinked_ancestor_directory() {
    let temp_root = make_temp_project_root("cli-test-discovery-ancestor-symlink");
    let real_parent = temp_root.join("real-parent");
    let real_tests_dir = real_parent.join("tests");
    fs::create_dir_all(&real_tests_dir).must("create real tests dir");
    fs::write(
        real_tests_dir.join("escape_test.arden"),
        "@Test\nfunction smoke(): None { return None; }\n",
    )
    .must("write test file");
    let linked_parent = temp_root.join("linked-parent");
    std::os::unix::fs::symlink(&real_parent, &linked_parent).must("create parent symlink");
    let linked_tests_dir = linked_parent.join("tests");

    let err = crate::cli::test_discovery::find_test_files(&linked_tests_dir)
        .must_err("symlink ancestor should fail");
    assert!(
        err.contains("must not traverse symlinked directories"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}
