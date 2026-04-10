use std::fs;
use std::path::Path;

use super::*;

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
}

#[cfg(unix)]
#[test]
fn find_test_files_ignores_symlink_entries() {
    let temp_root = make_temp_project_root("cli-test-discovery-symlink");
    let tests_dir = temp_root.join("tests");
    fs::create_dir_all(&tests_dir).expect("create tests dir");
    fs::write(
        tests_dir.join("real_test.arden"),
        "function main(): None { return None; }\n",
    )
    .expect("write real test");

    let linked_dir = temp_root.join("linked-tests");
    std::os::unix::fs::symlink(&tests_dir, &linked_dir).expect("create symlink");

    let discovered = crate::cli::test_discovery::find_test_files(&temp_root)
        .expect("discover test files should succeed");

    assert_eq!(discovered, vec![tests_dir.join("real_test.arden")]);

    let _ = fs::remove_dir_all(temp_root);
}
