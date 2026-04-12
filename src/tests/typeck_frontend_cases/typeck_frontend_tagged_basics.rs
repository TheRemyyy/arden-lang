use super::*;
use std::fs;

#[test]
fn rest_style_alias_heavy_tagged_pipeline_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Request {
    route: String;
    constructor(route: String) { this.route = route; }
}

class Response {
    code: Integer;
    body: String;
    constructor(code: Integer, body: String) {
        this.code = code;
        this.body = body;
    }
}

function decode(req: Request): Result<Option<Integer>, String> {
    if (req.route == "/users") {
        return Result.ok(Option.some(200));
    }
    return Result.error("missing");
}

function handle(req: Request, verbose: Boolean): Response {
    status: Integer = if (verbose) {
        match (decode(req)) {
            Success(inner) => match (inner) {
                Present(code) => code,
                Empty => 204,
            },
            Failure(err) => 500,
        }
    } else {
        400
    };
    return Response(status, "done");
}

function main(): Integer {
    return handle(Request("/users"), true).code;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn data_pipeline_tagged_container_growth_chains_survive_codegen() {
    let temp_root = make_temp_project_root("data-pipeline-tagged-container-runtime");
    let source_path = temp_root.join("data_pipeline_tagged_container_runtime.arden");
    let output_path = temp_root.join("data_pipeline_tagged_container_runtime");
    let source = r#"
            class Row {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function bump(): Row { return Row(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, Integer>, Option<Row>> {
                m: Map<Result<Option<Integer>, Integer>, Option<Row>> = Map<Result<Option<Integer>, Integer>, Option<Row>>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Result.ok(Option.some(i)), Option.some(Row(i)));
                    i = i + 1;
                }
                if (flag) {
                    m.set(Result.error(3), Option.some(Row(40)));
                }
                return m;
            }

            function main(): Integer {
                pipeline: Map<Result<Option<Integer>, Integer>, Option<Row>> = build(true);
                hit: Option<Row> = pipeline.get(Result.error(3));
                return hit.unwrap().bump().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("data pipeline tagged container runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled data pipeline tagged container binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn batch_style_tagged_container_mutation_chain_survives_codegen() {
    let temp_root = make_temp_project_root("batch-style-tagged-mutation-runtime");
    let source_path = temp_root.join("batch_style_tagged_mutation_runtime.arden");
    let output_path = temp_root.join("batch_style_tagged_mutation_runtime");
    let source = r#"
            class Row {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function bump(): Row { return Row(this.value + 1); }
            }

            function main(): Integer {
                queue: Map<Result<Option<Integer>, Integer>, Option<Row>> = Map<Result<Option<Integer>, Integer>, Option<Row>>();
                mut i: Integer = 0;
                while (i < 9) {
                    queue.set(Result.ok(Option.some(i)), Option.some(Row(i)));
                    i = i + 1;
                }
                queue.set(Result.error(3), Option.some(Row(10)));
                queue.set(Result.error(3), Option.some(Row(20)));
                had_old: Boolean = queue.contains(Result.ok(Option.some(4)));
                queue.set(Result.ok(Option.some(4)), Option.some(Row(30)));
                has_new: Boolean = queue.contains(Result.ok(Option.some(4)));
                picked: Option<Row> = queue.get(Result.error(3));
                restored: Option<Row> = queue.get(Result.ok(Option.some(4)));
                return if (had_old && has_new && queue.length() == 10) { picked.unwrap().bump().value + restored.unwrap().value } else { 0 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("batch-style tagged mutation runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled batch-style tagged mutation binary");
    assert_eq!(status.code(), Some(51));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn forward_declared_generic_class_enum_payload_survives_codegen() {
    let temp_root = make_temp_project_root("forward-declared-generic-enum-payload-runtime");
    let source_path = temp_root.join("forward_declared_generic_enum_payload_runtime.arden");
    let output_path = temp_root.join("forward_declared_generic_enum_payload_runtime");
    let source = r#"
            enum Choice {
                Boxed(Box<String>),
                Empty
            }

            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                current: Choice = Choice.Boxed(Box<String>("hi"));
                picked: Box<String> = match (current) {
                    Boxed(inner) => inner,
                    Empty => Box<String>("no")
                };
                return if (picked.value == "hi") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("forward-declared generic enum payload runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled forward-declared generic enum payload binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn forward_declared_generic_class_method_chain_survives_codegen() {
    let temp_root = make_temp_project_root("forward-declared-generic-method-chain-runtime");
    let source_path = temp_root.join("forward_declared_generic_method_chain_runtime.arden");
    let output_path = temp_root.join("forward_declared_generic_method_chain_runtime");
    let source = r#"
            enum Choice {
                Boxed(Box<String>),
                Empty
            }

            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                return if ({
                    current: Choice = Choice.Boxed(Box<String>("hi"));
                    match (current) {
                        Boxed(inner) => inner,
                        Empty => Box<String>("no")
                    }
                }.get().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("forward-declared generic method chain runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled forward-declared generic method chain binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn bound_generic_method_value_survives_codegen() {
    let temp_root = make_temp_project_root("bound-generic-method-value-runtime");
    let source_path = temp_root.join("bound_generic_method_value_runtime.arden");
    let output_path = temp_root.join("bound_generic_method_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                box: Box<String> = Box<String>("hello");
                getter: () -> String = box.get;
                return if (getter().length() == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("bound generic method value runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled bound generic method value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn alias_heavy_ultra_edge_tagged_container_method_chain_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(50)));
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(60)));
    }
    return store;
}

function main(): Integer {
    value: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Present(row) => Option.some(row.inc()),
        Empty => Option.some(Boxed(0)),
    };
    return value.unwrap().value;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn alias_heavy_tagged_runtime_pipeline_with_updates_and_chains() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    mut i: Integer = 0;
    while (i < 9) {
        store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
        i = i + 1;
    }
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(70)));
        store.set(Result.error("missing"), Option.some(Boxed(80)));
    }
    return store;
}

function main(): Integer {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = build(true);
    fallback: Option<Boxed> = store.get(Result.ok(Option.none()));
    value: Option<Boxed> = match (store.get(Result.error("missing"))) {
        Present(row) => Option.some(row.inc()),
        Empty => fallback,
    };
    return value.unwrap().value;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_tagged_key_pipeline_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("σφάλμα🚀"), Option.some(Boxed(90)));
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(5)));
    return store;
}

function main(): Integer {
    chosen: Option<Boxed> = match (build().get(Result.error("σφάλμα🚀"))) {
        Present(row) => Option.some(row.inc()),
        Empty => Option.some(Boxed(0)),
    };
    status: Boolean = build().contains(Result.error("σφάλμα🚀"));
    return if (status) { chosen.unwrap().value } else { 0 };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_tagged_join_and_equality_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function choose(flag: Boolean): Result<Option<Boxed>, String> {
    return if (flag) {
        Result.ok(Option.some(Boxed(91)))
    } else {
        Result.error("σφάλμα🚀")
    };
}

function main(): Integer {
    picked: Result<Option<Boxed>, String> = match (true) {
        true => choose(true),
        false => choose(false),
    };
    if (picked == Result.error("σφάλμα🚀")) {
        return 0;
    }
    value: Option<Boxed> = match (picked) {
        Success(row) => row,
        Failure(err) => Option.none(),
    };
    return match (value) {
        Present(row) => row.value,
        Empty => 0,
    };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_string_keyed_tagged_container_updates_survive_runtime() {
    let temp_root = make_temp_project_root("unicode-tagged-container-update-runtime");
    let source_path = temp_root.join("unicode_tagged_container_update_runtime.arden");
    let output_path = temp_root.join("unicode_tagged_container_update_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Integer> = Map<Result<Option<Integer>, String>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), i);
                    i = i + 1;
                }
                store.set(Result.error("σφάλμα🚀"), 100);
                store.set(Result.error("σφάλμα🚀"), 110);
                present: Boolean = store.contains(Result.error("σφάλμα🚀"));
                value: Integer = store.get(Result.error("σφάλμα🚀"));
                return if (present && value == 110) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode tagged container update runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled unicode tagged container update binary");
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
fn string_keyed_tagged_set_growth_remove_contains_survives_runtime() {
    let temp_root = make_temp_project_root("string-keyed-tagged-set-runtime");
    let source_path = temp_root.join("string_keyed_tagged_set_runtime.arden");
    let output_path = temp_root.join("string_keyed_tagged_set_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("missing"));
                seen.add(Result.error("missing"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                return if (removed && !seen.contains(Result.ok(Option.some(4))) && seen.contains(Result.error("missing")) && seen.length() == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string-keyed tagged set runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string-keyed tagged set binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn mixed_unicode_ascii_string_error_keys_survive_map_runtime() {
    let temp_root = make_temp_project_root("mixed-unicode-ascii-string-error-map-runtime");
    let source_path = temp_root.join("mixed_unicode_ascii_string_error_map_runtime.arden");
    let output_path = temp_root.join("mixed_unicode_ascii_string_error_map_runtime");
    let source = r#"
            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Integer> = Map<Result<Option<Integer>, String>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), i);
                    i = i + 1;
                }
                store.set(Result.error("missing"), 40);
                store.set(Result.error("σφάλμα🚀"), 50);
                store.set(Result.error("missing"), 41);
                return if (
                    store.contains(Result.error("missing"))
                    && store.contains(Result.error("σφάλμα🚀"))
                    && store.get(Result.error("missing")) == 41
                    && store.get(Result.error("σφάλμα🚀")) == 50
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed unicode/ascii string error map runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled mixed unicode/ascii string error map binary");
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
fn mixed_ascii_unicode_string_error_object_values_survive_runtime() {
    let temp_root = make_temp_project_root("mixed-ascii-unicode-string-error-object-runtime");
    let source_path = temp_root.join("mixed_ascii_unicode_string_error_object_runtime.arden");
    let output_path = temp_root.join("mixed_ascii_unicode_string_error_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(100)));
                store.set(Result.error("σφάλμα🚀"), Option.some(Boxed(200)));
                store.set(Result.error("missing"), Option.some(Boxed(110)));
                first: Option<Boxed> = store.get(Result.error("missing"));
                second: Option<Boxed> = store.get(Result.error("σφάλμα🚀"));
                return if (first.unwrap().inc().value == 111 && second.unwrap().inc().value == 201) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed ascii/unicode string error object runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled mixed ascii/unicode string error object binary");
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
fn alias_patterns_with_explicit_tagged_constructors_survive_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(5)));
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(95)));
    }
    return store;
}

function main(): Integer {
    fetched: Result<Option<Integer>, String> = Result.error("missing");
    value: Option<Boxed> = match (build(true).get(fetched)) {
        Present(row) => Option.some(Boxed(row.value + 1)),
        Empty => Option.some(Boxed(0)),
    };
    status: Integer = match (fetched) {
        Success(inner) => match (inner) {
            Present(code) => code,
            Empty => 204,
        },
        Failure(err) => 500,
    };
    return value.unwrap().value + status;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn nested_tagged_updates_match_values_and_chains_survive_runtime() {
    let temp_root = make_temp_project_root("nested-tagged-updates-match-values-runtime");
    let source_path = temp_root.join("nested_tagged_updates_match_values_runtime.arden");
    let output_path = temp_root.join("nested_tagged_updates_match_values_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(120)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(130)));
                }
                return store;
            }

            function main(): Integer {
                chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                fallback: Option<Boxed> = build(false).get(Result.error("missing"));
                return if (chosen.unwrap().value == 131 && fallback.unwrap().value == 120) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested tagged updates + match values runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled nested tagged updates + match values binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
