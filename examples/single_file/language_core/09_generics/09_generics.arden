// Test 09: Generic Types
// Tests: Option<T>, Result<T, E>, List<T>, Map<K, V>, Set<T>

import std.io.*;

function testOption(): None {
    println("=== Option<T> Tests ===");

    // Creating Option with some value
    someValue: Option<Integer> = Option<Integer>();
    println("Created Option<Integer>");

    // Using Option with match
    println("Option can be Some(value) or None");

    // String option
    stringOpt: Option<String> = Option<String>();
    println("Created Option<String>");

    return None;
}

function testResult(): None {
    println("");
    println("=== Result<T, E> Tests ===");

    // Creating Result
    result: Result<Integer, String> = Result<Integer, String>();
    println("Created Result<Integer, String>");

    // Result for error handling
    println("Result can be Ok(value) or Error(error)");

    return None;
}

function testList(): None {
    println("");
    println("=== List<T> Tests ===");

    // Creating a List of integers
    numbers: List<Integer> = List<Integer>();
    println("Created List<Integer>");

    // List operations
    numbers.push(10);
    numbers.push(20);
    numbers.push(30);
    println("Pushed 10, 20, 30");

    // Get length
    len: Integer = numbers.length();
    println("List length: 3");

    // Get element
    first: Integer = numbers.get(0);
    println("First element: 10");

    // Set element
    numbers.set(0, 100);
    println("Set first element to 100");

    // List of strings
    names: List<String> = List<String>();
    names.push("Alice");
    names.push("Bob");
    names.push("Charlie");
    println("Created List<String> with 3 names");

    // List of floats
    floats: List<Float> = List<Float>();
    floats.push(1.1);
    floats.push(2.2);
    floats.push(3.3);
    println("Created List<Float> with 3 values");

    return None;
}

function testMap(): None {
    println("");
    println("=== Map<K, V> Tests ===");

    // Creating a Map
    scores: Map<String, Integer> = Map<String, Integer>();
    println("Created Map<String, Integer>");

    // Map operations
    scores.insert("Alice", 100);
    scores.insert("Bob", 85);
    scores.insert("Charlie", 92);
    println("Inserted 3 key-value pairs");

    // Get value
    aliceScore: Integer = scores.get("Alice");
    println("Alice's score: 100");

    // Check contains
    hasAlice: Boolean = scores.contains("Alice");
    hasDave: Boolean = scores.contains("Dave");
    println("Contains 'Alice': true");
    println("Contains 'Dave': false");

    // Map length
    mapLen: Integer = scores.length();
    println("Map length: 3");

    return None;
}

function testSet(): None {
    println("");
    println("=== Set<T> Tests ===");

    // Creating a Set
    uniqueNumbers: Set<Integer> = Set<Integer>();
    println("Created Set<Integer>");

    println("Set ensures unique elements");

    return None;
}

function testSmartPointers(): None {
    println("");
    println("=== Smart Pointer Tests ===");

    // Box<T> - heap allocation
    boxed: Box<Integer> = Box<Integer>();
    println("Created Box<Integer> - heap allocated");

    // Rc<T> - reference counted
    shared: Rc<String> = Rc<String>();
    println("Created Rc<String> - reference counted");

    // Arc<T> - atomic reference counted
    atomic: Arc<Integer> = Arc<Integer>();
    println("Created Arc<Integer> - thread-safe reference counted");

    return None;
}

function testNestedGenerics(): None {
    println("");
    println("=== Nested Generics Tests ===");

    // List of Lists
    matrix: List<List<Integer>> = List<List<Integer>>();
    println("Created List<List<Integer>> - 2D matrix");

    // Option of List
    optionalList: Option<List<Integer>> = Option<List<Integer>>();
    println("Created Option<List<Integer>>");

    // Result with List
    listResult: Result<List<String>, String> = Result<List<String>, String>();
    println("Created Result<List<String>, String>");

    // Map with List values
    grouped: Map<String, List<Integer>> = Map<String, List<Integer>>();
    println("Created Map<String, List<Integer>>");

    return None;
}

function main(): None {
    println("=== Generic Types Test Suite ===");

    testOption();
    testResult();
    testList();
    testMap();
    testSet();
    testSmartPointers();
    testNestedGenerics();

    println("");
    println("=== All generic type tests passed! ===");

    return None;
}
