use super::TestExpectExt;
use crate::lint::lint_source;

#[test]
fn detects_duplicate_and_unsorted_imports() {
    let source = r#"import std.string.*;
import std.io.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert_eq!(result.findings.len(), 2);
    assert!(result.findings.iter().any(|f| f.code == "L001"));
    assert!(result.findings.iter().any(|f| f.code == "L002"));
}

#[test]
fn fixes_import_order_and_dedupes() {
    let source = r#"import std.string.*;
import std.io.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(fixed.starts_with("import std.io.*;\nimport std.string.*;"));
    assert_eq!(fixed.matches("import std.io.*;").count(), 1);
}

#[test]
fn fixes_imports_when_source_uses_lone_cr_line_endings() {
    let source =
        "import std.string.*;\rimport std.io.*;\rimport std.io.*;\r\rfunction main(): None {\rreturn None;\r}\r";
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with("import std.io.*;\nimport std.string.*;\n\n"),
        "{fixed}"
    );
    assert_eq!(fixed.matches("import std.io.*;").count(), 1, "{fixed}");
}

#[test]
fn flags_unused_specific_imports() {
    let source = r#"import project.helper;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L003"));
}

#[test]
fn flags_unused_variables() {
    let source = r#"function main(): None {
used: Integer = 1;
unused: Integer = 2;
_ignored: Integer = 3;
used = used + 1;
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    let unused_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.code == "L004")
        .collect();
    assert_eq!(unused_findings.len(), 1);
    assert!(unused_findings[0]
        .message
        .contains("Variable 'unused' is declared but never used"));
}

#[test]
fn flags_shadowed_variables() {
    let source = r#"function main(): None {
x: Integer = 1;
if (true) {
    x: Integer = 2;
    x = x + 1;
}
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result
        .findings
        .iter()
        .any(|f| f.code == "L005" && f.message.contains("Variable 'x' shadows an outer variable")));
}

#[test]
fn alias_imports_are_not_false_duplicate() {
    let source = r#"import std.io as io;
import std.io as io2;

function main(): None {
io.println("a");
io2.println("b");
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| f.code == "L001"));
}

#[test]
fn fix_keeps_import_with_trailing_comment() {
    let source = r#"import std.string.*; // needed for Str.len
import std.io.*;

function main(): None {
println(to_string(Str.len("abc")));
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(fixed.contains("import std.string.*;"));
    assert!(fixed.contains("import std.io.*;"));
}

#[test]
fn fix_keeps_import_with_trailing_block_comment() {
    let source = r#"import std.string.*; /* needed for Str.len */
import std.io.*;

function main(): None {
println(to_string(Str.len("abc")));
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(fixed.contains("import std.string.*;"));
    assert!(fixed.contains("import std.io.*;"));
}

#[test]
fn fix_preserves_shebang_line() {
    let source = r#"#!/usr/bin/env arden
import std.string.*;
import std.io.*;
function main(): None { return None; }
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(fixed.starts_with("#!/usr/bin/env arden\n"));
}

#[test]
fn flags_unused_stdlib_specific_imports() {
    let source = r#"import std.math.abs;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("specific import 'std.math.abs' appears unused")
    }));
}

#[test]
fn flags_shadowing_function_parameter() {
    let source = r#"function main(x: Integer): None {
x: Integer = 1;
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result
        .findings
        .iter()
        .any(|f| f.code == "L005" && f.message.contains("Variable 'x' shadows an outer variable")));
}

#[test]
fn flags_shadowing_for_loop_variable() {
    let source = r#"function main(): None {
i: Integer = 10;
for (i in range(0, 3)) {
    println(to_string(i));
}
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result
        .findings
        .iter()
        .any(|f| f.code == "L005" && f.message.contains("Variable 'i' shadows an outer variable")));
}

#[test]
fn flags_unused_for_loop_variable() {
    let source = r#"function main(): None {
for (i in range(0, 3)) {
    println("x");
}
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L004"
        && f.message
            .contains("Variable 'i' is declared but never used")));
}

