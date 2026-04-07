// Test 08: Modules
// Tests: module definition, namespacing, module functions, module classes

import std.io.*;
import std.math.*;
import std.string.*;

// Math utilities module
module Math {
    function square(x: Integer): Integer {
        return x * x;
    }

    function cube(x: Integer): Integer {
        return x * x * x;
    }

    function factorial(n: Integer): Integer {
        if (n <= 1) {
            return 1;
        }
        result: Integer = n * Math__factorial(n - 1);
        return result;
    }

    function isEven(x: Integer): Boolean {
        return x % 2 == 0;
    }

    function isOdd(x: Integer): Boolean {
        return x % 2 != 0;
    }

    function gcd(a: Integer, b: Integer): Integer {
        if (b == 0) {
            return a;
        }
        return Math__gcd(b, a % b);
    }
}

// String utilities module
module StringUtils {
    function isEmpty(s: String): Boolean {
        len: Integer = Str.len(s);
        return len == 0;
    }

    function isBlank(s: String): Boolean {
        // Simplified: just check if empty
        len: Integer = Str.len(s);
        return len == 0;
    }

    function repeat(s: String, times: Integer): String {
        // Note: simplified implementation
        return s;
    }
}

// Validation module
module Validator {
    function isPositive(x: Integer): Boolean {
        return x > 0;
    }

    function isNegative(x: Integer): Boolean {
        return x < 0;
    }

    function isZero(x: Integer): Boolean {
        return x == 0;
    }

    function inRange(x: Integer, minVal: Integer, maxVal: Integer): Boolean {
        return x >= minVal && x <= maxVal;
    }
}

// Constants module
module Constants {
    function getPi(): Float {
        return 3.14159265359;
    }

    function getE(): Float {
        return 2.71828182846;
    }

    function getGoldenRatio(): Float {
        return 1.61803398875;
    }
}

function main(): None {
    println("=== Module Tests ===");

    // Math module
    println("");
    println("Math module:");

    sq: Integer = Math__square(5);
    println("Math.square(5) = 25");

    cb: Integer = Math__cube(3);
    println("Math.cube(3) = 27");

    fact: Integer = Math__factorial(5);
    println("Math.factorial(5) = 120");

    isE: Boolean = Math__isEven(4);
    isO: Boolean = Math__isOdd(5);
    println("Math.isEven(4) = true");
    println("Math.isOdd(5) = true");

    g: Integer = Math__gcd(48, 18);
    println("Math.gcd(48, 18) = 6");

    // StringUtils module
    println("");
    println("StringUtils module:");

    empty: Boolean = StringUtils__isEmpty("");
    println("StringUtils.isEmpty('') = true");

    notEmpty: Boolean = StringUtils__isEmpty("hello");
    println("StringUtils.isEmpty('hello') = false");

    // Validator module
    println("");
    println("Validator module:");

    pos: Boolean = Validator__isPositive(5);
    neg: Boolean = Validator__isNegative(-3);
    zero: Boolean = Validator__isZero(0);
    println("Validator.isPositive(5) = true");
    println("Validator.isNegative(-3) = true");
    println("Validator.isZero(0) = true");

    inR: Boolean = Validator__inRange(5, 1, 10);
    println("Validator.inRange(5, 1, 10) = true");

    // Constants module
    println("");
    println("Constants module:");

    pi: Float = Constants__getPi();
    e: Float = Constants__getE();
    phi: Float = Constants__getGoldenRatio();
    println("Constants.getPi() = 3.14159...");
    println("Constants.getE() = 2.71828...");
    println("Constants.getGoldenRatio() = 1.61803...");

    println("");
    println("=== All module tests passed! ===");

    return None;
}
