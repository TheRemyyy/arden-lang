# Generics

## Why This Matters

Generics let you reuse logic without sacrificing compile-time type guarantees.

## Core Idea

Write one definition that works for many concrete types.

## Generic Functions

```arden
function identity<T>(value: T): T {
    return value;
}

a: Integer = identity<Integer>(10);
```

Type arguments can often be inferred when context is strong enough.

## Generic Classes

```arden
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    function get(): T {
        return this.value;
    }
}
```

## Generic Interfaces

```arden
interface Reader<T> {
    function read(): T;
}
```

## Constraints (`extends`)

Use interface bounds to require capabilities:

```arden
interface Named { function name(): String; }

function printName<T extends Named>(value: T): None {
    println(value.name());
    return None;
}
```

Bounds are interface-based; invalid/unknown/non-interface bounds are rejected.

## Nested Generic Types

Generic types compose naturally:

- `Option<List<Integer>>`
- `Result<Map<String, Integer>, String>`

## Practical Guidance

- add generic parameters only when reusability is real
- keep bounds minimal and meaningful
- prefer explicit type args when readability is improved

## Example

- [`09_generics`](../../examples/single_file/language_core/09_generics/09_generics.arden)
