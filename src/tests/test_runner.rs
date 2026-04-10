use super::{
    discover_tests, ensure_test_runner_imports, escape_arden_string_literal, escape_display_text,
    generate_test_runner_with_source, TestDiscovery, TestSuite,
};
use crate::{lexer::tokenize, parser::Parser};

fn generate_suite_runner(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));

    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
    }

    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        if test.ignored {
            code.push_str(&format!("    // @Test: {}\n", test.name));
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!(
                "    println(\"__ARDEN_TEST_SKIP__ {}\");\n",
                escape_arden_string_literal(&test.name)
            ));
            if let Some(reason) = test
                .ignore_reason
                .as_ref()
                .filter(|reason| !reason.is_empty())
            {
                code.push_str(&format!(
                    "    println(\"__ARDEN_TEST_SKIP_REASON__ {}\");\n",
                    escape_arden_string_literal(reason)
                ));
            }
        } else {
            if let Some(ref before_each_fn) = suite.before_each {
                code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
                code.push_str(&format!("    {}();\n", before_each_fn.name));
            }

            code.push_str(&format!("    // @Test: {}\n", test.name));
            code.push_str(&format!(
                "    println(\"__ARDEN_TEST_START__ {}\");\n",
                escape_arden_string_literal(&test.name)
            ));
            code.push_str(&format!("    {}();\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str(&format!(
                "    println(\"__ARDEN_TEST_PASS__ {}\");\n",
                escape_arden_string_literal(&test.name)
            ));

            if let Some(ref after_each_fn) = suite.after_each {
                code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
                code.push_str(&format!("    {}();\n", after_each_fn.name));
            }
        }
    }

    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {}();\n", after_all_fn.name));
    }
}

fn generate_test_runner(discovery: &TestDiscovery) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated test runner\n");
    code.push_str("import std.io.*;\n");
    code.push_str("import std.string.*;\n\n");
    code.push_str("function main(): None {\n");
    code.push_str("    // Test execution tracking\n");
    code.push_str("    mut tests_total: Integer = 0;\n");
    code.push_str("    mut tests_passed: Integer = 0;\n");
    code.push_str("    mut tests_failed: Integer = 0;\n");
    code.push_str("    mut tests_ignored: Integer = 0;\n\n");

    for suite in &discovery.suites {
        generate_suite_runner(&mut code, suite);
    }

    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        exit(1);\n");
    code.push_str("    }\n");
    code.push_str("    return None;\n");
    code.push_str("}\n");

    code
}

#[test]
fn injects_stdio_import_when_missing() {
    let source = "function helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(rewritten.starts_with("import std.io.*;"));
}

#[test]
fn preserves_package_when_injecting_stdio_import() {
    let source = "package tests;\nfunction helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(rewritten.starts_with("package tests;"));
    assert!(rewritten.contains("\n\nimport std.io.*;"));
}

#[test]
fn generated_runner_contains_stdio_import() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let generated =
        generate_test_runner_with_source(&discovery, "function helper(): None { return None; }\n");
    assert!(generated.contains("import std.io.*;"));
}

#[test]
fn generated_runner_uses_mutable_counters() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let generated = generate_test_runner(&discovery);
    assert!(generated.contains("mut tests_total: Integer = 0;"));
    assert!(generated.contains("mut tests_passed: Integer = 0;"));
}

#[test]
fn strips_public_main_function_from_source() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
public function main(): Integer {
return 0;
}
function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(!generated.contains("public function main(): Integer"));
    assert!(generated.contains("function helper(): None"));
}

#[test]
fn injects_stdio_import_even_if_comment_mentions_it() {
    let source = "// import std.io.*;\nfunction helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(rewritten.starts_with("import std.io.*;\n\n// import std.io.*;"));
}

#[test]
fn injects_stdio_import_when_block_comment_mentions_it() {
    let source = "/*\nimport std.io.*;\n*/\nfunction helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(
        rewritten.starts_with("import std.io.*;\n\n/*\nimport std.io.*;\n*/"),
        "{rewritten}"
    );
}

#[test]
fn injects_stdio_import_after_shebang() {
    let source = "#!/usr/bin/env arden\nfunction helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(
        rewritten.starts_with("#!/usr/bin/env arden\nimport std.io.*;\n\n"),
        "{rewritten}"
    );
}

#[test]
fn preserves_package_when_comments_have_semicolons() {
    let source =
        "// note; with semicolon\npackage tests;\nfunction helper(): None { return None; }\n";
    let rewritten = ensure_test_runner_imports(source);
    assert!(rewritten.contains("package tests;\n\nimport std.io.*;"));
}

#[test]
fn does_not_strip_comment_that_mentions_main_signature() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
// function main(): None { not real }
function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("// function main(): None { not real }"));
    assert!(generated.contains("function helper(): None"));
}

#[test]
fn strips_public_async_main_function_from_source() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
public async function main(): Integer {
return 0;
}
function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(!generated.contains("public async function main(): Integer"));
    assert!(generated.contains("function helper(): None"));
}

