# System Module

The `System` module provides functions for interacting with the operating system and environment.

## Functions

The `System` object provides static methods for system-level operations.

### `System.getenv(name: String): String`

Retrieves the value of an environment variable. Returns an empty string if the variable is not set.
Invalid UTF-8 environment values are rejected immediately instead of slipping into a broken `String`.

```apex
import std.io.*;

path: String = System.getenv("PATH");
println("Path: {path}");
```

### `System.shell(command: String): Integer`

Executes a command in the system shell and returns the exit code.
On POSIX hosts this is now the decoded process exit code, not the raw `system()` wait-status word.

```apex
exitCode: Integer = System.shell("echo Hello");
```

### `System.exec(command: String): String`

Executes a command in the system shell and captures its standard output (stdout).
The full stdout stream is returned; longer output is no longer truncated to a small fixed buffer.
Binary-looking stdout is rejected: embedded NUL bytes and invalid UTF-8 now fail immediately instead of slipping into a broken `String`.

```apex
import std.io.*;

output: String = System.exec("whoami");
println("Current user: {output}");
```

### `System.cwd(): String`

Returns the current working directory.
Deep working directories are supported without truncating or collapsing the result to an empty string.

```apex
import std.io.*;

currentDir: String = System.cwd();
println("Working in: {currentDir}");
```

### `System.os(): String`

Returns the name of the operating system ("windows", "macos", "linux", or "unknown").

```apex
import std.io.*;

os: String = System.os();
println("Running on {os}");
```

### `System.exit(code: Integer): None`

Terminates the program immediately with the specified exit code. `0` usually indicates success, while any other value indicates an error.

```apex
if (error_occurred) {
    System.exit(1);
}
```