#[test]
fn flags_unused_shadowed_variable_in_nested_block() {
    let source = r#"function main(): None {
x: Integer = 1;
if (true) {
    x: Integer = 2;
}
println(to_string(x));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    let unused = result
        .findings
        .iter()
        .filter(|f| f.code == "L004" && f.message.contains("Variable 'x'"))
        .collect::<Vec<_>>();
    assert_eq!(unused.len(), 1, "{:?}", result.findings);
}

#[test]
fn flags_unused_shadowed_lambda_parameter_independently() {
    let source = r#"function main(): None {
value: Integer = 1;
f: (Integer) -> Integer = (value: Integer) => 0;
println(to_string(value));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(
        result.findings.iter().any(|f| {
            f.code == "L004"
                && f.message
                    .contains("Variable 'value' is declared but never used")
        }),
        "{:?}",
        result.findings
    );
}

#[test]
fn does_not_flag_used_aliased_specific_import() {
    let source = r#"import std.math.Math__abs as abs_fn;

function main(): None {
x: Float = abs_fn(-1.0);
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("specific import 'std.math.Math__abs as abs_fn' appears unused")
    }));
}

#[test]
fn flags_unused_aliased_specific_import() {
    let source = r#"import std.math.Math__abs as abs_fn;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("specific import 'std.math.Math__abs as abs_fn' appears unused")
    }));
}

#[test]
fn flags_unused_module_local_specific_imports() {
    let source = r#"module Inner {
import util.helper as helper;

function main(): None {
return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("specific import 'util.helper as helper' appears unused")
    }));
}

#[test]
fn flags_duplicate_module_local_imports_within_same_scope() {
    let source = r#"module Inner {
import util.helper;
import util.helper;

function main(): None {
return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result
        .findings
        .iter()
        .any(|f| { f.code == "L001" && f.message.contains("duplicate import 'util.helper'") }));
}

#[test]
fn fix_does_not_hoist_imports_out_of_block_comments() {
    let source = r#"/*
import evil.pkg;
*/
import std.io.*;

function main(): None {
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(fixed.contains("/*\nimport evil.pkg;\n*/"), "{fixed}");
    assert_eq!(fixed.matches("import std.io.*;").count(), 1, "{fixed}");
}

#[test]
fn fix_does_not_hoist_module_local_imports() {
    let source = r#"package demo;
import std.string.*;
import std.io.*;

module Inner {
import util.helper;

function load(): None {
println("ok");
return None;
}
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with(
            "package demo;\n\nimport std.io.*;\nimport std.string.*;\n\nmodule Inner {"
        ),
        "{fixed}"
    );
    assert_eq!(fixed.matches("import util.helper;").count(), 1, "{fixed}");
    assert!(
        fixed.contains("module Inner {\nimport util.helper;\n\nfunction load(): None {"),
        "{fixed}"
    );
}

#[test]
fn fix_preserves_line_comments_before_package() {
    let source = r#"// generated file
package demo;
import std.string.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with(
            "// generated file\npackage demo;\n\nimport std.io.*;\nimport std.string.*;"
        ),
        "{fixed}"
    );
}

#[test]
fn fix_preserves_block_comments_before_package() {
    let source = r#"/*
 * generated file
 */
package demo;
import std.string.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with(
            "/*\n * generated file\n */\npackage demo;\n\nimport std.io.*;\nimport std.string.*;"
        ),
        "{fixed}"
    );
}

#[test]
fn fix_preserves_comments_between_package_and_imports() {
    let source = r#"package demo;
// stdlib imports
import std.string.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with(
            "package demo;\n\n// stdlib imports\nimport std.io.*;\nimport std.string.*;"
        ),
        "{fixed}"
    );
}

#[test]
fn fix_preserves_shebang_and_header_comment_order() {
    let source = r#"#!/usr/bin/env arden
// generated file
import std.string.*;
import std.io.*;

function main(): None {
println("ok");
return None;
}
"#;
    let result = lint_source(source, true).must("lint succeeds");
    let fixed = result.fixed_source.must("fixed source");
    assert!(
        fixed.starts_with(
            "#!/usr/bin/env arden\n// generated file\nimport std.io.*;\nimport std.string.*;"
        ),
        "{fixed}"
    );
}

