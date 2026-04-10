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
pub struct Test {
    pub name: String,
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

/// Discover all tests in a program
pub fn discover_tests(program: &Program) -> TestDiscovery {
    struct SuiteState {
        tests: Vec<Test>,
        before_all: Option<FunctionDecl>,
        before_each: Option<FunctionDecl>,
        after_each: Option<FunctionDecl>,
        after_all: Option<FunctionDecl>,
    }

    impl SuiteState {
        fn new() -> Self {
            Self {
                tests: Vec::new(),
                before_all: None,
                before_each: None,
                after_each: None,
                after_all: None,
            }
        }
    }

    let mut suites = Vec::new();
    let mut total_tests = 0;
    let mut ignored_tests = 0;

    fn collect_function_into_suite(
        func: &FunctionDecl,
        qualified_name: String,
        state: &mut SuiteState,
        total_tests: &mut usize,
        ignored_tests: &mut usize,
    ) {
        let mut qualified_func = func.clone();
        qualified_func.name = qualified_name.clone();

        if has_attribute(&func.attributes, Attribute::BeforeAll) {
            state.before_all = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::Before) {
            state.before_each = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::After) {
            state.after_each = Some(qualified_func);
            return;
        }
        if has_attribute(&func.attributes, Attribute::AfterAll) {
            state.after_all = Some(qualified_func);
            return;
        }

        if has_attribute(&func.attributes, Attribute::Test) {
            let ignored = has_ignore_attribute(&func.attributes);
            let ignore_reason = get_ignore_reason(&func.attributes);

            state.tests.push(Test {
                name: qualified_name,
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
        let mut state = SuiteState::new();

        for decl in declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let qualified_name = format!("{}__{}", suite_name, func.name);
                    collect_function_into_suite(
                        func,
                        qualified_name,
                        &mut state,
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

        if !state.tests.is_empty() {
            suites.push(TestSuite {
                name: suite_name,
                tests: state.tests,
                before_all: state.before_all,
                before_each: state.before_each,
                after_each: state.after_each,
                after_all: state.after_all,
            });
        }
    }

    let mut default_suite = SuiteState::new();

    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                collect_function_into_suite(
                    func,
                    func.name.clone(),
                    &mut default_suite,
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

    if !default_suite.tests.is_empty() {
        suites.insert(
            0,
            TestSuite {
                name: "default".to_string(),
                tests: default_suite.tests,
                before_all: default_suite.before_all,
                before_each: default_suite.before_each,
                after_each: default_suite.after_each,
                after_all: default_suite.after_all,
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
#[cfg(test)]
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

    for suite in &discovery.suites {
        generate_suite_runner(&mut code, suite);
    }

    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        exit(1);\n"); // Use exit directly
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

    for suite in &discovery.suites {
        generate_suite_runner_with_mut(&mut code, suite);
    }

    code.push_str("    if (tests_failed > 0) {\n");
    code.push_str("        exit(1);\n");
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

/// Generate runner code with mutable counters
fn generate_suite_runner_with_mut(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        if test.ignored {
            // Report ignore inline
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
            // BeforeEach
            if let Some(ref before_each_fn) = suite.before_each {
                code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
                code.push_str(&format!("    {}();\n", before_each_fn.name));
            }

            // Test itself
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

            // AfterEach
            if let Some(ref after_each_fn) = suite.after_each {
                code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
                code.push_str(&format!("    {}();\n", after_each_fn.name));
            }
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {}();\n", after_all_fn.name));
    }
}

/// Generate runner code for a single test suite
#[cfg(test)]
fn generate_suite_runner(code: &mut String, suite: &TestSuite) {
    code.push_str(&format!("    // Test Suite: {}\n", suite.name));

    // BeforeAll
    if let Some(ref before_all_fn) = suite.before_all {
        code.push_str(&format!("    // @BeforeAll: {}\n", before_all_fn.name));
        code.push_str(&format!("    {}();\n", before_all_fn.name));
    }

    // Each test
    for test in &suite.tests {
        code.push_str("    tests_total = tests_total + 1;\n");

        if test.ignored {
            // Report ignore inline
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
            // BeforeEach
            if let Some(ref before_each_fn) = suite.before_each {
                code.push_str(&format!("    // @Before: {}\n", before_each_fn.name));
                code.push_str(&format!("    {}();\n", before_each_fn.name));
            }

            // Test itself
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

            // AfterEach
            if let Some(ref after_each_fn) = suite.after_each {
                code.push_str(&format!("    // @After: {}\n", after_each_fn.name));
                code.push_str(&format!("    {}();\n", after_each_fn.name));
            }
        }
    }

    // AfterAll
    if let Some(ref after_all_fn) = suite.after_all {
        code.push_str(&format!("    // @AfterAll: {}\n", after_all_fn.name));
        code.push_str(&format!("    {}();\n", after_all_fn.name));
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

#[cfg(test)]
#[path = "../tests/test_runner.rs"]
mod tests;
