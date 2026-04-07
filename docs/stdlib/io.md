# Input / Output (I/O)

Arden provides built-in functions for console I/O operations. These are compiler intrinsics that map directly to system calls.

> For `print`, `println`, and `read_line`, add:
> `import std.io.*;`

## Output Functions

### `print(message: Integer | Float | Boolean | String | Char | None, ...): None`

Prints supported scalar values to standard output *without* a newline character at the end.
Multiple arguments are printed in order with no separator.
Complex values such as `Option<T>` are currently rejected until structured formatting exists.

```arden
import std.io.*;

print("Hello, ");
println("World!"); // Output: Hello, World!
print(true, " ", '🚀', " ", None); // Output: true 🚀 None
```

### `println(message: Integer | Float | Boolean | String | Char | None, ...): None`

Prints supported scalar values to standard output followed by a newline character.

```arden
import std.io.*;

println("Status: OK");
```

## Input Functions

### `read_line(): String`

Reads a line of text from standard input (stdin). Returns the string including the newline character (if present).
Longer input lines are supported without truncation through a growing internal buffer.

```arden
import std.io.*;

print("Enter name: ");
name: String = read_line();
println("Hello, " + name);
```

## File I/O Functions

The `File` object provides static methods for interacting with the file system.

### `File.read(path: String): String`

Reads the entire content of a text file. Missing or inaccessible files fail immediately instead of silently producing an empty string.
Files containing embedded `0x00` bytes are rejected at runtime instead of being silently truncated.
Invalid UTF-8 byte sequences are also rejected at load time instead of leaking into later string operations.
Non-seekable paths such as FIFOs are rejected with a direct runtime error instead of being treated like normal files.

```arden
content: String = File.read("data.txt");
```

### `File.write(path: String, content: String): Boolean`

Writes the given content to a file. Overwrites the file if it already exists. Returns `true` only if both the write and final close/flush succeed.

```arden
success: Boolean = File.write("output.txt", "Hello, Arden!");
```

### `File.exists(path: String): Boolean`

Checks if a readable regular file exists at the given path. Directories return `false`.

```arden
import std.io.*;

if (File.exists("config.json")) {
    println("Config found");
}
```

### `File.delete(path: String): Boolean`

Deletes a regular file from the file system. Returns `true` if successful. Directories return `false`.

```arden
File.delete("temp.log");
```
