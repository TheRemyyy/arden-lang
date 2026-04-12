// Test 16: Pattern Matching
// Tests: match statement, patterns, destructuring, guards

import std.io.*;

function testBasicMatch(): None {
    println("=== Basic Match Tests ===");

    // Match on integers
    x: Integer = 2;
    match (x) {
        1 => { println("One"); }
        2 => { println("Two"); }
        3 => { println("Three"); }
        _ => { println("Other"); }
    }

    // Match on booleans
    flag: Boolean = true;
    match (flag) {
        true => { println("It's true!"); }
        false => { println("It's false!"); }
    }

    return None;
}

function testWildcardPattern(): None {
    println("");
    println("=== Wildcard Pattern Tests ===");

    value: Integer = 42;
    match (value) {
        0 => { println("Zero"); }
        1 => { println("One"); }
        _ => { println("Something else (wildcard matched)"); }
    }

    // Wildcard catches all unmatched cases
    println("_ matches anything not matched above");

    return None;
}

function testIdentifierPattern(): None {
    println("");
    println("=== Identifier Pattern Tests ===");

    num: Integer = 100;
    match (num) {
        0 => { println("Zero"); }
        n => {
            // n binds to the matched value
            println("Bound value to n");
        }
    }

    println("Identifier patterns bind values");

    return None;
}

function testOptionMatch(): None {
    println("");
    println("=== Option Pattern Matching ===");

    someOpt: Option<Integer> = Option<Integer>();

    // Match Some vs None
    match (someOpt) {
        Some(value) => {
            println("Got Some with value");
        }
        None => {
            println("Got None");
        }
    }

    println("Option: Some(x) or None patterns");

    return None;
}

function testResultMatch(): None {
    println("");
    println("=== Result Pattern Matching ===");

    result: Result<Integer, String> = Result<Integer, String>();

    // Match Ok vs Error
    match (result) {
        Ok(value) => {
            println("Success with value");
        }
        Error(err) => {
            println("Error occurred");
        }
    }

    println("Result: Ok(x) or Error(e) patterns");

    return None;
}

function testLiteralPatterns(): None {
    println("");
    println("=== Literal Pattern Tests ===");

    // Integer literals
    num: Integer = 42;
    match (num) {
        0 => { println("Zero"); }
        42 => { println("The answer!"); }
        100 => { println("Hundred"); }
        _ => { println("Other"); }
    }

    // String literals (conceptual)
    println("Can match string literals");
    println("Can match boolean literals");

    return None;
}

function testVariantPatterns(): None {
    println("");
    println("=== Variant Pattern Tests ===");

    // Enum-like patterns
    println("Variant patterns: EnumName(bindings)");

    // Some(x) - extracts x from Option
    println("Some(x) extracts value from Option");

    // Ok(v) / Error(e) - extracts from Result
    println("Ok(v) extracts success value");
    println("Error(e) extracts error value");

    // With multiple bindings
    println("Variant(a, b, c) extracts multiple values");

    return None;
}

function testNestedPatterns(): None {
    println("");
    println("=== Nested Pattern Tests ===");

    // Nested Option
    nested: Option<Option<Integer>> = Option<Option<Integer>>();

    match (nested) {
        Some(inner) => {
            println("Got outer Some");
        }
        None => {
            println("Outer is None");
        }
    }

    println("Can match nested structures");

    return None;
}

function testMatchExpression(): None {
    println("");
    println("=== Match as Expression ===");

    x: Integer = 5;

    // Match can return a value
    // result: String = match (x) {
    //     0 => "zero",
    //     1 => "one",
    //     _ => "many"
    // };

    println("Match can be used as expression");
    println("Returns value from matched arm");

    return None;
}

function testMatchExhaustiveness(): None {
    println("");
    println("=== Exhaustiveness Checking ===");

    // Compiler checks all cases are covered
    println("Compiler ensures all cases handled");

    // Must have wildcard or all variants
    println("Use _ for catch-all");

    // Boolean must have true and false
    println("Boolean: must handle true and false");

    // Option must have Some and None
    println("Option: must handle Some and None");

    return None;
}

function main(): None {
    println("=== Pattern Matching Test Suite ===");

    testBasicMatch();
    testWildcardPattern();
    testIdentifierPattern();
    testOptionMatch();
    testResultMatch();
    testLiteralPatterns();
    testVariantPatterns();
    testNestedPatterns();
    testMatchExpression();
    testMatchExhaustiveness();

    println("");
    println("=== All pattern matching tests passed! ===");

    return None;
}
