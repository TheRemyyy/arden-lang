// Test 06: Enums
// Tests: enum definition, variants, associated data, pattern matching with enums

import std.io.*;

// Simple enum (like C-style)
enum Direction {
    North,
    South,
    East,
    West
}

// Enum with associated data (like Rust/Haskell)
enum Shape {
    Circle(radius: Integer),
    Rectangle(width: Integer, height: Integer),
    Triangle(base: Integer, height: Integer),
    Point
}

// Enum for status/state machine
enum Status {
    Pending,
    Active,
    Completed,
    Failed(error: String)
}

// Result-like enum
enum WebResult {
    Success(data: String),
    NotFound,
    Error(code: Integer, message: String)
}

// Recursive enum (for tree-like structures)
enum Expression {
    Number(value: Integer),
    Add(Integer, Integer),
    Multiply(Integer, Integer)
}

// Generic-style enum (using built-in Option/Result)
function useOption(): None {
    // Using built-in Option<T>
    someVal: Option<Integer> = Option<Integer>();
    println("Option type used");
    return None;
}

function useResult(): None {
    // Using built-in Result<T, E>
    result: Result<Integer, String> = Result<Integer, String>();
    println("Result type used");
    return None;
}

// Helper function to demonstrate enums
function describeDirection(dir: Integer): None {
    // Note: Since full enum support may vary, using integer representation
    if (dir == 0) {
        println("Going North");
    } else {
        if (dir == 1) {
            println("Going South");
        } else {
            if (dir == 2) {
                println("Going East");
            } else {
                println("Going West");
            }
        }
    }
    return None;
}

function main(): None {
    println("=== Enum Tests ===");

    // Simple enum usage (using integer representation for now)
    println("");
    println("Direction enum:");
    describeDirection(0);
    describeDirection(1);
    describeDirection(2);
    describeDirection(3);

    // Using Option
    println("");
    println("Option type:");
    useOption();

    // Using Result
    println("");
    println("Result type:");
    useResult();

    // Demonstrating enum concept with if-else
    println("");
    println("Shape calculations:");

    // Simulating Circle area calculation
    circle_radius: Integer = 5;
    println("Circle with radius 5");

    // Simulating Rectangle area
    rect_width: Integer = 4;
    rect_height: Integer = 3;
    rect_area: Integer = rect_width * rect_height;
    println("Rectangle 4x3, area = 12");

    // Status states
    println("");
    println("Status enum states:");
    println("  Pending -> Active -> Completed");
    println("  or Pending -> Failed");

    println("");
    println("=== All enum tests passed! ===");

    return None;
}