#[test]
fn exact_class_alias_usage_in_call_type_args_marks_import_as_used() {
    let source = r#"import util.Box as Boxed;

function main(): None {
List<Boxed>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Box as Boxed' appears unused")
    }));
}

#[test]
fn namespace_alias_usage_in_call_type_args_marks_import_as_used() {
    let source = r#"import util as u;

function main(): None {
List<u.Box>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'util as u' appears unused") }));
}

#[test]
fn nested_exact_class_alias_usage_in_call_type_args_marks_import_as_used() {
    let source = r#"import app.M.Box as Boxed;

function main(): None {
List<Boxed>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'app.M.Box as Boxed' appears unused")
    }));
}

#[test]
fn nested_namespace_alias_usage_in_call_type_args_marks_import_as_used() {
    let source = r#"import app as u;

function main(): None {
List<u.M.Box>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn exact_enum_alias_usage_in_call_type_args_marks_import_as_used() {
    let source = r#"import app.Result as Res;

function main(): None {
List<Res>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'app.Result as Res' appears unused")
    }));
}

#[test]
fn namespace_alias_usage_in_construct_type_marks_import_as_used() {
    let source = r#"import util as u;

function main(): None {
u.Box(1);
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'util as u' appears unused") }));
}

#[test]
fn nested_namespace_alias_usage_in_construct_type_marks_import_as_used() {
    let source = r#"import app as u;

function main(): None {
u.M.Box(1);
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn namespace_alias_usage_in_generic_construct_type_marks_import_as_used() {
    let source = r#"import util as u;

function main(): None {
List<u.Box>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'util as u' appears unused") }));
}

#[test]
fn nested_namespace_alias_usage_in_generic_construct_type_marks_import_as_used() {
    let source = r#"import app as u;

function main(): None {
List<u.M.Box>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn exact_class_alias_usage_in_generic_construct_type_marks_import_as_used() {
    let source = r#"import util.Box as Boxed;

function main(): None {
List<Boxed>();
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Box as Boxed' appears unused")
    }));
}

#[test]
fn specific_import_usage_in_interface_default_impl_marks_import_as_used() {
    let source = r#"import util.helper;

interface Runner {
function run(): Integer {
    return helper();
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003" && f.message.contains("import 'util.helper' appears unused")
    }));
}

#[test]
fn namespace_alias_usage_in_interface_default_impl_marks_import_as_used() {
    let source = r#"import std.io as io;

interface Runner {
function run(): None {
    io.println("ok");
    return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003" && f.message.contains("import 'std.io as io' appears unused")
    }));
}

#[test]
fn type_position_alias_usage_marks_import_as_used() {
    let source = r#"import util as u;

function main(value: u.Box): None {
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'util as u' appears unused") }));
}

#[test]
fn exact_variant_alias_usage_in_match_statement_marks_import_as_used() {
    let source = r#"import app.Option.None as Empty;

function main(value: Option<Integer>): None {
match (value) {
    Empty => { return None; }
    _ => { return None; }
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Option.None as Empty") }));
}

#[test]
fn exact_variant_alias_usage_in_match_expression_marks_import_as_used() {
    let source = r#"import app.Option.None as Empty;

function main(value: Option<Integer>): Integer {
return match (value) {
    Empty => 0,
    _ => 1,
};
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Option.None as Empty") }));
}

#[test]
fn namespace_alias_usage_in_match_statement_pattern_marks_import_as_used() {
    let source = r#"import app as core;

function main(value: Result<Integer, String>): None {
match (value) {
    core.Result.Ok(inner) => { println(to_string(inner)); return None; }
    _ => { return None; }
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| f.code == "L003" && f.message.contains("import 'app as core' appears unused")));
}

#[test]
fn namespace_alias_usage_in_match_expression_pattern_marks_import_as_used() {
    let source = r#"import app as core;

function main(value: Result<Integer, String>): Integer {
return match (value) {
    core.Result.Ok(inner) => inner,
    _ => 1,
};
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| f.code == "L003" && f.message.contains("import 'app as core' appears unused")));
}

#[test]
fn nested_enum_alias_usage_in_match_statement_pattern_marks_import_as_used() {
    let source = r#"import app.Result.Error as Failure;

function main(value: Result<Integer, String>): None {
match (value) {
    Failure(err) => { println(err); return None; }
    _ => { return None; }
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Result.Error as Failure") }));
}

#[test]
fn nested_enum_alias_usage_in_match_expression_pattern_marks_import_as_used() {
    let source = r#"import app.Result.Error as Failure;

function main(value: Result<Integer, String>): Integer {
return match (value) {
    Failure(err) => Str.len(err),
    _ => 0,
};
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Result.Error as Failure") }));
}

#[test]
fn exact_enum_alias_usage_in_match_statement_pattern_marks_import_as_used() {
    let source = r#"import app.Result as Res;

function main(value: Result<Integer, String>): None {
match (value) {
    Res.Ok(inner) => { println(to_string(inner)); return None; }
    _ => { return None; }
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Result as Res") }));
}

#[test]
fn exact_enum_alias_usage_in_match_expression_pattern_marks_import_as_used() {
    let source = r#"import app.Result as Res;

function main(value: Result<Integer, String>): Integer {
return match (value) {
    Res.Ok(inner) => inner,
    _ => 0,
};
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.Result as Res") }));
}

#[test]
fn nested_exact_enum_alias_usage_in_match_statement_pattern_marks_import_as_used() {
    let source = r#"import app.M.E as Enum;

function main(value: M.E): None {
match (value) {
    Enum.A(inner) => { println(to_string(inner)); return None; }
    _ => { return None; }
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.M.E as Enum") }));
}

#[test]
fn nested_exact_enum_alias_usage_in_match_expression_pattern_marks_import_as_used() {
    let source = r#"import app.M.E as Enum;

function main(value: M.E): Integer {
return match (value) {
    Enum.A(inner) => inner,
    _ => 0,
};
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("app.M.E as Enum") }));
}

#[test]
fn flags_unused_async_block_locals() {
    let source = r#"function main(): None {
task: Task<Integer> = async {
    temp: Integer = 1;
    return 2;
};
println(to_string(await task));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'temp' is declared but never used")
    }));
}

#[test]
fn flags_unused_if_expression_branch_locals() {
    let source = r#"function main(): None {
value: Integer = if (true) {
    then_only: Integer = 1;
    2
} else {
    else_only: Integer = 3;
    4
};
println(to_string(value));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'then_only' is declared but never used")
    }));
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'else_only' is declared but never used")
    }));
}

#[test]
fn flags_unused_match_expression_pattern_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

function main(value: Maybe): None {
result: Integer = match (value) {
    Some(inner) => 1,
    Empty => 0,
};
println(to_string(result));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'inner' is declared but never used")
    }));
}

