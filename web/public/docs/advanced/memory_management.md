# Memory Management

Arden is a native language with compiler-enforced ownership and borrowing.

It targets LLVM and does not rely on a tracing garbage collector for ordinary program execution.

## High-Level Model

- primitive values behave like ordinary native values
- heap-backed runtime values such as `String`, collections, and class instances are managed through the language runtime and ownership rules
- lifetimes and mutation hazards are checked statically where possible

## Ownership

Arden tracks ownership transfers and borrowed access.

That means the compiler can reject:

- use-after-move
- conflicting mutable and immutable borrows
- invalid mutation through immutable access paths

Primary reference:

- [Ownership and Borrowing](ownership.md)

## Stack vs Heap

As a practical mental model:

- small scalar values are cheap, local native values
- strings, collections, tasks, and class instances generally involve runtime-managed storage

The exact representation is an implementation detail, but the user-facing rule is: write code against ownership and borrowing semantics, not guessed storage trivia.

## Destruction And Scope

When values leave scope, Arden can run the necessary cleanup for the underlying runtime representation.

That is why destructors and ownership rules matter more than “manual free everywhere” style programming.

## Smart-Pointer-Like Types

Some generic ownership/container forms such as `Box<T>`, `Rc<T>`, and `Arc<T>` appear in examples and type surfaces.

Treat these as evolving language/runtime surface area and verify current behavior against examples or the compiler before documenting them as a stable low-level ABI guarantee.

