use super::*;
use crate::formatter::format_program_canonical;
use crate::lexer::tokenize;

fn parse_source(source: &str) -> Result<Program, ParseError> {
    let tokens = tokenize(source).map_err(|e| ParseError::new(e, 0..0))?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

#[test]
fn test_parse_test_attribute() {
    let source = r#"
        package test;
        
        @Test
        function testAddition(): Integer {
            return 2 + 2;
        }
    "#;

    let program = parse_source(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 1);

    match &program.declarations[0].node {
        Decl::Function(func) => {
            assert_eq!(func.name, "testAddition");
            assert_eq!(func.attributes.len(), 1);
            assert_eq!(func.attributes[0], Attribute::Test);
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_parse_ignore_attribute_with_reason() {
    let source = r#"
        package test;
        
        @Test
        @Ignore("Not implemented yet")
        function testDivision(): Integer {
            return 10 / 2;
        }
    "#;

    let program = parse_source(source).expect("Should parse successfully");

    match &program.declarations[0].node {
        Decl::Function(func) => {
            assert_eq!(func.attributes.len(), 2);
            assert_eq!(func.attributes[0], Attribute::Test);
            assert_eq!(
                func.attributes[1],
                Attribute::Ignore(Some("Not implemented yet".to_string()))
            );
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn parses_builtin_option_none_static_constructor_call() {
    let source = r#"
        function main(): Option<Integer> {
            return Option.None();
        }
    "#;

    parse_source(source).expect("Option.None() should parse");
}

#[test]
fn parses_root_namespace_alias_builtin_option_none_static_constructor_call() {
    let source = r#"
        package app;
        import app as root;

        function main(): Option<Integer> {
            return root.Option.None();
        }
    "#;

    parse_source(source).expect("root.Option.None() should parse");
}

#[test]
fn parses_builtin_option_none_variant_pattern() {
    let source = r#"
        function main(): Integer {
            return match (Option.None()) {
                Option.None => 0,
                Option.Some(_) => 1,
            };
        }
    "#;

    parse_source(source).expect("Option.None pattern should parse");
}

#[test]
fn parses_root_namespace_alias_builtin_option_none_variant_pattern() {
    let source = r#"
        package app;
        import app as root;

        function main(): Integer {
            return match (root.Option.None()) {
                root.Option.None => 0,
                root.Option.Some(_) => 1,
            };
        }
    "#;

    parse_source(source).expect("root.Option.None pattern should parse");
}

#[test]
fn test_parse_function_without_attributes() {
    let source = r#"
        package test;
        
        function normalFunction(): Integer {
            return 42;
        }
    "#;

    let program = parse_source(source).expect("Should parse successfully");

    match &program.declarations[0].node {
        Decl::Function(func) => {
            assert_eq!(func.name, "normalFunction");
            assert!(func.attributes.is_empty());
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_parse_public_top_level_function() {
    let source = r#"
        public function exported(): Integer {
            return 1;
        }
    "#;

    let program = parse_source(source).expect("Should parse public top-level function");
    match &program.declarations[0].node {
        Decl::Function(func) => {
            assert_eq!(func.name, "exported");
            assert_eq!(func.visibility, Visibility::Public);
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_unknown_attribute_error() {
    let source = r#"
        package test;
        
        @Unknown
        function testFunc(): Integer {
            return 42;
        }
    "#;

    let result = parse_source(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Unknown attribute"));
}

#[test]
fn test_parse_import_with_alias() {
    let source = r#"
        import std.math as math;
        function main(): Integer {
            return 0;
        }
    "#;
    let program = parse_source(source).expect("Should parse import alias");
    match &program.declarations[0].node {
        Decl::Import(import) => {
            assert_eq!(import.path, "std.math");
            assert_eq!(import.alias.as_deref(), Some("math"));
        }
        _ => panic!("Expected import declaration"),
    }
}

#[test]
fn test_reject_import_wildcard_with_alias() {
    let source = r#"
        import std.io.* as io;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("wildcard alias import should fail");
    assert!(
        err.message
            .contains("Cannot use alias with wildcard import"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_import_alias_without_identifier() {
    let source = r#"
        import std.math as ;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("import alias without identifier should fail");
    assert!(
        err.message.contains("Expected identifier"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_import_with_empty_path_segment() {
    let source = r#"
        import std..math;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("empty import path segment should fail");
    assert!(
        err.message
            .contains("Import path cannot contain an empty segment"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_first_error_for_keyword_alias_plus_generic_tail() {
    let source = r#"
        import app.Option.None as ;
        function main(): None {
            value.map<Integer,>(x => x);
            return None;
        }
    "#;
    let err =
        parse_source(source).expect_err("parser should stop at malformed keyword alias import");
    assert!(
        err.message.contains("Expected identifier"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_import_with_leading_dot() {
    let source = r#"
        import .std.math;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("leading-dot import should fail");
    assert!(
        err.message.contains("Import path cannot start with '.'"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_package_with_empty_path_segment() {
    let source = r#"
        package app..core;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("empty package path segment should fail");
    assert!(
        err.message
            .contains("Package path cannot contain an empty segment"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_package_with_leading_dot() {
    let source = r#"
        package .app.core;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("leading-dot package should fail");
    assert!(
        err.message.contains("Package path cannot start with '.'"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_package_with_trailing_dot() {
    let source = r#"
        package app.;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("package trailing dot should fail");
    assert!(
        err.message.contains("Package path cannot end with '.'"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_compound_assign_ident() {
    let source = r#"
        function main(): None {
            x: Integer = 1;
            x += 2;
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse compound assignment");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };

    let Stmt::Assign { target, value } = &func.body[1].node else {
        panic!("Expected assign statement");
    };
    let Expr::Ident(target_name) = &target.node else {
        panic!("Expected assign target ident");
    };
    assert_eq!(target_name, "x");

    let Expr::Binary { op, left, right } = &value.node else {
        panic!("Expected desugared binary expression");
    };
    assert_eq!(*op, BinOp::Add);
    let Expr::Ident(left_name) = &left.node else {
        panic!("Expected left ident");
    };
    let Expr::Literal(Literal::Integer(rhs)) = right.node else {
        panic!("Expected integer rhs");
    };
    assert_eq!(left_name, "x");
    assert_eq!(rhs, 2);
}

#[test]
fn test_parse_compound_assign_index_target() {
    let source = r#"
        function main(): None {
            items: List<Integer> = range(0, 3);
            items[0] -= 1;
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse index compound assignment");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };

    let Stmt::Assign { target, value } = &func.body[1].node else {
        panic!("Expected assign statement");
    };
    let Expr::Index { object, index } = &target.node else {
        panic!("Expected index target");
    };
    let Expr::Ident(obj_name) = &object.node else {
        panic!("Expected indexed object ident");
    };
    let Expr::Literal(Literal::Integer(idx)) = index.node else {
        panic!("Expected integer index");
    };
    assert_eq!(obj_name, "items");
    assert_eq!(idx, 0);

    let Expr::Binary { op, left, right } = &value.node else {
        panic!("Expected desugared binary expression");
    };
    assert_eq!(*op, BinOp::Sub);
    let Expr::Index { .. } = &left.node else {
        panic!("Expected cloned index expression on lhs");
    };
    let Expr::Literal(Literal::Integer(rhs)) = right.node else {
        panic!("Expected integer rhs");
    };
    assert_eq!(rhs, 1);
}

#[test]
fn test_parse_compound_assign_mod_target() {
    let source = r#"
        function main(): None {
            x: Integer = 7;
            x %= 3;
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse modulo compound assignment");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };

    let Stmt::Assign { target, value } = &func.body[1].node else {
        panic!("Expected assign statement");
    };
    let Expr::Ident(target_name) = &target.node else {
        panic!("Expected assign target ident");
    };
    assert_eq!(target_name, "x");

    let Expr::Binary { op, left, right } = &value.node else {
        panic!("Expected desugared binary expression");
    };
    assert_eq!(*op, BinOp::Mod);
    let Expr::Ident(left_name) = &left.node else {
        panic!("Expected left ident");
    };
    let Expr::Literal(Literal::Integer(rhs)) = right.node else {
        panic!("Expected integer rhs");
    };
    assert_eq!(left_name, "x");
    assert_eq!(rhs, 3);
}

#[test]
fn parser_reports_nested_match_error_before_outer_value_flow_noise() {
    let source = r#"
        function main(): None {
            value: Integer = match (1) {
                1 => match (2) {
                    2 => 3,
                value.map<Integer,>(x => x)
            };
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("nested malformed match should fail");
    assert!(
        err.message.contains("Expected RBrace")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_nested_match_generic_tail_before_fatarrow_noise() {
    let source = r#"
        function main(): None {
            value: Integer = match (1) {
                1 => match (2) {
                    2 => 3,
                }.map<Integer,>(x => x)
            };
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("malformed nested generic tail should fail");
    assert!(
        err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err
                .message
                .contains("Trailing comma is not allowed in generic call type arguments"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_pipe_lambda_syntax() {
    let source = r#"
        function main(): None {
            f: (Integer) -> Integer = |x: Integer| x + 1;
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("pipe lambda syntax should fail");
    assert!(
        err.message.contains("Expected expression") || err.message.contains("Unexpected token"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_zero_arg_pipe_lambda_syntax() {
    let source = r#"
        function main(): None {
            f: () -> Integer = || 42;
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("zero-arg pipe lambda syntax should fail");
    assert!(
        err.message.contains("Expected expression") || err.message.contains("Unexpected token"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_rest_style_alias_nested_match_noise_with_single_primary_error() {
    let source = r#"
        import app.Option.Some as Present;
        import app.Option.None as Empty;
        import app.Result.Ok as Success;
        import app.Result.Error as Failure;

        class Request {
            route: String;
        }

        function handle(req: Request, verbose: Boolean): Integer {
            return if (verbose) {
                match (decode(req)) {
                    Success(inner) => match (inner) {
                        Present(code) => code,
                        Empty => 204,
                    }.map<Integer,>(x => x)
                    Failure(err) => 500,
                }
            } else {
                400
            };
        }
    "#;
    let err =
        parse_source(source).expect_err("rest-style malformed nested generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_batch_style_tagged_map_noise_without_fatarrow_cascade() {
    let source = r#"
        class Row {
            value: Integer;
        }

        function run(flag: Boolean): Integer {
            queue: Map<Result<Option<Integer>, Integer>, Option<Row>> = Map<Result<Option<Integer>, Integer>, Option<Row>>();
            return if (flag) {
                match (queue.contains(Result.error(3))) {
                    true => queue.get(Result.error(3)).unwrap().bump<Integer,>(x => x).value
                    false => 0,
                }
            } else {
                1
            };
        }
    "#;
    let err =
        parse_source(source).expect_err("batch-style malformed generic method tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_unicode_tagged_pipeline_noise_without_fatarrow_cascade() {
    let source = r#"
        import app.Option.Some as Present;
        import app.Option.None as Empty;
        import app.Result.Ok as Success;
        import app.Result.Error as Failure;

        class Boxed {
            value: Integer;
        }

        function main(): Integer {
            return if (true) {
                match (build().contains(Result.error("σφάλμα🚀"))) {
                    true => build().get(Result.error("σφάλμα🚀")).unwrap().inc<Integer,>(x => x).value
                    false => 0,
                }
            } else {
                1
            };
        }
    "#;
    let err = parse_source(source).expect_err("unicode malformed generic method tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_repeated_update_tagged_pipeline_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
            store.set(Result.error("missing"), Option.some(Boxed(160)));
            if (flag) {
                store.set(Result.error("missing"), Option.some(Boxed(170)));
            }
            return store;
        }

        function main(): Integer {
            return if (true) {
                match (build(true).get(Result.error("missing"))) {
                    Some(row) => row.inc().map<Integer,>(x => x).value
                    None => 0,
                }
            } else {
                1
            };
        }
    "#;
    let err = parse_source(source)
        .expect_err("repeated-update malformed generic method tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_repeated_update_receiver_equality_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
            store.set(Result.error("missing"), Option.some(Boxed(200)));
            if (flag) {
                store.set(Result.error("missing"), Option.some(Boxed(210)));
            }
            return store;
        }

        function main(): Integer {
            return if (true) {
                match (build(true).get(Result.error("missing"))) {
                    Some(row) => row.inc().map<Integer,>(x => x).value == 211,
                    None => false,
                }
            } else {
                false
            };
        }
    "#;
    let err = parse_source(source)
        .expect_err("repeated-update receiver/equality malformed generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_boolean_join_tagged_pipeline_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
            store.set(Result.error("missing"), Option.some(Boxed(220)));
            if (flag) {
                store.set(Result.error("missing"), Option.some(Boxed(230)));
            }
            return store;
        }

        function main(): Integer {
            return if (true) {
                match (build(true).get(Result.error("missing"))) {
                    Some(row) => row.inc().map<Integer,>(x => x).value == 231 && build(true).contains(Result.error("missing")),
                    None => false,
                }
            } else {
                false
            };
        }
    "#;
    let err = parse_source(source).expect_err("boolean-join malformed generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_combined_map_set_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
            store.set(Result.error("missing"), Option.some(Boxed(250)));
            if (flag) {
                store.set(Result.error("missing"), Option.some(Boxed(260)));
            }
            return store;
        }

        function main(): Integer {
            seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
            seen.add(Result.error("missing"));
            return if (true) {
                match (build(true).get(Result.error("missing"))) {
                    Some(row) => row.inc().map<Integer,>(x => x).value == 261 && seen.contains(Result.error("missing")),
                    None => false,
                }
            } else {
                false
            };
        }
    "#;
    let err =
        parse_source(source).expect_err("combined map/set malformed generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_combined_map_set_equality_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
            store.set(Result.error("missing"), Option.some(Boxed(300)));
            if (flag) {
                store.set(Result.error("missing"), Option.some(Boxed(310)));
            }
            return store;
        }

        function main(): Integer {
            seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
            seen.add(Result.error("missing"));
            return if (true) {
                match (build(true).get(Result.error("missing"))) {
                    Some(row) => row.inc().map<Integer,>(x => x).value == 311 && seen.contains(Result.error("missing")),
                    None => false,
                }
            } else {
                false
            };
        }
    "#;
    let err = parse_source(source)
        .expect_err("combined map/set equality malformed generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn parser_reports_membership_branch_tagged_noise_without_fatarrow_cascade() {
    let source = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function inc(): Boxed { return Boxed(this.value + 1); }
        }

        function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
            store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
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
                    Some(row) => row.inc().map<Integer,>(x => x).value
                    None => Boxed(0).value,
                }
            } else {
                1
            };
        }
    "#;
    let err =
        parse_source(source).expect_err("membership-branch malformed generic tail should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments")
            || err.message.contains("Expected pattern")
            || err.message.contains("Expected RBrace")
            || err.message.contains("Expected Semi"),
        "{}",
        err.message
    );
    assert!(
        !err.message.contains("Expected FatArrow"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_visibility_modifier_on_constructor() {
    let source = r#"
        class C {
            private constructor() { }
        }
    "#;
    let err = parse_source(source).expect_err("private constructor modifier should fail");
    assert!(
        err.message
            .contains("Visibility modifiers are not supported on constructors"),
        "{}",
        err.message
    );
}

#[test]
fn test_match_arm_expression_keeps_expr_span() {
    let source = r#"
        function main(): None {
            match (1) {
                1 => foo,
                _ => bar,
            }
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse match statement");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Match { arms, .. } = &func.body[0].node else {
        panic!("Expected match statement");
    };
    let first_stmt = &arms[0].body[0];
    assert_ne!(first_stmt.span.start, 0);
    assert!(first_stmt.span.end > first_stmt.span.start);
}

#[test]
fn test_parse_if_expression() {
    let source = r#"
        function main(): None {
            x: Integer = if (true) { 1; } else { 2; };
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse if-expression initializer");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::IfExpr {
        condition,
        then_branch,
        else_branch,
    } = &value.node
    else {
        panic!("Expected if expression");
    };
    assert!(matches!(
        condition.node,
        Expr::Literal(Literal::Boolean(true))
    ));
    assert_eq!(then_branch.len(), 1);
    assert!(else_branch.as_ref().is_some_and(|b| b.len() == 1));
}

#[test]
fn test_parse_if_expression_without_else() {
    let source = r#"
        function main(): None {
            x: None = if (true) { println("x"); };
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse if-expression without else");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::IfExpr { else_branch, .. } = &value.node else {
        panic!("Expected if expression");
    };
    assert!(else_branch.is_none());
}

#[test]
fn test_parse_if_statement_with_else_if() {
    let source = r#"
        function main(): None {
            if (true) {
                return None;
            } else if (false) {
                return None;
            } else {
                return None;
            }
        }
    "#;
    let program = parse_source(source).expect("else-if statement should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::If { else_block, .. } = &func.body[0].node else {
        panic!("Expected if statement");
    };
    let else_block = else_block
        .as_ref()
        .expect("else-if should build else block");
    assert!(matches!(else_block[0].node, Stmt::If { .. }));
}

#[test]
fn test_parse_if_expression_with_else_if() {
    let source = r#"
        function main(): None {
            x: Integer = if (true) { 1; } else if (false) { 2; } else { 3; };
            return None;
        }
    "#;
    let program = parse_source(source).expect("else-if expression should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::IfExpr { else_branch, .. } = &value.node else {
        panic!("Expected if expression");
    };
    let else_branch = else_branch
        .as_ref()
        .expect("else-if should build else branch");
    assert!(matches!(
        else_branch[0].node,
        Stmt::Expr(Spanned {
            node: Expr::IfExpr { .. },
            ..
        })
    ));
}

#[test]
fn test_parse_if_expression_generic_constructor_branches() {
    let source = r#"
        class Boxed<T> {
            value: T;
        }

        function make(flag: Boolean): Boxed<Integer> {
            return if (flag) { Boxed<Integer>(1); } else { Boxed<Integer>(2); };
        }
    "#;
    let program =
        parse_source(source).expect("generic constructors in if-expression branches should parse");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected make function declaration");
    };
    let Stmt::Return(Some(value)) = &func.body[0].node else {
        panic!("Expected return statement");
    };
    let Expr::IfExpr {
        then_branch,
        else_branch,
        ..
    } = &value.node
    else {
        panic!("Expected if expression");
    };
    assert!(matches!(
        then_branch[0].node,
        Stmt::Expr(Spanned {
            node: Expr::Construct { .. },
            ..
        })
    ));
    let else_branch = else_branch.as_ref().expect("expected else branch");
    assert!(matches!(
        else_branch[0].node,
        Stmt::Expr(Spanned {
            node: Expr::Construct { .. },
            ..
        })
    ));
}

#[test]
fn test_parse_match_statement_with_trailing_semicolon() {
    let source = r#"
        function main(): None {
            match (1) {
                1 => { },
                _ => { },
            };
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse match statement with semicolon");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    assert!(matches!(func.body[0].node, Stmt::Match { .. }));
}

#[test]
fn test_reject_empty_match_statement() {
    let source = r#"
        function main(): None {
            match (1) {
            }
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("empty match statement should fail");
    assert!(
        err.message
            .contains("match statements must contain at least one arm"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_empty_match_expression() {
    let source = r#"
        function main(): None {
            x: Integer = match (1) {
            };
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("empty match expression should fail");
    assert!(
        err.message
            .contains("match expressions must contain at least one arm"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_if_expression_branch_match_statement_with_semicolon() {
    let source = r#"
        function main(): None {
            x: None = if (true) {
                match (1) {
                    1 => { },
                    _ => { },
                };
            } else {
                None;
            };
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse if-expression with match statement");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::IfExpr { then_branch, .. } = &value.node else {
        panic!("Expected if expression");
    };
    assert!(matches!(then_branch[0].node, Stmt::Match { .. }));
}

#[test]
fn test_parse_if_expression_branch_tail_expressions_without_semicolons() {
    let source = r#"
        function pick(flag: Boolean, value: Result<Option<Integer>, String>): Integer {
            return if (flag) {
                match (value) {
                    Ok(inner) => match (inner) {
                        Some(found) => found,
                        None => 0,
                    },
                    Error(err) => 0,
                }
            } else {
                0
            };
        }
    "#;
    parse_source(source)
        .expect("if-expression branches should accept trailing expressions without semicolons");
}

#[test]
fn test_uppercase_function_call_is_not_forced_constructor() {
    let source = r#"
        function Foo(): Integer { return 7; }
        function main(): None {
            x: Integer = Foo();
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse uppercase function call");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    match &value.node {
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            assert!(matches!(callee.node, Expr::Ident(ref n) if n == "Foo"));
            assert!(args.is_empty());
            assert!(type_args.is_empty());
        }
        other => panic!("Expected call expression, found {:?}", other),
    }
}

#[test]
fn test_forward_uppercase_function_call_is_call() {
    let source = r#"
        function main(): None {
            x: Integer = Foo();
            return None;
        }
        function Foo(): Integer { return 7; }
    "#;
    let program = parse_source(source).expect("Should parse forward uppercase function call");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Call { type_args, .. } = &value.node else {
        panic!("Expected call");
    };
    assert_eq!(type_args.len(), 0);
}

#[test]
fn test_parse_explicit_generic_method_call() {
    let source = r#"
        class C {
            function id<T>(x: T): T { return x; }
        }
        function main(): None {
            c: C = C();
            x: Integer = c.id<Integer>(1);
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse explicit generic method call");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[1].node else {
        panic!("Expected let statement");
    };
    let Expr::Call { type_args, .. } = &value.node else {
        panic!("Expected call");
    };
    assert_eq!(type_args.len(), 1);
}

#[test]
fn test_parse_explicit_generic_module_call() {
    let source = r#"
        module M { function id<T>(x: T): T { return x; } }
        function main(): None {
            x: Integer = M.id<Integer>(1);
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse explicit generic module call");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Call { type_args, .. } = &value.node else {
        panic!("Expected call");
    };
    assert_eq!(type_args.len(), 1);
}

#[test]
fn test_parse_explicit_generic_function_call() {
    let source = r#"
        function id<T>(x: T): T { return x; }
        function main(): None {
            x: Integer = id<Integer>(1);
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse explicit generic call");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Call { type_args, .. } = &value.node else {
        panic!("Expected call");
    };
    assert_eq!(type_args.len(), 1);
}

#[test]
fn test_parse_explicit_generic_function_value() {
    let source = r#"
        function id<T>(x: T): T { return x; }
        function main(): None {
            f: (Integer) -> Integer = id<Integer>;
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse explicit generic function value");
    let Decl::Function(func) = &program.declarations[1].node else {
        panic!("Expected main function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::GenericFunctionValue { type_args, .. } = &value.node else {
        panic!("Expected specialized function value");
    };
    assert_eq!(type_args.len(), 1);
}

#[test]
fn test_parse_generic_interface_reference_in_implements_clause() {
    let source = r#"
        interface I<T> {
            function get(): T;
        }

        class C implements I<String> {
            function get(): String { return "ok"; }
        }
    "#;
    let program = parse_source(source).expect("Should parse generic interface implements clause");
    let Decl::Class(class_decl) = &program.declarations[1].node else {
        panic!("Expected class declaration");
    };
    assert_eq!(class_decl.implements, vec!["I<String>".to_string()]);
}

#[test]
fn test_parse_await_expression_then_method_call() {
    let source = r#"
        async function run(): Integer {
            return await(make()).get();
        }
    "#;
    let program = parse_source(source).expect("Should parse await expression method chain");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Return(Some(value)) = &func.body[0].node else {
        panic!("Expected return statement");
    };
    let Expr::Call { callee, .. } = &value.node else {
        panic!("Expected outer method call");
    };
    let Expr::Field { object, field } = &callee.node else {
        panic!("Expected field callee");
    };
    assert_eq!(field, "get");
    let Expr::Await(inner) = &object.node else {
        panic!("Expected await receiver");
    };
    let Expr::Call {
        callee: inner_callee,
        ..
    } = &inner.node
    else {
        panic!("Expected awaited call");
    };
    let Expr::Ident(name) = &inner_callee.node else {
        panic!("Expected make identifier");
    };
    assert_eq!(name, "make");
}

#[test]
fn test_reject_function_type_trailing_comma() {
    let source = r#"
        function takes(f: (Integer,) -> Integer): None {
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("function type trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in function type parameters"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_zero_arg_function_type() {
    let source = r#"
        function takes(f: () -> Integer): None {
            return None;
        }
    "#;
    parse_source(source).expect("zero-arg function type should remain valid");
}

#[test]
fn test_reject_explicit_generic_function_call_with_trailing_comma() {
    let source = r#"
        function id<T>(x: T): T { return x; }
        function main(): None {
            x: Integer = id<Integer,>(1);
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("generic call trailing comma should fail");
    assert!(
        err.message.contains("Trailing comma") || err.message.contains("Expected"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_explicit_generic_method_call_with_trailing_comma() {
    let source = r#"
        function main(): None {
            value: Box<Integer> = Box<Integer>(1);
            value.map<Integer,>(x => x);
            return None;
        }
    "#;
    let err =
        parse_source(source).expect_err("method generic call with trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic call type arguments"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_explicit_generic_module_call_with_trailing_comma() {
    let source = r#"
        module A {
            module B {
                function id<T>(x: T): T { return x; }
            }
        }
        function main(): None {
            x: Integer = A.B.id<Integer,>(1);
            return None;
        }
    "#;
    let err =
        parse_source(source).expect_err("nested module generic call trailing comma should fail");
    assert!(
        err.message.contains("Trailing comma") || err.message.contains("Expected"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_empty_generic_parameter_list() {
    let source = r#"
        function id<>(): Integer {
            return 1;
        }
    "#;
    let err = parse_source(source).expect_err("empty generic parameter list should fail");
    assert!(
        err.message
            .contains("Generic parameter list cannot be empty"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_generic_parameter_list() {
    let source = r#"
        function id<T,>(x: T): T {
            return x;
        }
    "#;
    let err = parse_source(source).expect_err("generic parameter trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in generic parameter lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_qualified_generic_parameter_bound() {
    let source = r#"
        function render<T extends util.Api.Named>(value: T): T {
            return value;
        }
    "#;
    let program = parse_source(source).expect("qualified generic bound should parse");
    match &program.declarations[0].node {
        Decl::Function(func) => {
            assert_eq!(func.generic_params.len(), 1);
            assert_eq!(func.generic_params[0].bounds, vec!["util.Api.Named"]);
        }
        _ => panic!("Expected function declaration"),
    }
}

#[test]
fn test_parse_multiple_qualified_generic_parameter_bounds() {
    let source = r#"
        class Box<T extends util.Api.Named, util.Api.Serializable> {
            value: T;
        }
    "#;
    let program = parse_source(source).expect("multiple qualified generic bounds should parse");
    match &program.declarations[0].node {
        Decl::Class(class) => {
            assert_eq!(class.generic_params.len(), 1);
            assert_eq!(
                class.generic_params[0].bounds,
                vec!["util.Api.Named", "util.Api.Serializable"]
            );
        }
        _ => panic!("Expected class declaration"),
    }
}

#[test]
fn test_reject_trailing_comma_in_parameter_list() {
    let source = r#"
        function add(x: Integer,): Integer {
            return x;
        }
    "#;
    let err = parse_source(source).expect_err("parameter trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in parameter lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_extern_parameter_list() {
    let source = r#"
        extern(c) function puts(msg: String,): Integer;
    "#;
    let err = parse_source(source).expect_err("extern parameter trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in extern parameter lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_argument_list() {
    let source = r#"
        function add(x: Integer): Integer { return x; }
        function main(): None {
            value: Integer = add(1,);
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("argument trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in argument lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_implements_list() {
    let source = r#"
        class C implements A, {
        }
    "#;
    let err = parse_source(source).expect_err("implements trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in implements lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_empty_implements_list() {
    let source = r#"
        class C implements {
        }
    "#;
    let err = parse_source(source).expect_err("empty implements list should fail");
    assert!(
        err.message.contains("implements list cannot be empty"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_empty_class_extends_clause() {
    let source = r#"
        class Child extends {
        }
    "#;
    let err = parse_source(source).expect_err("empty class extends should fail");
    assert!(
        err.message.contains("Class extends clause cannot be empty"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_interface_extends_list() {
    let source = r#"
        interface Child extends Parent, {
            function run(): None;
        }
    "#;
    let err = parse_source(source).expect_err("interface extends trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in interface extends lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_empty_interface_extends_list() {
    let source = r#"
        interface Child extends {
            function run(): None;
        }
    "#;
    let err = parse_source(source).expect_err("empty interface extends list should fail");
    assert!(
        err.message
            .contains("interface extends list cannot be empty"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_visibility_modifier_on_module() {
    let source = r#"
        public module Tools {
        }
    "#;
    let err = parse_source(source).expect_err("module visibility modifier should fail");
    assert!(
        err.message
            .contains("Visibility modifiers are not supported on modules"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_visibility_modifier_on_import() {
    let source = r#"
        public import std.io.*;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("import visibility modifier should fail");
    assert!(
        err.message
            .contains("Visibility modifiers are not supported on imports"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_visibility_modifier_on_package() {
    let source = r#"
        public package app;
        function main(): None { return None; }
    "#;
    let err = parse_source(source).expect_err("package visibility modifier should fail");
    assert!(
        err.message
            .contains("Visibility modifiers are not supported on package declarations"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_class_extends_trailing_comma() {
    let source = r#"
        class Child extends Base, {
        }
    "#;
    let err = parse_source(source).expect_err("class extends trailing comma should fail");
    assert!(
        err.message
            .contains("Class extends clause accepts exactly one base class"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_enum_field_list() {
    let source = r#"
        enum Value {
            One(Integer,),
        }
    "#;
    let err = parse_source(source).expect_err("enum field trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in enum field lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_enum_variant_list() {
    let source = r#"
        enum Value {
            One,
        }
    "#;
    let err = parse_source(source).expect_err("enum variant trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in enum variant lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_pattern_binding_list() {
    let source = r#"
        enum Value {
            One(Integer)
        }

        function main(): None {
            match (One(1)) {
                One(x,) => { return None; },
                _ => { return None; }
            }
        }
    "#;
    let err = parse_source(source).expect_err("pattern binding trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in pattern binding lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_none_pattern_with_empty_binding_list() {
    let source = r#"
        function main(): None {
            match (None) {
                None() => { return None; },
                _ => { return None; }
            }
        }
    "#;
    let err = parse_source(source).expect_err("None() pattern should fail");
    assert!(
        err.message
            .contains("`None` pattern must not use an empty binding list"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_none_pattern_without_binding_list() {
    let source = r#"
        function main(): None {
            match (None) {
                None => { return None; },
                _ => { return None; }
            }
        }
    "#;
    let program = parse_source(source).expect("None pattern should parse without ()");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Match { arms, .. } = &func.body[0].node else {
        panic!("Expected match statement");
    };
    assert!(
        matches!(&arms[0].pattern, Pattern::Variant(name, bindings) if name == "None" && bindings.is_empty())
    );
}

#[test]
fn test_parse_qualified_enum_patterns() {
    let source = r#"
        function main(): None {
            match (x) {
                Enum.A(v) => { return None; },
                util.E.B(w) => { return None; }
            }
        }
    "#;
    let program = parse_source(source).expect("qualified enum patterns should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("expected function declaration");
    };
    let Stmt::Match { arms, .. } = &func.body[0].node else {
        panic!("expected match statement");
    };
    assert!(
        matches!(&arms[0].pattern, Pattern::Variant(name, bindings) if name == "Enum.A" && bindings == &vec!["v".to_string()])
    );
    assert!(
        matches!(&arms[1].pattern, Pattern::Variant(name, bindings) if name == "util.E.B" && bindings == &vec!["w".to_string()])
    );
}

#[test]
fn test_parse_qualified_enum_patterns_without_bindings() {
    let source = r#"
        function main(): None {
            match (x) {
                Enum.A => { return None; },
                util.E.B => { return None; }
            }
        }
    "#;
    let program =
        parse_source(source).expect("qualified enum patterns without bindings should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("expected function declaration");
    };
    let Stmt::Match { arms, .. } = &func.body[0].node else {
        panic!("expected match statement");
    };
    assert!(
        matches!(&arms[0].pattern, Pattern::Variant(name, bindings) if name == "Enum.A" && bindings.is_empty())
    );
    assert!(
        matches!(&arms[1].pattern, Pattern::Variant(name, bindings) if name == "util.E.B" && bindings.is_empty())
    );
}

#[test]
fn test_reject_empty_extern_options() {
    let source = r#"
        extern() function puts(msg: String): Integer;
    "#;
    let err = parse_source(source).expect_err("empty extern options should fail");
    assert!(
        err.message.contains("extern(...) options cannot be empty"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_extern_options() {
    let source = r#"
        extern(c,) function puts(msg: String): Integer;
    "#;
    let err = parse_source(source).expect_err("extern options trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in extern options"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_extra_extern_option_argument() {
    let source = r#"
        extern(c, "puts", "extra") function puts(msg: String): Integer;
    "#;
    let err = parse_source(source).expect_err("extra extern option should fail");
    assert!(
        err.message
            .contains("extern(...) accepts at most ABI and optional link name"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_trailing_comma_in_lambda_parameter_list() {
    let source = r#"
        function main(): None {
            f: (Integer) -> Integer = (x: Integer,) => 1;
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("lambda parameter trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in lambda parameter lists"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_zero_arg_lambda() {
    let source = r#"
        function main(): None {
            f: () -> Integer = () => 1;
            return None;
        }
    "#;

    parse_source(source).expect("zero-arg lambda should parse");
}

#[test]
fn test_reject_trailing_comma_in_require_call() {
    let source = r#"
        function main(): None {
            require(true,);
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("require trailing comma should fail");
    assert!(
        err.message
            .contains("Trailing comma is not allowed in require(...)"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_float_char_and_negative_match_patterns() {
    let source = r#"
        function main(): None {
            f: Float = 1.0;
            c: Char = 'a';
            i: Integer = -1;
            match (f) { 1.0 => { }, _ => { } }
            match (c) { 'a' => { }, _ => { } }
            match (i) { -1 => { }, _ => { } }
            return None;
        }
    "#;
    parse_source(source).expect("Should parse float/char/negative patterns");
}

#[test]
fn test_parse_enum_named_field_with_ptr_type() {
    let source = r#"
        enum Handle {
            Raw(ptr: Ptr<Char>)
        }
    "#;
    let program = parse_source(source).expect("Should parse Ptr in named enum fields");
    let Decl::Enum(en) = &program.declarations[0].node else {
        panic!("Expected enum declaration");
    };
    let field = &en.variants[0].fields[0];
    assert!(matches!(field.ty, Type::Ptr(_)));
}

#[test]
fn test_parse_class_method_attributes_before_visibility() {
    let source = r#"
        class Worker {
            @Io
            private function fetch(): Integer {
                return 1;
            }
        }
    "#;
    let program = parse_source(source).expect("class method attributes should parse");
    let Decl::Class(class_decl) = &program.declarations[0].node else {
        panic!("Expected class declaration");
    };
    let method = &class_decl.methods[0];
    assert_eq!(method.visibility, Visibility::Private);
    assert_eq!(method.attributes, vec![Attribute::EffectIo]);
}

#[test]
fn test_parse_class_method_attributes_after_visibility() {
    let source = r#"
        class Worker {
            private @Pure function value(): Integer {
                return 1;
            }
        }
    "#;
    let program = parse_source(source).expect("mixed method modifiers should parse");
    let Decl::Class(class_decl) = &program.declarations[0].node else {
        panic!("Expected class declaration");
    };
    let method = &class_decl.methods[0];
    assert_eq!(method.visibility, Visibility::Private);
    assert_eq!(method.attributes, vec![Attribute::Pure]);
}

#[test]
fn test_reject_duplicate_class_constructor() {
    let source = r#"
        class Worker {
            constructor() { }
            constructor() { }
        }
    "#;
    let err = parse_source(source).expect_err("duplicate constructor should fail");
    assert!(
        err.message
            .contains("Classes cannot declare more than one constructor"),
        "{}",
        err.message
    );
}

#[test]
fn test_reject_duplicate_class_destructor() {
    let source = r#"
        class Worker {
            destructor() { }
            destructor() { }
        }
    "#;
    let err = parse_source(source).expect_err("duplicate destructor should fail");
    assert!(
        err.message
            .contains("Classes cannot declare more than one destructor"),
        "{}",
        err.message
    );
}

#[test]
fn test_parse_match_expression_block_tail_without_semicolon() {
    let source = r#"
        function main(): Integer {
            return match (1) {
                1 => { 2 }
                _ => { 3 }
            };
        }
    "#;
    let program = parse_source(source).expect("match expression block tails should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Return(Some(expr)) = &func.body[0].node else {
        panic!("Expected return expression");
    };
    let Expr::Match { arms, .. } = &expr.node else {
        panic!("Expected match expression");
    };
    assert!(matches!(
        &arms[0].body[0].node,
        Stmt::Expr(Spanned {
            node: Expr::Literal(Literal::Integer(2)),
            ..
        })
    ));
}

#[test]
fn test_parse_async_block_tail_without_semicolon() {
    let source = r#"
        function main(): Task<Integer> {
            return async { 1 };
        }
    "#;
    let program = parse_source(source).expect("async block tail expressions should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Return(Some(expr)) = &func.body[0].node else {
        panic!("Expected return expression");
    };
    let Expr::AsyncBlock(body) = &expr.node else {
        panic!("Expected async block expression");
    };
    assert!(matches!(
        &body[0].node,
        Stmt::Expr(Spanned {
            node: Expr::Literal(Literal::Integer(1)),
            ..
        })
    ));
}

#[test]
fn test_parse_forward_public_uppercase_function_call_as_call() {
    let source = r#"
        function main(): Integer {
            return ParseValue(1);
        }

        public function ParseValue(value: Integer): Integer {
            return value;
        }
    "#;
    let program =
        parse_source(source).expect("public uppercase forward function call should parse");
    let Decl::Function(main) = &program.declarations[0].node else {
        panic!("Expected main function");
    };
    let Stmt::Return(Some(expr)) = &main.body[0].node else {
        panic!("Expected return expression");
    };
    let Expr::Call { callee, .. } = &expr.node else {
        panic!("Expected function call, not constructor");
    };
    assert!(matches!(&callee.node, Expr::Ident(name) if name == "ParseValue"));
}

#[test]
fn test_parse_forward_public_async_uppercase_function_call_as_call() {
    let source = r#"
        function main(): Task<Integer> {
            return LoadValue(1);
        }

        public async function LoadValue(value: Integer): Task<Integer> {
            return async { value };
        }
    "#;
    let program =
        parse_source(source).expect("public async uppercase forward function call should parse");
    let Decl::Function(main) = &program.declarations[0].node else {
        panic!("Expected main function");
    };
    let Stmt::Return(Some(expr)) = &main.body[0].node else {
        panic!("Expected return expression");
    };
    let Expr::Call { callee, .. } = &expr.node else {
        panic!("Expected async function call, not constructor");
    };
    assert!(matches!(&callee.node, Expr::Ident(name) if name == "LoadValue"));
}

#[test]
fn test_string_interp_empty_braces_stay_literal() {
    let source = r#"
        function main(): None {
            s: String = "before {} after";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "before {} after");
}

#[test]
fn test_string_interp_unclosed_brace_stays_literal() {
    let source = r#"
        function main(): None {
            s: String = "value: {x";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "value: {x");
}

#[test]
fn test_string_literal_decodes_common_escapes() {
    let source = r#"
        function main(): None {
            s: String = "line1\nline2\t\"ok\"\\";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "line1\nline2\t\"ok\"\\");
}

#[test]
fn test_string_literal_rejects_invalid_escape_sequences() {
    let source = r#"
        function main(): None {
            s: String = "bad \q escape";
            return None;
        }
    "#;
    let err = parse_source(source).expect_err("invalid string escape should fail");
    assert!(err.message.contains("Invalid escape sequence"), "{err:?}");
}

#[test]
fn test_string_interp_escaped_braces_stay_literal() {
    let source = r#"
        function main(): None {
            s: String = "\{x\}";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "{x}");
}

#[test]
fn test_string_interp_invalid_expression_stays_literal() {
    let source = r#"
        function main(): None {
            s: String = "value {1+}";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "value {1+}");
}

#[test]
fn test_string_interp_nested_braces_invalid_expr_stays_literal() {
    let source = r#"
        function main(): None {
            s: String = "value {{1+}}";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "value {{1+}}");
}

#[test]
fn test_string_interp_stray_closing_brace_stays_literal() {
    let source = r#"
        function main(): None {
            s: String = "abc }";
            return None;
        }
    "#;
    let program = parse_source(source).expect("Should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("Expected function declaration");
    };
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("Expected let statement");
    };
    let Expr::Literal(Literal::String(s)) = &value.node else {
        panic!("Expected string literal");
    };
    assert_eq!(s, "abc }");
}

#[test]
fn test_builtin_generic_type_rejects_wrong_arity() {
    let source = r#"
        function main(): Map<Integer> {
            return 0;
        }
    "#;
    let err = parse_source(source).expect_err("Map with one type arg should fail to parse");
    assert!(err
        .message
        .contains("Built-in type 'Map' expects 2 type arguments"));
}

#[test]
fn test_builtin_generic_type_rejects_empty_args() {
    let source = r#"
        function main(): Ptr<> {
            return 0;
        }
    "#;
    let err = parse_source(source).expect_err("Ptr<> should fail to parse");
    assert!(err
        .message
        .contains("Generic type argument list cannot be empty"));
}

#[test]
fn test_builtin_generic_type_rejects_trailing_comma() {
    let source = r#"
        function main(): Result<Integer,> {
            return 0;
        }
    "#;
    let err = parse_source(source).expect_err("Trailing comma in type args should fail");
    assert!(err
        .message
        .contains("Trailing comma is not allowed in generic type arguments"));
}

#[test]
fn test_qualified_types_parse_in_type_positions() {
    let source = r#"
        function main(): None {
            b: u.Box = make_box();
            e: u.E = make_enum();
            f: u.Box<Integer> = make_generic_box();
            return None;
        }
    "#;
    let program = parse_source(source).expect("qualified types should parse");
    let Decl::Function(func) = &program.declarations[0].node else {
        panic!("expected function declaration");
    };
    let Stmt::Let { ty, .. } = &func.body[0].node else {
        panic!("expected first let statement");
    };
    assert_eq!(ty, &Type::Named("u.Box".to_string()));
    let Stmt::Let { ty, .. } = &func.body[1].node else {
        panic!("expected second let statement");
    };
    assert_eq!(ty, &Type::Named("u.E".to_string()));
    let Stmt::Let { ty, .. } = &func.body[2].node else {
        panic!("expected third let statement");
    };
    assert_eq!(ty, &Type::Generic("u.Box".to_string(), vec![Type::Integer]));
}

#[test]
fn test_malformed_syntax_corpus_never_panics() {
    let malformed_cases = [
        "function main(: None { return None; }",
        "function main(): Map<Integer> { return 0; }",
        "function main(): None { x: Integer = id<Integer,>(1); return None; }",
        "function takes(f: (Integer,) -> Integer): None { return None; }",
        "import std..math; function main(): None { return None; }",
        "package app.; function main(): None { return None; }",
        "module A { function f(): None { return None; }",
        "function main(): None { s: String = \"value {1+}\"; return None; }",
        "function main(): None { match (1) { 1 => { }, _ => } return None; }",
        "function main(): None { x: List<Integer = range(0, 1); return None; }",
    ];

    for source in malformed_cases {
        let result = std::panic::catch_unwind(|| parse_source(source));
        assert!(
            result.is_ok(),
            "parser panicked on malformed input: {source}"
        );
    }
}

#[test]
fn test_valid_syntax_corpus_roundtrips_through_canonical_formatter() {
    let valid_cases = [
        r#"
            package app.core;
            import std.math as math;
            function main(): None {
                value: Integer = math.abs<Integer>(1);
                return None;
            }
        "#,
        r#"
            function takes(f: () -> Integer): Integer {
                return f();
            }
        "#,
        r#"
            module A {
                module B {
                    function id<T>(x: T): T { return x; }
                }
            }
            function main(): None {
                x: Integer = A.B.id<Integer>(1);
                return None;
            }
        "#,
        r#"
            function main(): None {
                msg: String = "hello {name}";
                x: Integer = if (true) { 1; } else { 2; };
                match (x) {
                    1 => { println(msg); },
                    _ => { println("fallback"); },
                }
                return None;
            }
        "#,
        r#"
            function main(): None {
                mut m: Map<String, Integer> = Map<String, Integer>();
                msg: String = "{m["x"]}";
                return None;
            }
        "#,
        r#"
            import std.string.*;
            function main(): None {
                msg: String = "{Str.contains("\{x\}", "{")}";
                end: String = "{'}'}";
                return None;
            }
        "#,
        r#"
            class Boxed<T> {
                value: T;

                function get(): T {
                    return self.value;
                }
            }
        "#,
    ];

    for source in valid_cases {
        let program = parse_source(source).expect("valid corpus should parse");
        let formatted = format_program_canonical(&program);
        parse_source(&formatted).expect("canonical formatted corpus should still parse");
    }
}

#[test]
fn test_generated_malformed_syntax_matrix_never_panics() {
    let prefixes = [
        "function main(): None { ",
        "module M { function main(): None { ",
        "class C { function main(): None { ",
    ];
    let fragments = [
        "x: Integer = (1 + );",
        "x: Integer = foo<Integer,>(1);",
        "x: Integer = if (true) { 1; } else ;",
        "x: String = \"value {1+}\";",
        "match (1) { 1 => { }, _ => };",
        "items: List<Integer = range(0, 1);",
        "x: Integer = foo(,);",
        "x: Integer = arr[);",
    ];
    let suffixes = [" return None; }", " } }", " return None; } }"];

    for prefix in prefixes {
        for fragment in fragments {
            for suffix in suffixes {
                let source = format!("{prefix}{fragment}{suffix}");
                let result = std::panic::catch_unwind(|| parse_source(&source));
                assert!(
                    result.is_ok(),
                    "parser panicked on generated input: {source}"
                );
            }
        }
    }
}

#[test]
#[ignore = "deterministic stress runner for manual hardening passes"]
fn stress_deterministic_generated_noise_never_panics() {
    let seeds = [1u64, 7, 17, 29, 53, 97, 193, 389];
    let alphabet = [
        "function", "main", "(", ")", "{", "}", "<", ">", ",", ";", ":", "=", "+", "-", "*", "/",
        "if", "else", "match", "module", "class", "import", "package", "foo", "bar", "baz",
        "Integer", "None", "\"x\"", "1", "true", "\n", " ",
    ];

    for seed in seeds {
        let mut state = seed;
        for _case in 0..256 {
            let mut source = String::new();
            let len = 8 + (state as usize % 48);
            for _ in 0..len {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let idx = (state as usize) % alphabet.len();
                source.push_str(alphabet[idx]);
            }
            let result = std::panic::catch_unwind(|| {
                let tokens = tokenize(&source).ok();
                if let Some(tokens) = tokens {
                    let mut parser = Parser::new(tokens);
                    let _ = parser.parse_program();
                }
            });
            assert!(result.is_ok(), "generated stress input panicked: {source}");
        }
    }
}

#[test]
fn missing_semicolon_reports_human_readable_token_names() {
    let source = r#"
        function main(): None {
            println("a")
            println("b");
        }
    "#;

    let tokens = tokenize(source).expect("tokenization succeeds");
    let mut parser = Parser::new(tokens);
    let error = parser.parse_program().expect_err("parse should fail");

    assert!(error.message.contains("Expected `;`"), "{}", error.message);
    assert!(
        error.message.contains("identifier `println`"),
        "{}",
        error.message
    );
    let insertion_offset = source.find("println(\"a\")").unwrap() + "println(\"a\")".len();
    assert_eq!(error.span.start, insertion_offset, "{:?}", error.span);
    assert_eq!(error.span.end, insertion_offset, "{:?}", error.span);
}