#[test]
fn flags_unused_match_statement_pattern_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

function main(value: Maybe): None {
match (value) {
    Some(inner) => { println("x"); }
    Empty => { println("y"); }
}
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'inner' is declared but never used")
    }));
}

#[test]
fn flags_unused_lambda_parameters() {
    let source = r#"function main(): None {
f: (Integer) -> Integer = (x: Integer) => 1;
println(to_string(f(2)));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L004"
        && f.message
            .contains("Variable 'x' is declared but never used")));
}

#[test]
fn flags_shadowing_inside_async_block() {
    let source = r#"function main(): None {
value: Integer = 1;
task: Task<Integer> = async {
    value: Integer = 2;
    return value;
};
println(to_string(await task));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L005"
        && f.message
            .contains("Variable 'value' shadows an outer variable")));
}

#[test]
fn flags_shadowing_inside_if_expression_branch() {
    let source = r#"function main(): None {
value: Integer = 1;
result: Integer = if (true) {
    value: Integer = 2;
    value
} else {
    0
};
println(to_string(result));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L005"
        && f.message
            .contains("Variable 'value' shadows an outer variable")));
}

#[test]
fn flags_shadowing_match_statement_pattern_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

function main(value: Maybe): None {
inner: Integer = 1;
match (value) {
    Some(inner) => { println(to_string(inner)); }
    Empty => { println("none"); }
}
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L005"
        && f.message
            .contains("Variable 'inner' shadows an outer variable")));
}

