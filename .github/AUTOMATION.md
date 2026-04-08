# GitHub Automation

This directory contains the repository automation used for CI and release builds.

The goal of these workflows is to validate the same core flows a user or contributor relies on locally: build the compiler, run Rust quality gates, smoke the CLI, and exercise the example corpus.

## Workflows

### `workflows/ci.yml`

Runs:

- release builds for Linux, macOS, and Windows
- Rust checks (`check`, `test`, `fmt`, `clippy`)
- CLI smoke tests
- example sweeps

This is the first place to inspect if you changed:

- environment variable names used by scripts
- LLVM installation assumptions
- platform-specific linker behavior
- any CLI smoke expectation

### `workflows/release.yml`

Builds tagged releases and publishes platform binaries as GitHub release assets.

If release packaging changes, keep installation docs and README instructions aligned with the new artifact names or platform expectations.

## Composite Actions

### `actions/install-llvm/`

Shared LLVM installation logic used by CI and release workflows.

## Related Docs

- [scripts/README.md](../scripts/README.md)
- [docs/getting_started/installation.md](../docs/getting_started/installation.md)
