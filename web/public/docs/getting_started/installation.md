# Installation

## Why This Matters

A clean toolchain setup removes most early friction.
Goal: get to a stable `arden --help`, `arden check`, and `arden run` loop fast.

## Prerequisites

- Rust toolchain (for building from source)
- LLVM + linker requirements for your platform

## Build From Source

From repo root:

```bash
cargo build --release
```

Compiler binary path:

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

Run CLI smoke bundle:

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/cli_smoke.sh
```

## Platform Notes

This repository uses explicit linker policy:

- Linux: `mold`
- macOS: LLVM `lld`
- Windows: LLVM `lld-link`

If build/link fails, verify linker and LLVM setup first.

## Portable Artifacts

CI release workflows also produce portable bundles (`linux/macOS/windows`) containing compiler + expected runtime tooling.
Use them when you want quick install without local Rust build.

Download locations:

- website installer page: [arden-lang.dev/install](https://www.arden-lang.dev/install)
- GitHub latest release assets: [github.com/TheRemyyy/arden-lang/releases/latest](https://github.com/TheRemyyy/arden-lang/releases/latest)

Quick portable flow:

1. download the matching archive for your OS/CPU from latest release assets
2. extract it
3. run the included launcher (`arden` / `arden.bat` depending on bundle)
4. verify with `arden --version` and `arden --help`

Portable asset names currently used:

- `arden-windows-x64-portable.zip`
- `arden-linux-x64-portable.tar.gz`
- `arden-macos-arm64-portable.tar.gz`
- `arden-macos-x64-portable.tar.gz`

Direct latest-download URL pattern:

```text
https://github.com/TheRemyyy/arden-lang/releases/latest/download/<ASSET_NAME>
```

Optional checksum verification:

```bash
curl -LO https://github.com/TheRemyyy/arden-lang/releases/latest/download/SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt
```

## Common Setup Problems

- compiler builds, but `arden` path points to old binary
- missing linker on platform-specific target
- running project commands outside project root (`arden.toml` missing)

## Next Step

Continue with [Quick Start](quick_start.md).
