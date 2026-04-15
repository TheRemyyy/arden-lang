# Scripts

This directory contains repository maintenance, smoke-test, and example-runner scripts.

They are intentionally documented because these scripts encode real repository workflow, not random one-off helpers. If a script is used by CI, release validation, or contributor guidance, it should stay understandable.

## Smoke / Validation

### `cli_smoke.sh`

End-to-end CLI smoke test used by CI.

It exercises flows such as:

- `new`
- `info`
- `check`
- `run`
- `fmt`
- `lint`
- `fix`
- `lex`
- `parse`
- `compile`
- `test`
- `bench`
- `profile`

Environment variables:

- `ARDEN_COMPILER_PATH` - explicit compiler binary path
- `CI_SKIP_COMPILER_BUILD=1` - skip rebuilding the compiler before the smoke run

Typical local usage:

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/debug/arden bash scripts/cli_smoke.sh
```

Use this when you changed CLI behavior, output shape, project generation, formatting, linting, or command dispatch logic.

### `cli_smoke_windows.ps1`

Windows wrapper around `cli_smoke.sh`.

### `scripts/ci/emit_codegen_artifacts.sh` and `windows_emit_codegen_artifacts.ps1`

CI-only crash triage helpers that dump `.ll` and best-effort object artifacts.

Environment variables:

- `ARDEN_COMPILER_PATH` - compiler binary used for dump compilation
- `ARDEN_FAILURE_SOURCE` - single source file to dump
- `ARDEN_FAILURE_SOURCES` - semicolon-separated source list to dump
- `ARDEN_FAILURE_CONTEXT` - context label used in artifact output folder
- `ARDEN_FAILURE_OUTPUT_ROOT` - root output directory for dumped artifacts

## Example Sweeps

- `examples_smoke_linux.sh`
- `examples_smoke_macos.sh`
- `examples_smoke.bat`

These run the example corpus and a small set of project-mode example checks.

Use them when you touched:

- parser or lexer behavior
- type checking and borrow checking
- code generation
- stdlib surface
- project-mode rewriting or module resolution

The point is to validate what real users are likely to run first, not just isolated unit tests.

## Maintenance Helpers

### `extract_modules.py`

Historical refactoring helper for extracting large sections out of `src/main.rs`.

### `fix_all.py`

Repository-specific helper for post-extraction visibility and module cleanup.

### `fix_visibility.py`

Helper for applying visibility fixes to extracted Rust modules.

These helpers are mostly repository-maintenance utilities. They are not part of the end-user Arden CLI, but they are still worth documenting because they explain how parts of the compiler codebase were reorganized.

## Updating Scripts Safely

If you change a script:

1. keep environment variable names aligned with CI
2. update the matching workflow file under `.github/workflows/` if behavior changed
3. update user-facing docs if the script is mentioned there
4. run the script locally on at least one supported path

The repo should not rely on undocumented shell glue.

## Notes

- the source of truth for CI behavior is `.github/workflows/`
- if you change script inputs or env vars, update both this file and the workflows
