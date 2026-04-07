// Test 30: Extern Variadic Interop
// Tests: extern variadic declaration and C printf call

import std.io.*;

extern function printf(fmt: String, ...): Integer;

function main(): None {
    println("=== Extern Variadic printf ===");
    printf("value=%lld, hex=0x%llx\n", 42, 42);
    printf("float=%.2f\n", 3.14159);
    println("done");
    return None;
}
