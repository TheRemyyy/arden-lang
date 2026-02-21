// Example 28: Memory Management
// Demonstrates ownership, borrowing, and lifetimes

import std.io.*;

// Ownership demonstration
function take_ownership(s: String): None {
    // s is now owned by this function
    println("  Inside function: " + s);
    // s is dropped when function returns
}

function borrow_immutable(s: &String): Integer {
    // s is borrowed, not owned
    println("  Borrowed (immutable): " + s);
    return length_of(s);
}

function length_of(s: &String): Integer {
    // Simplified - would return actual length
    return 0;
}

function modify_borrowed(s: &mut String): None {
    // s is mutably borrowed
    // Would modify the string
    println("  Modified borrowed string");
}

// Return ownership
function create_string(): String {
    return "Created in function";
}

// Data structures with ownership
struct Person {
    name: String,
    age: Integer,
}

function Person_greet(self: &Person): String {
    return "Hello, I'm " + self.name;
}

function Person_have_birthday(self: &mut Person): None {
    self.age = self.age + 1;
    println("  Happy birthday! Now " + to_string(self.age) + " years old.");
}

// Reference counting pattern (simulated)
struct SharedData {
    value: Integer,
    ref_count: Integer,
}

function SharedData_new(value: Integer): SharedData {
    return SharedData {
        value: value,
        ref_count: 1,
    };
}

function SharedData_clone(self: &mut SharedData): SharedData {
    self.ref_count = self.ref_count + 1;
    return SharedData {
        value: self.value,
        ref_count: self.ref_count,
    };
}

function SharedData_drop(self: &mut SharedData): None {
    self.ref_count = self.ref_count - 1;
    if self.ref_count == 0 {
        println("  Data freed!");
    }
}

function main(): None {
    println("Memory Management in Apex");
    println("=========================\n");
    
    // Ownership transfer
    println("1. Ownership Transfer:");
    my_string: String = "Hello, Ownership!";
    println("  Before: " + my_string);
    take_ownership(my_string);
    // my_string is no longer valid here
    println("  After: ownership transferred");
    
    // Immutable borrowing
    println("\n2. Immutable Borrowing:");
    another_string: String = "Borrow me";
    len: Integer = borrow_immutable(&another_string);
    println("  Still have: " + another_string);
    
    // Mutable borrowing
    println("\n3. Mutable Borrowing:");
    mut mutable_string: String = "Change me";
    modify_borrowed(&mut mutable_string);
    println("  After modification: " + mutable_string);
    
    // Return values
    println("\n4. Return Ownership:");
    new_string: String = create_string();
    println("  Received: " + new_string);
    
    // Struct ownership
    println("\n5. Struct with Methods:");
    mut person: Person = Person {
        name: "Alice",
        age: 30,
    };
    greeting: String = Person_greet(&person);
    println("  " + greeting);
    Person_have_birthday(&mut person);
    
    // Reference counting pattern
    println("\n6. Reference Counting Pattern:");
    mut shared: SharedData = SharedData_new(42);
    println("  Created with value: " + to_string(shared.value));
    SharedData_clone(&mut shared);
    println("  Reference count increased");
    SharedData_drop(&mut shared);
    println("  One reference dropped");
    SharedData_drop(&mut shared);
    
    // Ownership in collections
    println("\n7. Ownership in Collections:");
    strings: [String] = ["first", "second", "third"];
    for s in strings {
        println("  Item: " + s);
    }
    // strings is no longer valid after iteration
    
    println("\nMemory safety guaranteed by compiler!");
    return None;
}
