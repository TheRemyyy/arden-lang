# Types

Arden is strongly and statically typed. Every variable must have a type known at compile time.

## Primitive Types

| Type | Description | Example |
| :--- | :--- | :--- |
| `Integer` | 64-bit signed integer | `42`, `-1` |
| `Float` | 64-bit floating point | `3.14`, `-0.01` |
| `Boolean` | True or False | `true`, `false` |
| `Char` | Unicode character | `'a'`, `'🚀'` |
| `String` | UTF-8 encoded string | `"Hello"` |
| `None` | Unit type (empty value) | `None` |

### Integers

Currently, `Integer` is the primary integer type.

```arden
x: Integer = 100_000; // Underscores can be used for readability
```

### Floats

```arden
f: Float = 1.0;
sum: Float = 1 + 2.5;
same: Boolean = 1 == 1.0;
choice: Float = if (flag) { 1 } else { 2.5 };
```

Rules worth remembering:

- Arden promotes `Integer` to `Float` inside mixed scalar numeric expressions.
- Assignments still require a `Float` result on the right-hand side; there is no blanket implicit conversion step.
- Wrapped/container types stay invariant, so `Option<Integer>` does not implicitly become `Option<Float>`, and `Range<Integer>` does not implicitly become `Range<Float>`.

### Booleans

Used in conditional logic.

```arden
isValid: Boolean = true;
if (isValid) { ... }
```

### Strings

Strings are heap-allocated and UTF-8 encoded.

```arden
s: String = "Text";
```

### None

The `None` type represents the absence of a value, similar to `void` in C or `()` in Rust. It has a single value: `None`.

```arden
function doWork(): None {
    return None;
}
```

## Reference Types

Arden allows references to values.

- `&T`: Immutable reference.
- `&mut T`: Mutable reference.

See [Ownership and Borrowing](../advanced/ownership.md) for more details.

## Composite Types

- **Lists**: `List<T>` - See [Collections](../stdlib/collections.md#listt)
- **Maps**: `Map<K, V>` - See [Collections](../stdlib/collections.md#mapk-v)
- **User-defined**: Classes, Enums, Interfaces.

### Built-in Generic Constructors

Built-in generic constructor argument rules are checked at compile time:

- `List<T>()` and `List<T>(capacity: Integer)` are valid.
- `List<T>(capacity)` preallocates backing storage only; it does not create `capacity` elements or change `length()`.
- `Map<K, V>()`, `Set<T>()`, `Option<T>()`, and `Result<T, E>()` accept no value arguments.
- Passing extra or incompatible constructor arguments is a type error.
