use super::*;
use std::fs;

fn ir_contains_declaration_with_fragments(ir: &str, fragments: &[&str]) -> bool {
    ir.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("declare ")
            && fragments.iter().all(|fragment| trimmed.contains(fragment))
    })
}

#[test]
fn compile_source_runs_typed_option_constructor_through_function_return() {
    let temp_root = make_temp_project_root("typed-option-constructor-runtime");
    let source_path = temp_root.join("typed_option_constructor_runtime.arden");
    let output_path = temp_root.join("typed_option_constructor_runtime");
    let source = r#"
            function build(): Option<List<Option<Integer>>> {
                return Option<List<Option<Integer>>>();
            }

            function main(): Integer {
                value: Option<List<Option<Integer>>> = build();
                if (value.is_none()) {
                    return 82;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("typed option constructor return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled typed option constructor binary");
    assert_eq!(status.code(), Some(82));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_typed_result_constructor_through_function_return() {
    let temp_root = make_temp_project_root("typed-result-constructor-runtime");
    let source_path = temp_root.join("typed_result_constructor_runtime.arden");
    let output_path = temp_root.join("typed_result_constructor_runtime");
    let source = r#"
            function build(): Result<List<Option<Integer>>, String> {
                return Result<List<Option<Integer>>, String>();
            }

            function main(): Integer {
                result: Result<List<Option<Integer>>, String> = build();
                return match (result) {
                    Result.Ok(xs) => xs.length(),
                    Result.Error(err) => 83,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("typed result constructor return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled typed result constructor binary");
    assert_eq!(status.code(), Some(83));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_uses_typed_heap_sizes_for_builtin_smart_pointer_constructors() {
    let temp_root = make_temp_project_root("typed-smart-pointer-ir");
    let source_path = temp_root.join("typed_smart_pointer_ir.arden");
    let output_path = temp_root.join("typed_smart_pointer_ir");
    let source = r#"
            function main(): None {
                box_value: Box<List<Option<Integer>>> = Box<List<Option<Integer>>>();
                rc_value: Rc<List<Option<Integer>>> = Rc<List<Option<Integer>>>();
                arc_value: Arc<List<Option<Integer>>> = Arc<List<Option<Integer>>>();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(
        source,
        &source_path,
        &output_path,
        true,
        true,
        Some("0"),
        None,
    )
    .must("typed smart pointer constructors should codegen");

    let ir_path = output_path.with_extension("ll");
    let ir = fs::read_to_string(&ir_path).must("read generated llvm ir");
    let malloc_24_count = ir
        .lines()
        .filter(|line| line.contains("@malloc(i64 24)"))
        .count();
    assert!(
        malloc_24_count >= 3,
        "expected Box/Rc/Arc constructors to allocate 24-byte List<Option<Integer>> payloads, found {malloc_24_count} matching malloc calls in {}",
        ir_path.display()
    );
    assert!(
        !ir.contains("call ptr @malloc(i64 8)"),
        "builtin smart pointer constructors should not use hard-coded 8-byte payload allocations"
    );
    assert!(
        !ir.contains("call ptr @malloc(i64 16)"),
        "builtin smart pointer constructors should not use hard-coded 16-byte payload allocations"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_raw_ptr_deref_with_float_payloads() {
    let temp_root = make_temp_project_root("ptr-float-deref-codegen");
    let source_path = temp_root.join("ptr_float_deref_codegen.arden");
    let output_path = temp_root.join("ptr_float_deref_codegen");
    let source = r#"
            function load(slot: Ptr<Float>): Float {
                return *slot;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("raw Ptr<Float> deref should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_await_timeout_method_chain() {
    let temp_root = make_temp_project_root("async-block-await-timeout-runtime");
    let source_path = temp_root.join("async_block_await_timeout_runtime.arden");
    let output_path = temp_root.join("async_block_await_timeout_runtime");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = (async { 7 }).await_timeout(100);
                return if (value.unwrap() == 7) { 84 } else { 0 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("async block await_timeout chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled async block await_timeout binary");
    assert_eq!(status.code(), Some(84));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_await_timeout_zero_pending_runtime() {
    let temp_root = make_temp_project_root("await-timeout-zero-pending-runtime");
    let source_path = temp_root.join("await_timeout_zero_pending_runtime.arden");
    let output_path = temp_root.join("await_timeout_zero_pending_runtime");
    let source = r#"
            import std.time.*;
            import std.fs.*;

            function work(): Task<Integer> {
                return async {
                    Time.sleep(50);
                    return 7;
                };
            }

            function main(): Integer {
                maybe: Option<Integer> = work().await_timeout(0);
                return if (maybe.is_none()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("await_timeout(0) on pending task should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled await_timeout zero pending binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_supports_async_block_is_done_method_chain() {
    let temp_root = make_temp_project_root("async-block-is-done-codegen");
    let source_path = temp_root.join("async_block_is_done_codegen.arden");
    let output_path = temp_root.join("async_block_is_done_codegen");
    let source = r#"
            function main(): None {
                done: Boolean = (async { 1 }).is_done();
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("async block is_done chain should codegen");
    assert!(output_path.with_extension("ll").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_identity_equality() {
    let temp_root = make_temp_project_root("map-eq-runtime");
    let source_path = temp_root.join("map_eq_runtime.arden");
    let output_path = temp_root.join("map_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(2));
                if (m == m) { return 35; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map equality binary");
    assert_eq!(status.code(), Some(35));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_class_identity_equality() {
    let temp_root = make_temp_project_root("class-eq-runtime");
    let source_path = temp_root.join("class_eq_runtime.arden");
    let output_path = temp_root.join("class_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(2);
                if (b == b) { return 36; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("class identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled class equality binary");
    assert_eq!(status.code(), Some(36));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_emits_platform_correct_libc_signatures() {
    let temp_root = make_temp_project_root("libc-signatures-ir");
    let source_path = temp_root.join("libc_signatures_ir.arden");
    let output_path = temp_root.join("libc_signatures_ir");
    let source = r#"
            import std.time.*;
            import std.fs.*;

            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                xs.push(3);
                xs.push(4);
                xs.push(5);

                _s: String = "{xs.length()}";
                _now: String = Time.now("%Y-%m-%d %H:%M:%S");
                _file: String = File.read("does_not_exist.txt");
                _maybe: Option<Integer> = (async { return 2; }).await_timeout(0);

                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, true, true, None, None)
        .must("libc signature probe source should codegen");

    let ir_path = output_path.with_extension("ll");
    let ir = fs::read_to_string(&ir_path).must("read generated llvm ir");

    #[cfg(target_pointer_width = "32")]
    let size_t_ty = "i32";
    #[cfg(not(target_pointer_width = "32"))]
    let size_t_ty = "i64";

    #[cfg(windows)]
    let long_ty = "i32";
    #[cfg(not(windows))]
    let long_ty = size_t_ty;

    #[cfg(windows)]
    let time_ty = "i64";
    #[cfg(not(windows))]
    let time_ty = long_ty;

    assert!(
        ir_contains_declaration_with_fragments(&ir, &["@malloc(", &format!("({size_t_ty}")]),
        "expected platform-correct malloc signature using `{size_t_ty}` in {}",
        ir_path.display()
    );
    assert!(
        ir_contains_declaration_with_fragments(&ir, &["@snprintf(", size_t_ty, "..."]),
        "expected platform-correct snprintf signature using `{size_t_ty}` in {}",
        ir_path.display()
    );
    assert!(
        ir_contains_declaration_with_fragments(&ir, &["@fseek(", long_ty, "i32"]),
        "expected platform-correct fseek signature using `{long_ty}` in {}",
        ir_path.display()
    );
    assert!(
        ir_contains_declaration_with_fragments(&ir, &["@ftell(", &format!("{long_ty} @ftell(ptr")]),
        "expected platform-correct ftell signature using `{long_ty}` in {}",
        ir_path.display()
    );
    assert!(
        ir_contains_declaration_with_fragments(&ir, &["@time(", &format!("{time_ty} @time(ptr")]),
        "expected platform-correct time signature using `{time_ty}` in {}",
        ir_path.display()
    );

    #[cfg(not(windows))]
    {
        assert!(
            ir_contains_declaration_with_fragments(
                &ir,
                &[
                    "@pthread_join(",
                    &format!("i32 @pthread_join({long_ty}, ptr)")
                ]
            ),
            "expected platform-correct pthread_join signature using `{long_ty}` in {}",
            ir_path.display()
        );
    }

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_unwrap_object_identity_equality() {
    let temp_root = make_temp_project_root("option-unwrap-object-eq-runtime");
    let source_path = temp_root.join("option_unwrap_object_eq_runtime.arden");
    let output_path = temp_root.join("option_unwrap_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(3);
                x: Option<Boxed> = Option.some(b);
                if (x.unwrap() == b) { return 37; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Option.unwrap object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Option.unwrap object equality binary");
    assert_eq!(status.code(), Some(37));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_get_object_identity_equality() {
    let temp_root = make_temp_project_root("map-get-object-eq-runtime");
    let source_path = temp_root.join("map_get_object_eq_runtime.arden");
    let output_path = temp_root.join("map_get_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(4);
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, b);
                if (m.get(1) == b) { return 38; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Map.get object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Map.get object equality binary");
    assert_eq!(status.code(), Some(38));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_await_timeout_unwrap_object_identity_equality() {
    let temp_root = make_temp_project_root("await-timeout-unwrap-object-eq-runtime");
    let source_path = temp_root.join("await_timeout_unwrap_object_eq_runtime.arden");
    let output_path = temp_root.join("await_timeout_unwrap_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            async function work(): Boxed {
                return Boxed(5);
            }

            function main(): Integer {
                b: Boxed = work().await_timeout(100).unwrap();
                if (b == b) { return 39; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("await_timeout unwrap object identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled await_timeout unwrap object equality binary");
    assert_eq!(status.code(), Some(39));

    let _ = fs::remove_dir_all(temp_root);
}
