// Test 13: Error Handling and Try Operator
// Tests: Option, Result, try operator (?), require, error propagation

import std.io.*;

// Function that may fail - returns Option
function findElement(index: Integer): Option<Integer> {
    // Simulated array lookup
    opt: Option<Integer> = Option<Integer>();
    return opt;
}

// Function that may error - returns Result
function divide(a: Integer, b: Integer): Result<Integer, String> {
    result: Result<Integer, String> = Result<Integer, String>();
    return result;
}

// Function that uses try operator to propagate errors
function calculateRatio(x: Integer, y: Integer): Result<Float, String> {
    // Would use ? to propagate errors: value: Integer = divide(x, y)?;
    result: Result<Float, String> = Result<Float, String>();
    return result;
}

function testOption(): None {
    println("=== Option<T> Error Handling ===");

    // Creating Some value
    someValue: Option<Integer> = Option<Integer>();
    println("Created Option with value");

    // Creating None
    noValue: Option<Integer> = Option<Integer>();
    println("Created empty Option (None)");

    // Checking if has value
    println("Option.is_some() checks for value");
    println("Option.is_none() checks for no value");

    // Safe unwrap
    println("Option.unwrap() extracts value (panics if None)");

    // Pattern matching with Option
    println("Can pattern match: Some(x) or None");

    return None;
}

function testResult(): None {
    println("");
    println("=== Result<T, E> Error Handling ===");

    // Creating Ok value
    okResult: Result<Integer, String> = Result<Integer, String>();
    println("Created Ok result");

    // Creating Error value
    errResult: Result<Integer, String> = Result<Integer, String>();
    println("Created Error result");

    // Checking result type
    println("Result.is_ok() checks for success");
    println("Result.is_error() checks for failure");

    // Safe unwrap
    println("Result.unwrap() extracts Ok value (panics if Error)");

    // Pattern matching with Result
    println("Can pattern match: Ok(value) or Error(err)");

    return None;
}

function testTryOperator(): None {
    println("");
    println("=== Try Operator (?) Tests ===");

    // The ? operator unwraps Ok/Some or returns Error/None early
    println("? operator: unwraps or early returns");

    // Example of try operator usage:
    // function mayFail(): Result<Integer, String> {
    //     value: Integer = otherFunction()?;  // Returns Error if fails
    //     return Ok(value * 2);
    // }

    println("Simplifies error propagation");
    println("No need for explicit match/if-else");

    return None;
}

function testRequire(): None {
    println("");
    println("=== Require Assertions ===");

    // Basic require
    x: Integer = 10;
    require(x > 0);
    println("require(x > 0) passed");

    // Require with message
    y: Integer = 5;
    require(y >= 0, "y must be non-negative");
    println("require with message passed");

    // Multiple requires
    a: Integer = 1;
    b: Integer = 2;
    require(a > 0);
    require(b > a);
    require(a + b < 100);
    println("Multiple requires passed");

    // Require in function preconditions
    println("Use require() for function preconditions");

    return None;
}

function testErrorPropagation(): None {
    println("");
    println("=== Error Propagation Patterns ===");

    // Manual error handling
    println("Manual: check each result with match");

    // Using try operator
    println("? operator: automatic propagation");

    // Chain of operations
    println("Can chain: a()?.b()?.c()");

    // Converting between Option and Result
    println("Option -> Result: ok_or('error message')");
    println("Result -> Option: ok() discards error");

    return None;
}

// Function demonstrating error handling patterns
function processData(input: Integer): Result<String, String> {
    // Validate input
    if (input < 0) {
        result: Result<String, String> = Result<String, String>();
        return result;
    }

    // Process
    if (input == 0) {
        result: Result<String, String> = Result<String, String>();
        return result;
    }

    // Success
    result: Result<String, String> = Result<String, String>();
    return result;
}

function testRealWorldPatterns(): None {
    println("");
    println("=== Real-World Error Patterns ===");

    // File operations (simulated)
    println("File operations return Result");

    // Network operations (simulated)
    println("Network calls return Result");

    // Parsing (simulated)
    println("Parsing returns Option or Result");

    // Database operations (simulated)
    println("DB queries return Result");

    return None;
}

function main(): None {
    println("=== Error Handling Test Suite ===");

    testOption();
    testResult();
    testTryOperator();
    testRequire();
    testErrorPropagation();
    testRealWorldPatterns();

    println("");
    println("=== All error handling tests passed! ===");

    return None;
}
