# Memory Management

## Why This Matters

Arden prevents common memory and aliasing bugs at compile time while still producing native binaries.

## High-Level Model

- scalar values behave as native values
- runtime-backed values (`String`, collections, classes, tasks) are managed under ownership rules
- borrow/lifetime hazards are validated statically where possible

## Compiler Guarantees

Arden rejects:

- use-after-move
- conflicting mutable/immutable borrows
- invalid mutation through immutable paths

Primary reference:

- [Ownership and Borrowing](ownership.md)

## Minimal Example

```arden
import std.io.*;

function consume(owned s: String): None { return None; }

function main(): None {
    value: String = "hello";
    borrow_view: &String = &value;
    println(*borrow_view);

    // consume(value); // invalid while borrowed
    return None;
}
```

## Practical Rules

- code to ownership semantics, not guessed stack/heap internals
- keep borrow scopes small when values need to be moved later
- make mutability explicit at API boundaries (`borrow mut` where intended)

## Cleanup Model

When owning bindings leave scope, required runtime cleanup is performed according to value semantics.

## Lifetimes

Lifetimes are implicit in source syntax, but enforced by compiler.
You do not write explicit lifetime annotations today.

## Related

- [Ownership and Borrowing](ownership.md)
- [Types](../basics/types.md)
