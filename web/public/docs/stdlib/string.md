# Str Module (Strings)

String manipulation utilities. All functions are available as static methods on the `Str` object.

> **Note**: We use the `Str` object instead of `String` to avoid conflicts with the built-in `String` type.

## Functions

| Function | Type Signature | Description |
| :--- | :--- | :--- |
| `Str.len` | `(s: String) -> Integer` | Returns the length of the string in Unicode characters. |
| `Str.compare` | `(a: String, b: String) -> Integer` | Compares two strings. Returns 0 if equal. |
| `Str.concat` | `(a: String, b: String) -> String` | Concatenates two strings into a new one. |
| `Str.upper` | `(s: String) -> String` | Converts string to uppercase. |
| `Str.lower` | `(s: String) -> String` | Converts string to lowercase. |
| `Str.trim` | `(s: String) -> String` | Removes leading/trailing whitespace. |
| `Str.contains` | `(s: String, sub: String) -> Boolean` | Checks if string contains a substring. |
| `Str.startsWith` | `(s: String, pre: String) -> Boolean` | Checks if string starts with prefix. |
| `Str.endsWith` | `(s: String, suf: String) -> Boolean` | Checks if string ends with suffix. |

## Conversions

| Function | Type Signature | Description |
| :--- | :--- | :--- |
| `to_string` | `<T>(x: T) -> String` | Converts any type to its string representation (Global). |
