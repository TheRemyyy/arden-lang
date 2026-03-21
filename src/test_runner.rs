//! Apex Test Runner - Discovers and runs @Test annotated functions
//!
//! Supports:
//! - @Test: Marks a function as a test
//! - @Ignore: Skips a test (with optional reason)
//! - @Before: Runs before each test
//! - @After: Runs after each test
//! - @BeforeAll: Runs once before all tests
//! - @AfterAll: Runs once after all tests

use crate::ast::{Attribute, Decl, FunctionDecl, Program};
use colored::*;

/// Represents a discovered test
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Test {
    pub name: String,
    pub function: FunctionDecl,
    pub ignored: bool,
    pub ignore_reason: Option<String>,
}

/// Represents a test class/module with lifecycle hooks
#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<Test>,
    pub before_all: Option<FunctionDecl>,
    pub before_each: Option<FunctionDecl>,
    pub after_each: Option<FunctionDecl>,
    pub after_all: Option<FunctionDecl>,
}

/// Test discovery result
#[derive(Debug)]
pub struct TestDiscovery {
    pub suites: Vec<TestSuite>,
    pub total_tests: usize,
    pub ignored_tests: usize,
}

/// Test execution result
#[derive(Debug)]
#[allow(dead_code)]
pub struct TestResult {
    pub suite_name: String,
    pub test_name: String,
    pub passed: bool,
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

/// Overall test run summary
#[derive(Debug)]
#[allow(dead_code)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub results: Vec<TestResult>,
}

/// Discover all tests in a program
pub fn discover_tests(program: &Program) -> TestDiscovery {
    #[allow(clippy::too_many_arguments)]
    fn collect_suite_functions(
        declarations: &[crate::ast::Spanned<Decl>],
        module_prefix: Option<&str>,
        suite_tests: &mut Vec<Test>,
        before_all: &mut Option<FunctionDecl>,
        before_each: &mut Option<FunctionDecl>,
        after_each: &mut Option<FunctionDecl>,
        after_all: &mut Option<FunctionDecl>,
        total_tests: &mut usize,
        ignored_tests: &mut usize,
    ) {
        for decl in declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let qualified_name = module_prefix
                        .map(|prefix| format!("{}__{}", prefix, func.name))
                        .unwrap_or_else(|| func.name.clone());
                    let mut qualified_func = func.clone();
                    qualified_func.name = qualified_name.clone();

                    if has_attribute(&func.attributes, Attribute::BeforeAll) {
                        *before_all = Some(qualified_func);
                        continue;
                    }
                    if has_attribute(&func.attributes, Attribute::Before) {
                        *before_each = Some(qualified_func);
                        continue;
                    }
                    if has_attribute(&func.attributes, Attribute::After) {
                        *after_each = Some(qualified_func);
                        continue;
                    }
                    if has_attribute(&func.attributes, Attribute::AfterAll) {
                        *after_all = Some(qualified_func);
                        continue;
                    }

                    if has_attribute(&func.attributes, Attribute::Test) {
                        let ignored = has_ignore_attribute(&func.attributes);
                        let ignore_reason = get_ignore_reason(&func.attributes);

                        suite_tests.push(Test {
                            name: qualified_name,
                            function: qualified_func,
                            ignored,
                            ignore_reason,
                        });

                        *total_tests += 1;
                        if ignored {
                            *ignored_tests += 1;
                        }
                    }
                }
                Decl::Module(module) => {
                    let next_prefix = module_prefix
                        .map(|prefix| format!("{}__{}", prefix, module.name))
                        .unwrap_or_else(|| module.name.clone());
                    collect_suite_functions(
                        &module.declarations,
                        Some(&next_prefix),
                        suite_tests,
                        before_all,
                        before_each,
                        after_each,
                        after_all,
                        total_tests,
                        ignored_tests,
                    );
                }
                Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
    }

    let mut suites = Vec::new();
    let mut total_tests = 0;
    let mut ignored_tests = 0;

    // For now, we create a single "default" suite for all top-level test functions
    // In the future, we can support test classes/modules
    let mut suite_tests = Vec::new();
    let mut before_all = None;
    let mut before_each = None;
    let mut after_each = None;
    let mut after_all = None;

    collect_suite_functions(
        &program.declarations,
        None,
        &mut suite_tests,
        &mut before_all,
        &mut before_each,
        &mut after_each,
        &mut after_all,
        &mut total_tests,
        &mut ignored_tests,
    );

    // Create default suite if we found any tests
    if !suite_tests.is_empty() {
        suites.push(TestSuite {
            name: "default".to_string(),
            tests: suite_tests,
            before_all,
            before_each,
            after_each,
            after_all,
        });
    }

    TestDiscovery {
        suites,
        total_tests,
        ignored_tests,
    }
}

