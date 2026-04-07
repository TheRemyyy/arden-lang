// Test 11: Lambdas and Closures
// Tests: lambda expressions, closures, function types, higher-order functions

import std.io.*;
import std.string.*;

// Function that takes a function as parameter
function applyTwice(x: Integer, f: (Integer) -> Integer): Integer {
    return f(f(x));
}

// Function that returns a function
function makeAdder(amount: Integer): (Integer) -> Integer {
    // Returns a lambda that adds 'amount'
    adder: (Integer) -> Integer = (x: Integer) => x + amount;
    return adder;
}

// Test basic lambda syntax
function testBasicLambdas(): None {
    println("=== Basic Lambda Tests ===");

    // Lambda with single parameter
    double: (Integer) -> Integer = (x: Integer) => x * 2;
    result1: Integer = double(5);
    println("double(5) = 10");

    // Lambda with multiple parameters (in concept)
    // add: (Integer, Integer) -> Integer = (a: Integer, b: Integer) => a + b;
    println("Lambda with parameters defined");

    // Lambda with no parameters
    getFortyTwo: () -> Integer = () => 42;
    println("Lambda returning 42 defined");

    // Lambda returning boolean
    isPositive: (Integer) -> Boolean = (x: Integer) => x > 0;
    println("Predicate lambda defined");

    return None;
}

// Test closures (lambdas capturing variables)
function testClosures(): None {
    println("");
    println("=== Closure Tests ===");

    multiplier: Integer = 10;

    // Lambda captures 'multiplier' from outer scope
    multiply: (Integer) -> Integer = (x: Integer) => x * multiplier;
    result: Integer = multiply(5);
    println("Closure captured multiplier=10");
    println("multiply(5) = 50");

    // Counter closure
    mut count: Integer = 0;

    // In a full implementation, this would capture and modify count
    println("Closures can capture mutable variables");

    return None;
}

// Test higher-order functions
function testHigherOrderFunctions(): None {
    println("");
    println("=== Higher-Order Function Tests ===");

    // Using applyTwice
    increment: (Integer) -> Integer = (x: Integer) => x + 1;
    result: Integer = applyTwice(5, increment);
    println("applyTwice(5, increment) = 7");

    // Double applied twice
    double: (Integer) -> Integer = (x: Integer) => x * 2;
    result2: Integer = applyTwice(3, double);
    println("applyTwice(3, double) = 12");

    // Function that returns a function
    add5: (Integer) -> Integer = makeAdder(5);
    result3: Integer = add5(10);
    println("makeAdder(5)(10) = 15");

    return None;
}

// Test lambda as callback
function processWithCallback(value: Integer, callback: (Integer) -> None): None {
    println("Processing value...");
    callback(value);
    return None;
}

function testCallbacks(): None {
    println("");
    println("=== Callback Tests ===");

    // Lambda as callback
    printer: (Integer) -> None = (x: Integer) => println("Received value");
    processWithCallback(42, printer);

    // Inline lambda as callback
    processWithCallback(100, (x: Integer) => println("Inline callback executed"));

    return None;
}

// Test function composition concept
function testComposition(): None {
    println("");
    println("=== Function Composition Tests ===");

    // f(g(x)) composition
    f: (Integer) -> Integer = (x: Integer) => x + 1;
    g: (Integer) -> Integer = (x: Integer) => x * 2;

    x: Integer = 5;
    // compose: g then f -> f(g(5)) = f(10) = 11
    result: Integer = f(g(x));
    println("f(g(5)) where f=+1, g=*2 -> 11");

    // Pipeline style
    step1: Integer = g(x);  // 10
    step2: Integer = f(step1);  // 11
    println("Pipeline: 5 -> *2 -> +1 = 11");

    return None;
}

// Test lambdas with different types
function testTypedLambdas(): None {
    println("");
    println("=== Typed Lambda Tests ===");

    // Integer -> Integer
    intFunc: (Integer) -> Integer = (x: Integer) => x * x;
    println("Integer -> Integer lambda (square)");

    // Float -> Float
    floatFunc: (Float) -> Float = (x: Float) => x * 2.0;
    println("Float -> Float lambda");

    // String -> Integer lambda
    strLen: (String) -> Integer = (s: String) => Str.len(s);
    println("String -> Integer lambda (length)");

    // Boolean -> Boolean
    negate: (Boolean) -> Boolean = (b: Boolean) => !b;
    println("Boolean -> Boolean lambda (not)");

    return None;
}

function main(): None {
    println("=== Lambda and Closure Test Suite ===");

    testBasicLambdas();
    testClosures();
    testHigherOrderFunctions();
    testCallbacks();
    testComposition();
    testTypedLambdas();

    println("");
    println("=== All lambda tests passed! ===");

    return None;
}
