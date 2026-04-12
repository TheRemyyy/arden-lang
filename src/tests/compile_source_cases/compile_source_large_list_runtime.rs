use super::*;
use std::fs;

#[test]
fn compile_source_runs_large_nested_list_workload_without_runtime_crash() {
    let temp_root = make_temp_project_root("large-nested-list-runtime");
    let source_path = temp_root.join("large_nested_list_runtime.arden");
    let output_path = temp_root.join("large_nested_list_runtime");
    let source = r#"
            function idx(i: Integer, j: Integer, n: Integer): Integer {
                return i * n + j;
            }

            function main(): Integer {
                size: Integer = 90;
                total: Integer = size * size;

                a: List<Integer> = List<Integer>();
                b: List<Integer> = List<Integer>();
                c: List<Integer> = List<Integer>();

                for (p in 0..total) {
                    a.push(((p * 17 + 13) % 97) - 48);
                    b.push(((p * 31 + 7) % 89) - 44);
                    c.push(0);
                }

                mut i: Integer = 0;
                while (i < size) {
                    mut j: Integer = 0;
                    while (j < size) {
                        mut sum: Integer = 0;
                        mut k: Integer = 0;
                        while (k < size) {
                            left: Integer = a.get(idx(i, k, size));
                            right: Integer = b.get(idx(k, j, size));
                            sum = sum + (left * right);
                            k = k + 1;
                        }
                        c.set(idx(i, j, size), sum);
                        j = j + 1;
                    }
                    i = i + 1;
                }

                mut checksum: Integer = 0;
                mut q: Integer = 0;
                while (q < total) {
                    checksum = checksum + c.get(q);
                    q = q + 1;
                }

                return if (checksum != 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("large nested list runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled large nested list binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
