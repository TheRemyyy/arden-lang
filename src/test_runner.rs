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

    for decl in &program.declarations {
        if let Decl::Function(func) = &decl.node {
            // Check for lifecycle hooks
            if has_attribute(&func.attributes, Attribute::BeforeAll) {
                before_all = Some(func.clone());
                continue;
            }
            if has_attribute(&func.attributes, Attribute::Before) {
                before_each = Some(func.clone());
                continue;
            }
            if has_attribute(&func.attributes, Attribute::After) {
                after_each = Some(func.clone());
                continue;
            }
            if has_attribute(&func.attributes, Attribute::AfterAll) {
                after_all = Some(func.clone());
                continue;
            }

            // Check for @Test attribute
            if has_attribute(&func.attributes, Attribute::Test) {
                let ignore_reason = get_ignore_reason(&func.attributes);
                let is_ignored = ignore_reason.is_some();

                suite_tests.push(Test {
                    name: func.name.clone(),
                    function: func.clone(),
                    ignore_reason,
                });

                total_tests += 1;
                if is_ignored {
                    ignored_tests += 1;
                }
            }
        }
    }

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

/// Get ignore reason if test is marked with @Ignore
fn get_ignore_reason(attributes: &[Attribute]) -> Option<String> {
    for attr in attributes {
        if let Attribute::Ignore(reason) = attr {
            return reason.clone();
        }
    }
    None
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
    code.push_str("    tests_total: Integer = 0;\n");
    code.push_str("    tests_passed: Integer = 0;\n");
    code.push_str("    tests_failed: Integer = 0;\n");
    code.push_str("    tests_ignored: Integer = 0;\n\n");

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
    let filtered_source = filter_out_main_function(original_source);
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

/// Simple filter to remove existing main function from source
fn filter_out_main_function(source: &str) -> String {
    let mut result = String::new();
    let mut in_main = false;
    let mut brace_depth = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip package declaration (will be regenerated)
        if trimmed.starts_with("package ") {
            continue;
        }

        // Detect main function start
        if trimmed.starts_with("function main()") && !in_main {
            in_main = true;
            continue;
        }

        if in_main {
            // Track braces to find end of main
            for c in trimmed.chars() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_main = false;
                        break;
                    }
                }
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Generate runner code with mutable counters
fn generate_suite_runner_with_mut(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));
    code.push_str("    println(\"\\n--- Running Tests ---\");\n");
    code.push_str("    println(\"\");\n\n");

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {};\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        // BeforeEach
        if let Some(ref before_each_fn) = suite.before_each {
            code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
            code.push_str(&format!("    {};\n", before_each_fn.name));
        }

        // Test itself
        code.push_str(&format!("    // @Test: {}\n", test.name));

        if let Some(ref reason) = test.ignore_reason {
            // Report ignore inline
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!("    println(\"[IGNORE] {}\");\n", test.name));
            if !reason.is_empty() {
                code.push_str(&format!(
                    "    println(\"      Reason: {}\");\n",
                    reason.replace("\"", "\\\"")
                ));
            }
        } else {
            // Run the test
            code.push_str("    tests_total = tests_total + 1;\n");
            code.push_str(&format!("    print(\"Running: {}... \");\n", test.name));
            code.push_str(&format!("    {};\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");
        }
        code.push_str("    println(\"\");\n\n");

        // AfterEach
        if let Some(ref after_each_fn) = suite.after_each {
            code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
            code.push_str(&format!("    {};\n", after_each_fn.name));
            code.push_str("    println(\"\");\n\n");
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {};\n", after_all_fn.name));
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
        code.push_str(&format!("    {};\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        // BeforeEach
        if let Some(ref before_each_fn) = suite.before_each {
            code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
            code.push_str(&format!("    {};\n", before_each_fn.name));
        }

        // Test itself
        code.push_str(&format!("    // @Test: {}\n", test.name));

        if let Some(ref reason) = test.ignore_reason {
            // Report ignore inline
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!("    println(\"[IGNORE] {}\");\n", test.name));
            if !reason.is_empty() {
                code.push_str(&format!(
                    "    println(\"      Reason: {}\");\n",
                    reason.replace("\"", "\\\"")
                ));
            }
        } else {
            // Run the test
            code.push_str("    tests_total = tests_total + 1;\n");
            code.push_str(&format!("    print(\"Running: {}... \");\n", test.name));
            code.push_str(&format!("    {};\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");
        }
        code.push_str("    println(\"\");\n\n");

        // AfterEach
        if let Some(ref after_each_fn) = suite.after_each {
            code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
            code.push_str(&format!("    {};\n", after_each_fn.name));
            code.push_str("    println(\"\");\n\n");
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {};\n", after_all_fn.name));
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
            if let Some(ref reason) = test.ignore_reason {
                println!(
                    "  {} {} - {}",
                    "@Test".cyan(),
                    test.name.yellow(),
                    format!("(ignored: {})", reason).yellow()
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
