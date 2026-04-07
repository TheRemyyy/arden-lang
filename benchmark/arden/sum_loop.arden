import std.io.*;

function main(): None {
    iterations: Integer = 50000000;
    modVal: Integer = 2147483647;

    mut x: Integer = 1;
    mut acc: Integer = 0;
    mut i: Integer = 0;

    while (i < iterations) {
        x = (x * 1664525 + 1013904223) % modVal;
        acc = acc + (x % 1024);
        i = i + 1;
    }

    println(to_string(acc));
    return None;
}
