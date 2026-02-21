// Example 29: Advanced Pattern Matching
// Deep pattern matching with enums and structures

import std.io.*;

// Complex enum for expression AST
enum Expr {
    Number(Integer),
    Variable(String),
    Add { left: Expr, right: Expr },
    Subtract { left: Expr, right: Expr },
    Multiply { left: Expr, right: Expr },
    Divide { left: Expr, right: Expr },
    Negate(Expr),
}

// Evaluate expression using pattern matching
function eval(expr: &Expr, env: &[(String, Integer)]): Integer {
    return match expr {
        Expr::Number(n) => n,
        Expr::Variable(name) => lookup_variable(env, name),
        Expr::Add { left, right } => {
            l: Integer = eval(left, env);
            r: Integer = eval(right, env);
            return l + r;
        },
        Expr::Subtract { left, right } => {
            l: Integer = eval(left, env);
            r: Integer = eval(right, env);
            return l - r;
        },
        Expr::Multiply { left, right } => {
            l: Integer = eval(left, env);
            r: Integer = eval(right, env);
            return l * r;
        },
        Expr::Divide { left, right } => {
            l: Integer = eval(left, env);
            r: Integer = eval(right, env);
            if r == 0 {
                return 0; // Error case
            }
            return l / r;
        },
        Expr::Negate(inner) => {
            v: Integer = eval(inner, env);
            return -v;
        },
    };
}

function lookup_variable(env: &[(String, Integer)], name: String): Integer {
    for pair in env {
        if pair[0] == name {
            return pair[1];
        }
    }
    return 0;
}

// Pattern for shapes with calculation
enum Shape {
    Circle { radius: Float },
    Rectangle { width: Float, height: Float },
    Triangle { base: Float, height: Float },
    Square { side: Float },
}

function calculate_area(shape: &Shape): Float {
    return match shape {
        Shape::Circle { radius } => 3.14159 * radius * radius,
        Shape::Rectangle { width, height } => width * height,
        Shape::Triangle { base, height } => 0.5 * base * height,
        Shape::Square { side } => side * side,
    };
}

function describe_shape(shape: &Shape): String {
    return match shape {
        Shape::Circle { radius } => "Circle with radius " + to_string(radius),
        Shape::Rectangle { width, height } => "Rectangle " + to_string(width) + "x" + to_string(height),
        Shape::Triangle { base, height } => "Triangle base=" + to_string(base) + " height=" + to_string(height),
        Shape::Square { side } => "Square side=" + to_string(side),
    };
}

// Optional chaining with pattern matching
function find_user(users: &[String], id: Integer): Option<String> {
    if id < 0 || id >= length_of(users) {
        return None;
    }
    return Some(users[id]);
}

function length_of(arr: &[String]): Integer {
    // Simplified
    return 3;
}

// Pattern matching on tuples (simulated with arrays)
function classify_point(point: [Integer]): String {
    x: Integer = point[0];
    y: Integer = point[1];
    
    return match true {
        _ if x == 0 && y == 0 => "Origin",
        _ if x == 0 => "On Y-axis",
        _ if y == 0 => "On X-axis",
        _ if x > 0 && y > 0 => "Quadrant I",
        _ if x < 0 && y > 0 => "Quadrant II",
        _ if x < 0 && y < 0 => "Quadrant III",
        _ => "Quadrant IV",
    };
}

// Guard patterns
enum TrafficLight {
    Red,
    Yellow,
    Green,
}

function can_go(light: TrafficLight, has_emergency: Boolean): Boolean {
    return match light {
        TrafficLight::Green => true,
        TrafficLight::Yellow => has_emergency,
        TrafficLight::Red => has_emergency,
    };
}

// Recursive pattern matching with lists
enum List<T> {
    Empty,
    Cons { head: T, tail: List<T> },
}

function sum_list(list: &List<Integer>): Integer {
    return match list {
        List::Empty => 0,
        List::Cons { head, tail } => head + sum_list(tail),
    };
}

function main(): None {
    println("Advanced Pattern Matching in Apex");
    println("=================================\n");
    
    // Expression evaluation
    println("1. Expression Evaluation:");
    expr: Expr = Expr::Add {
        left: Expr::Number(5),
        right: Expr::Multiply {
            left: Expr::Number(3),
            right: Expr::Number(4),
        },
    };
    env: [(String, Integer)] = [];
    result: Integer = eval(&expr, &env);
    println("  5 + (3 * 4) = " + to_string(result));
    
    // Negation
    neg_expr: Expr = Expr::Negate(Expr::Number(10));
    neg_result: Integer = eval(&neg_expr, &env);
    println("  -(10) = " + to_string(neg_result));
    
    // Shape matching
    println("\n2. Shape Matching:");
    shapes: [Shape] = [
        Shape::Circle { radius: 5.0 },
        Shape::Rectangle { width: 10.0, height: 20.0 },
        Shape::Triangle { base: 8.0, height: 4.0 },
        Shape::Square { side: 6.0 },
    ];
    
    for shape in shapes {
        desc: String = describe_shape(&shape);
        area: Float = calculate_area(&shape);
        println("  " + desc + " - Area: " + to_string(area));
    }
    
    // Option matching
    println("\n3. Optional Value Matching:");
    users: [String] = ["Alice", "Bob", "Charlie"];
    found: Option<String> = find_user(&users, 1);
    match found {
        Some(name) => println("  Found user: " + name),
        None => println("  User not found"),
    };
    
    not_found: Option<String> = find_user(&users, 10);
    match not_found {
        Some(name) => println("  Found user: " + name),
        None => println("  User not found (expected)"),
    };
    
    // Point classification
    println("\n4. Point Classification:");
    points: [[Integer]] = [
        [0, 0],
        [5, 0],
        [0, 5],
        [3, 4],
        [-2, 3],
        [-1, -1],
        [4, -3],
    ];
    for point in points {
        classification: String = classify_point(point);
        println("  Point (" + to_string(point[0]) + ", " + to_string(point[1]) + "): " + classification);
    }
    
    // Traffic light
    println("\n5. Traffic Light Pattern:");
    light: TrafficLight = TrafficLight::Red;
    can_pass: Boolean = can_go(light, false);
    println("  Red light, normal: " + to_string(can_pass));
    can_pass_emergency: Boolean = can_go(light, true);
    println("  Red light, emergency: " + to_string(can_pass_emergency));
    
    return None;
}
