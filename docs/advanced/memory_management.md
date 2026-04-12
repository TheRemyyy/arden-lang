# Memory Management

## Why This Matters

Arden aims to prevent common memory/aliasing bugs at compile time while still producing native binaries.

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
function consume(owned s: String): None { return None; }

function main(): None {
    value: String = "hello";
    borrow_view: &String = &value;
    println(*borrow_view);

    // consume(value); // invalid while borrowed
    return None;
}
```

## Practical Rule

Code to ownership semantics, not guessed stack/heap internals.

## Cleanup Model

When owning bindings leave scope, required runtime cleanup is performed according to value semantics.

## Lifetimes

Lifetimes are currently implicit in source syntax, but still enforced by the compiler.
