# Projects

## Why This Matters

Single-file mode is great for experiments. Project mode is what you want for real multi-file development.

## What Project Mode Gives You

- explicit source graph (`files` in `arden.toml`)
- explicit entrypoint (`entry`)
- deterministic project `check/build/run/test`
- cache reuse via `.ardencache/`

## Minimal `arden.toml`

```toml
name = "app"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
```

## Typical Loop

```bash
arden info
arden check
arden run
```

## Practical Guidance

- keep file list explicit and reviewed
- use `arden check` in CI for fast semantic validation
- use `--timings` on `check/build` to inspect phase cost

## Deep Dive

- [Multi-File Projects](features/projects.md)
- [CLI Reference](compiler/cli.md)
