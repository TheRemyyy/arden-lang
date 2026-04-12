use super::*;
use crate::typeck::TypeChecker;
use std::fs;

#[test]
fn alias_tagged_container_valid_and_invalid_pair_reports_cleanly() {
    let valid = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(value: Result<Option<Integer>, String>): Integer {
    return match (value) {
        Success(inner) => match (inner) {
            Present(code) => code,
            Empty => 204,
        },
        Failure(err) => 500,
    };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(value: Result<Option<Integer>, String>, cond: Boolean): Integer {
    return if (cond) {
        match (value) {
            Success(inner) => match (inner) {
                Present(code) => code,
                Empty => 204,
            },
            Failure(err) => 500,
        }
    } else {
        "oops"
    };
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid paired source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn nested_tagged_match_get_chain_survives_runtime() {
    let temp_root = make_temp_project_root("nested-tagged-match-get-chain-runtime");
    let source_path = temp_root.join("nested_tagged_match_get_chain_runtime.arden");
    let output_path = temp_root.join("nested_tagged_match_get_chain_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(140)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(150)));
                }
                return store;
            }

            function main(): Integer {
                selected: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                fallback: Option<Boxed> = match (build(false).get(Result.error("missing"))) {
                    Some(row) => Option.some(row),
                    None => Option.none(),
                };
                return if (selected.unwrap().value == 151 && fallback.unwrap().value == 140) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested tagged match/get chain runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled nested tagged match/get chain binary");
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
fn paired_nested_tagged_match_get_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function main(): Integer {
    chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap().value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function run(flag: Boolean): Integer {
    return if (flag) {
        match (build(true).get(Result.error("missing"))) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid nested tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn multi_function_tagged_pipeline_returned_values_and_chains_survive_runtime() {
    let temp_root = make_temp_project_root("multi-function-tagged-pipeline-runtime");
    let source_path = temp_root.join("multi_function_tagged_pipeline_runtime.arden");
    let output_path = temp_root.join("multi_function_tagged_pipeline_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(340)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(350)));
                }
                return store;
            }

            function fetch(flag: Boolean): Option<Boxed> {
                return build(flag).get(Result.error("missing"));
            }

            function elevate(flag: Boolean): Boxed {
                chosen: Option<Boxed> = match (fetch(flag)) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return chosen.unwrap();
            }

            function main(): Integer {
                latest: Boxed = elevate(true);
                earlier: Boxed = elevate(false);
                return if (latest.value == 351 && earlier.value == 341) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("multi-function tagged pipeline runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled multi-function tagged pipeline binary");
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
fn multi_function_tagged_pipeline_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function elevate(flag: Boolean): Boxed {
    chosen: Option<Boxed> = match (fetch(flag)) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap();
}

function main(): Integer {
    return elevate(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    return if (flag) {
        match (fetch(flag)) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-function tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn multi_helper_tagged_runtime_pipeline_survives_codegen() {
    let temp_root = make_temp_project_root("multi-helper-tagged-runtime");
    let source_path = temp_root.join("multi_helper_tagged_runtime.arden");
    let output_path = temp_root.join("multi_helper_tagged_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(400)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(410)));
                }
                return store;
            }

            function fetch(flag: Boolean): Option<Boxed> {
                return build(flag).get(Result.error("missing"));
            }

            function elevate(flag: Boolean): Boxed {
                return match (fetch(flag)) {
                    Some(row) => row.inc(),
                    None => Boxed(0),
                };
            }

            function score(flag: Boolean): Integer {
                current: Boxed = elevate(flag);
                return if (build(flag).contains(Result.error("missing")) && current.value == if (flag) { 411 } else { 401 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("multi-helper tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled multi-helper tagged runtime binary");
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
fn multi_helper_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function elevate(flag: Boolean): Boxed {
    chosen: Option<Boxed> = match (fetch(flag)) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap();
}

function main(): Integer {
    return elevate(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    return if (fetch(flag).is_some()) {
        match (fetch(flag)) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-nested-tagged-runtime");
    let source_path = temp_root.join("fresh_nested_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(500))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(510))));
                }
                return store;
            }

            function lift(flag: Boolean): Boxed {
                chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                payload: Option<Boxed> = match (chosen) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return payload.unwrap().inc();
            }

            function main(): Integer {
                latest: Boxed = lift(true);
                earlier: Boxed = lift(false);
                return if (latest.value == 511 && earlier.value == 501) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh nested tagged runtime binary");
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
fn nested_tagged_value_flow_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lift(flag: Boolean): Boxed {
    chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    payload: Option<Boxed> = match (chosen) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return payload.unwrap();
}

function main(): Integer {
    return lift(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function run(flag: Boolean): Integer {
    chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    payload: Option<Boxed> = match (chosen) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        payload.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid nested tagged value-flow source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn deeper_multi_helper_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("deeper-multi-helper-nested-tagged-runtime");
    let source_path = temp_root.join("deeper_multi_helper_nested_tagged_runtime.arden");
    let output_path = temp_root.join("deeper_multi_helper_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(600))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(610))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function lift(flag: Boolean): Option<Boxed> {
                selected: Result<Option<Boxed>, String> = fetch(flag);
                return match (selected) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
            }

            function score(flag: Boolean): Integer {
                latest: Option<Boxed> = lift(flag);
                current: Boxed = latest.unwrap().inc();
                return if (current.value == if (flag) { 611 } else { 601 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("deeper multi-helper nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled deeper multi-helper nested tagged runtime binary");
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
fn deeper_multi_helper_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function lift(flag: Boolean): Option<Boxed> {
    selected: Result<Option<Boxed>, String> = fetch(flag);
    return match (selected) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
}

function main(): Integer {
    return lift(true).unwrap().value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function lift(flag: Boolean): Option<Boxed> {
    selected: Result<Option<Boxed>, String> = fetch(flag);
    return match (selected) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
}

function run(flag: Boolean): Integer {
    return if (flag) {
        lift(flag).unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid deeper multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_multi_stage_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-multi-stage-tagged-runtime");
    let source_path = temp_root.join("fresh_multi_stage_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_multi_stage_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(700))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(710))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function project(flag: Boolean): Boxed {
                staged: Option<Boxed> = match (fetch(flag)) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return staged.unwrap().inc();
            }

            function main(): Integer {
                latest: Boxed = project(true);
                earlier: Boxed = project(false);
                current: Result<Option<Boxed>, String> = build(true).get(Result.error("missing"));
                same: Boolean = match (current) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value == 710,
                        None => false,
                    },
                    Error(err) => false,
                };
                return if (same && latest.value == 711 && earlier.value == 701) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh multi-stage tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh multi-stage tagged runtime binary");
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
fn scalar_observation_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("scalar-observation-nested-tagged-runtime");
    let source_path = temp_root.join("scalar_observation_nested_tagged_runtime.arden");
    let output_path = temp_root.join("scalar_observation_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(800))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(810))));
                }
                return store;
            }

            function fetch_value(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().inc().value;
            }

            function main(): Integer {
                latest: Integer = fetch_value(true);
                earlier: Integer = fetch_value(false);
                still_present: Boolean = build(true).contains(Result.error("missing"));
                return if (still_present && latest == 811 && earlier == 801) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("scalar observation nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled scalar observation nested tagged runtime binary");
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
fn scalar_only_nested_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function run(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalar-only tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_scalar_join_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-scalar-join-tagged-runtime");
    let source_path = temp_root.join("fresh_scalar_join_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_scalar_join_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(900))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(910))));
                }
                return store;
            }

            function scalar(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                value: Integer = boxed.unwrap().value;
                present: Boolean = build(flag).contains(Result.error("missing"));
                return if (present && value == if (flag) { 910 } else { 900 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (scalar(true) == 1 && scalar(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh scalar join tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh scalar join tagged runtime binary");
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
fn scalar_only_multi_helper_tagged_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return scalar(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalar-only multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn three_helper_scalar_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("three-helper-scalar-tagged-runtime");
    let source_path = temp_root.join("three_helper_scalar_tagged_runtime.arden");
    let output_path = temp_root.join("three_helper_scalar_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1000))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1010))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function extract(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = fetch(flag);
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function score(flag: Boolean): Integer {
                value: Integer = extract(flag);
                return if (value == if (flag) { 1010 } else { 1000 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("three-helper scalar tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled three-helper scalar tagged runtime binary");
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
fn three_helper_scalar_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return extract(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid three-helper scalar tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_three_helper_repeated_update_scalar_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-three-helper-repeated-update-scalar-runtime");
    let source_path = temp_root.join("fresh_three_helper_repeated_update_scalar_runtime.arden");
    let output_path = temp_root.join("fresh_three_helper_repeated_update_scalar_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1100))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1110))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function extract(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = fetch(flag);
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function score(flag: Boolean): Integer {
                latest: Integer = extract(flag);
                present: Boolean = build(flag).contains(Result.error("missing"));
                return if (present && latest == if (flag) { 1110 } else { 1100 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh three-helper repeated-update scalar runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh three-helper repeated-update scalar runtime binary");
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
fn three_helper_repeated_update_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return extract(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return extract(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid three-helper repeated-update scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn independent_lookup_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("independent-lookup-nested-tagged-runtime");
    let source_path = temp_root.join("independent_lookup_nested_tagged_runtime.arden");
    let output_path = temp_root.join("independent_lookup_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1200))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1300))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1210))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                a: Integer = lookup(true, "missing");
                b: Integer = lookup(false, "missing");
                c: Integer = lookup(true, "other");
                d: Boolean = build(true).contains(Result.error("missing"));
                e: Boolean = build(true).contains(Result.error("other"));
                return if (a == 1210 && b == 1200 && c == 1300 && d && e) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("independent lookup nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled independent lookup nested tagged runtime binary");
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
fn parameterized_lookup_tagged_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized lookup tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn parameterized_scalar_lookup_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("parameterized-scalar-lookup-runtime");
    let source_path = temp_root.join("parameterized_scalar_lookup_runtime.arden");
    let output_path = temp_root.join("parameterized_scalar_lookup_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1400))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1500))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1410))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                a: Integer = lookup(true, "missing");
                b: Integer = lookup(false, "missing");
                c: Integer = lookup(true, "other");
                d: Boolean = build(true).contains(Result.error("missing"));
                e: Boolean = build(true).contains(Result.error("other"));
                return if (a == 1410 && b == 1400 && c == 1500 && d && e) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("parameterized scalar lookup runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled parameterized scalar lookup binary");
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
fn parameterized_multi_key_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized multi-key tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn parameterized_multi_key_scalar_join_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("parameterized-multi-key-scalar-join-runtime");
    let source_path = temp_root.join("parameterized_multi_key_scalar_join_runtime.arden");
    let output_path = temp_root.join("parameterized_multi_key_scalar_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1600))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1700))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1610))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                latest: Integer = lookup(true, "missing");
                earlier: Integer = lookup(false, "missing");
                other: Integer = lookup(true, "other");
                has_missing: Boolean = build(true).contains(Result.error("missing"));
                has_other: Boolean = build(true).contains(Result.error("other"));
                return if (latest == 1610 && earlier == 1600 && other == 1700 && has_missing && has_other) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("parameterized multi-key scalar join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled parameterized multi-key scalar join binary");
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
fn parameterized_multi_key_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized multi-key scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_multi_key_repeated_update_scalar_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-multi-key-repeated-update-scalar-runtime");
    let source_path = temp_root.join("fresh_multi_key_repeated_update_scalar_runtime.arden");
    let output_path = temp_root.join("fresh_multi_key_repeated_update_scalar_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1800))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1900))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1810))));
                }
                return store;
            }

            function fetch_value(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                latest: Integer = fetch_value(true, "missing");
                earlier: Integer = fetch_value(false, "missing");
                other: Integer = fetch_value(true, "other");
                present_missing: Boolean = build(true).contains(Result.error("missing"));
                present_other: Boolean = build(true).contains(Result.error("other"));
                return if (latest == 1810 && earlier == 1800 && other == 1900 && present_missing && present_other) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh multi-key repeated-update scalar runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh multi-key repeated-update scalar runtime binary");
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
fn fresh_multi_key_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return fetch_value(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid fresh multi-key scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn new_multi_key_nested_join_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("new-multi-key-nested-join-runtime");
    let source_path = temp_root.join("new_multi_key_nested_join_runtime.arden");
    let output_path = temp_root.join("new_multi_key_nested_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2000))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3000))));
                if (flag) {
                    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2010))));
                }
                return store;
            }

            function value_for(flag: Boolean, key: String): Integer {
                entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                payload: Option<Boxed> = match (entry) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return payload.unwrap().value;
            }

            function joined(flag: Boolean): Integer {
                left: Integer = value_for(flag, "alpha");
                right: Integer = value_for(true, "beta");
                left_present: Boolean = build(flag).contains(Result.error("alpha"));
                right_present: Boolean = build(true).contains(Result.error("beta"));
                return if (left_present && right_present && left == if (flag) { 2010 } else { 2000 } && right == 3000) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (joined(true) == 1 && joined(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("new multi-key nested join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled new multi-key nested join binary");
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
fn multi_key_joined_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function value_for(flag: Boolean, key: String): Integer {
    entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    payload: Option<Boxed> = match (entry) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return payload.unwrap().value;
}

function main(): Integer {
    return value_for(true, "alpha");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function value_for(flag: Boolean, key: String): Integer {
    entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    payload: Option<Boxed> = match (entry) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        payload.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return value_for(true, "alpha");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-key joined tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn static_constructor_comparison_multi_key_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("static-constructor-comparison-multi-key-runtime");
    let source_path = temp_root.join("static_constructor_comparison_multi_key_runtime.arden");
    let output_path = temp_root.join("static_constructor_comparison_multi_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
                if (flag) {
                    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
                }
                return store;
            }

            function key_matches(flag: Boolean, key: String, expected: Integer): Boolean {
                entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                scalar_tag: Result<Option<Integer>, String> = match (entry) {
                    Ok(inner) => match (inner) {
                        Some(row) => Result.ok(Option.some(row.value)),
                        None => Result.ok(Option.none()),
                    },
                    Error(err) => Result.error(err),
                };
                same_tag: Boolean = scalar_tag == Result.ok(Option.some(expected));
                payload: Option<Boxed> = match (entry) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return same_tag && payload.unwrap().value == expected;
            }

            function main(): Integer {
                return if (key_matches(true, "alpha", 2) && key_matches(false, "alpha", 1) && key_matches(true, "beta", 3)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("static constructor comparison multi-key runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled static constructor comparison multi-key binary");
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
fn fresh_scalar_only_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-scalar-only-tagged-runtime");
    let source_path = temp_root.join("fresh_scalar_only_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_scalar_only_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2100))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(2200))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2110))));
                }
                return store;
            }

            function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                return match (current) {
                    Ok(inner) => match (inner) {
                        Some(row) => Result.ok(Option.some(row.value)),
                        None => Result.ok(Option.none()),
                    },
                    Error(err) => Result.error(err),
                };
            }

            function main(): Integer {
                a: Result<Option<Integer>, String> = scalar(true, "missing");
                b: Result<Option<Integer>, String> = scalar(false, "missing");
                c: Result<Option<Integer>, String> = scalar(true, "other");
                d: Result<Option<Integer>, String> = Result.error("missing");
                return if (
                    a == Result.ok(Option.some(2110))
                    && b == Result.ok(Option.some(2100))
                    && c == Result.ok(Option.some(2200))
                    && d == Result.error("missing")
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh scalar-only tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh scalar-only tagged runtime binary");
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
fn scalarized_nested_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    return match (current) {
        Ok(inner) => match (inner) {
            Some(row) => Result.ok(Option.some(row.value)),
            None => Result.ok(Option.none()),
        },
        Error(err) => Result.error(err),
    };
}

function main(): Integer {
    return if (scalar(true, "missing") == Result.ok(Option.some(2))) { 0 } else { 1 };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    return if (flag) {
        match (current) {
            Ok(inner) => match (inner) {
                Some(row) => Result.ok(Option.some(row.value)),
                None => Result.ok(Option.none()),
            },
            Error(err) => Result.error(err),
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return if (scalar(true, "missing") == Result.ok(Option.some(2))) { 0 } else { 1 };
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalarized nested tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}
