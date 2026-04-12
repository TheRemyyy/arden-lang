// Test 22: Command Line Arguments
// Tests: Args.count, Args.get

import std.io.*;
import std.args.*;

function main(): None {
    println("=== Command Line Arguments Test ===");

    // 1. Get argument count
    count: Integer = Args.count();
    println("Number of arguments: {count}");

    // 2. Iterate through arguments
    println("Arguments list:");
    for (i in 0..count) {
        arg: String = Args.get(i);
        println("  [{i}]: {arg}");
    }

    // 3. Simple logic based on args
    if (count > 1) {
        println("");
        firstArg: String = Args.get(1);
        println("The first user argument is: {firstArg}");
    } else {
        println("");
        println("No user arguments provided.");
    }

    println("");
    println("=== End of Args test ===");
    return None;
}
