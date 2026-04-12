# Str Module (Strings)

## Why This Matters

Most user-facing output and parsing logic is string-heavy. `Str` provides common operations without manual boilerplate.

Use `Str` (module object), not `String` (type name).

Import:

```arden
import std.string.*;
```

## Core Operations

- `Str.len(s: String): Integer`
- `Str.compare(a: String, b: String): Integer`
- `Str.concat(a: String, b: String): String`
- `Str.upper(s: String): String`
- `Str.lower(s: String): String`
- `Str.trim(s: String): String`
- `Str.contains(s: String, sub: String): Boolean`
- `Str.startsWith(s: String, pre: String): Boolean`
- `Str.endsWith(s: String, suf: String): Boolean`

## Practical Example

```arden
import std.io.*;
import std.string.*;

raw: String = "  arden lang  ";
clean: String = Str.trim(raw);
upper: String = Str.upper(clean);

println("clean={clean}");
println("upper={upper}");
println("has-lang={Str.contains(clean, "lang")}");
```

## `to_string(...)`

Global helper for scalar conversion to `String`.

```arden
text: String = to_string(42);
```

Interpolation follows the same scalar display model as `to_string`.

## Example In Repo

- [`23_str_utils`](../../examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden)
