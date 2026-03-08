import std.io.*;

// Typical output shape from `apex bindgen`.
extern(c) function puts(msg: String): Integer;
extern(c) function printf(fmt: String, ...): Integer;

function main(): None {
    puts("Generated binding call");
    printf("Value: %d\n", 42);
    return None;
}
