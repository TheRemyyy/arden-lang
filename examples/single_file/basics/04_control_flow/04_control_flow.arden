// Test 04: Control Flow
// Tests: if/else, while, for, break, continue

import std.io.*;

function main(): None {
    println("=== If/Else Tests ===");

    // Simple if
    x: Integer = 10;
    if (x > 5) {
        println("x is greater than 5");
    }

    // If-else
    y: Integer = 3;
    if (y > 5) {
        println("y is greater than 5");
    } else {
        println("y is not greater than 5");
    }

    // Nested if
    z: Integer = 7;
    if (z > 0) {
        if (z < 10) {
            println("z is between 0 and 10");
        }
    }

    // Boolean conditions
    is_valid: Boolean = true;
    if (is_valid) {
        println("Valid!");
    }

    if (!false) {
        println("Not false is true!");
    }

    // Combined conditions
    a: Integer = 5;
    b: Integer = 10;
    if (a > 0 && b > 0) {
        println("Both positive");
    }

    if (a < 0 || b > 0) {
        println("At least one condition is true");
    }

    println("");
    println("=== While Loop Tests ===");

    // Simple while
    mut counter: Integer = 0;
    while (counter < 5) {
        println("Counter loop iteration");
        counter = counter + 1;
    }

    // While with break
    mut i: Integer = 0;
    while (true) {
        i = i + 1;
        if (i >= 3) {
            break;
        }
    }
    println("Broke out of infinite loop");

    // While with continue
    mut j: Integer = 0;
    mut skipped: Integer = 0;
    while (j < 10) {
        j = j + 1;
        if (j % 2 == 0) {
            continue;
        }
        skipped = skipped + 1;
    }
    println("Continue skipped even numbers");

    println("");
    println("=== For Loop Tests ===");

    // Simple for loop
    for (n in 5) {
        println("For loop iteration");
    }

    // For with break
    for (k in 100) {
        if (k >= 3) {
            break;
        }
        println("For with break");
    }

    // Nested loops
    for (outer in 2) {
        for (inner in 2) {
            println("Nested loop");
        }
    }

    println("");
    println("=== All control flow tests passed! ===");

    return None;
}
