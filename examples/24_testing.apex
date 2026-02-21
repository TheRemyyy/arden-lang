// Example 24: Testing with Apex
// Demonstrates how to write tests in Apex

import std.io.*;

// Functions to test
function add(a: Integer, b: Integer): Integer {
    return a + b;
}

function is_even(n: Integer): Boolean {
    return n % 2 == 0;
}

function factorial(n: Integer): Integer {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

// Tests
#[test]
function test_add_positive_numbers(): Boolean {
    return add(2, 3) == 5;
}

#[test]
function test_add_negative_numbers(): Boolean {
    return add(-2, -3) == -5;
}

#[test]
function test_is_even_true(): Boolean {
    return is_even(4) == true;
}

#[test]
function test_is_even_false(): Boolean {
    return is_even(3) == false;
}

#[test]
function test_factorial_base_case(): Boolean {
    return factorial(0) == 1;
}

#[test]
function test_factorial_recursive(): Boolean {
    return factorial(5) == 120;
}

#[test]
#[ignore]
function test_work_in_progress(): Boolean {
    // This test is ignored
    return false;
}

// Run: apex test
function main(): None {
    println("Run tests with: apex test");
    println("");
    println("Available tests:");
    println("  - test_add_positive_numbers");
    println("  - test_add_negative_numbers");
    println("  - test_is_even_true");
    println("  - test_is_even_false");
    println("  - test_factorial_base_case");
    println("  - test_factorial_recursive");
    return None;
}
