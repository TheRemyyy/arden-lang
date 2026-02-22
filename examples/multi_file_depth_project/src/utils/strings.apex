// String utilities - namespace: utils.strings
package utils.strings;

import std.string.*;

function repeat(s: String, times: Integer): String {
    mut result: String = "";
    mut i: Integer = 0;
    while (i < times) {
        result = result + s;
        i = i + 1;
    }
    return result;
}

function pad_left(s: String, length: Integer, pad_char: String): String {
    mut result: String = s;
    mut pad_count: Integer = length - Str.len(s);
    if (pad_count > 0) {
        mut i: Integer = 0;
        while (i < pad_count) {
            result = pad_char + result;
            i = i + 1;
        }
    }
    return result;
}

function greet(name: String): String {
    return "Hello, " + name + "!";
}

function truncate(s: String, max_len: Integer): String {
    // Simplified version without substring
    if (Str.len(s) <= max_len) {
        return s;
    }
    return "[truncated]";
}
