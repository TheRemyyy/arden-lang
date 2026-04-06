use super::*;
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::stdlib::stdlib_registry;

fn check_import_errors(source: &str) -> Vec<ImportError> {
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let namespace = program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string());
    let imports = program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            Decl::Import(i) => Some(i.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let function_namespaces = extract_function_namespaces(&program, &namespace);
    let known_namespace_paths = extract_known_namespace_paths(&program, &namespace);
    let mut checker = ImportChecker::new(
        Arc::new(function_namespaces),
        Arc::new(known_namespace_paths),
        namespace,
        imports,
        stdlib_registry(),
    );

    checker.check_program(&program).err().unwrap_or_default()
}

#[test]
fn module_local_namespace_alias_does_not_leak_to_top_level_import_check() {
    let source = r#"
module Inner {
import std.math as math;

function keep(): Float {
    return math.abs(-1.0);
}
}

function main(): Float {
return math.abs(-1.0);
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].function_name, "math.abs");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn local_function_can_shadow_stdlib_name() {
    let source = r#"
function print(owned s: String): None { return None; }
function main(): None {
s: String = "x";
print(s);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty());
}

#[test]
fn module_dot_stdlib_call_requires_import() {
    let source = r#"
function main(): None {
x: Float = Math.abs(-1.0);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn alias_import_allows_namespaced_stdlib_calls() {
    let source = r#"
import std.io as io;
import std.math as math;
import std.string as str;

function main(): None {
io.println("x");
y: Integer = math.abs(-2);
z: Integer = str.len("ok");
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty());
}

#[test]
fn dotted_module_alias_allows_module_style_calls() {
    let source = r#"
package lib;
import lib.A.X as ax;

module A {
module X {
    function f(): Integer { return 1; }
}
}

function main(): None {
x: Integer = ax.f();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn alias_call_still_checks_nested_argument_calls() {
    let source = r#"
import std.io as io;
function main(): None {
io.println(to_string(Math.abs(-3)));
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn invalid_namespace_alias_reports_import_error_on_use() {
    let source = r#"
import does_not_exist as dne;
function main(): None {
dne.print("x");
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "dne.print");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_dotted_namespace_alias_reports_import_error_on_use() {
    let source = r#"
import nope.ns as n;
function main(): None {
n.call();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "n.call");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_constructor_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(): None {
alias.Box();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Box");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_type_annotation_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(value: alias.Box): None {
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Box");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_interface_extends_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
interface Printable extends alias.Named {
function print_me(): Integer;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn stale_exact_imported_interface_alias_in_implements_reports_unresolved_import_alias() {
    let source = r#"
package app;
module M { interface Labelled { function name(): Integer; } }
import app.M.Named as Named;
class Book implements Named {
constructor() {}
function name(): Integer { return 1; }
}
function main(): Integer { return 0; }
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unresolved import alias>");
}

#[test]
fn invalid_namespace_alias_inside_constructor_type_args_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(): Integer {
return List<alias.Box>().length();
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].function_name, "alias.Box");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_pattern_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(value: Integer): None {
match (value) {
    alias.Result.Ok(inner) => { return None; },
    _ => { return None; }
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Result.Ok");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_match_expression_pattern_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(value: Integer): Integer {
return match (value) {
    alias.Result.Ok(inner) => { inner; },
    _ => { 0; }
};
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Result.Ok");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_assignment_value_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
function main(): None {
value: Integer = 0;
value = alias.Box();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.Box");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_assignment_target_reports_import_error_on_use() {
    let source = r#"
import nope.missing as alias;
class Holder {
value: Integer;
}

function main(): None {
holder: Holder = Holder(0);
alias.store().value = 1;
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias.store");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_call_type_arg_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Box as Boxed;
function consume<T>(): None { return None; }
function main(): None {
consume<Boxed>();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Boxed");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_lambda_param_type_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Box as Boxed;
function main(): None {
mapper: (Integer) -> Integer = (value: Boxed) => 1;
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Boxed");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_function_generic_bound_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Named as Named;
function render<T extends Named>(value: T): None {
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_class_generic_bound_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Named as Named;
class Box<T extends Named> {
value: Integer;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_method_generic_bound_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Named as Named;
class Worker {
function render<T extends Named>(value: T): None {
    return None;
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_enum_generic_bound_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Named as Named;
enum Result<T extends Named> {
Ok(value: T)
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_interface_generic_bound_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Named as Named;
interface Renderable<T extends Named> {
function render(value: T): None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Named");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_exact_type_alias_annotation_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Box as Boxed;
function main(value: Boxed): None {
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Boxed");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_exact_type_alias_constructor_reports_import_error_on_use() {
    let source = r#"
import nope.missing.Box as Boxed;
function main(): None {
Boxed();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Boxed");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn exact_imported_enum_alias_allows_variant_calls() {
    let source = r#"
package app;
import util.E as Enum;

function main(): None {
Enum.A(1);
return None;
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let imports = program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            Decl::Import(i) => Some(i.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut checker = ImportChecker::new(
        Arc::new(HashMap::new()),
        Arc::new(HashSet::from([
            "util".to_string(),
            "util.E".to_string(),
            "util.E.A".to_string(),
        ])),
        "app".to_string(),
        imports,
        stdlib_registry(),
    );
    let errors = checker.check_program(&program).err().unwrap_or_default();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn invalid_namespace_alias_format_message_is_actionable() {
    let err = ImportError {
        function_name: "dne.print".to_string(),
        defined_in: "<unknown namespace alias>".to_string(),
        used_in: "app".to_string(),
        span: 0..0,
        suggestion: None,
    };

    let rendered = err.format();
    assert!(rendered.contains("Unknown namespace alias usage 'dne.print'"));
    assert!(rendered.contains("import <namespace> as <alias>;"));
    assert!(!rendered.contains("<unknown namespace alias>.dne.print"));
}

#[test]
fn if_expression_condition_checks_missing_imports() {
    let source = r#"
function main(): None {
x: Integer = if (Math.abs(-1.0) > 0.0) { 1; } else { 2; };
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn if_expression_branch_checks_missing_imports() {
    let source = r#"
function main(): None {
x: Float = if (true) { Math.abs(-1.0); } else { 0.0; };
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn require_expression_checks_missing_imports() {
    let source = r#"
function main(): None {
require(Math.abs(-1.0) > 0.0, "x");
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn async_block_checks_missing_imports() {
    let source = r#"
function main(): None {
t: Task<Integer> = async { return Math.abs(-1); };
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn class_method_checks_missing_imports() {
    let source = r#"
class C {
function compute(): Float {
    return Math.abs(-1.0);
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn constructor_checks_missing_imports() {
    let source = r#"
class C {
constructor() {
    x: Float = Math.abs(-2.0);
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn module_function_checks_missing_imports() {
    let source = r#"
module Utils {
function f(): Float {
    return Math.abs(-3.0);
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn interface_default_impl_checks_missing_imports() {
    let source = r#"
interface I {
function f(): Float {
    return Math.abs(-4.0);
}
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn extracts_module_functions_as_mangled_namespaces() {
    let source = r#"
module MathEx {
function addOne(x: Integer): Integer { return x + 1; }
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let map = extract_function_namespaces(&program, "demo");
    assert!(map.contains_key("MathEx__addOne"));
    assert!(!map.contains_key("addOne"));
}

#[test]
fn extracts_nested_module_functions_as_deep_mangled_namespaces() {
    let source = r#"
module Outer {
module Inner {
    function ping(): Integer { return 1; }
}
}
"#;
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let map = extract_function_namespaces(&program, "demo");
    assert!(map.contains_key("Outer__Inner__ping"));
    assert!(!map.contains_key("Inner__ping"));
}

#[test]
fn alias_namespace_does_not_import_direct_mangled_stdlib_calls() {
    let source = r#"
import std.math as math;
function main(): None {
x: Float = Math__abs(-1.0);
y: Float = math.abs(-2.0);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn alias_namespace_does_not_import_module_style_symbol_without_alias() {
    let source = r#"
import std.math as math;
function main(): None {
x: Float = Math.abs(-1.0);
y: Float = math.abs(-2.0);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "Math__abs");
}

#[test]
fn invalid_namespace_alias_direct_call_is_reported_at_import_check_time() {
    let source = r#"
import nope.missing as alias;
function main(): None {
alias();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].function_name, "alias");
    assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
}

#[test]
fn invalid_namespace_alias_direct_call_format_message_is_actionable() {
    let source = r#"
import nope.missing as alias;
function main(): None {
alias();
return None;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1);
    let rendered = errors[0].format();
    assert!(rendered.contains("Unknown namespace alias usage 'alias'"));
    assert!(rendered.contains("import <namespace> as <alias>;"));
    assert!(!rendered.contains("<invalid import alias>"));
}

#[test]
fn local_lambda_binding_call_does_not_require_import() {
    let source = r#"
function main(): None {
f: (Integer) -> Integer = (x: Integer) => x + 1;
value: Integer = f(2);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn function_parameter_call_does_not_require_import() {
    let source = r#"
function apply(f: (Integer) -> Integer, value: Integer): Integer {
return f(value);
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn lambda_parameter_call_does_not_require_import() {
    let source = r#"
function main(): None {
f: ((Integer) -> Integer) -> Integer = (g: (Integer) -> Integer) => g(1);
value: Integer = f((x: Integer) => x + 1);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn async_block_local_lambda_call_does_not_require_import() {
    let source = r#"
function main(): None {
task: Task<Integer> = async {
    f: (Integer) -> Integer = (x: Integer) => x + 1;
    return f(2);
};
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn exact_imported_stdlib_function_alias_call_is_treated_as_imported() {
    let source = r#"
import std.math.Math__abs as abs_fn;

function main(): None {
value: Float = abs_fn(-1.0);
return None;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn exact_imported_user_function_alias_call_is_treated_as_imported() {
    let source = r#"
package app;
import util.math.add_one as inc;

function main(): Integer {
return inc(1);
}
"#;
    let function_namespaces = Arc::new(HashMap::from([(
        "add_one".to_string(),
        "util.math".to_string(),
    )]));
    let known_namespace_paths = Arc::new(HashSet::from(["util.math".to_string()]));
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let imports = program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            Decl::Import(i) => Some(i.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut checker = ImportChecker::new(
        function_namespaces,
        known_namespace_paths,
        "app".to_string(),
        imports,
        stdlib_registry(),
    );

    let errors = checker.check_program(&program).err().unwrap_or_default();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn exact_imported_enum_variant_alias_pattern_is_not_flagged_as_unknown_namespace_alias() {
    let source = r#"
enum E { A(Integer), B(Integer) }
import E.A as First;
function main(): Integer {
return match (E.B(2)) {
    First(v) => v,
    E.B(v) => v + 1
};
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn invalid_namespace_alias_nested_enum_variant_reports_unresolved_member() {
    let source = r#"
package app;
import app as root;
module M { enum E { B(Integer) } }
function main(): Integer {
    return match (root.M.E.A(2)) {
    root.M.E.A(v) => v,
    root.M.E.B(v) => v + 1
};
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors
            .iter()
            .all(|error| error.defined_in == "<unresolved namespace alias member>"),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| error.function_name == "root.M.E.A"),
        "{errors:?}"
    );
}

#[test]
fn stale_exact_imported_nested_enum_variant_alias_reports_unresolved_import_alias() {
    let source = r#"
package app;
module M { enum E { B(Integer) } }
import app.M.E.A as First;
function main(): Integer {
return match (M.E.B(2)) {
    First(v) => v,
    M.E.B(v) => v + 1
};
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].function_name, "First");
    assert_eq!(errors[0].defined_in, "<unresolved import alias>");
}

#[test]
fn stale_exact_imported_nested_enum_alias_type_reports_unresolved_import_alias() {
    let source = r#"
package app;
module M { enum F { A(Integer), B(Integer) } }
import app.M.E as Enum;
function main(): Integer {
value: Enum = Enum.B(2);
    return 0;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors.iter().all(|error| error.function_name == "Enum"),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .all(|error| error.defined_in == "<unresolved import alias>"),
        "{errors:?}"
    );
}

#[test]
fn stale_wildcard_imported_nested_enum_type_reports_unresolved_wildcard_import() {
    let source = r#"
package app;
module M { enum F { A(Integer), B(Integer) } }
import app.M.*;
function main(): Integer {
value: E = E.B(2);
return 0;
}
"#;
    let errors = check_import_errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].function_name, "E");
    assert_eq!(errors[0].defined_in, "<unresolved wildcard import>");
}

#[test]
fn exact_imported_top_level_type_alias_is_not_flagged_as_unknown_namespace_alias() {
    let source = r#"
class Box {
value: Integer;
constructor(value: Integer) { this.value = value; }
}
import Box as B;
function main(): Integer {
b: B = B(2);
return b.value;
}
"#;
    let errors = check_import_errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}
