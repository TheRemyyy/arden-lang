<div align="center">

# Apex Programming Language


**Modern Systems Programming with Safety and Performance**

[![Website](https://img.shields.io/badge/Website-apex--compiler.vercel.app-white?style=flat-square&logo=vercel)](https://apex-compiler.vercel.app/)
[![Rust](https://img.shields.io/badge/Rust-1.83+-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-21.0+-blue.svg?style=flat-square)](https://llvm.org/) 

*Strong static typing • Ownership & borrowing • Async/await • Zero-cost abstractions*

[Quick Start](docs/getting_started/quick_start.md) • [Examples](examples/) • [Documentation](docs/)

</div>

---

## Overview

Apex is a modern systems programming language that combines the safety of Rust with the expressiveness of modern high-level languages. Built on LLVM, Apex compiles to native machine code with zero runtime overhead while providing strong compile-time guarantees through its advanced type system and borrow checker.

### Key Features

- **🔒 Memory Safety** - Ownership prevents races, null pointers, and use-after-free bugs at compile time
- **⚡ Zero-Cost Abstractions** - High-level features compile down to machine code with no runtime penalty
- **🎯 Strong Static Typing** - Comprehensive type system with generics, traits, and algebraic data types
- **🔄 Async/Await** - First-class support for asynchronous programming with Task types
- **📦 Pattern Matching** - Exhaustive pattern matching for control flow and destructuring
- **🧩 Generics** - Full generic programming support with type parameters and constraints
- **🛠️ Modern Tooling** - Fast compilation, helpful error messages, and integrated toolchain
- **🚀 LLVM Backend** - Leverages LLVM for world-class optimization and cross-platform support
- **📁 Multi-File Projects** - Organize code with `apex.toml` project files
- **📦 Java-Style Namespaces** - Simple package/import system (no `mod.rs` needed)

## Documentation

Detailed documentation is available in the `docs/` directory:

### Getting Started

- **[Installation](docs/getting_started/installation.md)**: How to build and install Apex.
- **[Quick Start](docs/getting_started/quick_start.md)**: Write your first Hello World program.
- **[Editor Setup](docs/getting_started/editor_setup.md)**: Recommended VS Code settings.

### Language Guide

- **[Syntax](docs/basics/syntax.md)**: Basic syntax rules.
- **[Variables & Mutability](docs/basics/variables.md)**: `val` vs `var`, ownership.
- **[Types](docs/basics/types.md)**: Primitives and composite types.
- **[Control Flow](docs/basics/control_flow.md)**: `if`, `while`, `for`, `match`.
- **[Functions](docs/features/functions.md)**: Definition, lambdas, higher-order functions.
- **[Classes](docs/features/classes.md)**: OOP features.
- **[Interfaces](docs/features/interfaces.md)**: Polymorphism.
- **[Enums](docs/features/enums.md)**: ADTs and pattern matching.
- **[Ranges](docs/features/ranges.md)**: Iterator-based numeric sequences.
- **[Modules](docs/features/modules.md)**: Code organization.

### Advanced

- **[Ownership & Borrowing](docs/advanced/ownership.md)**: Apex's core safety model.
- **[Generics](docs/advanced/generics.md)**: Flexible type reuse.
- **[Async/Await](docs/advanced/async.md)**: Concurrency model.
- **[Error Handling](docs/advanced/error_handling.md)**: `Result` and `Option` types.

## ⚡ Quick Install

```bash
git clone https://github.com/TheRemyyy/apex-compiler.git
cd apex-compiler
cargo build --release
```

Add `target/release` to your PATH.

## 🧪 Example Test Scripts

Run all bundled examples with platform-specific scripts from the repo root:

- **Windows**: `scripts\\test_examples.bat`
- **Linux**: `bash scripts/test_examples_linux.sh`
- **macOS**: `bash scripts/test_examples_macos.sh`

Async runtime examples and task controls (`await`, `await_timeout`, `is_done`, `cancel`) are intended to behave consistently across Linux, macOS, and Windows.

## Compiler Hardening

For compiler robustness work there are now 2 additional loops beyond the normal unit/integration suite:

- Ignored deterministic stress tests:
  - `cargo test -- --ignored`
- `cargo-fuzz` lexer/parser target:
  - `cargo install cargo-fuzz`
  - `cargo fuzz run lexer_parser`

The ignored stress tests stay out of the default fast suite, while `cargo-fuzz` is intended for longer local or CI hardening runs.

Diagnostics and editor tooling notes:
- Shebang-based Apex entry files (`#!/usr/bin/env apex`) now preserve absolute lexer spans, so parse errors, source underlines, and LSP symbol locations stay aligned with the real file offsets.
- Project auto-discovery now only trusts real `apex.toml` files, not directories with that name.

## 📁 Quick Start: Multi-File Project

```bash
# Create new project
apex new my_project
cd my_project

# Project structure:
# ├── apex.toml
# └── src/
#     ├── utils.apex    # package utils;
#     └── main.apex     # package main;

# Build and run
apex run
```

### Java-Style Namespaces

```apex
// src/utils/math.apex
package utils.math;

function factorial(n: Integer): Integer {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}

// src/main.apex
package main;

import utils.math.*;           // Wildcard import
import utils.math.factorial;   // Specific import

function main(): None {
    result: Integer = factorial(5);
    println("5! = " + to_string(result));
    return None;
}
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">
<sub>Built with ❤️ and Rust</sub>
</div>
