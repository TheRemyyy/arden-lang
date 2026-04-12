use super::*;
use std::fs;

#[test]
fn source_driven_nested_tagged_storage_path_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-nested-tagged-storage-runtime");
    let source_path = temp_root.join("source_driven_nested_tagged_storage_runtime.arden");
    let output_path = temp_root.join("source_driven_nested_tagged_storage_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(10))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(20))));
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(11))));

                has_alpha: Boolean = store.contains(Result.error("alpha"));
                has_beta: Boolean = store.contains(Result.error("beta"));

                alpha: Result<Option<Boxed>, String> = store.get(Result.error("alpha"));
                beta: Result<Option<Boxed>, String> = store.get(Result.error("beta"));

                alpha_value: Integer = match (alpha) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value,
                        None => -1,
                    },
                    Error(err) => -2,
                };
                beta_value: Integer = match (beta) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value,
                        None => -3,
                    },
                    Error(err) => -4,
                };

                return if (has_alpha && has_beta && alpha_value == 11 && beta_value == 20 && store.length() == 11) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven nested tagged storage runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven nested tagged storage binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_set_remove_shift_scalar_observation_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-set-remove-shift-runtime");
    let source_path = temp_root.join("source_driven_set_remove_shift_runtime.arden");
    let output_path = temp_root.join("source_driven_set_remove_shift_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("alpha"));
                seen.add(Result.error("beta"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                has_alpha: Boolean = seen.contains(Result.error("alpha"));
                has_beta: Boolean = seen.contains(Result.error("beta"));
                has_four: Boolean = seen.contains(Result.ok(Option.some(4)));
                len: Integer = seen.length();
                return if (removed && has_alpha && has_beta && !has_four && len == 10) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven set remove shift runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven set remove shift binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_multi_overwrite_tagged_map_runtime_survives_codegen() {
    let temp_root = make_temp_project_root("source-driven-multi-overwrite-tagged-map-runtime");
    let source_path = temp_root.join("source_driven_multi_overwrite_tagged_map_runtime.arden");
    let output_path = temp_root.join("source_driven_multi_overwrite_tagged_map_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Result<Option<Integer>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(i + 100)));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(10)));
                store.set(Result.error("beta"), Result.ok(Option.some(20)));
                store.set(Result.error("alpha"), Result.ok(Option.some(11)));
                store.set(Result.error("beta"), Result.ok(Option.some(21)));

                a: Result<Option<Integer>, String> = store.get(Result.error("alpha"));
                b: Result<Option<Integer>, String> = store.get(Result.error("beta"));

                a_value: Integer = match (a) {
                    Ok(inner) => match (inner) {
                        Some(v) => v,
                        None => -1,
                    },
                    Error(err) => -2,
                };
                b_value: Integer = match (b) {
                    Ok(inner) => match (inner) {
                        Some(v) => v,
                        None => -3,
                    },
                    Error(err) => -4,
                };

                has_alpha: Boolean = store.contains(Result.error("alpha"));
                has_beta: Boolean = store.contains(Result.error("beta"));
                return if (a_value == 11 && b_value == 21 && has_alpha && has_beta && store.length() == 11) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven multi-overwrite tagged map runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven multi-overwrite tagged map binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_set_add_remove_readd_scalar_observation_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-set-readd-runtime");
    let source_path = temp_root.join("source_driven_set_readd_runtime.arden");
    let output_path = temp_root.join("source_driven_set_readd_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("alpha"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                seen.add(Result.ok(Option.some(4)));
                has_alpha: Boolean = seen.contains(Result.error("alpha"));
                has_four: Boolean = seen.contains(Result.ok(Option.some(4)));
                len: Integer = seen.length();
                return if (removed && has_alpha && has_four && len == 10) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven set readd runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven set readd binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
