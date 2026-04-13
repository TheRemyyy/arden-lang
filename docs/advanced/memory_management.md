# Memory Management

## Why This Matters

Arden prevents common memory and aliasing bugs at compile time while still producing native binaries.
This is one of the core reasons to use Arden instead of manual-memory systems code.

## High-Level Model

- scalar values behave as native values
- runtime-backed values (`String`, collections, classes, tasks) are managed under ownership rules
- borrow/lifetime hazards are validated statically where possible

Think in three layers:

1. owner (who controls lifetime)
2. borrows (who can observe/mutate now)
3. scope (when each access ends)

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

## Mutation Rules in Practice

- immutable binding: value cannot be reassigned
- `mut` binding: value can be reassigned
- `&T`: read-only borrow
- `&mut T`: exclusive mutable borrow
- `borrow mut` parameter: explicit borrow-mut mode in function signature

Current compiler behavior:

- `borrow mut` requires mutable caller binding
- inside callee, `borrow mut` parameters can be read and reassigned
- caller-visible mutation propagation is type-dependent
- prefer `&mut T` for explicit/predictable caller-visible in-place mutation APIs

If you are new to ownership, start with:

1. immutable by default
2. introduce `mut` only where needed
3. keep `&mut` scopes as short as possible

## Practical Rules

- code to ownership semantics, not guessed stack/heap internals
- keep borrow scopes small when values need to be moved later
- make mutability explicit at API boundaries (`&mut T` for in-place mutation)

## Common Compile Errors and Fix Direction

- "value moved" -> clone before move or change function to borrow
- "cannot borrow as mutable because it is also borrowed as immutable" -> end immutable borrow scope first
- "cannot assign through immutable reference" -> switch to mutable path (`mut` + `&mut`)

## Cleanup Model

When owning bindings leave scope, required runtime cleanup is performed according to value semantics.

## Lifetimes

Lifetimes are implicit in source syntax, but enforced by compiler.
You do not write explicit lifetime annotations today.

What this means in practice:

- you do not annotate lifetimes manually
- but code still fails if a reference can outlive its owner
- fixing usually means moving code blocks or returning owned values instead of references

## Related

- [Ownership and Borrowing](ownership.md)
- [Types](../basics/types.md)