/// Check if function has a specific attribute
fn has_attribute(attributes: &[Attribute], target: Attribute) -> bool {
    attributes.iter().any(|attr| {
        matches!(
            (attr, &target),
            (Attribute::Test, Attribute::Test)
                | (Attribute::Before, Attribute::Before)
                | (Attribute::After, Attribute::After)
                | (Attribute::BeforeAll, Attribute::BeforeAll)
                | (Attribute::AfterAll, Attribute::AfterAll)
        )
    })
}

fn has_ignore_attribute(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| matches!(attr, Attribute::Ignore(_)))
}

/// Get ignore reason if test is marked with @Ignore
fn get_ignore_reason(attributes: &[Attribute]) -> Option<String> {
    for attr in attributes {
        if let Attribute::Ignore(reason) = attr {
            return reason.clone();
        }
    }
    None
}

fn escape_apex_string_literal(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\\\n"),
            '\r' => escaped.push_str("\\\\r"),
            '\t' => escaped.push_str("\\\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn escape_display_text(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Generate test runner code for compilation
#[allow(dead_code)]
pub fn generate_test_runner(discovery: &TestDiscovery) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated test runner\n");
    code.push_str("import std.io.*;\n");
    code.push_str("import std.string.*;\n\n");

    // Generate main test runner function
    code.push_str("function main(): None {\n");

    // Local test counters
    code.push_str("    // Test execution tracking\n");
    code.push_str("    mut tests_total: Integer = 0;\n");
    code.push_str("    mut tests_passed: Integer = 0;\n");
    code.push_str("    mut tests_failed: Integer = 0;\n");
    code.push_str("    mut tests_ignored: Integer = 0;\n\n");

    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"         Apex Test Runner\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"\");\n\n");

    for suite in &discovery.suites {
        generate_suite_runner(&mut code, suite);
    }

    // Summary
    code.push_str("    println(\"\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"         Test Summary\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"Total:   \" + to_string(tests_total));\n");
    code.push_str("    println(\"Passed:  \" + to_string(tests_passed));\n");
    code.push_str("    println(\"Failed:  \" + to_string(tests_failed));\n");
    code.push_str("    println(\"Ignored: \" + to_string(tests_ignored));\n");
    code.push_str("    println(\"\");\n");
    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        println(\"SOME TESTS FAILED\");\n");
    code.push_str("        exit(1);\n"); // Use exit directly
    code.push_str("    } else {\n");
    code.push_str("        println(\"ALL TESTS PASSED\");\n");
    code.push_str("    }\n");
    code.push_str("    return None;\n");
    code.push_str("}\n");

    code
}

/// Generate test runner that includes the original source code
pub fn generate_test_runner_with_source(
    discovery: &TestDiscovery,
    original_source: &str,
) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated test runner with original source\n");

    // Extract just the declarations from original source (skip package, imports)
    // For simplicity, we include the whole source but filter out any existing main()
    let filtered_source = ensure_test_runner_imports(&filter_out_main_function(original_source));
    code.push_str(&filtered_source);
    code.push('\n');

    // Add the test runner main
    code.push_str("// Test runner entry point\n");
    code.push_str("function __test_main(): None {\n");

    // Local test counters
    code.push_str("    // Test execution tracking\n");
    code.push_str("    mut tests_total: Integer = 0;\n");
    code.push_str("    mut tests_passed: Integer = 0;\n");
    code.push_str("    mut tests_failed: Integer = 0;\n");
    code.push_str("    mut tests_ignored: Integer = 0;\n\n");

    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"         Apex Test Runner\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"\");\n\n");

    for suite in &discovery.suites {
        generate_suite_runner_with_mut(&mut code, suite);
    }

    // Summary
    code.push_str("    println(\"\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"         Test Summary\");\n");
    code.push_str("    println(\"========================================\");\n");
    code.push_str("    println(\"Total:   \" + to_string(tests_total));\n");
    code.push_str("    println(\"Passed:  \" + to_string(tests_passed));\n");
    code.push_str("    println(\"Failed:  \" + to_string(tests_failed));\n");
    code.push_str("    println(\"Ignored: \" + to_string(tests_ignored));\n");
    code.push_str("    println(\"\");\n");
    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        println(\"SOME TESTS FAILED\");\n");
    code.push_str("        exit(1);\n");
    code.push_str("    } else {\n");
    code.push_str("        println(\"ALL TESTS PASSED\");\n");
    code.push_str("    }\n");
    code.push_str("    return None;\n");
    code.push_str("}\n");

    // Redirect main to __test_main
    code.push_str("\n// Entry point redirects to test runner\n");
    code.push_str("function main(): Integer {\n");
    code.push_str("    __test_main();\n");
    code.push_str("    return 0;\n");
    code.push_str("}\n");

    code
}

