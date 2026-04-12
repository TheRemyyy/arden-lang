# Types

Arden is strongly and statically typed: every expression has a known compile-time type.

## Why This Matters

Type checks move failures from runtime into `arden check`/`arden build`.
That gives faster feedback loops and safer refactors.

## Primitive Types

| Type | Description | Example |
| :--- | :--- | :--- |
| `Integer` | 64-bit signed integer | `42`, `-1` |
| `Float` | 64-bit floating point | `3.14`, `-0.01` |
| `Boolean` | Boolean value | `true`, `false` |
| `Char` | Unicode scalar | `'a'`, `'🚀'` |
| `String` | UTF-8 string | `"Hello"` |
| `None` | Unit type | `None` |

## Numeric Rules

```arden
a: Integer = 1;
b: Float = 2.5;
c: Float = a + b; // Integer promotes to Float inside numeric expression
```

Important constraints:

- mixed numeric expressions can widen `Integer` to `Float`
- assignments still require type-compatible RHS
- container/generic types are invariant (`Option<Integer>` is not `Option<Float>`)

## Strings and `None`

```arden
import std.io.*;

text: String = "Arden";

function logDone(): None {
    println("done");
    return None;
}
```

## Reference Types

Arden supports borrowed references:

- `&T` immutable reference
- `&mut T` mutable reference

```arden
mut x: Integer = 1;
r: &Integer = &x;
rx: &mut Integer = &mut x;
*rx = 2;
```

Reference safety rules are documented in [Ownership and Borrowing](../advanced/ownership.md).

## Composite Types

- `List<T>` - ordered dynamic collection
- `Map<K, V>` - key/value collection
- `Set<T>` - unique-value collection
- `Range<T>` - iterator-like range type
- classes, enums, interfaces

See [Collections](../stdlib/collections.md) and feature docs for details.

## Built-in Generic Constructors

Constructor argument shapes are checked statically:

- `List<T>()` and `List<T>(capacity: Integer)` are valid
- `Map<K, V>()`, `Set<T>()`, `Option<T>()`, `Result<T, E>()` take no value args
- incompatible arity/types are compile errors

## `Task<T>` and `Ptr<T>` (Compiler Feature)

Arden type system includes:

- `Task<T>` for async values
- `Ptr<T>` for low-level FFI pointer surfaces

Important: these are not normal constructor-based runtime collections.
Treat them as special language/runtime boundary types.

Use `async`/`await` APIs for `Task<T>` flows and `extern` boundaries for `Ptr<T>` usage.

## Related

- [Async / Await](../advanced/async.md)
- [Extern and FFI](../advanced/ffi.md)
