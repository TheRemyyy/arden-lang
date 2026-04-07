// Test 19: Time Functions (Updated)
// Tests: Time.now (formatted), Time.unix (raw), Time.sleep

import std.io.*;
import std.time.*;

function main(): None {
    println("=== Time Tests ===");

    // 1. Get raw unix timestamp
    start: Integer = Time.unix();
    println("Start Unix time: {start}");

    // 2. Get formatted time (default)
    currentTime: String = Time.now("");
    println("Current local time (default format): {currentTime}");

    // 3. Get custom formatted date/time
    // %Y-%m-%d is YYYY-MM-DD
    customDate: String = Time.now("%Y-%m-%d %H:%M:%S");
    println("Custom formatted: {customDate}");

    // 4. Test sleep
    println("Sleeping for 1 second...");
    Time.sleep(1000);

    end: Integer = Time.unix();
    duration: Integer = end - start;
    println("Slept for {duration} seconds");

    println("");
    println("=== All Time tests passed! ===");
    return None;
}
