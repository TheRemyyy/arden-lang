# Apex Overview

Apex is a systems programming language focused on safety, performance, and practical ergonomics.

## Core Principles

- Strong static typing with early compiler feedback
- Ownership and borrowing for memory safety
- LLVM backend for native code generation
- Zero-cost abstractions for runtime efficiency

## Language Features

- Functions, classes, interfaces, enums, modules
- Generics and pattern matching
- Async/await with `Task<T>`
- Range iterators and collection types
- Built-in test attributes (`@Test`, `@Before`, etc.)

## Standard Library Notes

- Stdlib APIs are implemented as compiler intrinsics.
- `print`, `println`, and `read_line` are in `std.io` and should be imported (`import std.io.*;`).
- Builtins such as `to_string`, `range`, `exit`, and assertion helpers are available without import.

## Next Steps

- [Getting Started](getting_started/quick_start.md)
- [Language Basics](basics/syntax.md)
- [Features](features/functions.md)
- [Standard Library](stdlib/overview.md)
- [Compiler](compiler/architecture.md)
