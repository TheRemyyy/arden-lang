// Test 21: Type Conversion Functions
// Tests: to_int, to_float, to_string

import std.io.*;
import std.math.*;
import std.string.*;

function main(): None {
    println("=== Type Conversion Tests ===");

    // 1. Float to Integer (truncation)
    f: Float = 3.99;
    i: Integer = to_int(f);
    println("Float 3.99 to Integer: {i}");

    // 2. Integer to Float
    i2: Integer = 42;
    f2: Float = to_float(i2);
    println("Integer 42 to Float: {f2}");

    // 3. String conversions
    // From Integer
    s1: String = to_string(12345);
    println("Integer 12345 to String: {s1}");

    // From Float
    s2: String = to_string(3.14159);
    println("Float 3.14159 to String: {s2}");

    // From Boolean
    s3: String = to_string(true);
    s4: String = to_string(false);
    println("Boolean true to String: {s3}");
    println("Boolean false to String: {s4}");

    // 4. Practical usage in math
    radius: Integer = 10;
    // Calculation requires float
    area: Float = Math.pi() * to_float(radius) * to_float(radius);
    println("Area of circle (radius 10): {area}");

    // 5. String manipulation with conversions
    count: Integer = 5;
    msg: String = Str.concat("The count is: ", to_string(count));
    println(msg);

    println("");
    println("=== All conversion tests passed! ===");
    return None;
}
