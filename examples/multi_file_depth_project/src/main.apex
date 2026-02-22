// Main entry point - namespace: main
package main;

// Import std.io for println
import std.io.*;

// Import specific functions
import utils.math.factorial;
import utils.math.power;
import utils.math.is_prime;

// Import all from strings
import utils.strings.*;

function main(): None {
    println("=== Java-Style Namespace Demo ===");
    println("");
    
    // Using imported math functions
    println("Math functions (imported individually):");
    println("----------------------------------------");
    
    fact_result: Integer = factorial(5);
    println("factorial(5) = " + to_string(fact_result));
    
    pow_result: Float = power(2.0, 10);
    println("power(2.0, 10) = " + to_string(pow_result));
    
    is_17_prime: Boolean = is_prime(17);
    println("is_prime(17) = " + to_string(is_17_prime));
    
    println("");
    
    // Using wildcard imported string functions
    println("String functions (wildcard import):");
    println("-----------------------------------");
    
    stars: String = repeat("*", 15);
    println("repeat('*', 15) = " + stars);
    
    greeting: String = greet("Apex");
    println(greeting);
    
    padded: String = pad_left("42", 6, "0");
    println("pad_left('42', 6, '0') = " + padded);
    
    println("");
    println("=== Demo Complete ===");
    
    return None;
}
