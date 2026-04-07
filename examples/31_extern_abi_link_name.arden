// Test 31: Extern ABI + Link Name
// Tests: extern(c) and symbol aliasing with link_name

import std.io.*;

extern(c, "atoll") function c_atoll(input: String): Integer;

function main(): None {
    println("=== Extern ABI + Link Name ===");
    parsed: Integer = c_atoll("123456789");
    println("atoll via alias = {parsed}");
    println("done");
    return None;
}
