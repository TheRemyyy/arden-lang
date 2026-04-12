// Test 26: Effect System (MVP)
// Tests: @Pure, @Io, @Thread and effect propagation rules

import std.io.*;
import std.time.*;

@Pure
function addPure(a: Integer, b: Integer): Integer {
    return a + b;
}

@Io
function logLine(msg: String): None {
    println(msg);
    return None;
}

@Io
function showComputation(): None {
    value: Integer = addPure(20, 22);
    logLine("Computed value = {value}");
    return None;
}

@Thread
function pauseShortly(): None {
    Time.sleep(10);
    return None;
}

function main(): None {
    println("=== Effect System Tests ===");

    result: Integer = addPure(2, 3);
    println("Pure function result = {result}");

    showComputation();
    pauseShortly();

    println("Effect checks passed for @Pure/@Io/@Thread");
    println("Tip: calling println inside @Pure should fail at typecheck time.");
    return None;
}
