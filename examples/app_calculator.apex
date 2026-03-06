// Application: Scientific Calculator
// A comprehensive calculator demonstrating Apex features

import std.io.*;

// Calculator class with operations
class Calculator {
    mut lastResult: Float;
    mut memory: Float;

    constructor() {
        this.lastResult = 0.0;
        this.memory = 0.0;
    }

    // Basic arithmetic operations
    function add(a: Float, b: Float): Float {
        result: Float = a + b;
        this.lastResult = result;
        return result;
    }

    function subtract(a: Float, b: Float): Float {
        result: Float = a - b;
        this.lastResult = result;
        return result;
    }

    function multiply(a: Float, b: Float): Float {
        result: Float = a * b;
        this.lastResult = result;
        return result;
    }

    function divide(a: Float, b: Float): Float {
        if (b == 0.0) {
            println("Error: Division by zero!");
            return 0.0;
        }
        result: Float = a / b;
        this.lastResult = result;
        return result;
    }

    // Advanced operations
    function power(base: Float, exp: Float): Float {
        result: Float = Math.pow(base, exp);
        this.lastResult = result;
        return result;
    }

    function squareRoot(x: Float): Float {
        if (x < 0.0) {
            println("Error: Cannot take square root of negative number!");
            return 0.0;
        }
        result: Float = Math.sqrt(x);
        this.lastResult = result;
        return result;
    }

    function absolute(x: Float): Float {
        result: Float = Math.abs(x);
        this.lastResult = result;
        return result;
    }

    // Trigonometric functions
    function sine(x: Float): Float {
        result: Float = Math.sin(x);
        this.lastResult = result;
        return result;
    }

    function cosine(x: Float): Float {
        result: Float = Math.cos(x);
        this.lastResult = result;
        return result;
    }

    function tangent(x: Float): Float {
        result: Float = Math.tan(x);
        this.lastResult = result;
        return result;
    }

    // Logarithmic functions
    function naturalLog(x: Float): Float {
        result: Float = Math.log(x);
        this.lastResult = result;
        return result;
    }

    function log10(x: Float): Float {
        result: Float = Math.log10(x);
        this.lastResult = result;
        return result;
    }

    function exponential(x: Float): Float {
        result: Float = Math.exp(x);
        this.lastResult = result;
        return result;
    }

    // Rounding functions
    function roundDown(x: Float): Float {
        result: Float = Math.floor(x);
        this.lastResult = result;
        return result;
    }

    function roundUp(x: Float): Float {
        result: Float = Math.ceil(x);
        this.lastResult = result;
        return result;
    }

    function roundNearest(x: Float): Float {
        result: Float = Math.round(x);
        this.lastResult = result;
        return result;
    }

    // Memory operations
    function memoryClear(): None {
        this.memory = 0.0;
        println("Memory cleared");
        return None;
    }

    function memoryStore(x: Float): None {
        this.memory = x;
        println("Stored in memory");
        return None;
    }

    function memoryRecall(): Float {
        return this.memory;
    }

    function memoryAdd(x: Float): None {
        this.memory = this.memory + x;
        println("Added to memory");
        return None;
    }

    // Get last result
    function getLastResult(): Float {
        return this.lastResult;
    }
}

// Statistical functions module
module Stats {
    function average(a: Float, b: Float): Float {
        return (a + b) / 2.0;
    }

    function variance(a: Float, b: Float, mean: Float): Float {
        diff1: Float = a - mean;
        diff2: Float = b - mean;
        return (diff1 * diff1 + diff2 * diff2) / 2.0;
    }
}

// Constants
module MathConstants {
    function pi(): Float {
        return 3.14159265358979;
    }

    function e(): Float {
        return 2.71828182845905;
    }

    function goldenRatio(): Float {
        return 1.61803398874989;
    }

    function sqrt2(): Float {
        return 1.41421356237310;
    }
}

function main(): None {
    println("========================================");
    println("       APEX SCIENTIFIC CALCULATOR       ");
    println("========================================");
    println("");

    mut calc: Calculator = Calculator();

    // Basic arithmetic demo
    println("=== Basic Arithmetic ===");

    r1: Float = calc.add(10.0, 5.0);
    println("10 + 5 = 15");

    r2: Float = calc.subtract(10.0, 5.0);
    println("10 - 5 = 5");

    r3: Float = calc.multiply(10.0, 5.0);
    println("10 * 5 = 50");

    r4: Float = calc.divide(10.0, 5.0);
    println("10 / 5 = 2");

    // Advanced operations demo
    println("");
    println("=== Advanced Operations ===");

    r5: Float = calc.power(2.0, 10.0);
    println("2^10 = 1024");

    r6: Float = calc.squareRoot(144.0);
    println("sqrt(144) = 12");

    r7: Float = calc.absolute(-42.5);
    println("abs(-42.5) = 42.5");

    // Trigonometry demo
    println("");
    println("=== Trigonometry ===");

    pi: Float = MathConstants__pi();

    s1: Float = calc.sine(0.0);
    println("sin(0) = 0");

    c1: Float = calc.cosine(0.0);
    println("cos(0) = 1");

    t1: Float = calc.tangent(0.0);
    println("tan(0) = 0");

    // Logarithms demo
    println("");
    println("=== Logarithms ===");

    e: Float = MathConstants__e();

    l1: Float = calc.naturalLog(e);
    println("ln(e) = 1");

    l2: Float = calc.log10(100.0);
    println("log10(100) = 2");

    ex1: Float = calc.exponential(1.0);
    println("exp(1) = e ~ 2.718");

    // Rounding demo
    println("");
    println("=== Rounding ===");

    rd1: Float = calc.roundDown(3.7);
    println("floor(3.7) = 3");

    rd2: Float = calc.roundUp(3.2);
    println("ceil(3.2) = 4");

    rd3: Float = calc.roundNearest(3.5);
    println("round(3.5) = 4");

    // Memory demo
    println("");
    println("=== Memory Operations ===");

    calc.memoryStore(100.0);
    calc.memoryAdd(50.0);
    mem: Float = calc.memoryRecall();
    println("Memory: 150");
    calc.memoryClear();

    // Statistics demo
    println("");
    println("=== Statistics ===");

    avg: Float = Stats__average(10.0, 20.0);
    println("Average of 10 and 20 = 15");

    // Constants
    println("");
    println("=== Mathematical Constants ===");

    println("Pi = 3.14159265...");
    println("e = 2.71828182...");
    println("Golden Ratio = 1.61803398...");
    println("sqrt(2) = 1.41421356...");

    // Complex calculation
    println("");
    println("=== Complex Calculation ===");
    println("Computing: sqrt(3^2 + 4^2) [Pythagorean theorem]");

    a: Float = 3.0;
    b: Float = 4.0;
    aSquared: Float = calc.power(a, 2.0);
    bSquared: Float = calc.power(b, 2.0);
    sum: Float = calc.add(aSquared, bSquared);
    hypotenuse: Float = calc.squareRoot(sum);
    println("Result: 5 (3-4-5 right triangle)");

    println("");
    println("========================================");
    println("        Calculator Demo Complete        ");
    println("========================================");

    return None;
}
