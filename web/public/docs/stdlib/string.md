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

## `Str.compare` Contract (Read This First)

`Str.compare(a, b)` is an ordering function, not a boolean predicate.

- returns `< 0` when `a` is ordered before `b`
- returns `0` when `a` and `b` are equal
- returns `> 0` when `a` is ordered after `b`

Correct usage patterns:

```arden
import std.io.*;
import std.string.*;

function main(): None {
    a: String = "alpha";
    b: String = "beta";
    eq: Boolean = Str.compare(a, b) == 0;
    lt: Boolean = Str.compare(a, b) < 0;
    gt: Boolean = Str.compare(a, b) > 0;
    println("eq={eq}, lt={lt}, gt={gt}");
    return None;
}
```

Do not treat `Str.compare(...)` as `Boolean`.

## String Workflow Pattern

A reliable default pipeline for user/input text:

1. `trim` incoming text
2. normalize (`lower`/`upper`) if case-insensitive logic is needed
3. compare/search (`compare`, `contains`, `startsWith`, `endsWith`)
4. format output via interpolation or `concat`

## Practical Example

```arden
import std.io.*;
import std.string.*;

function main(): None {
    raw: String = "  arden lang  ";
    clean: String = Str.trim(raw);
    upper: String = Str.upper(clean);
    has_lang: Boolean = Str.contains(clean, "lang");

    println("clean={clean}");
    println("upper={upper}");
    println("has-lang={has_lang}");
    return None;
}
```

## Case-Insensitive Compare Pattern

```arden
import std.string.*;

function sameTag(a: String, b: String): Boolean {
    return Str.compare(Str.lower(Str.trim(a)), Str.lower(Str.trim(b))) == 0;
}
```

## Common Compare Anti-Pattern

Wrong mental model:

```arden
// wrong: compare does not return Boolean
// if (Str.compare(a, b)) { ... }
```

Right mental model:

```arden
if (Str.compare(a, b) == 0) {
    // equal
}
```

## `to_string(...)`

Global helper for scalar conversion to `String`.

```arden
function main(): None {
    text: String = to_string(42);
    return None;
}
```

Interpolation follows same scalar display model as `to_string`.

## Performance/Readability Rule

If a transform chain gets long, name intermediate values:

- easier debugging (`println` each stage)
- easier review (clear intent per step)
- fewer accidental compare/trim/case-order bugs

## Common Mistakes

- confusing `String` type with `Str` module
- chaining many string transforms without intermediate naming/debug points
- forgetting normalization (`trim/lower`) before comparisons
- assuming `Str.compare` is boolean (it returns an integer relation)

## Example In Repo

- [`23_str_utils`](../../examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden)
