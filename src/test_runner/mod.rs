//! Arden Test Runner - Discovers and runs @Test annotated functions
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
    let mut suites = Vec::new();
    let mut total_tests = 0;
    let mut ignored_tests = 0;

    #[allow(clippy::too_many_arguments)]
    fn collect_function_into_suite(
        func: &FunctionDecl,
        qualified_name: String,
        suite_tests: &mut Vec<Test>,
        before_all: &mut Option<FunctionDecl>,
        before_each: &mut Option<FunctionDecl>,
        after_each: &mut Option<FunctionDecl>,
        after_all: &mut Option<FunctionDecl>,
        total_tests: &mut usize,
        ignored_tests: &mut usize,
    ) {
        let mut qualified_func = func.clone();
        qualified_func.name = qualified_name.clone();

        if has_attribute(&func.attributes, Attribute::BeforeAll) {
            *before_all = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::Before) {
            *before_each = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::After) {
            *after_each = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::AfterAll) {
            *after_all = Some(qualified_func);
            return;
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

    fn collect_module_suite(
        declarations: &[crate::ast::Spanned<Decl>],
        suite_name: String,
        total_tests: &mut usize,
        ignored_tests: &mut usize,
        suites: &mut Vec<TestSuite>,
    ) {
        let mut suite_tests = Vec::new();
        let mut before_all = None;
        let mut before_each = None;
        let mut after_each = None;
        let mut after_all = None;

        for decl in declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let qualified_name = format!("{}__{}", suite_name, func.name);
                    collect_function_into_suite(
                        func,
                        qualified_name,
                        &mut suite_tests,
                        &mut before_all,
                        &mut before_each,
                        &mut after_each,
                        &mut after_all,
                        total_tests,
                        ignored_tests,
                    );
                }
                Decl::Module(module) => {
                    let nested_suite = format!("{}__{}", suite_name, module.name);
                    collect_module_suite(
                        &module.declarations,
                        nested_suite,
                        total_tests,
                        ignored_tests,
                        suites,
                    );
                }
                Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
            }
        }

        if !suite_tests.is_empty() {
            suites.push(TestSuite {
                name: suite_name,
                tests: suite_tests,
                before_all,
                before_each,
                after_each,
                after_all,
            });
        }
    }

    let mut default_tests = Vec::new();
    let mut before_all = None;
    let mut before_each = None;
    let mut after_each = None;
    let mut after_all = None;

    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                collect_function_into_suite(
                    func,
                    func.name.clone(),
                    &mut default_tests,
                    &mut before_all,
                    &mut before_each,
                    &mut after_each,
                    &mut after_all,
                    &mut total_tests,
                    &mut ignored_tests,
                );
            }
            Decl::Module(module) => {
                collect_module_suite(
                    &module.declarations,
                    module.name.clone(),
                    &mut total_tests,
                    &mut ignored_tests,
                    &mut suites,
                );
            }
            Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    if !default_tests.is_empty() {
        suites.insert(
            0,
            TestSuite {
                name: "default".to_string(),
                tests: default_tests,
                before_all,
                before_each,
                after_each,
                after_all,
            },
        );
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

fn escape_arden_string_literal(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
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

    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"| Arden Test Runner                    |\");\n");
    code.push_str("    println(\"| mode    suite-driven execution       |\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"\");\n\n");

    for suite in &discovery.suites {
        generate_suite_runner(&mut code, suite);
    }

    // Summary
    code.push_str("    println(\"\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"| Test Summary                         |\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"total    \" + to_string(tests_total));\n");
    code.push_str("    println(\"passed   \" + to_string(tests_passed));\n");
    code.push_str("    println(\"failed   \" + to_string(tests_failed));\n");
    code.push_str("    println(\"ignored  \" + to_string(tests_ignored));\n");
    code.push_str("    println(\"\");\n");
    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        println(\"status   SOME TESTS FAILED\");\n");
    code.push_str("        exit(1);\n"); // Use exit directly
    code.push_str("    } else {\n");
    code.push_str("        println(\"status   ALL TESTS PASSED\");\n");
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

    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"| Arden Test Runner                    |\");\n");
    code.push_str("    println(\"| mode    suite-driven execution       |\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"\");\n\n");

    for suite in &discovery.suites {
        generate_suite_runner_with_mut(&mut code, suite);
    }

    // Summary
    code.push_str("    println(\"\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"| Test Summary                         |\");\n");
    code.push_str("    println(\"+======================================+\");\n");
    code.push_str("    println(\"total    \" + to_string(tests_total));\n");
    code.push_str("    println(\"passed   \" + to_string(tests_passed));\n");
    code.push_str("    println(\"failed   \" + to_string(tests_failed));\n");
    code.push_str("    println(\"ignored  \" + to_string(tests_ignored));\n");
    code.push_str("    println(\"\");\n");
    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        println(\"status   SOME TESTS FAILED\");\n");
    code.push_str("        exit(1);\n");
    code.push_str("    } else {\n");
    code.push_str("        println(\"status   ALL TESTS PASSED\");\n");
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
    let mut in_block_comment = false;
    if source.lines().any(|line| {
        let trimmed = line.trim_start();
        let has_import = !in_block_comment
            && !trimmed.starts_with("//")
            && trimmed.starts_with("import std.io.*;");

        if !in_block_comment {
            if let Some(start) = trimmed.find("/*") {
                let ends_after_start = trimmed[start + 2..].contains("*/");
                if !ends_after_start {
                    in_block_comment = true;
                }
            }
        } else if trimmed.contains("*/") {
            in_block_comment = false;
        }

        has_import
    }) {
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

    if lines
        .first()
        .is_some_and(|line| line.trim_start().starts_with("#!"))
    {
        lines.insert(1, "import std.io.*;");
        lines.insert(2, "");
        return format!("{}\n", lines.join("\n"));
    }

    format!("import std.io.*;\n\n{}", source)
}

