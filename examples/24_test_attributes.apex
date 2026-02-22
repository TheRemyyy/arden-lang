// Test 24: Function Attributes/Annotations
// Tests: @Test, @Ignore attributes for testing framework
// Note: This demonstrates the syntax. Full test runner coming soon.

import std.io.*;

// A test function marked with @Test attribute
@Test
function testAddition(): Integer {
    return 2 + 2;
}

// Another test function
@Test
function testStringConcat(): String {
    return "Hello, " + "World!";
}

// Ignored test with reason
@Test
@Ignore("Not implemented yet")
function testDivision(): Integer {
    // This test is skipped during test run
    return 10 / 2;
}

// Ignored test without reason
@Test
@Ignore
function testFeatureX(): Boolean {
    // This test is also skipped
    return true;
}

// Normal function without attributes
function helperFunction(): Integer {
    return 42;
}

function main(): None {
    println("Test Attributes Demo");
    println("====================");
    
    // For now, we call the test functions manually
    // In the future, 'apex test' will discover and run @Test functions automatically
    
    println("Running testAddition: " + to_string(testAddition()));
    println("Running testStringConcat: " + testStringConcat());
    println("Running helperFunction: " + to_string(helperFunction()));
    
    println("");
    println("Note: @Test and @Ignore attributes are parsed but not yet enforced.");
    println("Future versions will include 'apex test' command for automated testing.");
    
    return None;
}
