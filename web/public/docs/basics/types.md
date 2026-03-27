# Types

Apex is strongly and statically typed. Every variable must have a type known at compile time.

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

```apex
x: Integer = 100_000; // Underscores can be used for readability
```

### Floats

```apex
f: Float = 1.0;
// Note: implicit conversion from Integer to Float is not performed automatically in assignments
sum: Float = 1 + 2.5; // Mixed numeric expressions promote Integer operands to Float
same: Boolean = 1 == 1.0; // Mixed numeric comparisons and equality use the same promotion rule
choice: Float = if (flag) { 1 } else { 2.5 }; // Branches also promote to the common numeric type
lifted: () -> Float = () => 1; // Contextual Float returns also promote Integer values
task: Task<Float> = async { 1 }; // Async block tails follow the same rule
named: () -> Float = one; // Named function values and retyped function variables follow it too
scale: (Integer) -> Float = widen; // Function values can also widen Integer parameters to Float safely
nestedTask: Task<Float> = async { if (flag) { return 1; } return 2.5; }; // Nested explicit returns merge to Float too
for (x: Float in range(1, 4)) { println(x); } // Typed loop bindings widen Integer iterables too
```

This promotion is scalar-only. Wrapped/container types stay invariant, so `Option<Integer>` does not implicitly become `Option<Float>`, and `Range<Integer>` does not implicitly become `Range<Float>`.

### Booleans

Used in conditional logic.

```apex
isValid: Boolean = true;
if (isValid) { ... }
```

### Strings

Strings are heap-allocated and UTF-8 encoded.

```apex
s: String = "Text";
```

### None

The `None` type represents the absence of a value, similar to `void` in C or `()` in Rust. It has a single value: `None`.

```apex
function doWork(): None {
    return None;
}
```

## Reference Types

Apex allows references to values.

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
- `Map<K, V>()`, `Set<T>()`, `Option<T>()`, and `Result<T, E>()` accept no value arguments.
- Passing extra or incompatible constructor arguments is a type error.
