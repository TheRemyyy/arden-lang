use super::*;
use crate::typeck::TypeChecker;
use std::fs;

#[test]
fn repeated_update_tagged_pipeline_match_get_equality_and_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-pipeline-runtime");
    let source_path = temp_root.join("repeated_update_tagged_pipeline_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_pipeline_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(160)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(170)));
                }
                return store;
            }

            function main(): Integer {
                latest: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                earlier: Option<Boxed> = build(false).get(Result.error("missing"));
                latest_key_present: Boolean = build(true).contains(Result.error("missing"));
                same_value: Boolean = build(true).get(Result.error("missing")).unwrap().value == 170;
                return if (latest_key_present && same_value && latest.unwrap().value == 171 && earlier.unwrap().value == 160) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update tagged pipeline runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update tagged pipeline binary");
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
fn repeated_update_tagged_match_value_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-match-value-runtime");
    let source_path = temp_root.join("repeated_update_tagged_match_value_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_match_value_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(180)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(190)));
                }
                return store;
            }

            function choose(flag: Boolean): Option<Boxed> {
                picked: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return picked;
            }

            function main(): Integer {
                latest: Option<Boxed> = choose(true);
                earlier: Option<Boxed> = choose(false);
                return if (latest.unwrap().value == 191 && earlier.unwrap().value == 181) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update tagged match value runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update tagged match value binary");
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
fn repeated_update_tagged_valid_invalid_pair_reports_single_primary_error() {
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
    picked: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return picked.unwrap().value;
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
        .must_err("invalid repeated-update tagged source should fail");
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
fn repeated_update_match_receiver_equality_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-match-receiver-equality-runtime");
    let source_path = temp_root.join("repeated_update_match_receiver_equality_runtime.arden");
    let output_path = temp_root.join("repeated_update_match_receiver_equality_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(200)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(210)));
                }
                return store;
            }

            function pick(flag: Boolean): Boxed {
                chosen: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return chosen.unwrap();
            }

            function main(): Integer {
                latest: Boxed = pick(true);
                earlier: Boxed = pick(false);
                same: Boolean = build(true).get(Result.error("missing")).unwrap().value == 210;
                return if (same && latest.value == 211 && earlier.value == 201) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated-update match receiver equality runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated-update match receiver equality binary");
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
fn repeated_update_boolean_join_match_receiver_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-boolean-join-runtime");
    let source_path = temp_root.join("repeated_update_boolean_join_runtime.arden");
    let output_path = temp_root.join("repeated_update_boolean_join_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(220)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(230)));
                }
                return store;
            }

            function select(flag: Boolean): Integer {
                chosen: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                current_ok: Boolean = build(flag).contains(Result.error("missing"));
                current_val_ok: Boolean = build(flag).get(Result.error("missing")).unwrap().value == if (flag) { 230 } else { 220 };
                return if (current_ok && current_val_ok && chosen.unwrap().value == if (flag) { 231 } else { 221 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (select(true) == 1 && select(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update boolean join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update boolean join binary");
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
fn repeated_update_tagged_set_boolean_join_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-set-boolean-join-runtime");
    let source_path = temp_root.join("repeated_update_tagged_set_boolean_join_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_set_boolean_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("missing"));
                if (flag) {
                    seen.remove(Result.ok(Option.some(4)));
                }
                return seen;
            }

            function select(flag: Boolean): Boxed {
                return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
                    Boxed(240).inc()
                } else {
                    Boxed(0)
                };
            }

            function main(): Integer {
                latest: Boxed = select(true);
                earlier: Boxed = select(false);
                return if (latest.value == 241 && earlier.value == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated-update tagged set boolean join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated-update tagged set boolean join binary");
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
fn tagged_set_valid_invalid_boolean_join_pair_reports_single_primary_error() {
    let valid = r#"
function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
    seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
    mut i: Integer = 0;
    while (i < 9) {
        seen.add(Result.ok(Option.some(i)));
        i = i + 1;
    }
    seen.add(Result.error("missing"));
    if (flag) {
        seen.remove(Result.ok(Option.some(4)));
    }
    return seen;
}

function run(flag: Boolean): Integer {
    return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
        1
    } else {
        0
    };
}

function main(): Integer {
    return if (run(true) == 1 && run(false) == 0) { 0 } else { 1 };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
    seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
    mut i: Integer = 0;
    while (i < 9) {
        seen.add(Result.ok(Option.some(i)));
        i = i + 1;
    }
    seen.add(Result.error("missing"));
    if (flag) {
        seen.remove(Result.ok(Option.some(4)));
    }
    return seen;
}

function run(flag: Boolean): Integer {
    return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
        1
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
        .must_err("invalid tagged set pair should fail");
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
fn combined_map_set_tagged_pipeline_survives_runtime() {
    let temp_root = make_temp_project_root("combined-map-set-tagged-runtime");
    let source_path = temp_root.join("combined_map_set_tagged_runtime.arden");
    let output_path = temp_root.join("combined_map_set_tagged_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(250)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(260)));
                }
                return store;
            }

            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                seen.add(Result.ok(Option.some(1)));
                picked: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return if (
                    seen.contains(Result.error("missing"))
                    && seen.contains(Result.ok(Option.some(1)))
                    && picked.unwrap().value == 261
                    && build(false).get(Result.error("missing")).unwrap().value == 250
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("combined map/set tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled combined map/set tagged runtime binary");
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
fn combined_map_set_repeated_update_equality_chain_survives_runtime() {
    let temp_root = make_temp_project_root("combined-map-set-repeated-update-equality-runtime");
    let source_path = temp_root.join("combined_map_set_repeated_update_equality_runtime.arden");
    let output_path = temp_root.join("combined_map_set_repeated_update_equality_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(300)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(310)));
                }
                return store;
            }

            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                seen.add(Result.ok(Option.some(1)));
                chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                same_latest: Boolean = build(true).get(Result.error("missing")).unwrap().value == 310;
                same_earlier: Boolean = build(false).get(Result.error("missing")).unwrap().value == 300;
                return if (
                    seen.contains(Result.error("missing"))
                    && seen.contains(Result.ok(Option.some(1)))
                    && same_latest
                    && same_earlier
                    && chosen.unwrap().value == 311
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("combined map/set repeated update equality runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled combined map/set repeated update equality binary");
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
fn tagged_membership_branch_match_value_equality_survives_runtime() {
    let temp_root = make_temp_project_root("tagged-membership-branch-match-value-runtime");
    let source_path = temp_root.join("tagged_membership_branch_match_value_runtime.arden");
    let output_path = temp_root.join("tagged_membership_branch_match_value_runtime");
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
                store.set(Result.error("missing"), Option.some(Boxed(320)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(330)));
                }
                return store;
            }

            function choose(flag: Boolean): Option<Boxed> {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                return if (seen.contains(Result.error("missing"))) {
                    match (build(flag).get(Result.error("missing"))) {
                        Some(row) => Option.some(row.inc()),
                        None => Option.some(Boxed(0)),
                    }
                } else {
                    Option.some(Boxed(1))
                };
            }

            function main(): Integer {
                latest: Option<Boxed> = choose(true);
                earlier: Option<Boxed> = choose(false);
                return if (latest.unwrap().value == 331 && earlier.unwrap().value == 321) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("tagged membership branch match value runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled tagged membership branch match value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
