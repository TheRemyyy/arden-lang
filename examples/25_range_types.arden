// Range Types Example
// Demonstrates the Range<T> type with iterator support

import std.io.*;

function main(): None {
    println("=== Range Types Example ===");
    
    // Basic range iteration
    println("\n1. Basic range(0, 5):");
    r: Range<Integer> = range(0, 5);
    while (r.has_next()) {
        val: Integer = r.next();
        println("  Value: " + to_string(val));
    }
    
    // Range with step
    println("\n2. Range with step(0, 10, 2):");
    r2: Range<Integer> = range(0, 10, 2);
    while (r2.has_next()) {
        val2: Integer = r2.next();
        println("  Value: " + to_string(val2));
    }
    
    // Counting down
    println("\n3. Counting down(10, 0, -1):");
    r3: Range<Integer> = range(10, 0, -1);
    while (r3.has_next()) {
        val3: Integer = r3.next();
        println("  Value: " + to_string(val3));
    }
    
    // Calculate sum using range
    println("\n4. Calculate sum using range:");
    mut sum: Integer = 0;
    r4: Range<Integer> = range(1, 6);
    while (r4.has_next()) {
        sum = sum + r4.next();
    }
    println("  Sum of 1 to 5: " + to_string(sum));

    println("\n5. Float range(0.5, 2.0, 0.5):");
    r5: Range<Float> = range(0.5, 2.0, 0.5);
    while (r5.has_next()) {
        val5: Float = r5.next();
        println("  Value: " + to_string(val5));
    }
    
    println("\n=== Range Example Complete ===");
    return None;
}