fn ensure_test_runner_imports(source: &str) -> String {
    if source
        .lines()
        .any(|line| line.trim_start().starts_with("import std.io.*;"))
    {
        return source.to_string();
    }

    let mut lines: Vec<&str> = source.lines().collect();
    if let Some(idx) = lines
        .iter()
        .position(|line| line.trim_start().starts_with("package "))
    {
        lines.insert(idx + 1, "");
        lines.insert(idx + 2, "import std.io.*;");
        return format!("{}\n", lines.join("\n"));
    }

    format!("import std.io.*;\n\n{}", source)
}

/// Simple filter to remove existing main function from source
fn filter_out_main_function(source: &str) -> String {
    let mut result = String::new();
    let mut in_main = false;
    let mut brace_depth = 0;
    let mut seen_main_open_brace = false;

    fn brace_delta(line: &str) -> i32 {
        line.chars().fold(0, |acc, c| match c {
            '{' => acc + 1,
            '}' => acc - 1,
            _ => acc,
        })
    }

    fn is_main_signature(line: &str) -> bool {
        let mut rest = line.trim_start();
        for vis in ["public ", "private ", "protected "] {
            if rest.starts_with(vis) {
                rest = &rest[vis.len()..];
                break;
            }
        }
        if rest.starts_with("async ") {
            rest = &rest["async ".len()..];
        }
        rest.starts_with("function main(")
    }

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip package declaration (will be regenerated)
        if trimmed.starts_with("package ") {
            continue;
        }

        // Detect main function start
        if is_main_signature(trimmed) && !in_main {
            in_main = true;
            brace_depth = brace_delta(trimmed);
            seen_main_open_brace = trimmed.contains('{');
            continue;
        }

        if in_main {
            // Track braces to find end of main; include the opening brace on signature line.
            brace_depth += brace_delta(trimmed);
            if trimmed.contains('{') {
                seen_main_open_brace = true;
            }
            if seen_main_open_brace && brace_depth <= 0 {
                in_main = false;
                brace_depth = 0;
                seen_main_open_brace = false;
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{
        discover_tests, ensure_test_runner_imports, escape_display_text, generate_test_runner,
        generate_test_runner_with_source, TestDiscovery,
    };
    use crate::{lexer::tokenize, parser::Parser};

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
        let generated = generate_test_runner_with_source(
            &discovery,
            "function helper(): None { return None; }\n",
        );
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
        assert!(generated.contains("println(\"[IGNORE] skipped\");"));
        assert!(!generated.contains("Running: skipped..."));
        assert!(!generated.contains("Reason:"));
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
        let source = "@Test\n@Ignore(\"c:\\\\tmp\\\\foo\\nline2\")\nfunction skipped(): None { return None; }\n";
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let discovery = discover_tests(&program);

        let generated = generate_test_runner_with_source(&discovery, source);
        assert!(generated.contains("Reason: c:\\\\tmp\\\\foo\\\\nline2"));
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
}

/// Generate runner code with mutable counters
fn generate_suite_runner_with_mut(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));
    code.push_str("    println(\"\\n--- Running Tests ---\");\n");
    code.push_str("    println(\"\");\n\n");

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        // BeforeEach
        if let Some(ref before_each_fn) = suite.before_each {
            code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
            code.push_str(&format!("    {}();\n", before_each_fn.name));
        }

        // Test itself
        code.push_str(&format!("    // @Test: {}\n", test.name));

        if test.ignored {
            // Report ignore inline
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!("    println(\"[IGNORE] {}\");\n", test.name));
            if let Some(reason) = test
                .ignore_reason
                .as_ref()
                .filter(|reason| !reason.is_empty())
            {
                code.push_str(&format!(
                    "    println(\"      Reason: {}\");\n",
                    escape_apex_string_literal(reason)
                ));
            }
        } else {
            // Run the test
            code.push_str(&format!("    print(\"Running: {}... \");\n", test.name));
            code.push_str(&format!("    {}();\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");
        }
        code.push_str("    println(\"\");\n\n");

        // AfterEach
        if let Some(ref after_each_fn) = suite.after_each {
            code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
            code.push_str(&format!("    {}();\n", after_each_fn.name));
            code.push_str("    println(\"\");\n\n");
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {}();\n", after_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }
}

/// Generate runner code for a single test suite
#[allow(dead_code)]
fn generate_suite_runner(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));
    code.push_str("    println(\"\\n--- Running Tests ---\");\n");
    code.push_str("    println(\"\");\n\n");

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        // BeforeEach
        if let Some(ref before_each_fn) = suite.before_each {
            code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
            code.push_str(&format!("    {}();\n", before_each_fn.name));
        }

        // Test itself
        code.push_str(&format!("    // @Test: {}\n", test.name));

        if test.ignored {
            // Report ignore inline
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!("    println(\"[IGNORE] {}\");\n", test.name));
            if let Some(reason) = test
                .ignore_reason
                .as_ref()
                .filter(|reason| !reason.is_empty())
            {
                code.push_str(&format!(
                    "    println(\"      Reason: {}\");\n",
                    escape_apex_string_literal(reason)
                ));
            }
        } else {
            // Run the test
            code.push_str(&format!("    print(\"Running: {}... \");\n", test.name));
            code.push_str(&format!("    {}();\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");
        }
        code.push_str("    println(\"\");\n\n");

        // AfterEach
        if let Some(ref after_each_fn) = suite.after_each {
            code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
            code.push_str(&format!("    {}();\n", after_each_fn.name));
            code.push_str("    println(\"\");\n\n");
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {}();\n", after_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }
}

/// Print test discovery info
pub fn print_discovery(discovery: &TestDiscovery) {
    println!("{}", "========================================".cyan());
    println!("{}", "         Test Discovery".cyan().bold());
    println!("{}", "========================================".cyan());
    println!();

    if discovery.suites.is_empty() {
        println!("{}", "No tests found.".yellow());
        println!();
        println!("Mark functions with @Test to create tests:");
        println!("  {} function myTest(): None {{ ... }}", "@Test".cyan());
        return;
    }

    for suite in &discovery.suites {
        println!("Suite: {}", suite.name.green().bold());

        if let Some(ref fn_decl) = suite.before_all {
            println!("  {}: {}", "@BeforeAll".blue(), fn_decl.name);
        }
        if let Some(ref fn_decl) = suite.before_each {
            println!("  {}: {}", "@Before".blue(), fn_decl.name);
        }

        for test in &suite.tests {
            if test.ignored {
                let status = test
                    .ignore_reason
                    .as_ref()
                    .filter(|reason| !reason.is_empty())
                    .map(|reason| format!("(ignored: {})", escape_display_text(reason)))
                    .unwrap_or_else(|| "(ignored)".to_string());
                println!(
                    "  {} {} - {}",
                    "@Test".cyan(),
                    test.name.yellow(),
                    status.yellow()
                );
            } else {
                println!("  {} {}", "@Test".cyan(), test.name.green());
            }
        }

        if let Some(ref fn_decl) = suite.after_each {
            println!("  {}: {}", "@After".blue(), fn_decl.name);
        }
        if let Some(ref fn_decl) = suite.after_all {
            println!("  {}: {}", "@AfterAll".blue(), fn_decl.name);
        }
        println!();
    }

    println!(
        "Total: {} tests ({} ignored)",
        discovery.total_tests.to_string().cyan().bold(),
        discovery.ignored_tests.to_string().yellow()
    );
}

/// Print test summary
#[allow(dead_code)]
pub fn print_summary(summary: &TestSummary) {
    println!();
    println!("{}", "========================================".cyan());
    println!("{}", "         Test Summary".cyan().bold());
    println!("{}", "========================================".cyan());

    println!("Total:   {}", summary.total.to_string().cyan().bold());
    println!("Passed:  {}", summary.passed.to_string().green().bold());
    println!(
        "Failed:  {}",
        if summary.failed > 0 {
            summary.failed.to_string().red().bold()
        } else {
            summary.failed.to_string().green()
        }
    );
    println!("Ignored: {}", summary.ignored.to_string().yellow());

    println!();
    if summary.failed > 0 {
        println!("{}", "SOME TESTS FAILED".red().bold());
    } else if summary.passed > 0 {
        println!("{}", "ALL TESTS PASSED".green().bold());
    } else {
        println!("{}", "NO TESTS RUN".yellow());
    }
}
