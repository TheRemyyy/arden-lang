<div align="center">

# Apex Programming Language

**Blazing-fast systems language with static safety checks and near-instant rebuilds.**

Apex is built for fast feedback loops, not long rebuild cycles.

[![Website](https://img.shields.io/badge/Website-apex--compiler.vercel.app-white?style=flat-square&logo=vercel)](https://apex-compiler.vercel.app/)
[![Rust](https://img.shields.io/badge/Rust-1.83+-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-21.0+-blue.svg?style=flat-square)](https://llvm.org/)

[Quick Start](docs/getting_started/quick_start.md) • [Examples](examples/) • [Documentation](docs/) • [Benchmarks](benchmark/)

</div>

---

## Why Apex

Apex is trying to be useful now, not "interesting someday". The current compiler already ships a real CLI, native code generation through LLVM, multi-file project support, a borrow checker, async tasks, formatting/linting, a test runner, and benchmark tooling in one repo.

If you care about compile-time feedback and iteration speed, Apex is strongest when used as:

- a native language with ownership and borrowing
- a project-oriented compiler with incremental build caching
- an experimental language that already has enough tooling to build, run, format, lint, test, benchmark, and inspect code from the CLI

## Try It Quickly

```bash
# Requires the toolchain from docs/getting_started/installation.md:
# Rust 1.83+, LLVM 21+, clang, and mold/lld depending on platform.

git clone https://github.com/TheRemyyy/apex-compiler.git
cd apex-compiler
cargo build --release

echo 'import std.io.*; function main(): None { println("Hello"); return None; }' > hello.apex
./target/release/apex-compiler run hello.apex
```

## Performance Snapshot

The repo includes a reproducible benchmark runner in [`benchmark/run.py`](benchmark/run.py) that compares Apex, Rust, and Go on the same workloads.

Beyond the baseline suite, the benchmark harness now also includes:

- `matrix_mul_heavy` for a more meaningful CPU-bound runtime pass
- `compile_project_extreme_graph` for a much larger synthetic project compile
- `incremental_rebuild_extreme_graph*` for harsher invalidation/rebuild scenarios
- `--apex-timings` to capture Apex phase breakdowns from `apex build --timings`

I verified a small subset locally on **April 2, 2026** using:

```bash
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build
python3 benchmark/run.py --bench compile_project_10_files --compile-mode hot --repeats 1 --warmup 0 --no-build
python3 benchmark/run.py --bench incremental_rebuild_1_file --repeats 1 --warmup 0 --no-build --apex-timings
```

Current local snapshot for the heavier CPU-bound runtime benchmark (`matrix_mul_heavy`):

| Language | Runtime mean |
|---|---:|
| Apex | 0.0106 s |
| Rust | 0.0218 s |
| Go | 0.0144 s |

Current local snapshot for the generated 10-file compile benchmark:

| Language | Hot compile mean |
|---|---:|
| Apex | 0.106 s |
| Rust | 0.131 s |
| Go | 3.097 s |

Current local snapshot for incremental rebuild after changing one file:

| Language | Full compile mean | Rebuild mean |
|---|---:|---:|
| Apex | 0.1846 s | 0.0108 s |
| Rust | 0.1288 s | 0.1647 s |
| Go | 3.0512 s | 0.1497 s |

In this measured scenario, Apex rebuilds were roughly 13x faster than Rust and roughly 14x faster than Go.

With `--apex-timings`, the benchmark report also captures where Apex spends build time. On the verified 1-file rebuild scenario, the cold build was dominated by `object codegen` and `final link`, while the hot rebuild collapsed to a cache-heavy path with only `parse + symbol scan`, `dependency graph`, and `semantic cache gate` showing up materially.

I also ran an Apex-only extreme synthetic project probe outside the cross-language suite:

- cold build of a generated 2200-file graph: about `21.0 s`
- hot rebuild after one leaf edit: about `0.628 s`
- biggest cold-build phases: `rewrite ~8.17 s`, `object codegen ~11.57 s`

These are single-run sanity checks on one machine, not publication-grade benchmark claims. The important point is that Apex now has runtime benchmarks, incremental compile benchmarks, harsher synthetic compile scenarios, and phase-level timing data instead of a vague "fast compilation" bullet.

For the full suite and workload descriptions, see [`benchmark/README.md`](benchmark/README.md).

## Status

Apex is an experimental but actively developed compiler.

- core language features, the project CLI, and benchmark tooling are working today
- incremental caching and per-phase build timings are implemented and measurable
- expect rough edges and ongoing compiler work, but this is not a toy parser demo

## When Apex Makes Sense

- You want fast compile times and fast rebuild feedback on native projects.
- You like Rust-style safety pressure but want a simpler, more integrated CLI workflow.
- You want one toolchain for `build`, `run`, `check`, `fmt`, `lint`, `test`, `bench`, and `profile`.

## What Is Implemented

The repository currently includes:

- **Ownership and borrowing** with a dedicated borrow checker in [`src/borrowck.rs`](src/borrowck.rs)
- **Strong static typing** with type checking in [`src/typeck.rs`](src/typeck.rs)
- **Async/await and `Task<T>`** with runtime controls such as `await_timeout`, `is_done`, and `cancel`
- **Pattern matching, enums, interfaces, classes, generics, lambdas, ranges, and effects**
- **Multi-file projects** with `apex.toml`, package declarations, imports, and project rewriting
- **LLVM-based native codegen** in [`src/codegen/`](src/codegen/)
- **CLI tooling** for `build`, `run`, `compile`, `check`, `fmt`, `lint`, `fix`, `test`, `bench`, `profile`, `bindgen`, `lex`, `parse`, and `lsp`

## Quick Start

### Requirements

Build-from-source currently requires:

- Rust `1.83+`
- LLVM `21+`
- Clang
- `mold` on Linux, or LLVM `lld` on macOS/Windows

See [Installation](docs/getting_started/installation.md) for platform details.

### Build The Compiler

```bash
git clone https://github.com/TheRemyyy/apex-compiler.git
cd apex-compiler
cargo build --release
```

The built binary is `target/release/apex-compiler`. README examples use `apex`; if you have not added an alias or symlink yet, substitute `target/release/apex-compiler`.

### Run A Single File

```bash
cat > hello.apex <<'EOF'
import std.io.*;

function main(): None {
    println("Hello, Apex!");
    return None;
}
EOF

apex run hello.apex
```

Other useful commands:

```bash
apex check hello.apex
apex compile hello.apex
apex fmt hello.apex
apex lint hello.apex
apex profile hello.apex
```

### Create A Project

```bash
apex new my_project
cd my_project
apex run
```

This flow was smoke-tested against the current compiler build and produces a runnable project skeleton.

## Example Features

### Ownership And Borrowing

```apex
function readData(borrow data: Data): Integer {
    return data.value;
}

function modifyValue(borrow mut x: Integer): None {
    x = x + 10;
    return None;
}
```

See [`examples/10_ownership.apex`](examples/10_ownership.apex).

### Async Tasks

```apex
async function delayedValue(ms: Integer, value: Integer): Task<Integer> {
    std.time.sleep(ms);
    return async { value };
}
```

See [`examples/14_async.apex`](examples/14_async.apex) and [`examples/28_async_runtime_control.apex`](examples/28_async_runtime_control.apex).

### Multi-File Projects

```apex
package main;

import utils.math.factorial;

function main(): None {
    println("5! = " + to_string(factorial(5)));
    return None;
}
```

See [`examples/multi_file_project`](examples/multi_file_project/) and [`examples/multi_file_depth_project`](examples/multi_file_depth_project/).

## Tooling

The current CLI surface is broader than a toy compiler:

```bash
apex build
apex run [file]
apex check [file]
apex fmt [path]
apex lint [path]
apex fix [path]
apex test --list --path examples/24_test_attributes.apex
apex bindgen path/to/header.h
apex bench hello.apex --iterations 5
apex profile hello.apex
apex lsp
```

Also available:

- platform-specific example test scripts in [`scripts/`](scripts/)
- a fuzz target in [`fuzz/`](fuzz/)
- ignored stress tests via `cargo test -- --ignored`

## Documentation

Start here:

- [Installation](docs/getting_started/installation.md)
- [Quick Start](docs/getting_started/quick_start.md)
- [Language Overview](docs/overview.md)
- [Compiler CLI](docs/compiler/cli.md)
- [Ownership](docs/advanced/ownership.md)
- [Async](docs/advanced/async.md)
- [Modules and projects](docs/features/modules.md)
- [Testing](docs/features/testing.md)
- [Standard library](docs/stdlib/overview.md)

## Examples

Useful entry points:

- [`examples/01_hello.apex`](examples/01_hello.apex)
- [`examples/10_ownership.apex`](examples/10_ownership.apex)
- [`examples/14_async.apex`](examples/14_async.apex)
- [`examples/16_pattern_matching.apex`](examples/16_pattern_matching.apex)
- [`examples/24_test_attributes.apex`](examples/24_test_attributes.apex)
- [`examples/37_interfaces_contracts.apex`](examples/37_interfaces_contracts.apex)
- [`examples/insane_showcase_project`](examples/insane_showcase_project/)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
