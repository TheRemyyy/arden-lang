# Installation

## Why This Matters

A clean toolchain setup removes 80% of early friction. This page is about getting to a stable `arden run` quickly.

## Prerequisites

- Rust toolchain (for building from source)
- LLVM + linker requirements for your platform

## Build Arden

From repo root:

```bash
cargo build --release
```

Compiler binary:

```text
target/release/arden
```

## Verify Installation

```bash
./target/release/arden --version
./target/release/arden --help
```

Run a smoke example:

```bash
./target/release/arden run examples/single_file/basics/01_hello/01_hello.arden
```

## Platform Notes

Arden uses explicit linker policy in this repo setup:

- Linux: `mold`
- macOS: LLVM `lld`
- Windows: LLVM `lld-link`

If build/link fails, check your linker and LLVM environment first.

## Next Step

Continue with [Quick Start](quick_start.md).
