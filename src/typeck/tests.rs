use super::*;
use crate::lexer::tokenize;
use crate::parser::Parser;

fn check_source(source: &str) -> Result<(), Vec<TypeError>> {
    let tokens = tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

fn empty_interface() -> InterfaceInfo {
    InterfaceInfo {
        methods: HashMap::new(),
        generic_param_names: Vec::new(),
        generic_type_vars: Vec::new(),
        extends: Vec::new(),
        span: 0..0,
    }
}

#[test]
fn resolves_nested_namespace_aliased_function_type_source_inside_generic_container() {
    let mut checker = TypeChecker::new();
    checker
        .import_aliases
        .insert("root".to_string(), "app".to_string());
    checker
        .interfaces
        .insert("app__M__Api__Named".to_string(), empty_interface());

    let resolved = checker
        .resolve_type_source("List<(root.M.Api.Named) -> Integer>")
        .expect("type source should parse");

    assert_eq!(
        resolved,
        ResolvedType::List(Box::new(ResolvedType::Function(
            vec![ResolvedType::Class("app__M__Api__Named".to_string())],
            Box::new(ResolvedType::Integer),
        )))
    );
}

#[test]
fn parses_nested_namespace_aliased_function_type_string_inside_generic_container() {
    let mut checker = TypeChecker::new();
    checker
        .import_aliases
        .insert("root".to_string(), "app".to_string());
    checker
        .interfaces
        .insert("app__M__Api__Named".to_string(), empty_interface());

    let parsed = checker.parse_type_string("List<(root.M.Api.Named) -> Integer>");

    assert_eq!(
        parsed,
        ResolvedType::List(Box::new(ResolvedType::Function(
            vec![ResolvedType::Class("app__M__Api__Named".to_string())],
            Box::new(ResolvedType::Integer),
        )))
    );
}

#[test]
fn rejects_private_member_access_from_outside_class() {
    let src = r#"
        class Secret {
            private value: Integer;
            constructor(v: Integer) { this.value = v; }
            private function getV(): Integer { return this.value; }
        }
        function main(): Integer {
            s: Secret = Secret(1);
            x: Integer = s.value;
            y: Integer = s.getV();
            return x + y;
        }
    "#;
    let errors = check_source(src).expect_err("visibility violation should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("private"), "{joined}");
}

#[test]
fn rejects_private_class_construction_from_outside() {
    let src = r#"
        private class Secret {
            constructor() {}
        }
        function main(): None {
            s: Secret = Secret();
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("private class use should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Class 'Secret' is private"), "{joined}");
}

#[test]
fn rejects_private_class_in_function_signature() {
    let src = r#"
        private class Secret { constructor() {} }
        function take(s: Secret): None { return None; }
        function main(): None { return None; }
    "#;
    let errors = check_source(src).expect_err("private class in signature should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Class 'Secret' is private"), "{joined}");
}

#[test]
fn rejects_extending_private_class_from_outside() {
    let src = r#"
        private class Base { constructor() {} }
        class Child extends Base { constructor() {} }
        function main(): None { return None; }
    "#;
    let errors = check_source(src).expect_err("extending private base should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Class 'Base' is private"), "{joined}");
}

#[test]
fn rejects_private_class_in_interface_signature() {
    let src = r#"
        private class Secret { constructor() {} }
        interface I {
            function leak(s: Secret): None;
        }
        function main(): None { return None; }
    "#;
    let errors = check_source(src).expect_err("private class in interface signature should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Class 'Secret' is private"), "{joined}");
}

#[test]
fn supports_inherited_method_lookup() {
    let src = r#"
        class Base {
            public function greet(): Integer { return 7; }
        }
        class Child extends Base {
            constructor() {}
        }
        function main(): Integer {
            c: Child = Child();
            return c.greet();
        }
    "#;
    check_source(src).expect("inherited method should typecheck");
}

#[test]
fn supports_extending_namespace_aliased_module_class() {
    let src = r#"
        import Lib as u;
        module Lib {
            class Base {
                constructor() {}
                public function greet(): Integer { return 7; }
            }
        }
        class Child extends u.Base {
            constructor() {}
        }
        function main(): Integer {
            c: Child = Child();
            return c.greet();
        }
    "#;
    check_source(src).expect("aliased base class should typecheck");
}

#[test]
fn supports_extending_nested_namespace_aliased_module_class() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Models {
                class Base {
                    constructor() {}
                    public function greet(): Integer { return 7; }
                }
            }
        }
        class Child extends u.Models.Base {
            constructor() {}
        }
        function main(): Integer {
            c: Child = Child();
            return c.greet();
        }
    "#;
    check_source(src).expect("nested aliased base class should typecheck");
}

#[test]
fn supports_implementing_namespace_aliased_module_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Printable {
                function print_me(): Integer;
            }
        }
        class Book implements u.Printable {
            constructor() {}
            function print_me(): Integer { return 7; }
        }
        function main(): Integer {
            b: Book = Book();
            return b.print_me();
        }
    "#;
    check_source(src).expect("aliased interface should typecheck");
}

#[test]
fn supports_implementing_nested_namespace_aliased_module_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Printable {
                    function print_me(): Integer;
                }
            }
        }
        class Book implements u.Api.Printable {
            constructor() {}
            function print_me(): Integer { return 7; }
        }
        function main(): Integer {
            b: Book = Book();
            return b.print_me();
        }
    "#;
    check_source(src).expect("nested aliased interface should typecheck");
}

#[test]
fn supports_implementing_multiple_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Named {
                function name(): Integer;
            }
            interface Printable {
                function print_me(): Integer;
            }
        }
        class Book implements u.Named, u.Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            b: Book = Book();
            return b.name() + b.print_me();
        }
    "#;
    check_source(src).expect("multiple aliased interfaces should typecheck");
}

#[test]
fn supports_implementing_multiple_nested_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
        }
        class Book implements u.Api.Named, u.Api.Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            b: Book = Book();
            return b.name() + b.print_me();
        }
    "#;
    check_source(src).expect("multiple nested aliased interfaces should typecheck");
}

#[test]
fn supports_interface_extending_namespace_aliased_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Named {
                function name(): Integer;
            }
        }
        interface Printable extends u.Named {
            function print_me(): Integer;
        }
        class Report implements Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    check_source(src).expect("aliased parent interface should typecheck");
}

#[test]
fn supports_interface_extending_nested_namespace_aliased_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Named {
                    function name(): Integer;
                }
            }
        }
        interface Printable extends u.Api.Named {
            function print_me(): Integer;
        }
        class Report implements Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    check_source(src).expect("nested aliased parent interface should typecheck");
}

#[test]
fn supports_interface_extending_multiple_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Named {
                function name(): Integer;
            }
            interface Printable {
                function print_me(): Integer;
            }
        }
        interface Reportable extends u.Named, u.Printable {}
        class Report implements Reportable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    check_source(src).expect("multiple aliased parent interfaces should typecheck");
}

#[test]
fn supports_interface_extending_multiple_nested_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
        }
        interface Reportable extends u.Api.Named, u.Api.Printable {}
        class Report implements Reportable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    check_source(src).expect("multiple nested aliased parent interfaces should typecheck");
}

