// Test 40: Borrow Scope Recovery
// Tests: immutable borrow in inner scope is released after scope exit

import std.io.*;

function consume(owned s: String): None {
    println("consumed = {s}");
    return None;
}

function main(): None {
    println("=== Borrow Scope Recovery ===");

    s: String = "hello";
    if (true) {
        r: &String = &s;
        println("borrowed = {s}");
    }

    // Borrow must be released here, so moving `s` is valid.
    consume(s);

    println("=== Borrow Scope Recovery OK ===");
    return None;
}
