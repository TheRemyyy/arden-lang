# Input / Output (I/O)

Apex provides built-in functions for console I/O operations. These are compiler intrinsics that map directly to system calls.

> For `print`, `println`, and `read_line`, add:
> `import std.io.*;`

## Output Functions

### `print(message: String): None`

Prints a message to standard output *without* a newline character at the end.

```apex
import std.io.*;

print("Hello, ");
println("World!"); // Output: Hello, World!
```

### `println(message: String): None`

Prints a message to standard output followed by a newline character.

```apex
import std.io.*;

println("Status: OK");
```

## Input Functions

### `read_line(): String`

Reads a line of text from standard input (stdin). Returns the string including the newline character (if present).
Longer input lines are supported without truncation through a growing internal buffer.

```apex
import std.io.*;

print("Enter name: ");
name: String = read_line();
println("Hello, " + name);
```

## File I/O Functions

The `File` object provides static methods for interacting with the file system.

### `File.read(path: String): String`

Reads the entire content of a text file. Returns an empty string if the file cannot be read.
Files containing embedded `0x00` bytes are rejected at runtime instead of being silently truncated.
Invalid UTF-8 byte sequences are also rejected at load time instead of leaking into later string operations.

```apex
content: String = File.read("data.txt");
```

### `File.write(path: String, content: String): Boolean`

Writes the given content to a file. Overwrites the file if it already exists. Returns `true` if successful.

```apex
success: Boolean = File.write("output.txt", "Hello, Apex!");
```

### `File.exists(path: String): Boolean`

Checks if a file exists and is accessible.

```apex
import std.io.*;

if (File.exists("config.json")) {
    println("Config found");
}
```

### `File.delete(path: String): Boolean`

Deletes a file from the file system. Returns `true` if successful.

```apex
File.delete("temp.log");
```