#[test]
fn enforces_interface_contracts() {
    let src = r#"
        interface Printable {
            function print_me(): None;
        }
        class Book implements Printable {
            constructor() {}
            function other(): None { return None; }
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("missing interface method should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("must implement interface method"),
        "{joined}"
    );
}

#[test]
fn supports_module_local_interface_implements() {
    let src = r#"
        module M {
            interface Named {
                function name(): Integer;
            }
            class Book implements Named {
                constructor() {}
                function name(): Integer { return 1; }
            }
        }
        function main(): Integer { return 0; }
    "#;
    check_source(src).expect("module-local interface implements should typecheck");
}

#[test]
fn supports_module_local_nested_interface_implements() {
    let src = r#"
        module M {
            module Api {
                interface Named {
                    function name(): Integer;
                }
            }
            class Book implements Api.Named {
                constructor() {}
                function name(): Integer { return 1; }
            }
        }
        function main(): Integer { return 0; }
    "#;
    check_source(src).expect("module-local nested interface implements should typecheck");
}

#[test]
fn supports_module_local_interface_extends() {
    let src = r#"
        module M {
            interface Named {
                function name(): Integer;
            }
            interface Printable extends Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
        }
        function main(): Integer { return 0; }
    "#;
    check_source(src).expect("module-local interface extends should typecheck");
}

#[test]
fn supports_module_local_nested_interface_extends() {
    let src = r#"
        module M {
            module Api {
                interface Named {
                    function name(): Integer;
                }
            }
            interface Printable extends Api.Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
        }
        function main(): Integer { return 0; }
    "#;
    check_source(src).expect("module-local nested interface extends should typecheck");
}

#[test]
fn rejects_unknown_function_generic_bound() {
    let src = r#"
        function render<T extends Missing>(value: T): None {
            return None;
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("unknown generic bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Function 'render' generic parameter 'T' extends unknown interface 'Missing'"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_non_interface_function_generic_bound() {
    let src = r#"
        class Secret { constructor() {} }
        function render<T extends Secret>(value: T): None {
            return None;
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("class generic bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Function 'render' generic parameter 'T' must use an interface bound, found 'Secret'"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_unknown_class_generic_bound() {
    let src = r#"
        class Box<T extends Missing> {
            value: Integer;
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("unknown class generic bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Class 'Box' generic parameter 'T' extends unknown interface 'Missing'"),
        "{joined}"
    );
}

#[test]
fn rejects_unknown_enum_generic_bound() {
    let src = r#"
        enum Maybe<T extends Missing> {
            Some(value: T),
            Empty
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("unknown enum generic bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Enum 'Maybe' generic parameter 'T' extends unknown interface 'Missing'"),
        "{joined}"
    );
}

#[test]
fn rejects_unknown_interface_generic_bound() {
    let src = r#"
        interface Renderable<T extends Missing> {
            function render(value: T): None;
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("unknown interface generic bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Interface 'Renderable' generic parameter 'T' extends unknown interface 'Missing'"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_explicit_function_type_arg_that_violates_interface_bound() {
    let src = r#"
        interface Named { function name(): Integer; }
        class Plain { constructor() {} }
        function render<T extends Named>(value: T): Integer {
            return 1;
        }
        function main(): Integer {
            return render<Plain>(Plain());
        }
    "#;
    let errors = check_source(src).expect_err("explicit generic arg violating bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Function 'render' type argument Plain does not satisfy bound(s) Named"),
        "{joined}"
    );
}

#[test]
fn rejects_inferred_function_arg_that_violates_interface_bound() {
    let src = r#"
        interface Named { function name(): Integer; }
        class Plain { constructor() {} }
        function render<T extends Named>(value: T): Integer {
            return 1;
        }
        function main(): Integer {
            return render(Plain());
        }
    "#;
    let errors = check_source(src).expect_err("inferred generic arg violating bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Argument type mismatch: expected ?T"),
        "{joined}"
    );
}

#[test]
fn allows_method_calls_through_generic_interface_bound() {
    let src = r#"
        interface Named { function name(): Integer; }
        class Person implements Named {
            constructor() {}
            function name(): Integer { return 7; }
        }
        function read_name<T extends Named>(value: T): Integer {
            return value.name();
        }
        function main(): Integer {
            return read_name(Person());
        }
    "#;
    check_source(src).expect("bounded generic interface method calls should typecheck");
}

#[test]
fn rejects_ambiguous_bounded_generic_method_signatures() {
    let src = r#"
        interface A { function render(): Integer; }
        interface B { function render(): String; }
        function read<T extends A, B>(value: T): Integer {
            return value.render();
        }
        function main(): Integer { return 0; }
    "#;
    let errors =
        check_source(src).expect_err("conflicting bounded generic method signatures should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined
            .contains("Generic bound method 'B.render' has incompatible signatures across bounds"),
        "{joined}"
    );
}

#[test]
fn rejects_interface_inheriting_conflicting_parent_method_signatures() {
    let src = r#"
        interface A { function render(): Integer; }
        interface B { function render(): String; }
        interface C extends A, B {}
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("conflicting parent interface methods should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Interface 'C' inherits incompatible signatures for method 'render' from 'A' and 'B'"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_interface_overriding_parent_method_with_incompatible_signature() {
    let src = r#"
        interface A { function render(): Integer; }
        interface C extends A {
            function render(): String;
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("incompatible interface override should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Interface 'C.render' overrides inherited method from 'A' with an incompatible signature"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_class_implementing_conflicting_interface_method_requirements() {
    let src = r#"
        interface A { function render(): Integer; }
        interface B { function render(): String; }
        class Both implements A, B {
            constructor() {}
            function render(): Integer { return 1; }
        }
        function main(): Integer { return 0; }
    "#;
    let errors =
        check_source(src).expect_err("conflicting implemented interface methods should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains(
            "Class 'Both' implements incompatible interface requirements for method 'render' from 'A' and 'B'"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_interface_implementation_with_narrower_parameter_type() {
    let src = r#"
        class Animal { constructor() {} }
        class Dog extends Animal { constructor() {} }
        interface Feeder { function feed(animal: Animal): Integer; }
        class Kennel implements Feeder {
            constructor() {}
            function feed(animal: Dog): Integer { return 1; }
        }
        function main(): Integer { return 0; }
    "#;
    let errors =
        check_source(src).expect_err("narrower interface implementation parameter should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Method 'Kennel.feed' does not match interface signature"),
        "{joined}"
    );
}

#[test]
fn rejects_constructor_type_arg_that_violates_interface_bound() {
    let src = r#"
        interface Named { function name(): Integer; }
        class Plain { constructor() {} }
        class Box<T extends Named> {
            value: Integer;
            constructor() { this.value = 1; }
        }
        function main(): Integer {
            bad: Box<Plain> = Box<Plain>();
            return bad.value;
        }
    "#;
    let errors =
        check_source(src).expect_err("constructor generic arg violating bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Constructor type argument Plain does not satisfy bound(s) Named"),
        "{joined}"
    );
}

#[test]
fn rejects_annotation_only_generic_type_arg_that_violates_interface_bound() {
    let src = r#"
        interface Named { function name(): Integer; }
        class Plain { constructor() {} }
        class Box<T extends Named> {
            value: Integer;
            constructor() { this.value = 1; }
        }
        function main(): Integer {
            bad: Box<Plain> = Box<Plain>();
            return 0;
        }
    "#;
    let errors =
        check_source(src).expect_err("annotation-only generic arg violating bound should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Type type argument Plain does not satisfy bound(s) Named"),
        "{joined}"
    );
}

#[test]
fn allows_interface_typed_parameters() {
    let src = r#"
        interface Printable {
            function print_me(): None;
        }
        class Book implements Printable {
            constructor() {}
            function print_me(): None { return None; }
        }
        function show(item: Printable): None {
            item.print_me();
            return None;
        }
        function main(): Integer {
            b: Book = Book();
            show(b);
            return 0;
        }
    "#;
    check_source(src).expect("interface-typed calls should typecheck");
}

#[test]
fn rejects_protected_member_access_from_non_subclass() {
    let src = r#"
        class Base {
            protected value: Integer;
            constructor(v: Integer) { this.value = v; }
        }
        class Other {
            function leak(b: Base): Integer {
                return b.value;
            }
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("protected visibility violation should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("protected"), "{joined}");
}

#[test]
fn enforces_parent_interface_methods_when_implementing_child_interface() {
    let src = r#"
        interface Named {
            function name(): Integer;
        }
        interface Printable extends Named {
            function print_me(): None;
        }
        class Report implements Printable {
            constructor() {}
            function print_me(): None { return None; }
        }
        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("missing parent-interface method should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("must implement interface method"),
        "{joined}"
    );
}

#[test]
fn rejects_invalid_list_constructor_arguments() {
    let src = r#"
        function main(): None {
            xs: List<Integer> = List<Integer>("bad", true, 5);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("invalid List constructor should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("expects 0 or 1 arguments"), "{joined}");
}

#[test]
fn rejects_invalid_map_set_constructor_arguments() {
    let src = r#"
        function main(): None {
            m: Map<String, Integer> = Map<String, Integer>(1);
            s: Set<Integer> = Set<Integer>(1, 2);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("invalid Map/Set constructors should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Constructor Map<String, Integer> expects 0 arguments"),
        "{joined}"
    );
    assert!(
        joined.contains("Constructor Set<Integer> expects 0 arguments"),
        "{joined}"
    );
}

#[test]
fn rejects_non_numeric_math_min_max_arguments() {
    let src = r#"
        function main(): None {
            low: Boolean = Math.min(true, false);
            high: Boolean = Math.max(true, false);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("non-numeric Math.min/max should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Math.min() arguments must be numeric types, got Boolean and Boolean"),
        "{joined}"
    );
    assert!(
        joined.contains("Math.max() arguments must be numeric types, got Boolean and Boolean"),
        "{joined}"
    );
}

#[test]
fn rejects_non_numeric_math_function_value_signatures() {
    let src = r#"
        import std.math.*;

        function main(): None {
            abs_fn: (Boolean) -> Boolean = Math.abs;
            min_fn: (Boolean, Boolean) -> Boolean = Math.min;
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("non-numeric Math function values should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Type mismatch: expected (Boolean) -> Boolean, got (unknown) -> unknown"),
        "{joined}"
    );
    assert!(
        joined.contains(
            "Type mismatch: expected (Boolean, Boolean) -> Boolean, got (unknown, unknown) -> unknown"
        ),
        "{joined}"
    );
}

#[test]
fn rejects_invalid_builtin_function_value_signatures_with_unknown_placeholders() {
    let src = r#"
        function main(): None {
            to_float_fn: (Boolean) -> Float = to_float;
            to_int_fn: (Boolean) -> Integer = to_int;
            fail_fn: (Boolean) -> None = fail;
            assert_true_fn: (Integer) -> None = assert_true;
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("invalid builtin function values should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Type mismatch: expected (Boolean) -> Float, got (unknown) -> Float"),
        "{joined}"
    );
    assert!(
        joined.contains("Type mismatch: expected (Boolean) -> Integer, got (unknown) -> Integer"),
        "{joined}"
    );
    assert!(
        joined.contains("Type mismatch: expected (Boolean) -> None, got (unknown) -> None"),
        "{joined}"
    );
    assert!(
        joined.contains("Type mismatch: expected (Integer) -> None, got (unknown) -> None"),
        "{joined}"
    );
}

#[test]
fn accepts_valid_builtin_generic_constructors() {
    let src = r#"
        function main(): None {
            xs: List<Integer> = List<Integer>();
            ys: List<Integer> = List<Integer>(32);
            box_empty: Box<Integer> = Box<Integer>();
            box_value: Box<Integer> = Box<Integer>(7);
            rc_empty: Rc<Integer> = Rc<Integer>();
            rc_value: Rc<Integer> = Rc<Integer>(8);
            arc_empty: Arc<Integer> = Arc<Integer>();
            arc_value: Arc<Integer> = Arc<Integer>(9);
            m: Map<String, Integer> = Map<String, Integer>();
            s: Set<Integer> = Set<Integer>();
            o: Option<Integer> = Option<Integer>();
            r: Result<Integer, String> = Result<Integer, String>();
            return None;
        }
    "#;
    check_source(src).expect("valid built-in generic constructors should typecheck");
}

#[test]
fn rejects_invalid_box_rc_arc_constructor_arguments() {
    let src = r#"
        function main(): None {
            b: Box<Integer> = Box<Integer>(1, 2);
            r: Rc<Integer> = Rc<Integer>(1, 2);
            a: Arc<Integer> = Arc<Integer>(1, 2);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("invalid Box/Rc/Arc constructors should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Constructor Box<Integer> expects 0 or 1 arguments, got 2"),
        "{joined}"
    );
    assert!(
        joined.contains("Constructor Rc<Integer> expects 0 or 1 arguments, got 2"),
        "{joined}"
    );
    assert!(
        joined.contains("Constructor Arc<Integer> expects 0 or 1 arguments, got 2"),
        "{joined}"
    );
}

#[test]
fn rejects_ptr_task_range_constructor_calls() {
    let src = r#"
        function main(): None {
            p: Ptr<Integer> = Ptr<Integer>();
            t: Task<Integer> = Task<Integer>();
            r: Range<Integer> = Range<Integer>();
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("Ptr/Task/Range constructors should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot construct built-in type 'Ptr<Integer>'"),
        "{joined}"
    );
    assert!(
        joined.contains("Cannot construct built-in type 'Task<Integer>'"),
        "{joined}"
    );
    assert!(
        joined.contains("Cannot construct built-in type 'Range<Integer>'"),
        "{joined}"
    );
}

#[test]
fn rejects_negative_list_constructor_capacity() {
    let src = r#"
        function main(): None {
            xs: List<Integer> = List<Integer>(-1);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("negative list capacity should fail");
    let joined = errors
        .iter()
        .map(|error| error.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("List constructor capacity cannot be negative"),
        "{joined}"
    );
}

#[test]
fn accepts_explicit_generic_function_values() {
    let src = r#"
        function id<T>(x: T): T { return x; }

        function main(): None {
            f: (Integer) -> Integer = id<Integer>;
            value: Integer = f(7);
            return None;
        }
    "#;
    check_source(src).expect("explicit generic function value should typecheck");
}

#[test]
fn accepts_generic_interface_references_in_implements_clauses() {
    let src = r#"
        interface I<T> {
            function get(): T;
        }

        class C implements I<String> {
            function get(): String { return "ok"; }
        }

        function main(): None {
            value: I<String> = C();
            out: String = value.get();
            return None;
        }
    "#;
    check_source(src).expect("generic interface implements clause should typecheck");
}

#[test]
fn accepts_specialized_parent_interface_methods_via_child_interface() {
    let src = r#"
        interface Reader<T> {
            function read(): T;
        }

        interface StringReader extends Reader<String> {}

        class FileReader implements StringReader {
            function read(): String { return "ok"; }
        }

        function main(): None {
            reader: StringReader = FileReader();
            value: String = reader.read();
            f: () -> String = reader.read;
            check: String = f();
            return None;
        }
    "#;
    check_source(src).expect("specialized parent interface methods should typecheck");
}

#[test]
fn accepts_map_indexing_with_non_integer_key_types() {
    let src = r#"
        function main(): None {
            m: Map<String, Integer> = Map<String, Integer>();
            m.set("x", 7);
            value: Integer = m["x"];
            return None;
        }
    "#;
    check_source(src).expect("Map indexing should accept key-typed indices");
}

#[test]
fn rejects_map_indexing_with_wrong_key_type() {
    let src = r#"
        function main(): None {
            m: Map<String, Integer> = Map<String, Integer>();
            value: Integer = m[1];
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("wrong map key index type should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Map index type mismatch: expected String, got Integer"),
        "{joined}"
    );
}

#[test]
fn accepts_user_defined_generic_class_construction_and_methods() {
    let src = r#"
        class Boxed<T> {
            value: T;
            constructor(value: T) { this.value = value; }
            function get(): T { return this.value; }
        }
        function main(): None {
            b: Boxed<Integer> = Boxed<Integer>(1);
            out: Integer = if (true) { b.get(); } else { b.value; };
            return None;
        }
    "#;
    check_source(src).expect("generic class construction and member use should typecheck");
}

#[test]
fn rejects_explicit_type_args_on_non_generic_function() {
    let src = r#"
        function f(x: Integer): Integer { return x; }
        function main(): None {
            y: Integer = f<String>(1);
            return None;
        }
    "#;
    let errors = check_source(src)
        .expect_err("non-generic function call with explicit type args should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("is not generic"), "{joined}");
}

#[test]
fn rejects_explicit_type_arg_arity_mismatch() {
    let src = r#"
        function id<T>(x: T): T { return x; }
        function main(): None {
            y: Integer = id<Integer, String>(1);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("generic arity mismatch should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("expects 1 type arguments"), "{joined}");
}

#[test]
fn rejects_unknown_explicit_type_argument() {
    let src = r#"
        function id<T>(x: T): T { return x; }
        function main(): None {
            y: Integer = id<Nope>(1);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("unknown explicit type arg should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Unknown type: Nope"), "{joined}");
}

#[test]
fn explicit_generic_method_call_typechecks() {
    let src = r#"
        class C {
            constructor() {}
            function id<T>(x: T): T { return x; }
        }
        function main(): None {
            c: C = C();
            y: Integer = c.id<Integer>(1);
            return None;
        }
    "#;
    check_source(src).expect("explicit generic method call should typecheck");
}

#[test]
fn explicit_generic_module_call_typechecks() {
    let src = r#"
        module M {
            function id<T>(x: T): T { return x; }
        }
        function main(): None {
            y: Integer = M.id<Integer>(1);
            return None;
        }
    "#;
    check_source(src).expect("explicit generic module call should typecheck");
}

#[test]
fn explicit_generic_nested_module_mangled_call_typechecks() {
    let src = r#"
        module A {
            module X {
                function id<T>(x: T): T { return x; }
            }
            module Y {
                function add(a: Integer, b: Integer): Integer { return a + b; }
            }
        }
        function main(): None {
            y: Integer = A__X__id<Integer>(A__Y__add(1, 2));
            return None;
        }
    "#;
    check_source(src).expect("explicit generic nested module mangled call should typecheck");
}

#[test]
fn nested_module_dot_call_typechecks() {
    let src = r#"
        module A {
            module Y {
                function add(a: Integer, b: Integer): Integer { return a + b; }
            }
        }
        function main(): None {
            y: Integer = A.Y.add(1, 2);
            return None;
        }
    "#;
    check_source(src).expect("nested module dot call should typecheck");
}

#[test]
fn explicit_generic_nested_module_dot_call_typechecks() {
    let src = r#"
        module A {
            module X {
                function id<T>(x: T): T { return x; }
            }
            module Y {
                function add(a: Integer, b: Integer): Integer { return a + b; }
            }
        }
        function main(): None {
            y: Integer = A.X.id<Integer>(A.Y.add(1, 2));
            return None;
        }
    "#;
    check_source(src).expect("explicit generic nested module dot call should typecheck");
}

#[test]
fn list_of_function_types_typechecks() {
    let src = r#"
        function main(): None {
            fs: List<(Integer) -> Integer> = List<(Integer) -> Integer>();
            return None;
        }
    "#;
    check_source(src).expect("list of function types should typecheck");
}

#[test]
fn option_some_static_constructor_typechecks() {
    let src = r#"
        function main(): None {
            maybe: Option<Integer> = Option.some(1);
            return None;
        }
    "#;
    check_source(src).expect("Option.some should typecheck");
}

#[test]
fn option_of_function_type_typechecks() {
    let src = r#"
        function add1(x: Integer): Integer { return x + 1; }
        function main(): None {
            maybe: Option<(Integer) -> Integer> = Option.some(add1);
            return None;
        }
    "#;
    check_source(src).expect("Option of function type should typecheck");
}

#[test]
fn function_valued_field_call_typechecks() {
    let src = r#"
        class C {
            f: (Integer) -> Integer;
            constructor() { this.f = (n: Integer) => n + 1; }
        }
        function main(): None {
            c: C = C();
            x: Integer = c.f(2);
            return None;
        }
    "#;
    check_source(src).expect("function-valued field calls should typecheck");
}

#[test]
fn module_alias_function_values_typecheck() {
    let src = r#"
        module util {
            function add1(x: Integer): Integer { return x + 1; }
            function twice(f: (Integer) -> Integer, x: Integer): Integer { return f(f(x)); }
        }

        function main(): None {
            f: (Integer) -> Integer = util.add1;
            y: Integer = util.twice(f, 1);
            return None;
        }
    "#;
    check_source(src).expect("module alias-style function values should typecheck");
}

#[test]
fn rejects_field_assignment_through_immutable_owner() {
    let src = r#"
        class C {
            mut v: Integer;
            constructor() { this.v = 1; }
        }
        function main(): None {
            c: C = C();
            c.v = 2;
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("immutable owner field assignment should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot assign to immutable variable 'c'"),
        "{joined}"
    );
}

#[test]
fn rejects_index_assignment_through_immutable_owner() {
    let src = r#"
        function main(): None {
            xs: List<Integer> = List<Integer>();
            xs.push(1);
            xs[0] = 2;
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("immutable owner index assignment should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot assign to immutable variable 'xs'"),
        "{joined}"
    );
}

#[test]
fn local_io_variable_does_not_act_as_stdlib_alias() {
    let src = r#"
        import std.io as io;
        function main(): None {
            io: Integer = 1;
            io.println("x");
            return None;
        }
    "#;
    let errors =
        check_source(src).expect_err("local variable named io must not be treated as std.io alias");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot call method on type Integer"),
        "{joined}"
    );
}

#[test]
fn specific_stdlib_alias_import_resolves_ident_call() {
    let src = r#"
        import std.io.*;
        import std.math.Math__abs as abs_fn;
        function main(): None {
            x: Float = abs_fn(-2.5);
            println(to_string(x));
            return None;
        }
    "#;
    check_source(src).expect("specific stdlib alias import call should typecheck");
}

#[test]
fn if_expression_branches_typecheck() {
    let src = r#"
        function main(): None {
            x: Integer = if (true) { 1; } else { 2; };
            return None;
        }
    "#;
    check_source(src).expect("if expression with matching branch types should typecheck");
}

#[test]
fn borrowed_read_accesses_typecheck() {
    let src = r#"
        class Boxed {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
            function get(): Integer { return this.value; }
        }

        function main(): None {
            s: String = "ab";
            xs: List<Integer> = List<Integer>();
            xs.push(40);
            m: Map<String, Integer> = Map<String, Integer>();
            m.set("k", 41);
            b: Boxed = Boxed(42);

            rs: &String = &s;
            rxs: &List<Integer> = &xs;
            rm: &Map<String, Integer> = &m;
            rb: &Boxed = &b;

            a: Integer = rb.value;
            c: Integer = rb.get();
            d: Char = rs[1];
            e: Integer = rxs[0];
            f: Integer = rxs.get(0);
            g: Integer = rxs.length();
            h: Integer = rm["k"];
            i: Integer = rm.get("k");
            j: Boolean = rm.contains("k");
            return None;
        }
    "#;
    check_source(src).expect("borrowed read accesses should typecheck");
}

#[test]
fn borrowed_mutating_accesses_typecheck() {
    let src = r#"
        class Bag {
            mut xs: List<Integer>;
            mut m: Map<String, Integer>;
            mut s: Set<Integer>;
            mut r: Range<Integer>;

            constructor() {
                this.xs = List<Integer>();
                this.m = Map<String, Integer>();
                this.s = Set<Integer>();
                this.r = range(0, 3);
            }
        }

        function main(): None {
            mut xs: List<Integer> = List<Integer>();
            mut m: Map<String, Integer> = Map<String, Integer>();
            mut s: Set<Integer> = Set<Integer>();
            mut r: Range<Integer> = range(0, 2);
            mut bag: Bag = Bag();

            rxs: &mut List<Integer> = &mut xs;
            rm: &mut Map<String, Integer> = &mut m;
            rs: &mut Set<Integer> = &mut s;
            rr: &mut Range<Integer> = &mut r;
            rb: &mut Bag = &mut bag;

            rxs.push(1);
            rxs.set(0, 2);
            value: Integer = rxs.pop();
            rm.set("k", value);
            rs.add(value);
            rs.remove(value);
            x: Integer = rr.next();
            require(x == 0);

            rb.xs.push(3);
            rb.m.set("k2", 4);
            rb.s.add(5);
            y: Integer = rb.r.next();
            require(y == 0);
            return None;
        }
    "#;
    check_source(src).expect("borrowed mutating accesses should typecheck");
}

#[test]
fn borrowed_mutating_index_assignments_typecheck() {
    let src = r#"
        class Bag {
            mut xs: List<Integer>;
            mut m: Map<String, Integer>;

            constructor() {
                this.xs = List<Integer>();
                this.m = Map<String, Integer>();
            }
        }

        function main(): None {
            mut xs: List<Integer> = List<Integer>();
            xs.push(1);
            mut m: Map<String, Integer> = Map<String, Integer>();
            mut bag: Bag = Bag();

            rxs: &mut List<Integer> = &mut xs;
            rm: &mut Map<String, Integer> = &mut m;
            rb: &mut Bag = &mut bag;

            rxs[0] = 2;
            rm["k"] = 7;
            rb.xs.push(1);
            rb.xs[0] = 3;
            rb.m["k2"] = 4;
            return None;
        }
    "#;
    check_source(src).expect("borrowed mutating index assignments should typecheck");
}

#[test]
fn immutable_reference_index_assignment_rejected() {
    let src = r#"
        function main(): None {
            mut xs: List<Integer> = List<Integer>();
            xs.push(1);
            mut m: Map<String, Integer> = Map<String, Integer>();

            rxs: &List<Integer> = &xs;
            rm: &Map<String, Integer> = &m;

            rxs[0] = 2;
            rm["k"] = 7;
            return None;
        }
    "#;
    let errors =
        check_source(src).expect_err("immutable reference index assignments should be rejected");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot assign through immutable reference 'rxs'"),
        "{joined}"
    );
    assert!(
        joined.contains("Cannot assign through immutable reference 'rm'"),
        "{joined}"
    );
}

#[test]
fn immutable_reference_deref_assignment_rejected() {
    let src = r#"
        function main(): None {
            mut x: Integer = 1;
            r: &Integer = &x;
            *r = 2;
            return None;
        }
    "#;
    let errors =
        check_source(src).expect_err("immutable reference deref assignment should be rejected");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Cannot assign through immutable reference 'r'"),
        "{joined}"
    );
}

#[test]
fn raw_ptr_deref_typechecks_for_non_integer_payloads() {
    let src = r#"
        function load(slot: Ptr<Float>): Float {
            return *slot;
        }

        function load_list(xs: Ptr<List<Option<Integer>>>): List<Option<Integer>> {
            return *xs;
        }

        function main(): None {
            return None;
        }
    "#;

    check_source(src).expect("raw Ptr<T> deref should typecheck for typed payloads");
}

#[test]
fn if_expression_branch_type_mismatch_fails() {
    let src = r#"
        function main(): None {
            x: Integer = if (true) { 1; } else { "bad"; };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("if expression branch mismatch should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("If expression branch type mismatch"),
        "{joined}"
    );
}

#[test]
fn if_expression_without_else_is_none_typed() {
    let src = r#"
        function main(): None {
            x: Integer = if (true) { 1; };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("if expression without else should be None-typed");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Type mismatch: cannot assign None to variable of type Integer"),
        "{joined}"
    );
}

#[test]
fn match_expression_branch_type_mismatch_fails() {
    let src = r#"
        function main(): None {
            x: Integer = match (1) {
                1 => 1,
                _ => "bad",
            };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("match expression branch mismatch should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Match expression arm type mismatch")
            || joined.contains("Type mismatch: cannot assign"),
        "{joined}"
    );
}

#[test]
fn match_expression_boolean_non_exhaustive_fails() {
    let src = r#"
        function main(): None {
            x: Integer = match (true) {
                true => 1,
            };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("non-exhaustive boolean match should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Non-exhaustive match expression"),
        "{joined}"
    );
}

#[test]
fn match_statement_boolean_non_exhaustive_fails() {
    let src = r#"
        function main(): None {
            match (true) {
                true => { }
            }
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("non-exhaustive boolean match stmt should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Non-exhaustive match statement"),
        "{joined}"
    );
}

#[test]
fn empty_match_statement_fails() {
    let src = r#"
        function main(): None {
            match (1) {
            }
            return None;
        }
    "#;
    let tokens = tokenize(src).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let err = parser
        .parse_program()
        .expect_err("empty match statement should now fail in parser");
    assert!(
        err.message
            .contains("match statements must contain at least one arm"),
        "{}",
        err.message
    );
}

#[test]
fn integer_match_expression_requires_catch_all() {
    let src = r#"
        function main(): None {
            x: Integer = match (2) {
                1 => 1,
            };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("integer match expression without catch-all");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Non-exhaustive match expression"),
        "{joined}"
    );
}

#[test]
fn empty_match_expression_fails() {
    let src = r#"
        function main(): None {
            n: None = match (1) {
            };
            return None;
        }
    "#;
    let tokens = tokenize(src).expect("tokenize");
    let mut parser = Parser::new(tokens);
    let err = parser
        .parse_program()
        .expect_err("empty match expression should now fail in parser");
    assert!(
        err.message
            .contains("match expressions must contain at least one arm"),
        "{}",
        err.message
    );
}

#[test]
fn if_expression_reports_single_undefined_identifier_error() {
    let src = r#"
        function main(): None {
            x: Integer = if (true) { y; } else { 1; };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("undefined variable should fail");
    let undef_count = errors
        .iter()
        .filter(|e| e.message.contains("Undefined variable: y"))
        .count();
    assert_eq!(undef_count, 1, "{:?}", errors);
}

#[test]
fn match_expression_reports_single_undefined_identifier_error() {
    let src = r#"
        function main(): None {
            x: Integer = match (1) {
                1 => { y; },
                _ => { 0; }
            };
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("undefined variable should fail");
    let undef_count = errors
        .iter()
        .filter(|e| e.message.contains("Undefined variable: y"))
        .count();
    assert_eq!(undef_count, 1, "{:?}", errors);
}

#[test]
fn qualified_enum_patterns_typecheck_against_leaf_variant_names() {
    let src = r#"
        enum E {
            A(Integer),
            B(Integer)
        }

        function main(): None {
            value: E = E.A(1);
            match (value) {
                Enum.A(v) => { require(v == 1); }
                util.E.B(w) => { require(w == 2); }
                _ => { }
            }
            return None;
        }
    "#;
    check_source(src).expect("qualified enum patterns should typecheck");
}

#[test]
fn qualified_module_type_paths_typecheck_against_mangled_symbols() {
    let src = r#"
        module util {
            class Item {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }
            function mk(): Item { return Item(7); }
        }

        function main(): Integer {
            item: util.Item = util.mk();
            return item.get();
        }
    "#;

    check_source(src).expect("qualified module type paths should resolve to mangled symbols");
}

#[test]
fn user_defined_generic_classes_named_like_builtins_typecheck() {
    let src = r#"
        class Box<T> {
            value: T;
            constructor(value: T) { this.value = value; }
            function get(): T { return this.value; }
        }

        function mk(value: Integer): Box<Integer> {
            return Box<Integer>(value);
        }

        function main(): Integer {
            return mk(42).get();
        }
    "#;

    check_source(src).expect("user-defined generic classes named like built-ins should typecheck");
}

#[test]
fn enum_match_expression_is_exhaustive_without_wildcard() {
    let src = r#"
        enum E {
            A(Integer)
        }

        function main(): None {
            value: E = E.A(1);
            result: Integer = match (value) {
                E.A(v) => v
            };
            require(result == 1);
            return None;
        }
    "#;
    check_source(src).expect("single-variant enum match should be exhaustive");
}

#[test]
fn rejects_extern_function_values_during_typecheck() {
    let src = r#"
        extern(c, "puts") function puts(s: String): Integer;

        function main(): None {
            f: (String) -> Integer = puts;
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("extern function value should fail during typecheck");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("extern function 'puts' cannot be used as a first-class value"),
        "{joined}"
    );
}

#[test]
fn rejects_unsupported_enum_payload_types_during_typecheck() {
    let src = r#"
        class C {
            value: Integer;
            constructor(v: Integer) { this.value = v; }
        }

        enum EF {
            A((Integer) -> Integer)
        }

        enum EL {
            A(List<Integer>)
        }

        enum EO {
            A(Option<C>)
        }

        function inc(x: Integer): Integer { return x + 1; }

        function main(): None {
            ef: EF = EF.A(inc);
            el: EL = EL.A(List<Integer>());
            eo: EO = EO.A(Option.some(C(1)));
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("unsupported enum payload types should fail early");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Enum payload type '(Integer) -> Integer' is not supported yet"));
    assert!(joined.contains("Enum payload type 'List<Integer>' is not supported yet"));
    assert!(joined.contains("Enum payload type 'Option<C>' is not supported yet"));
}

#[test]
fn rejects_nested_enum_payload_types_during_typecheck() {
    let src = r#"
        enum Inner {
            X(Integer)
        }

        enum Outer {
            A(Inner)
        }

        function main(): Integer { return 0; }
    "#;
    let errors = check_source(src).expect_err("nested enum payload types should fail early");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Enum payload type 'Inner' is not supported yet"));
}

#[test]
fn supports_unit_enum_variant_values() {
    let src = r#"
        enum E { A, B }

        function main(): None {
            e: E = E.A;
            match (e) {
                E.A => { }
                E.B => { }
            }
            return None;
        }
    "#;
    check_source(src).expect("unit enum variants should typecheck as values");
}

#[test]
fn accepts_bound_generic_method_value_field_access() {
    let src = r#"
        class Box<T> {
            value: T;
            constructor(value: T) { this.value = value; }
            function get(): T { return this.value; }
        }

        function main(): Integer {
            box: Box<String> = Box<String>("hello");
            getter: () -> String = box.get;
            return getter().length();
        }
    "#;
    check_source(src).expect("bound generic method values should typecheck as functions");
}

#[test]
fn allows_interface_method_dispatch_during_typecheck() {
    let src = r#"
        interface Named {
            function get(): String;
        }

        class Boxed implements Named {
            value: String;
            constructor(value: String) { this.value = value; }
            function get(): String { return this.value; }
        }

        function main(): Integer {
            n: Named = Boxed("abc");
            return n.get().length();
        }
    "#;
    check_source(src).expect("interface method dispatch should typecheck");
}

#[test]
fn allows_interface_bound_method_values_during_typecheck() {
    let src = r#"
        interface Named {
            function get(): String;
        }

        class Boxed implements Named {
            value: String;
            constructor(value: String) { this.value = value; }
            function get(): String { return this.value; }
        }

        function main(): Integer {
            n: Named = Boxed("abc");
            getter: () -> String = n.get;
            return getter().length();
        }
    "#;
    check_source(src).expect("interface bound method values should typecheck");
}

#[test]
fn accepts_forward_declared_generic_class_in_enum_payload_constructor() {
    let src = r#"
        enum Choice {
            Boxed(Box<String>),
            Empty
        }

        class Box<T> {
            value: T;
            constructor(value: T) { this.value = value; }
        }

        function main(): Integer {
            current: Choice = Choice.Boxed(Box<String>("hi"));
            return 0;
        }
    "#;
    check_source(src)
        .expect("enum payload constructor should accept forward-declared generic classes");
}

#[test]
fn accepts_forward_declared_generic_class_in_match_expression_arms() {
    let src = r#"
        enum Choice {
            Boxed(Box<String>),
            Empty
        }

        class Box<T> {
            value: T;
            constructor(value: T) { this.value = value; }
        }

        function main(): Integer {
            current: Choice = Choice.Empty;
            picked: Box<String> = match (current) {
                Boxed(inner) => inner,
                Empty => Box<String>("no")
            };
            return 0;
        }
    "#;
    check_source(src)
        .expect("match arms should join on forward-declared generic class payload types");
}

#[test]
fn accepts_forward_declared_generic_class_payload_block_receiver_chain() {
    let src = r#"
        enum Choice {
            Boxed(Box<String>),
            Empty
        }

        class Box<T> {
            value: T;
            constructor(value: T) { this.value = value; }
            function get(): T { return this.value; }
        }

        function main(): Integer {
            return if ({
                current: Choice = Choice.Boxed(Box<String>("hi"));
                match (current) {
                    Boxed(inner) => inner,
                    Empty => Box<String>("no")
                }
            }.get().length() == 2) { 0 } else { 1 };
        }
    "#;
    check_source(src).expect(
        "downstream method calls should work on forward-declared generic class match payloads",
    );
}

#[test]
fn accepts_async_block_tail_expression_type() {
    let src = r#"
        function main(): None {
            task: Task<Integer> = async { 7 };
            value: Integer = await(task);
            return None;
        }
    "#;
    check_source(src).expect("async block tail expression should infer Task<Integer>");
}

#[test]
fn accepts_async_block_unary_tail_expression_type() {
    let src = r#"
        function main(): None {
            task: Task<Integer> = async { -7 };
            flag: Task<Boolean> = async { !false };
            a: Integer = await(task);
            b: Boolean = await(flag);
            return None;
        }
    "#;
    check_source(src).expect("async block unary tail expressions should infer correct Task<T>");
}

#[test]
fn accepts_async_block_binary_tail_expression_type() {
    let src = r#"
        function main(): None {
            sum_task: Task<Integer> = async { 2 + 5 };
            cmp_task: Task<Boolean> = async { 2 + 5 == 7 };
            a: Integer = await(sum_task);
            b: Boolean = await(cmp_task);
            return None;
        }
    "#;
    check_source(src).expect("async block binary tail expressions should infer correct Task<T>");
}

#[test]
fn accepts_async_block_function_value_tail_expression_type() {
    let src = r#"
        function inc(x: Integer): Integer { return x + 1; }

        function main(): None {
            task: Task<(Integer) -> Integer> = async { inc };
            f: (Integer) -> Integer = await(task);
            value: Integer = f(1);
            return None;
        }
    "#;
    check_source(src)
        .expect("async block function-value tail expressions should infer correct Task<T>");
}

#[test]
fn accepts_async_block_unit_enum_value_tail_expression_type() {
    let src = r#"
        enum E { A, B }

        function main(): None {
            task: Task<E> = async { E.A };
            value: E = await(task);
            return None;
        }
    "#;
    check_source(src).expect("async block unit-enum tail expressions should infer correct Task<T>");
}

#[test]
fn accepts_builtin_and_reference_async_block_tail_expression_types() {
    let src = r#"
        import std.string.*;
        import std.io.println;

        function main(): None {
            some_task: Task<Option<Integer>> = async { Option.some(7) };
            none_task: Task<Option<Integer>> = async { Option.none() };
            ok_task: Task<Result<Integer, String>> = async { Result.ok(7) };
            err_task: Task<Result<Integer, String>> = async { Result.error("boom") };
            len_task: Task<Integer> = async { Str.len("abc") };
            compare_task: Task<Integer> = async { Str.compare("a", "a") };
            concat_task: Task<String> = async { Str.concat("a", "b") };
            upper_task: Task<String> = async { Str.upper("ab") };
            lower_task: Task<String> = async { Str.lower("AB") };
            trim_task: Task<String> = async { Str.trim("  ok  ") };
            contains_task: Task<Boolean> = async { Str.contains("abc", "b") };
            starts_task: Task<Boolean> = async { Str.startsWith("abc", "a") };
            ends_task: Task<Boolean> = async { Str.endsWith("abc", "c") };
            string_task: Task<String> = async { to_string(7) };
            print_task: Task<None> = async { println("hi") };
            require_task: Task<None> = async { require(true) };
            range_task: Task<Range<Integer>> = async { range(0, 3) };
            lambda_task: Task<(Integer) -> Integer> = async { |x: Integer| x + 1 };
            if_task: Task<Integer> = async { if (true) { Str.len("abc") } else { Str.len("ab") } };
            match_task: Task<String> = async {
                match (1) {
                    1 => { to_string(7) }
                    _ => { to_string(8) }
                }
            };

            require(await(some_task).unwrap() == 7);
            require(await(none_task).is_none());
            require(await(ok_task).unwrap() == 7);
            require(await(err_task).is_error());
            require(await(len_task) == 3);
            require(await(compare_task) == 0);
            require(await(concat_task) == "ab");
            require(await(upper_task) == "AB");
            require(await(lower_task) == "ab");
            require(await(trim_task) == "ok");
            require(await(contains_task));
            require(await(starts_task));
            require(await(ends_task));
            require(await(string_task) == "7");
            await(print_task);
            await(require_task);
            require(await(range_task).has_next());
            require((await(lambda_task))(1) == 2);
            require(await(if_task) == 3);
            require(await(match_task) == "7");
            return None;
        }
    "#;
    check_source(src)
        .expect("builtin and reference async block tails should infer correct Task<T>");
}

#[test]
fn accepts_function_types_inside_generic_class_arguments() {
    let src = r#"
        class Holder<T> {
            value: T;
            constructor(value: T) { this.value = value; }
            function get(): T { return this.value; }
        }

        function add(x: Integer, y: Integer): Integer { return x + y; }

        function main(): None {
            holder: Holder<(Integer, Integer) -> Integer> = Holder<(Integer, Integer) -> Integer>(add);
            f: (Integer, Integer) -> Integer = holder.get();
            return None;
        }
    "#;
    check_source(src).expect("generic classes should preserve function-type arguments");
}

#[test]
fn rejects_async_blocks_returning_borrowed_references() {
    let src = r#"
        function inc(x: Integer): Integer { return x + 1; }

        function main(): None {
            task: Task<&(Integer) -> Integer> = async {
                return &inc;
            };
            return None;
        }
    "#;
    let errors =
        check_source(src).expect_err("async block returning borrowed reference should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Async block cannot return a value containing borrowed references"));
}

#[test]
fn rejects_async_functions_returning_borrowed_references() {
    let src = r#"
        function inc(x: Integer): Integer { return x + 1; }

        async function make_ref(): Task<&(Integer) -> Integer> {
            return &inc;
        }
    "#;
    let errors =
        check_source(src).expect_err("async function returning borrowed reference should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains(
        "Async function 'make_ref' cannot return a value containing borrowed references"
    ));
}

#[test]
fn rejects_async_functions_accepting_borrowed_reference_parameters() {
    let src = r#"
        async function read_ref(r: &Integer): Task<Integer> {
            return *r;
        }
    "#;
    let errors = check_source(src)
        .expect_err("async function accepting borrowed reference parameter should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains(
        "Async function 'read_ref' cannot accept a parameter containing borrowed references"
    ));
}

#[test]
fn rejects_async_blocks_capturing_borrowed_reference_variables() {
    let src = r#"
        function main(): None {
            x: Integer = 1;
            r: &Integer = &x;
            task: Task<Integer> = async {
                return *r;
            };
            return None;
        }
    "#;
    let errors = check_source(src)
        .expect_err("async block capturing borrowed reference variable should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined
        .contains("Async block cannot capture 'r' because its type contains borrowed references"));
}

#[test]
fn rejects_undocumented_task_result_type_method() {
    let src = r#"
        async function make(): Task<Integer> {
            return 1;
        }

        function main(): Integer {
            t: Task<Integer> = make();
            return t.result_type();
        }
    "#;
    let errors = check_source(src).expect_err("Task.result_type should fail during typecheck");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Unknown Task method: result_type"));
}

#[test]
fn range_accepts_float_arguments() {
    let src = r#"
        function main(): None {
            r: Range<Float> = range(0.0, 3.0, 1.0);
            return None;
        }
    "#;
    check_source(src).expect("float range arguments should type check");
}

#[test]
fn range_rejects_mixed_numeric_arguments() {
    let src = r#"
        function main(): None {
            r = range(0, 3.0, 1.0);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("mixed numeric range arguments should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("range() arguments must use the same numeric type"),
        "{joined}"
    );
}

#[test]
fn range_rejects_zero_literal_step() {
    let src = r#"
        function main(): None {
            r: Range<Integer> = range(0, 3, 0);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("zero range step should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("range() step cannot be 0"), "{joined}");
}

#[test]
fn range_rejects_zero_float_literal_step() {
    let src = r#"
        function main(): None {
            r: Range<Float> = range(0.0, 3.0, 0.0);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("zero float range step should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("range() step cannot be 0"), "{joined}");
}

#[test]
fn range_rejects_constant_integer_zero_step_expression() {
    let src = r#"
        function main(): None {
            r: Range<Integer> = range(0, 3, 1 - 1);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("constant integer zero step should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("range() step cannot be 0"), "{joined}");
}

#[test]
fn range_rejects_constant_float_zero_step_expression() {
    let src = r#"
        function main(): None {
            r: Range<Float> = range(0.0, 3.0, 0.5 - 0.5);
            return None;
        }
    "#;
    let errors = check_source(src).expect_err("constant float zero step should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("range() step cannot be 0"), "{joined}");
}

#[test]
fn integer_division_rejects_constant_zero_divisor() {
    let src = r#"
        function main(): Integer {
            return 6 / (2 - 2);
        }
    "#;
    let errors = check_source(src).expect_err("constant integer zero divisor should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Integer division by zero"), "{joined}");
}

#[test]
fn integer_modulo_rejects_constant_zero_divisor() {
    let src = r#"
        function main(): Integer {
            return 6 % (2 - 2);
        }
    "#;
    let errors = check_source(src).expect_err("constant integer zero modulo divisor should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Integer modulo by zero"), "{joined}");
}

#[test]
fn await_timeout_rejects_negative_constant_literal() {
    let src = r#"
        async function work(): Integer {
            return 1;
        }

        function main(): Integer {
            maybe: Option<Integer> = work().await_timeout(-1);
            if (maybe.is_some()) { return 1; }
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative await_timeout literal should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Task.await_timeout() timeout must be non-negative"),
        "{joined}"
    );
}

#[test]
fn await_timeout_rejects_negative_constant_expression() {
    let src = r#"
        async function work(): Integer {
            return 1;
        }

        function main(): Integer {
            maybe: Option<Integer> = work().await_timeout(1 - 2);
            if (maybe.is_some()) { return 1; }
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative await_timeout expression should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Task.await_timeout() timeout must be non-negative"),
        "{joined}"
    );
}

#[test]
fn time_sleep_rejects_negative_constant_literal() {
    let src = r#"
        import std.time.*;

        function main(): Integer {
            Time.sleep(-1);
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative Time.sleep literal should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Time.sleep() milliseconds must be non-negative"),
        "{joined}"
    );
}

#[test]
fn time_sleep_rejects_negative_constant_expression() {
    let src = r#"
        import std.time.*;

        function main(): Integer {
            Time.sleep(1 - 2);
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative Time.sleep expression should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Time.sleep() milliseconds must be non-negative"),
        "{joined}"
    );
}

#[test]
fn args_get_rejects_negative_constant_literal() {
    let src = r#"
        import std.args.*;

        function main(): Integer {
            value: String = Args.get(-1);
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative Args.get literal should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Args.get() index cannot be negative"),
        "{joined}"
    );
}

#[test]
fn read_line_imported_from_std_io_typechecks() {
    let src = r#"
        import std.io.*;

        function main(): Integer {
            line: String = read_line();
            return 0;
        }
    "#;
    check_source(src).expect("read_line should typecheck from std.io wildcard import");
}

#[test]
fn list_get_rejects_negative_constant_index() {
    let src = r#"
        function main(): Integer {
            values: List<Integer> = List<Integer>();
            values.push(1);
            return values.get(-1);
        }
    "#;
    let errors = check_source(src).expect_err("negative list.get index should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("List.get() index cannot be negative"),
        "{joined}"
    );
}

#[test]
fn list_index_rejects_negative_constant_index() {
    let src = r#"
        function main(): Integer {
            values: List<Integer> = List<Integer>();
            values.push(1);
            return values[-1];
        }
    "#;
    let errors = check_source(src).expect_err("negative list index should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("List index cannot be negative"), "{joined}");
}

#[test]
fn list_set_rejects_negative_constant_index() {
    let src = r#"
        function main(): Integer {
            values: List<Integer> = List<Integer>();
            values.push(1);
            values.set(-1, 2);
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("negative list.set index should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("List.set() index cannot be negative"),
        "{joined}"
    );
}

#[test]
fn string_index_rejects_negative_constant_index() {
    let src = r#"
        function main(): Char {
            return "abc"[-1];
        }
    "#;
    let errors = check_source(src).expect_err("negative string index should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("String index cannot be negative"),
        "{joined}"
    );
}

#[test]
fn string_index_rejects_constant_out_of_bounds_literal_index() {
    let src = r#"
        function bad(): Char {
            return "abc"[5];
        }

        function main(): Integer {
            c: Char = bad();
            return 0;
        }
    "#;
    let errors = check_source(src).expect_err("constant out-of-bounds string index should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("String index out of bounds"), "{joined}");
}

#[test]
fn string_index_rejects_unicode_literal_index_past_char_len() {
    let src = r#"
        function bad(): Char {
            return "🚀"[1];
        }

        function main(): Integer {
            c: Char = bad();
            return 0;
        }
    "#;
    let errors =
        check_source(src).expect_err("unicode string literal char index past len should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("String index out of bounds"), "{joined}");
}

#[test]
fn main_rejects_non_integer_or_none_return_type() {
    let src = r#"
        function main(): String {
            return "oops";
        }
    "#;
    let errors = check_source(src).expect_err("main string return type should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("main() must return None or Integer"),
        "{joined}"
    );
}

#[test]
fn main_rejects_parameters() {
    let src = r#"
        function main(x: Integer): Integer {
            return x;
        }
    "#;
    let errors = check_source(src).expect_err("main parameters should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("main() cannot declare parameters"),
        "{joined}"
    );
}

#[test]
fn main_rejects_async_entrypoint() {
    let src = r#"
        async function main(): Task<Integer> {
            return 1;
        }
    "#;
    let errors = check_source(src).expect_err("async main should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("main() cannot be async; use a synchronous main() entrypoint"),
        "{joined}"
    );
}

#[test]
fn try_on_result_requires_result_return_context() {
    let src = r#"
        function choose(): Result<Integer, String> { return Result.ok(1); }
        function helper(): Integer {
            value: Integer = choose()?;
            return value;
        }
        function main(): Integer {
            return helper();
        }
    "#;
    let errors =
        check_source(src).expect_err("try on Result outside Result return context should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("'?' on Result requires the enclosing function to return Result"),
        "{joined}"
    );
}

#[test]
fn try_on_option_requires_option_return_context() {
    let src = r#"
        function choose(): Option<Integer> { return Option.some(1); }
        function helper(): Result<Integer, String> {
            value: Integer = choose()?;
            return Result.ok(value);
        }
        function main(): Integer {
            return helper().unwrap();
        }
    "#;
    let errors =
        check_source(src).expect_err("try on Option inside Result-returning function should fail");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("'?' on Option requires the enclosing function to return Option"),
        "{joined}"
    );
}

#[test]
fn try_inside_lambda_does_not_inherit_outer_result_context() {
    let src = r#"
        function choose(): Result<Integer, String> { return Result.ok(1); }
        function wrap(): Result<Integer, String> {
            f: () -> Integer = () => choose()?;
            return Result.ok(f());
        }
        function main(): Integer {
            return wrap().unwrap();
        }
    "#;
    let errors = check_source(src)
        .expect_err("try inside lambda should not inherit outer Result return context");
    let joined = errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("'?' on Result requires the enclosing function to return Result"),
        "{joined}"
    );
}

#[test]
fn seeded_check_supports_interface_extending_namespace_aliased_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Named {
                function name(): Integer;
            }
        }
        interface Printable extends u.Named {
            function print_me(): Integer;
        }
        class Report implements Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    let tokens = tokenize(src).expect("tokenize seeded alias interface source");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .expect("parse seeded alias interface source");
    let mut type_checker = TypeChecker::new();
    type_checker
        .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
        .expect("seeded check should support aliased parent interface");
}

#[test]
fn seeded_check_supports_interface_extending_nested_namespace_aliased_interface() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Named {
                    function name(): Integer;
                }
            }
        }
        interface Printable extends u.Api.Named {
            function print_me(): Integer;
        }
        class Report implements Printable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    let tokens = tokenize(src).expect("tokenize seeded nested alias source");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .expect("parse seeded nested alias source");
    let mut type_checker = TypeChecker::new();
    type_checker
        .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
        .expect("seeded check should support nested aliased parent interface");
}

#[test]
fn seeded_check_supports_interface_extending_multiple_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            interface Named {
                function name(): Integer;
            }
            interface Printable {
                function print_me(): Integer;
            }
        }
        interface Reportable extends u.Named, u.Printable {}
        class Report implements Reportable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    let tokens = tokenize(src).expect("tokenize seeded multi alias interface source");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .expect("parse seeded multi alias interface source");
    let mut type_checker = TypeChecker::new();
    type_checker
        .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
        .expect("seeded check should support multiple aliased parent interfaces");
}

#[test]
fn seeded_check_supports_interface_extending_multiple_nested_namespace_aliased_interfaces() {
    let src = r#"
        import Lib as u;
        module Lib {
            module Api {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
        }
        interface Reportable extends u.Api.Named, u.Api.Printable {}
        class Report implements Reportable {
            constructor() {}
            function name(): Integer { return 1; }
            function print_me(): Integer { return 2; }
        }
        function main(): Integer {
            r: Report = Report();
            return r.name() + r.print_me();
        }
    "#;
    let tokens = tokenize(src).expect("tokenize seeded multi nested alias interface source");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .expect("parse seeded multi nested alias interface source");
    let mut type_checker = TypeChecker::new();
    type_checker
        .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
        .expect("seeded check should support multiple nested aliased parent interfaces");
}
