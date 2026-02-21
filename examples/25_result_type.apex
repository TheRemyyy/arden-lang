// Example 25: Result Type for Error Handling
// Demonstrates Result<T, E> pattern for explicit error handling

import std.io.*;

// Function that might fail
function divide(a: Integer, b: Integer): Result<Integer, String> {
    if b == 0 {
        return Err("Cannot divide by zero");
    }
    return Ok(a / b);
}

function parse_number(s: String): Result<Integer, String> {
    // Simplified parsing - in real code would actually parse
    if s == "42" {
        return Ok(42);
    }
    return Err("Failed to parse: " + s);
}

// Using Results with pattern matching
function main(): None {
    // Successful division
    result1: Result<Integer, String> = divide(10, 2);
    match result1 {
        Ok(value) => println("10 / 2 = " + to_string(value)),
        Err(msg) => println("Error: " + msg),
    };
    
    // Division by zero
    result2: Result<Integer, String> = divide(10, 0);
    match result2 {
        Ok(value) => println("Result: " + to_string(value)),
        Err(msg) => println("Expected error: " + msg),
    };
    
    // Chaining operations
    println("\nChaining operations:");
    final_result: Result<Integer, String> = divide(20, 4);
    match final_result {
        Ok(value) => {
            doubled: Integer = value * 2;
            println("(20 / 4) * 2 = " + to_string(doubled));
        },
        Err(msg) => println("Error: " + msg),
    };
    
    // Parse examples
    println("\nParsing:");
    parse1: Result<Integer, String> = parse_number("42");
    match parse1 {
        Ok(num) => println("Parsed: " + to_string(num)),
        Err(e) => println("Parse error: " + e),
    };
    
    parse2: Result<Integer, String> = parse_number("not_a_number");
    match parse2 {
        Ok(num) => println("Parsed: " + to_string(num)),
        Err(e) => println("Expected parse error: " + e),
    };
    
    return None;
}