#[test]
fn flags_shadowing_match_expression_pattern_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

function main(value: Maybe): None {
inner: Integer = 1;
result: Integer = match (value) {
    Some(inner) => inner,
    Empty => 0,
};
println(to_string(result));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| f.code == "L005"
        && f.message
            .contains("Variable 'inner' shadows an outer variable")));
}

#[test]
fn flags_shadowing_lambda_parameters() {
    let source = r#"function main(): None {
x: Integer = 1;
f: (Integer) -> Integer = (x: Integer) => x + 1;
println(to_string(f(2)));
return None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result
        .findings
        .iter()
        .any(|f| f.code == "L005" && f.message.contains("Variable 'x' shadows an outer variable")));
}

#[test]
fn flags_unused_interface_default_impl_locals() {
    let source = r#"interface Runner {
function run(): Integer {
    temp: Integer = 1;
    return 0;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'temp' is declared but never used")
    }));
}

#[test]
fn flags_unused_interface_default_impl_loop_variables() {
    let source = r#"interface Runner {
function run(): None {
    for (item in range(0, 2)) {
        println("x");
    }
    return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'item' is declared but never used")
    }));
}

#[test]
fn flags_unused_interface_default_impl_lambda_parameters() {
    let source = r#"interface Runner {
function run(): Integer {
    f: (Integer) -> Integer = (x: Integer) => 1;
    return f(2);
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'x' is declared but never used")
    }));
}

#[test]
fn flags_unused_interface_default_impl_match_statement_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

interface Runner {
function run(value: Maybe): None {
    match (value) {
        Some(inner) => { println("x"); }
        Empty => { println("y"); }
    }
    return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'inner' is declared but never used")
    }));
}

#[test]
fn flags_unused_interface_default_impl_match_expression_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

interface Runner {
function run(value: Maybe): Integer {
    result: Integer = match (value) {
        Some(inner) => 1,
        Empty => 0,
    };
    return result;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L004"
            && f.message
                .contains("Variable 'inner' is declared but never used")
    }));
}

#[test]
fn flags_shadowing_interface_default_impl_parameters() {
    let source = r#"interface Runner {
function run(value: Integer): Integer {
    value: Integer = 1;
    return value;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L005"
            && f.message
                .contains("Variable 'value' shadows an outer variable")
    }));
}

#[test]
fn flags_shadowing_interface_default_impl_loop_variables() {
    let source = r#"interface Runner {
function run(): None {
    item: Integer = 1;
    for (item in range(0, 2)) {
        println(to_string(item));
    }
    return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L005"
            && f.message
                .contains("Variable 'item' shadows an outer variable")
    }));
}

#[test]
fn flags_shadowing_interface_default_impl_lambda_parameters() {
    let source = r#"interface Runner {
function run(): Integer {
    x: Integer = 1;
    f: (Integer) -> Integer = (x: Integer) => x + 1;
    return f(2);
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L005" && f.message.contains("Variable 'x' shadows an outer variable")
    }));
}

#[test]
fn flags_shadowing_interface_default_impl_match_statement_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

interface Runner {
function run(value: Maybe): None {
    inner: Integer = 1;
    match (value) {
        Some(inner) => { println(to_string(inner)); }
        Empty => { println("none"); }
    }
    return None;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L005"
            && f.message
                .contains("Variable 'inner' shadows an outer variable")
    }));
}

