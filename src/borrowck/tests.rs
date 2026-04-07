use super::{format_borrow_errors, BorrowChecker, BorrowError};
use crate::parser::Parser;
use crate::{ast::Program, lexer};

fn parse_program(source: &str) -> Program {
    let tokens = lexer::tokenize(source).expect("tokenization should succeed");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("parse should succeed")
}

fn borrow_errors(source: &str) -> Vec<String> {
    let program = parse_program(source);
    let mut checker = BorrowChecker::new();
    checker
        .check(&program)
        .expect_err("borrow check should fail")
        .into_iter()
        .map(|e| e.message)
        .collect()
}

fn borrow_ok(source: &str) {
    let program = parse_program(source);
    let mut checker = BorrowChecker::new();
    checker.check(&program).expect("borrow check should pass");
}

#[test]
fn formatting_handles_inverted_spans_without_panicking() {
    let errors = vec![BorrowError {
        message: "broken span".to_string(),
        span: 3..3,
        note: None,
    }];

    let rendered = format_borrow_errors(&errors, "let x = 1;\n", "sample.arden");

    assert!(rendered.contains("broken span"), "{rendered}");
}

#[test]
fn detects_use_after_move() {
    let source = r#"
        import std.io.*;
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "hello";
            consume(s);
            println(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| m.contains("Use of moved value 's'")));
}

