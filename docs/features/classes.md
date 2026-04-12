# Classes

## Why This Matters

Use a class when data and behavior belong together and you want one clear API around that state.
For beginners: a class is a custom type that stores fields and exposes methods.

## Mental Model

- `class` defines a type
- fields store data (`name: Type`)
- `mut` on a field means that field can change
- methods are functions attached to the class
- constructor sets initial valid state

## Minimal Runnable Example

```arden
import std.io.*;

class Counter {
    mut value: Integer;

    constructor(start: Integer) {
        this.value = start;
    }

    function increment(): None {
        this.value += 1;
        return None;
    }

    function current(): Integer {
        return this.value;
    }
}

function main(): None {
    c: Counter = Counter(10);
    c.increment();
    println("counter={c.current()}");
    return None;
}
```

## Field Mutability (`mut`) vs Immutable Fields

```arden
class User {
    id: Integer;
    mut points: Integer;

    constructor(id: Integer) {
        this.id = id;
        this.points = 0;
    }
}
```

- `id` cannot be reassigned after construction
- `points` can be updated in methods

Practical rule:

- keep fields immutable unless you have a clear state transition reason
- expose state changes through methods, not ad-hoc writes everywhere

## Visibility and Inheritance

Class fields/methods can use visibility modifiers (`public`, `protected`, `private`) and classes can extend base classes with `extends`.

For practical examples:

- [`35_visibility_enforcement`](../../examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden)
- [`36_inheritance_extends`](../../examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden)

## Destructors

Arden classes support `destructor()` for cleanup-oriented logic on object teardown.

Practical rules:

- keep destructor logic minimal and deterministic
- avoid complex control flow in destructors
- class can define at most one destructor

## Constructor Guidance

Constructors are where you enforce invariants early.
If a value must never be negative/empty/invalid, validate before storing.

## API Boundary Rule

If external code should not directly mutate internals, keep fields non-public and
offer explicit methods (`deposit`, `rename`, `setStatus`) that validate changes.

## Common Mistakes

- forgetting `mut` on fields that methods change
- exposing fields and mutating everywhere instead of centralizing mutation in methods
- using classes for data that has no behavior (module + plain values may be cleaner)

## When To Use What

- class: state + methods travel together
- enum: finite known states/variants
- module: namespace/grouping of functions

## Related

- [Interfaces](interfaces.md)
- [Language Edges](language_edges.md)
- [Ownership](../advanced/ownership.md)
- Example: [`05_classes`](../../examples/single_file/language_core/05_classes/05_classes.arden)