#[test]
fn flags_shadowing_interface_default_impl_match_expression_bindings() {
    let source = r#"enum Maybe { Some(Integer), Empty }

interface Runner {
function run(value: Maybe): Integer {
    inner: Integer = 1;
    result: Integer = match (value) {
        Some(inner) => inner,
        Empty => 0,
    };
    return result;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(result.findings.iter().any(|f| {
        f.code == "L005"
            && f.message
                .contains("Variable 'inner' shadows an outer variable")
    }));
}

#[test]
fn function_generic_bound_exact_alias_marks_import_as_used() {
    let source = r#"import util.Comparable as Cmp;

function sort<T extends Cmp>(value: T): T {
return value;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable as Cmp' appears unused")
    }));
}

#[test]
fn function_generic_bound_specific_import_marks_import_as_used() {
    let source = r#"import util.Comparable;

function sort<T extends Comparable>(value: T): T {
return value;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable' appears unused")
    }));
}

#[test]
fn class_generic_bound_exact_alias_marks_import_as_used() {
    let source = r#"import util.Comparable as Cmp;

class Box<T extends Cmp> {
value: T;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable as Cmp' appears unused")
    }));
}

#[test]
fn class_generic_bound_specific_import_marks_import_as_used() {
    let source = r#"import util.Comparable;

class Box<T extends Comparable> {
value: T;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable' appears unused")
    }));
}

#[test]
fn class_method_generic_bound_exact_alias_marks_import_as_used() {
    let source = r#"import util.Comparable as Cmp;

class Box {
function sort<T extends Cmp>(value: T): T {
    return value;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable as Cmp' appears unused")
    }));
}

#[test]
fn class_method_generic_bound_specific_import_marks_import_as_used() {
    let source = r#"import util.Comparable;

class Box {
function sort<T extends Comparable>(value: T): T {
    return value;
}
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable' appears unused")
    }));
}

#[test]
fn enum_generic_bound_exact_alias_marks_import_as_used() {
    let source = r#"import util.Comparable as Cmp;

enum Maybe<T extends Cmp> {
Some(T),
Empty
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable as Cmp' appears unused")
    }));
}

#[test]
fn enum_generic_bound_specific_import_marks_import_as_used() {
    let source = r#"import util.Comparable;

enum Maybe<T extends Comparable> {
Some(T),
Empty
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable' appears unused")
    }));
}

#[test]
fn interface_generic_bound_exact_alias_marks_import_as_used() {
    let source = r#"import util.Comparable as Cmp;

interface Sorter<T extends Cmp> {
function sort(value: T): T;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable as Cmp' appears unused")
    }));
}

#[test]
fn interface_generic_bound_specific_import_marks_import_as_used() {
    let source = r#"import util.Comparable;

interface Sorter<T extends Comparable> {
function sort(value: T): T;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result.findings.iter().any(|f| {
        f.code == "L003"
            && f.message
                .contains("import 'util.Comparable' appears unused")
    }));
}

#[test]
fn class_extends_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

class Child extends u.Base {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn class_extends_nested_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

class Child extends u.Models.Base {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn class_implements_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

class Child implements u.Serializable {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn class_implements_nested_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

class Child implements u.Api.Serializable {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn class_implements_multiple_namespace_aliases_marks_import_as_used() {
    let source = r#"import app as u;

class Child implements u.Serializable, u.Named {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn class_implements_multiple_nested_namespace_aliases_marks_import_as_used() {
    let source = r#"import app as u;

class Child implements u.Api.Serializable, u.Api.Named {
value: Integer;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn interface_extends_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

interface Child extends u.Base {
function run(): None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn interface_extends_nested_namespace_alias_marks_import_as_used() {
    let source = r#"import app as u;

interface Child extends u.Api.Base {
function run(): None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn interface_extends_multiple_namespace_aliases_marks_import_as_used() {
    let source = r#"import app as u;

interface Child extends u.Base, u.Named {
function run(): None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}

#[test]
fn interface_extends_multiple_nested_namespace_aliases_marks_import_as_used() {
    let source = r#"import app as u;

interface Child extends u.Api.Base, u.Api.Named {
function run(): None;
}
"#;
    let result = lint_source(source, false).must("lint succeeds");
    assert!(!result
        .findings
        .iter()
        .any(|f| { f.code == "L003" && f.message.contains("import 'app as u' appears unused") }));
}
