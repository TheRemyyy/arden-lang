# Time Module

The `Time` module provides functions for working with system time and controlling program execution flow.

## Functions

The `Time` object provides static methods for time retrieval and manipulation.

### `Time.now(format: String): String`

Returns the current local time as a formatted string. If an empty string `""` is provided as the format, it defaults to `"%H:%M:%S"`.

Uses standard C `strftime` format specifiers.

```apex
// Default format (HH:MM:SS)
currentTime: String = Time.now("");

// Custom format (YYYY-MM-DD)
today: String = Time.now("%Y-%m-%d");
```

### `Time.unix(): Integer`

Returns the current Unix timestamp (number of seconds since January 1, 1970).

```apex
timestamp: Integer = Time.unix();
```

### `Time.sleep(ms: Integer): None`

Suspends the execution of the current thread for the specified number of milliseconds.

Negative millisecond values are invalid. Constant negative arguments are rejected during `apex check`, and dynamic negative values fail fast at runtime with a direct diagnostic.

```apex
import std.io.*;

println("Waiting...");
Time.sleep(1000); // Wait for 1 second
println("Done!");
```