#[test]
fn detects_move_while_borrowed() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "hello";
            r: &String = &s;
            consume(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn detects_double_mutable_borrow() {
    let source = r#"
        function main(): None {
            mut x: Integer = 1;
            a: &mut Integer = &mut x;
            b: &mut Integer = &mut x;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
}

#[test]
fn immutable_borrow_released_after_scope() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "hello";
            if (true) {
                r: &String = &s;
            }
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn lambda_capture_marks_move() {
    let source = r#"
        import std.io.*;
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "hello";
            f: () -> None = () => consume(s);
            println(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| m.contains("Use of moved value 's'")));
}

#[test]
fn lambda_owned_capture_does_not_fail_inside_lambda() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "hello";
            f: () -> None = () => consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn compound_assign_on_mut_borrowed_variable_is_rejected() {
    let source = r#"
        function main(): None {
            mut x: Integer = 10;
            r: &mut Integer = &mut x;
            x += 1;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign to 'x' while mutably borrowed")));
}

#[test]
fn field_assign_on_borrowed_owner_is_rejected() {
    let source = r#"
        class C {
            mut value: Integer;
            constructor(v: Integer) { this.value = v; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            c.value += 1;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign through 'c' while borrowed")));
}

#[test]
fn this_method_uses_declared_param_modes() {
    let source = r#"
        import std.io.*;
        class A {
            function take(borrow s: String): None { return None; }
            function run(): None {
                s: String = "x";
                this.take(s);
                println(s);
                return None;
            }
        }
        function main(): None {
            a: A = A();
            a.run();
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn invalid_assign_does_not_clear_borrow_state() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        function main(): None {
            mut s: String = "a";
            r: &String = &s;
            s = "b";
            consume(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign to 's' while borrowed")));
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn stdlib_alias_call_borrows_instead_of_moves() {
    let source = r#"
        import std.io as io;
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            io.println(s);
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn reference_return_from_borrow_keeps_source_borrowed() {
    let source = r#"
        function id_borrow(borrow s: String): &String { return &s; }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            r: &String = id_borrow(s);
            consume(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn lambda_borrow_capture_blocks_move_after_creation() {
    let source = r#"
        function take_borrow(borrow s: String): None { return None; }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            f: () -> None = () => take_borrow(s);
            consume(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn immutable_borrow_blocks_mutating_method_call() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            c.touch();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'c' while immutably borrowed")));
}

#[test]
fn immutable_borrow_allows_read_only_method_call() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function get(): Integer { return this.v; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            x: Integer = c.get();
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn immutable_borrow_blocks_transitively_mutating_method_call() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch2(): None { this.v += 1; return None; }
            function wrapper(): None { this.touch2(); return None; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            c.wrapper();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'c' while immutably borrowed")));
}

#[test]
fn immutable_borrow_blocks_method_with_mutating_builtin_field_call() {
    let source = r#"
        class Bag {
            mut xs: List<Integer>;
            constructor() { this.xs = List<Integer>(); }
            function add_one(): None {
                this.xs.push(1);
                return None;
            }
        }
        function main(): None {
            mut bag: Bag = Bag();
            rb: &Bag = &bag;
            bag.add_one();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'bag' while immutably borrowed")));
}

#[test]
fn immutable_borrow_blocks_transitively_mutating_builtin_field_method_call() {
    let source = r#"
        class Bag {
            mut xs: List<Integer>;
            constructor() { this.xs = List<Integer>(); }
            function add_one_impl(): None {
                this.xs.push(1);
                return None;
            }
            function add_one(): None {
                this.add_one_impl();
                return None;
            }
        }
        function main(): None {
            mut bag: Bag = Bag();
            rb: &Bag = &bag;
            bag.add_one();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'bag' while immutably borrowed")));
}

#[test]
fn immutable_borrow_blocks_method_with_nested_mutating_builtin_field_call() {
    let source = r#"
        class Inner {
            mut xs: List<Integer>;
            constructor() { this.xs = List<Integer>(); }
        }
        class Outer {
            mut inner: Inner;
            constructor() { this.inner = Inner(); }
            function add_one(): None {
                this.inner.xs.push(1);
                return None;
            }
        }
        function main(): None {
            mut outer: Outer = Outer();
            ro: &Outer = &outer;
            outer.add_one();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'outer' while immutably borrowed")));
}

#[test]
fn mutating_method_receiver_borrow_is_temporary() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        function main(): None {
            mut c: C = C(1);
            c.touch();
            c.touch();
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn method_call_with_expression_receiver_does_not_force_owned_args() {
    let source = r#"
        import std.io.*;
        class C {
            function use(borrow s: String): None { println(s); return None; }
        }
        function mk(): C { return C(); }
        function main(): None {
            s: String = "x";
            mk().use(s);
            println(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn immutable_receiver_cannot_call_mutating_method() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        function main(): None {
            c: C = C(1);
            c.touch();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow immutable variable 'c'")));
}

#[test]
fn mutating_method_inference_respects_short_circuit_literals() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch_flag(): Boolean { this.v += 1; return true; }
            function maybe_touch(): None {
                if (true || this.touch_flag()) {
                }
                return None;
            }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            c.maybe_touch();
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn async_borrow_capture_blocks_move_after_creation() {
    let source = r#"
        function take_borrow(borrow s: String): None { return None; }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            t: Task<None> = async { take_borrow(s); return None; };
            consume(s);
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn async_mut_borrow_capture_blocks_assignment_after_creation() {
    let source = r#"
        function main(): None {
            mut x: Integer = 1;
            t: Task<None> = async {
                r: &mut Integer = &mut x;
                return None;
            };
            x += 1;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'x' while borrowed")
            || m.contains("Cannot assign to 'x' while mutably borrowed")
    }));
}

#[test]
fn async_mut_borrow_capture_blocks_later_immutable_borrow() {
    let source = r#"
        function main(): None {
            mut x: Integer = 1;
            t: Task<None> = async {
                r: &mut Integer = &mut x;
                return None;
            };
            y: &Integer = &x;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
}

#[test]
fn async_mut_borrow_capture_blocks_later_mutable_borrow_with_correct_reason() {
    let source = r#"
        function main(): None {
            mut x: Integer = 1;
            t: Task<None> = async {
                r: &mut Integer = &mut x;
                return None;
            };
            y: &mut Integer = &mut x;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
}

#[test]
fn immutable_borrow_blocks_mutating_nested_receiver_call() {
    let source = r#"
        class B {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        class A {
            mut b: B;
            constructor(v: Integer) { this.b = B(v); }
        }
        function main(): None {
            mut a: A = A(1);
            r: &A = &a;
            a.b.touch();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'a' while immutably borrowed")));
}

#[test]
fn immutable_reference_receiver_blocks_mutating_method_call() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &C = &c;
            r.touch();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| { m.contains("Cannot call mutating method through immutable reference 'r'") }));
}

#[test]
fn mutable_reference_receiver_allows_mutating_method_call_without_mut_binding() {
    let source = r#"
        class C {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
            function get(): Integer { return this.v; }
        }
        function main(): None {
            mut c: C = C(1);
            r: &mut C = &mut c;
            r.touch();
            x: Integer = r.get();
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn immutable_reference_receiver_blocks_mutating_builtin_methods() {
    let source = r#"
        function main(): None {
            mut xs: List<Integer> = List<Integer>();
            mut m: Map<String, Integer> = Map<String, Integer>();
            mut s: Set<Integer> = Set<Integer>();
            mut r: Range<Integer> = range(0, 2);

            rxs: &List<Integer> = &xs;
            rm: &Map<String, Integer> = &m;
            rs: &Set<Integer> = &s;
            rr: &Range<Integer> = &r;

            rxs.push(1);
            rxs.set(0, 2);
            rxs.pop();
            rm.set("k", 1);
            rs.add(1);
            rs.remove(1);
            rr.next();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot call mutating method through immutable reference 'rxs'")));
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot call mutating method through immutable reference 'rm'")));
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot call mutating method through immutable reference 'rs'")));
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot call mutating method through immutable reference 'rr'")));
}

#[test]
fn mutable_reference_receiver_allows_mutating_builtin_methods_without_mut_binding() {
    let source = r#"
        function main(): None {
            mut xs: List<Integer> = List<Integer>();
            mut m: Map<String, Integer> = Map<String, Integer>();
            mut s: Set<Integer> = Set<Integer>();
            mut r: Range<Integer> = range(0, 3);

            rxs: &mut List<Integer> = &mut xs;
            rm: &mut Map<String, Integer> = &mut m;
            rs: &mut Set<Integer> = &mut s;
            rr: &mut Range<Integer> = &mut r;

            rxs.push(1);
            rxs.set(0, 2);
            x: Integer = rxs.pop();
            rm.set("k", x);
            ok1: Boolean = rs.add(x);
            ok2: Boolean = rs.remove(x);
            first: Integer = rr.next();
            more: Boolean = rr.has_next();
            require(first == 0);
            require(rm.contains("k"));
            require(ok1 || !ok1);
            require(ok2 || !ok2);
            require(more);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn immutable_borrow_blocks_nested_mutating_builtin_field_calls() {
    let source = r#"
        class Bag {
            mut xs: List<Integer>;
            mut m: Map<String, Integer>;
            mut s: Set<Integer>;
            mut r: Range<Integer>;

            constructor() {
                this.xs = List<Integer>();
                this.m = Map<String, Integer>();
                this.s = Set<Integer>();
                this.r = range(0, 2);
            }
        }

        function main(): None {
            mut bag: Bag = Bag();
            rb: &Bag = &bag;
            bag.xs.push(1);
            bag.m.set("k", 2);
            bag.s.add(3);
            bag.r.next();
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'bag' while immutably borrowed")));
}

#[test]
fn mutable_reference_receiver_allows_nested_mutating_builtin_field_calls() {
    let source = r#"
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
            mut bag: Bag = Bag();
            rb: &mut Bag = &mut bag;
            rb.xs.push(1);
            rb.xs.set(0, 2);
            value: Integer = rb.xs.pop();
            rb.m.set("k", value);
            rb.s.add(value);
            rb.s.remove(value);
            first: Integer = rb.r.next();
            require(first == 0);
            require(rb.m.contains("k"));
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn mutable_reference_index_assignments_are_allowed_without_mut_binding() {
    let source = r#"
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
    borrow_ok(source);
}

#[test]
fn immutable_reference_index_assignments_are_rejected() {
    let source = r#"
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
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign through immutable reference 'rxs'")));
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign through immutable reference 'rm'")));
}

#[test]
fn immutable_reference_deref_assignment_is_rejected() {
    let source = r#"
        function main(): None {
            mut x: Integer = 1;
            r: &Integer = &x;
            *r = 2;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign through immutable reference 'r'")));
}

#[test]
fn async_lambda_capture_blocks_nested_mutating_receiver_call() {
    let source = r#"
        class B {
            mut v: Integer;
            constructor(v: Integer) { this.v = v; }
            function touch(): None { this.v += 1; return None; }
        }
        class A {
            mut b: B;
            constructor(v: Integer) { this.b = B(v); }
        }
        function main(): None {
            mut a: A = A(1);
            f: () -> Task<None> = () => async {
                a.b.touch();
                return None;
            };
            r: &A = &a;
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot mutably borrow 'a' while immutably borrowed")));
}

#[test]
fn alias_pattern_binding_preserves_borrow_state_through_match_arm() {
    let source = r#"
        import app.Result.Ok as Success;
        import app.Result.Error as Failure;
        function consume(owned s: String): None { return None; }
        function main(result: Result<String, String>): None {
            match (result) {
                Success(value) => {
                    r: &String = &value;
                    consume(value);
                },
                Failure(err) => {
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 'value' while borrowed")));
}

#[test]
fn alias_pattern_binding_preserves_mut_borrow_state_through_match_arm() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            match (value) {
                Wrapped(inner) => {
                    r: &Integer = &inner;
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot assign to 'inner' while borrowed")));
}

#[test]
fn alias_pattern_binding_async_capture_blocks_move_in_arm() {
    let source = r#"
        import app.Result.Ok as Success;
        import app.Result.Error as Failure;
        function consume(owned s: String): None { return None; }
        function main(result: Result<String, String>): None {
            match (result) {
                Success(value) => {
                    t: Task<None> = async {
                        println(value);
                        return None;
                    };
                    consume(value);
                },
                Failure(err) => {
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 'value' while borrowed")));
}

#[test]
fn alias_pattern_binding_lambda_capture_blocks_assignment_in_arm() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            match (value) {
                Wrapped(inner) => {
                    f: () -> Integer = () => inner;
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_delayed_capture_blocks_move_before_early_return() {
    let source = r#"
        import app.Result.Ok as Success;
        import app.Result.Error as Failure;
        function consume(owned s: String): None { return None; }
        function main(result: Result<String, String>): None {
            match (result) {
                Success(value) => {
                    f: () -> String = () => value;
                    consume(value);
                    return None;
                },
                Failure(err) => {
                    return None;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 'value' while borrowed")));
}

#[test]
fn alias_pattern_binding_async_capture_survives_if_merge_assignment() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            match (value) {
                Wrapped(inner) => {
                    t: Task<None> = async { return None; };
                    if (true) {
                        t = async {
                            println(inner);
                            return None;
                        };
                    } else {
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_lambda_reassignment_survives_if_merge_assignment() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            match (value) {
                Wrapped(inner) => {
                    f: () -> Integer = () => 0;
                    if (true) {
                        f = () => inner;
                    } else {
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_async_capture_survives_else_merge_assignment() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            match (value) {
                Wrapped(inner) => {
                    t: Task<None> = async { return None; };
                    if (false) {
                    } else {
                        t = async {
                            println(inner);
                            return None;
                        };
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_capture_survives_continue_path() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut done: Boolean = false;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        f = () => inner;
                        done = true;
                        continue;
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_capture_survives_break_path() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut done: Boolean = false;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        f = () => inner;
                        done = true;
                        break;
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_capture_survives_nested_break_return_path() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (true) {
                        if (true) {
                            f = () => inner;
                            break;
                        } else {
                            return None;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_capture_survives_nested_while_branch_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut done: Boolean = false;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        if (true) {
                            f = () => inner;
                            done = true;
                            break;
                        } else {
                            done = true;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_lambda_capture_survives_while_return_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (true) {
                        if (true) {
                            f = () => inner;
                            break;
                        } else {
                            return None;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_lambda_reassignment_survives_continue_break_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut done: Boolean = false;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        if (true) {
                            f = () => inner;
                            done = true;
                            continue;
                        } else {
                            break;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_lambda_reassignment_survives_else_if_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut mode: Integer = 0;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (mode < 1) {
                        if (mode == 0) {
                            f = () => inner;
                            mode = 1;
                            continue;
                        } else if (mode == 1) {
                            break;
                        } else {
                            mode = 1;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_repeated_async_reassignment_stays_borrowed() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut i: Integer = 0;
            t: Task<None> = async { return None; };
            match (value) {
                Wrapped(inner) => {
                    while (i < 2) {
                        t = async {
                            println(inner);
                            return None;
                        };
                        i += 1;
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_async_capture_survives_for_body_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            t: Task<None> = async { return None; };
            match (value) {
                Wrapped(inner) => {
                    for (i in 2) {
                        t = async {
                            println(inner);
                            return None;
                        };
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_async_capture_survives_for_break_continue_merge() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            t: Task<None> = async { return None; };
            match (value) {
                Wrapped(inner) => {
                    for (i in 3) {
                        if (i == 0) {
                            t = async {
                                println(inner);
                                return None;
                            };
                            continue;
                        } else if (i == 1) {
                            break;
                        }
                    }
                    inner += 1;
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors.iter().any(|m| {
        m.contains("Cannot assign to 'inner' while borrowed")
            || m.contains("Cannot assign to 'inner' while mutably borrowed")
    }));
}

#[test]
fn alias_pattern_binding_lambda_capture_survives_nested_match_loop_move() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        function consume(owned x: Integer): None { return None; }
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            mut done: Boolean = false;
            f: () -> Integer = () => 0;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        match (1) {
                            1 => {
                                f = () => inner;
                                done = true;
                            },
                            _ => {
                            }
                        }
                    }
                    consume(inner);
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 'inner' while borrowed")));
}

#[test]
fn alias_pattern_binding_capture_survives_composite_control_flow_move() {
    let source = r#"
        import app.E.Boxed as Wrapped;
        function consume(owned x: Integer): None { return None; }
        enum E {
            Boxed(value: Integer)
        }
        function main(value: E): None {
            f: () -> Integer = () => 0;
            mut done: Boolean = false;
            match (value) {
                Wrapped(inner) => {
                    while (!done) {
                        if (true) {
                            match (1) {
                                1 => {
                                    f = () => inner;
                                    done = true;
                                },
                                _ => {
                                }
                            }
                        }
                    }
                    consume(inner);
                }
            }
            return None;
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 'inner' while borrowed")));
}

#[test]
fn short_circuit_or_with_true_literal_does_not_move_rhs() {
    let source = r#"
        function takes(owned s: String): Boolean { return true; }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            if (true || takes(s)) {
            }
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn short_circuit_and_with_false_literal_does_not_move_rhs() {
    let source = r#"
        function takes(owned s: String): Boolean { return true; }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            if (false && takes(s)) {
            }
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn constant_if_with_early_return_does_not_move_unreachable_path() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            if (true) {
                consume(s);
                return None;
            }
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}

#[test]
fn constructor_borrow_params_cannot_be_moved() {
    let source = r#"
        function consume(owned s: String): None { return None; }
        class Boxed {
            constructor(borrow s: String) {
                consume(s);
                return None;
            }
        }
    "#;
    let errors = borrow_errors(source);
    assert!(errors
        .iter()
        .any(|m| m.contains("Cannot move 's' while borrowed")));
}

#[test]
fn nested_module_borrow_calls_keep_argument_usable() {
    let source = r#"
        module Outer {
            module Inner {
                function keep(borrow s: String): None { return None; }
            }
        }
        function consume(owned s: String): None { return None; }
        function main(): None {
            s: String = "x";
            Outer.Inner.keep(s);
            consume(s);
            return None;
        }
    "#;
    borrow_ok(source);
}