/// Simple filter to remove existing main function from source
fn filter_out_main_function(source: &str) -> String {
    let mut result = String::new();
    let mut pending_attributes: Vec<&str> = Vec::new();
    let mut in_main = false;
    let mut brace_depth = 0;
    let mut seen_main_open_brace = false;
    let mut in_block_comment = false;

    fn scan_code_braces(line: &str, in_block_comment: &mut bool) -> (i32, bool) {
        let mut delta = 0;
        let mut saw_open_brace = false;
        let mut chars = line.chars().peekable();
        let mut in_string = false;
        let mut in_char = false;
        let mut escape = false;

        while let Some(ch) = chars.next() {
            if *in_block_comment {
                if ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    *in_block_comment = false;
                }
                continue;
            }

            if in_string {
                if escape {
                    escape = false;
                } else if ch == '\\' {
                    escape = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }

            if in_char {
                if escape {
                    escape = false;
                } else if ch == '\\' {
                    escape = true;
                } else if ch == '\'' {
                    in_char = false;
                }
                continue;
            }

            if ch == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                *in_block_comment = true;
                continue;
            }

            match ch {
                '"' => in_string = true,
                '\'' => in_char = true,
                '{' => {
                    delta += 1;
                    saw_open_brace = true;
                }
                '}' => delta -= 1,
                _ => {}
            }
        }

        (delta, saw_open_brace)
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

        if !in_main && trimmed.starts_with('@') {
            pending_attributes.push(line);
            continue;
        }

        // Detect main function start
        if is_main_signature(trimmed) && !in_main {
            pending_attributes.clear();
            in_main = true;
            let (delta, saw_open_brace) = scan_code_braces(line, &mut in_block_comment);
            brace_depth = delta;
            seen_main_open_brace = saw_open_brace;
            continue;
        }

        if in_main {
            // Track braces to find end of main; include the opening brace on signature line.
            let (delta, saw_open_brace) = scan_code_braces(line, &mut in_block_comment);
            brace_depth += delta;
            if saw_open_brace {
                seen_main_open_brace = true;
            }
            if seen_main_open_brace && brace_depth <= 0 {
                in_main = false;
                brace_depth = 0;
                seen_main_open_brace = false;
            }
            continue;
        }

        if !pending_attributes.is_empty() {
            for attr_line in pending_attributes.drain(..) {
                result.push_str(attr_line);
                result.push('\n');
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    if !pending_attributes.is_empty() {
        for attr_line in pending_attributes {
            result.push_str(attr_line);
            result.push('\n');
        }
    }

    result
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests;

/// Generate runner code with mutable counters
fn generate_suite_runner_with_mut(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));
    code.push_str("    println(\"\");\n");
    code.push_str(&format!(
        "    println(\"suite    {}\");\n",
        escape_arden_string_literal(&suite.name)
    ));
    code.push_str("    println(\"----------------------------------------\");\n\n");

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        if test.ignored {
            // Report ignore inline
            code.push_str(&format!("    // @Test: {}\n", test.name));
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!(
                "    println(\"  [SKIP] {}  [ignored]\");\n",
                escape_arden_string_literal(&test.name)
            ));
            if let Some(reason) = test
                .ignore_reason
                .as_ref()
                .filter(|reason| !reason.is_empty())
            {
                code.push_str(&format!(
                    "    println(\"           reason: {}\");\n",
                    escape_arden_string_literal(reason)
                ));
            }
        } else {
            // BeforeEach
            if let Some(ref before_each_fn) = suite.before_each {
                code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
                code.push_str(&format!("    {}();\n", before_each_fn.name));
            }

            // Test itself
            code.push_str(&format!("    // @Test: {}\n", test.name));
            // Run the test
            code.push_str(&format!(
                "    print(\"  [TEST] {} ... \");\n",
                escape_arden_string_literal(&test.name)
            ));
            code.push_str(&format!("    {}();\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");

            // AfterEach
            if let Some(ref after_each_fn) = suite.after_each {
                code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
                code.push_str(&format!("    {}();\n", after_each_fn.name));
                code.push_str("    println(\"\");\n\n");
            }
        }
        code.push_str("    println(\"\");\n");
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
    code.push_str("    println(\"\");\n");
    code.push_str(&format!(
        "    println(\"suite    {}\");\n",
        escape_arden_string_literal(&suite.name)
    ));
    code.push_str("    println(\"----------------------------------------\");\n\n");

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
        code.push_str("    println(\"\");\n\n");
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        if test.ignored {
            // Report ignore inline
            code.push_str(&format!("    // @Test: {}\n", test.name));
            code.push_str("    tests_ignored = tests_ignored + 1;\n");
            code.push_str(&format!(
                "    println(\"  [SKIP] {}  [ignored]\");\n",
                escape_arden_string_literal(&test.name)
            ));
            if let Some(reason) = test
                .ignore_reason
                .as_ref()
                .filter(|reason| !reason.is_empty())
            {
                code.push_str(&format!(
                    "    println(\"           reason: {}\");\n",
                    escape_arden_string_literal(reason)
                ));
            }
        } else {
            // BeforeEach
            if let Some(ref before_each_fn) = suite.before_each {
                code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
                code.push_str(&format!("    {}();\n", before_each_fn.name));
            }

            // Test itself
            code.push_str(&format!("    // @Test: {}\n", test.name));
            // Run the test
            code.push_str(&format!(
                "    print(\"  [TEST] {} ... \");\n",
                escape_arden_string_literal(&test.name)
            ));
            code.push_str(&format!("    {}();\n", test.name));
            code.push_str("    tests_passed = tests_passed + 1;\n");
            code.push_str("    println(\"[PASS]\");\n");

            // AfterEach
            if let Some(ref after_each_fn) = suite.after_each {
                code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
                code.push_str(&format!("    {}();\n", after_each_fn.name));
                code.push_str("    println(\"\");\n\n");
            }
        }
        code.push_str("    println(\"\");\n");
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
    println!(
        "{}",
        "+======================================+"
            .truecolor(217, 178, 158)
            .bold()
    );
    println!(
        "{}",
        "| Test Summary                         |"
            .truecolor(255, 255, 255)
            .bold()
    );
    println!(
        "{}",
        "+======================================+"
            .truecolor(217, 178, 158)
            .bold()
    );

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
