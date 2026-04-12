# Input / Output (I/O)

## Why This Matters

I/O is the most common runtime boundary. Clear behavior here prevents silent data loss and confusing failures.

## Console I/O

Import:

```arden
import std.io.*;
```

### `print(...): None`

Writes scalar values without trailing newline.

### `println(...): None`

Writes scalar values with trailing newline.

### `read_line(): String`

Reads one line from stdin.

```arden
import std.io.*;

print("Enter name: ");
name: String = read_line();
println("Hello, " + name);
```

## File APIs (`File.*`)

Import:

```arden
import std.fs.*;
```

### `File.read(path: String): String`

Reads full text file. Missing/invalid input fails explicitly.

### `File.write(path: String, content: String): Boolean`

Writes full content (overwrite semantics). Returns `true` only on successful write+close.

### `File.exists(path: String): Boolean`

Checks for readable regular file.

### `File.delete(path: String): Boolean`

Deletes regular file.

```arden
import std.fs.*;
import std.io.*;

if (File.exists("notes.txt")) {
    text: String = File.read("notes.txt");
    println(text);
}
```

## Notes

- invalid UTF-8 and NUL-containing file contents are rejected
- non-seekable/unsupported paths fail with explicit runtime error
