// Test 28: Async Runtime Control
// Tests: true task runtime, Task.is_done(), Task.await_timeout(), Task.cancel()

import std.io.*;
import std.time.*;

async function delayedValue(ms: Integer, value: Integer): Task<Integer> {
    Time.sleep(ms);
    return value;
}

async function runAsyncControl(): Task<None> {
    println("=== Async Runtime Control Tests ===");

    t: Task<Integer> = delayedValue(120, 42);
    println("Task started");

    doneEarly: Boolean = t.is_done();
    println("is_done() immediately = {doneEarly}");

    early: Option<Integer> = t.await_timeout(10);
    finishedEarly: Boolean = early.is_some();
    println("await_timeout(10) returned Some = {finishedEarly}");
    println("Expected: false on a healthy async runtime.");

    finalValue: Integer = await t;
    println("await task result = {finalValue}");

    t2: Task<Integer> = delayedValue(500, 7);
    println("Second task started");
    t2.cancel();
    println("cancel() called");

    canceledDone: Boolean = t2.is_done();
    println("is_done() after cancel = {canceledDone}");

    canceledValue: Integer = await t2;
    println("await canceled task value (runtime default) = {canceledValue}");

    return None;
}

function main(): None {
    controller: Task<None> = runAsyncControl();
    await controller;
    return None;
}
