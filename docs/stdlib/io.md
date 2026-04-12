# Input / Output (I/O)

## Why This Matters

I/O is the most common runtime boundary. Clear behavior here prevents silent data loss and confusing failures.

## Console I/O

Import:

```arden
import std.io.*;
```

### `print(...): None`

Writes scalar value(s) without trailing newline.

### `println(...): None`

Writes scalar value(s) with trailing newline.

Current display support includes:

- `Integer`, `Float`, `Boolean`, `String`, `Char`, `None`
- `Option<T>` and `Result<T, E>` when payload types are display-compatible

### `read_line(): String`

Reads one line from stdin.

```arden
import std.io.*;

function main(): None {
    print("Enter name: ");
    name: String = read_line();
    println("Hello, " + name);
    return None;
}
```

Display with `Option`/`Result` payloads:

```arden
import std.io.*;

function main(): None {
    ok: Result<Integer, String> = Result.ok(7);
    maybe: Option<String> = Option.some("hello");
    println("ok={ok}, maybe={maybe}");
    return None;
}
```

## File APIs (`File.*`)

Import:

```arden
import std.fs.*;
```

### `File.read(path: String): String`

Reads full text file. Missing/invalid input fails explicitly.

### `File.write(path: String, content: String): Boolean`

Writes full content (overwrite semantics). Returns `true` on successful write+close.

### `File.exists(path: String): Boolean`

Checks for readable regular file.

### `File.delete(path: String): Boolean`

Deletes regular file.

```arden
import std.fs.*;
import std.io.*;

function main(): None {
    if (File.exists("notes.txt")) {
        text: String = File.read("notes.txt");
        println(text);
    }
    return None;
}
```

## Behavioral Notes

- invalid UTF-8 and NUL-containing file contents are rejected
- non-seekable/unsupported paths fail with explicit runtime error
- write/delete return `Boolean`; check return values in automation code

## Effect Note

I/O functions participate in effect checks and require `io` capability.
See [Effects](../advanced/effects.md).

## Common Mistakes

- assuming `File.write` appends (it overwrites)
- ignoring `File.write`/`File.delete` return values
- using unvalidated user-controlled paths directly
