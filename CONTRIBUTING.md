# Contributing to Arden

Arden needs contributions across compiler internals, examples, docs, scripts, CI, and web docs.

## Before You Start

Read:

- [README.md](README.md)
- [docs/getting_started/installation.md](docs/getting_started/installation.md)
- [docs/compiler/architecture.md](docs/compiler/architecture.md)
- [scripts/README.md](scripts/README.md)

If your change touches user-facing behavior, also read the matching page under `docs/` or `examples/` first. The repo treats documentation and runnable examples as part of the product surface, not as afterthoughts.

## Local Setup

```bash
git clone https://github.com/TheRemyyy/apex-compiler.git arden
cd arden
cargo build
```

## Useful Checks

```bash
cargo test
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- --help
```

Compiler-facing smoke checks:

```bash
cargo run -- check examples/01_hello.arden
bash scripts/cli_smoke.sh
bash scripts/examples_smoke_linux.sh
```

If you changed documentation generation for the website, also run the web sync/build path from `web/`.

## Where To Contribute

### Compiler

Core implementation lives in `src/`.

Key areas:

- `src/lexer/`
- `src/parser/`
- `src/typeck/`
- `src/borrowck/`
- `src/codegen/`
- `src/project/`
- `src/test_runner/`

### Docs

Repository source docs live in:

- `README.md`
- `docs/`
- `examples/README.md`
- `benchmark/README.md`
- `scripts/README.md`

The web docs are built from the repository docs, so updating source markdown improves both the repo and the website.

Good docs changes are:

- accurate to current behavior
- specific about commands, files, and outputs
- backed by runnable examples where possible
- structured so a new reader can find the next step quickly

### Examples

Examples in `examples/` should be:

- runnable
- focused
- representative of current compiler behavior
- worth pointing users at

### CI / Release

Automation lives in:

- `.github/workflows/`
- `.github/actions/`

If you change scripts or release behavior, update the docs that explain them too.

## Expectations For Changes

- keep docs aligned with actual behavior
- prefer concrete examples over vague promises
- add or update tests when behavior changes
- avoid stale placeholder language in docs

## Pull Requests

A strong PR usually includes:

- the code change
- updated docs or examples if user-facing behavior changed
- tests or smoke coverage when relevant
- concise explanation of what changed and why

If you remove material from docs, make sure you are removing stale or duplicate content, not deleting the explanation that new users actually need.
