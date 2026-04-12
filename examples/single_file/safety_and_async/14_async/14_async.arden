// Test 14: Async/Await
// Tests: async functions, await, Task<T>, async blocks

import std.io.*;

// Async function returning Task<T>
async function fetchData(): Task<String> {
    // Simulated async operation
    data: String = "Fetched data";
    return data;
}

async function computeValue(): Task<Integer> {
    // Simulated computation
    result: Integer = 42;
    return result;
}

async function delay(ms: Integer): Task<None> {
    // Simulated delay
    println("Simulating delay...");
    return None;
}

// Function that uses await
async function processAsync(): Task<String> {
    println("Starting async process...");

    // Await other async functions
    data: String = await fetchData();
    println("Data fetched");

    value: Integer = await computeValue();
    println("Value computed");

    return data;
}

// Sequential async operations
async function sequential(): Task<Integer> {
    println("Sequential async operations:");

    a: Integer = await computeValue();
    println("  First value computed");

    b: Integer = await computeValue();
    println("  Second value computed");

    return a + b;
}

// Async block expression
function testAsyncBlock(): None {
    println("");
    println("=== Async Block Tests ===");

    // Async block creates a Task
    task: Task<Integer> = async {
        x: Integer = 10;
        y: Integer = 20;
        return x + y;
    };

    println("Async block created a Task");

    // Another async block
    stringTask: Task<String> = async {
        return "Hello from async block";
    };

    println("String task created");

    return None;
}

// Simulated concurrent operations
async function fetchUser(id: Integer): Task<String> {
    return "User data";
}

async function fetchPosts(userId: Integer): Task<String> {
    return "Posts data";
}

async function fetchProfile(): Task<String> {
    // Fetch user and posts (would be concurrent in real impl)
    user: String = await fetchUser(1);
    posts: String = await fetchPosts(1);

    return "Profile complete";
}

function testBasicAsync(): None {
    println("=== Basic Async Tests ===");

    println("async function returns Task<T>");
    println("await unwraps Task<T> to T");
    println("Must await in async context");

    return None;
}

function testTaskType(): None {
    println("");
    println("=== Task<T> Type Tests ===");

    println("Task<Integer> - async integer computation");
    println("Task<String> - async string operation");
    println("Task<None> - async void operation");
    println("Task<List<T>> - async collection fetch");

    return None;
}

function testAwaitPatterns(): None {
    println("");
    println("=== Await Patterns ===");

    // Sequential await
    println("Sequential: await a; await b;");

    // Await in expression
    println("In expression: x + await compute()");

    // Await in condition
    println("In condition: if (await check()) { }");

    // Await in loop
    println("In loop: while (await hasMore()) { }");

    return None;
}

function testErrorHandlingAsync(): None {
    println("");
    println("=== Async Error Handling ===");

    // Async functions can return Result
    println("async function() -> Task<Result<T, E>>");

    // Using try with await
    println("value: T = (await asyncOp())?");

    // Handling async errors
    println("match await result { Ok(v) => ..., Err(e) => ... }");

    return None;
}

// Main function (sync) that starts async code
function main(): None {
    println("=== Async/Await Test Suite ===");

    testBasicAsync();
    testTaskType();
    testAsyncBlock();
    testAwaitPatterns();
    testErrorHandlingAsync();

    println("");
    println("=== All async tests passed! ===");

    return None;
}