#[test]
fn discover_tests_marks_ignore_without_reason_as_ignored() {
    let source = r#"
@Test
@Ignore
function skipped(): None {
return None;
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");

    let discovery = discover_tests(&program);
    assert_eq!(discovery.ignored_tests, 1);
    assert!(discovery.suites[0].tests[0].ignored);
    assert_eq!(discovery.suites[0].tests[0].ignore_reason, None);
}

#[test]
fn generated_runner_skips_ignore_without_reason() {
    let source = r#"
@Test
@Ignore
function skipped(): None {
fail("should not run");
return None;
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("println(\"__ARDEN_TEST_SKIP__ skipped\");"));
    assert!(!generated.contains("__ARDEN_TEST_PASS__ skipped"));
    assert!(!generated.contains("__ARDEN_TEST_SKIP_REASON__"));
}

#[test]
fn generated_runner_counts_ignored_tests_in_total() {
    let source = r#"
@Test
function runs(): None {
return None;
}

@Test
@Ignore
function skipped(): None {
return None;
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    let generated = generate_test_runner_with_source(&discovery, source);
    assert_eq!(
        generated.matches("tests_total = tests_total + 1;").count(),
        2
    );
}

#[test]
fn generated_runner_escapes_ignore_reason_control_chars() {
    let source =
        "@Test\n@Ignore(\"c:\\\\tmp\\\\foo\\nline2\")\nfunction skipped(): None { return None; }\n";
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("__ARDEN_TEST_SKIP_REASON__ c:\\\\tmp\\\\foo\\\\nline2"));
}

#[test]
fn discovery_print_escapes_ignore_reason_control_chars() {
    assert_eq!(
        escape_display_text("c:\\tmp\\foo\nline2\tz"),
        "c:\\\\tmp\\\\foo\\nline2\\tz"
    );
}

#[test]
fn discover_tests_in_nested_modules() {
    let source = r#"
module Tests {
@Before
function setup(): None { return None; }

@Test
function works(): None { return None; }
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    assert_eq!(discovery.total_tests, 1);
    assert_eq!(discovery.suites[0].tests[0].name, "Tests__works");
    assert_eq!(
        discovery.suites[0]
            .before_each
            .as_ref()
            .expect("before hook")
            .name,
        "Tests__setup"
    );
}

#[test]
fn discover_tests_keeps_module_hooks_isolated_per_suite() {
    let source = r#"
module Alpha {
@Before
function setup(): None { return None; }

@Test
function works(): None { return None; }
}

module Beta {
@Before
function setup(): None { return None; }

@Test
function works(): None { return None; }
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    assert_eq!(discovery.suites.len(), 2, "{discovery:#?}");
    assert_eq!(discovery.suites[0].name, "Alpha");
    assert_eq!(
        discovery.suites[0]
            .before_each
            .as_ref()
            .expect("alpha before hook")
            .name,
        "Alpha__setup"
    );
    assert_eq!(discovery.suites[1].name, "Beta");
    assert_eq!(
        discovery.suites[1]
            .before_each
            .as_ref()
            .expect("beta before hook")
            .name,
        "Beta__setup"
    );
}

#[test]
fn strips_main_without_eating_following_code_when_string_contains_braces() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
println("}");
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("function helper(): None"), "{generated}");
}

#[test]
fn strips_main_without_eating_following_code_when_string_contains_open_brace() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
println("{");
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("function helper(): None"), "{generated}");
    assert!(!generated.contains("println(\"{\")"), "{generated}");
}

#[test]
fn strips_main_without_leaking_body_when_line_comment_contains_closing_brace() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
// }
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(!generated.contains("// }"), "{generated}");
    assert!(generated.contains("function helper(): None"), "{generated}");
}

#[test]
fn strips_main_without_eating_following_code_when_line_comment_contains_open_brace() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
// {
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("function helper(): None"), "{generated}");
    assert!(!generated.contains("// {"), "{generated}");
}

#[test]
fn strips_main_without_leaking_body_when_block_comment_contains_closing_brace() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
/* } */
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(!generated.contains("/* } */"), "{generated}");
    assert!(generated.contains("function helper(): None"), "{generated}");
}

#[test]
fn strips_main_without_eating_following_code_when_block_comment_contains_open_brace() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main(): Integer {
/* { */
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(generated.contains("function helper(): None"), "{generated}");
    assert!(!generated.contains("/* { */"), "{generated}");
}

#[test]
fn strips_multiline_main_signature_without_removing_following_functions() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
function main()
: Integer
{
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(
        !generated.contains("function main()\n: Integer\n{"),
        "{generated}"
    );
    assert!(generated.contains("function helper(): None"), "{generated}");
}

#[test]
fn generated_runner_escapes_ignore_reason_braces() {
    let source = "@Test\n@Ignore(\"\\{danger\\}\")\nfunction skipped(): None { return None; }\n";
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(
        generated.contains("__ARDEN_TEST_SKIP_REASON__ \\{danger\\}"),
        "{generated}"
    );
}

#[test]
fn ignored_tests_do_not_run_before_or_after_hooks() {
    let source = r#"
@Before
function setup(): None { return None; }

@After
function teardown(): None { return None; }

@Test
@Ignore("later")
function skipped(): None { return None; }
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let discovery = discover_tests(&program);

    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(
        !generated.contains("tests_total = tests_total + 1;\n    // @Before: setup\n    setup();\n    // @Test: skipped"),
        "{generated}"
    );
    assert!(
        !generated.contains(
            "println(\"__ARDEN_TEST_SKIP__ skipped\");\n    // @After: teardown\n    teardown();"
        ),
        "{generated}"
    );
}

#[test]
fn strips_attributes_attached_to_main_function() {
    let discovery = TestDiscovery {
        suites: vec![],
        total_tests: 0,
        ignored_tests: 0,
    };
    let source = r#"
@Test
@Ignore("not a real test")
function main(): Integer {
return 0;
}

function helper(): None { return None; }
"#;
    let generated = generate_test_runner_with_source(&discovery, source);
    assert!(!generated.contains("@Test"), "{generated}");
    assert!(
        !generated.contains("@Ignore(\"not a real test\")"),
        "{generated}"
    );
    assert!(generated.contains("function helper(): None"), "{generated}");
}
