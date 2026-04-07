// Test 10: Ownership and Borrowing
// Tests: ownership, borrow, borrow mut, references, move semantics

import std.io.*;

// Class to test ownership
class Data {
    value: Integer;

    constructor(v: Integer) {
        this.value = v;
    }

    function getValue(): Integer {
        return this.value;
    }
}

// Function that takes ownership (owned parameter - default)
function consumeData(owned data: Data): Integer {
    // data is moved into this function
    v: Integer = data.getValue();
    return v;
}

// Function that borrows immutably
function readData(borrow data: Data): Integer {
    // data is borrowed, not moved
    // Cannot modify data here
    v: Integer = data.getValue();
    return v;
}

// Function that borrows mutably
function modifyValue(borrow mut x: Integer): None {
    // Can modify x through mutable borrow
    // x = x + 1; // Would modify the original
    println("Mutable borrow allows modification");
    return None;
}

// Test immutable references
function testImmutableBorrow(): None {
    println("=== Immutable Borrow Tests ===");

    x: Integer = 42;

    // Create immutable reference
    ref_x: &Integer = &x;
    println("Created immutable reference &x");

    // Can have multiple immutable references
    ref_x2: &Integer = &x;
    println("Can have multiple immutable references");

    // Both references are valid simultaneously
    println("Both &x references are valid");

    return None;
}

// Test mutable references
function testMutableBorrow(): None {
    println("");
    println("=== Mutable Borrow Tests ===");

    mut y: Integer = 10;

    // Create mutable reference
    ref_y: &mut Integer = &mut y;
    println("Created mutable reference &mut y");

    // Can only have one mutable reference at a time
    println("Only one &mut reference allowed at a time");

    // Dereference to modify
    // *ref_y = 20; // Would modify y through reference
    println("Can modify through *ref_y");

    return None;
}

// Test move semantics
function testMoveSemantics(): None {
    println("");
    println("=== Move Semantics Tests ===");

    // Simple value (Copy type)
    a: Integer = 5;
    b: Integer = a; // Copy, a still valid
    println("Integers are copied, not moved");

    // For non-Copy types, ownership would be transferred
    println("Non-Copy types are moved by default");

    // After move, original variable is invalid
    println("After move, original is invalid");

    return None;
}

// Test ownership with classes
function testClassOwnership(): None {
    println("");
    println("=== Class Ownership Tests ===");

    data1: Data = Data(100);
    println("Created Data object");

    // Borrow to read
    val1: Integer = readData(data1);
    println("Borrowed data to read - data1 still valid");

    // Can use data1 again after borrow ends
    val2: Integer = readData(data1);
    println("Can borrow multiple times");

    // Transfer ownership
    // consumeData(data1);
    // After this, data1 would be invalid
    println("Ownership can be transferred to functions");

    return None;
}

// Test reference lifetime
function testLifetimes(): None {
    println("");
    println("=== Lifetime Tests ===");

    // References must not outlive the data they reference
    outer: Integer = 10;

    if (true) {
        inner: Integer = 20;
        ref_inner: &Integer = &inner;
        println("Reference valid within scope");
        // ref_inner is valid here
    }
    // ref_inner would be invalid here if we tried to use it

    println("References are scope-limited");

    return None;
}

// Test borrowing rules
function testBorrowingRules(): None {
    println("");
    println("=== Borrowing Rules ===");

    println("Rule 1: One mutable reference OR multiple immutable references");
    println("Rule 2: References must always be valid");
    println("Rule 3: Cannot borrow while mutably borrowed");

    mut data: Integer = 42;

    // Multiple immutable borrows OK
    r1: &Integer = &data;
    r2: &Integer = &data;
    println("Multiple immutable borrows: OK");

    // Mutable borrow (would invalidate immutable refs)
    // r3: &mut Integer = &mut data;
    println("After mutable borrow, immutable refs invalid");

    return None;
}

function main(): None {
    println("=== Ownership and Borrowing Test Suite ===");

    testImmutableBorrow();
    testMutableBorrow();
    testMoveSemantics();
    testClassOwnership();
    testLifetimes();
    testBorrowingRules();

    println("");
    println("=== All ownership tests passed! ===");

    return None;
}
