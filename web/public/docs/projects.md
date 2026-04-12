# Projects

## Why This Matters

Single-file mode is perfect for quick experiments.
Project mode is what you want for real code: explicit file graph, deterministic commands, and predictable CI behavior.

## What Project Mode Gives You

- explicit source graph via `files` in `arden.toml`
- explicit entrypoint via `entry`
- project-scoped `check/build/run/test`
- cache reuse via `.ardencache/`

## Minimal `arden.toml`

```toml
name = "app"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
```

## Fast Daily Loop

```bash
arden info
arden check
arden run
arden test
```

## How To Think About `files`

`files` is the compiler's explicit world-state.
If a source file is missing there, it is outside the project graph and cannot be relied on.

Practical rule: treat `files` updates like API changes and review them in PRs.

## Common Mistakes

- adding new source file but forgetting to update `files`
- relying on ad-hoc local paths that differ in CI
- skipping `arden check` and discovering import/typing issues later in build

## Performance Tip

Use `--timings` on `check`/`build` when feedback loops get slow.

```bash
arden check --timings
arden build --timings
```

## Advanced Perf Knobs (Large Projects)

If you are diagnosing project build throughput, the compiler exposes shard tuning
env vars for object codegen:

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

- `ARDEN_OBJECT_SHARD_THRESHOLD` controls when sharding starts
- `ARDEN_OBJECT_SHARD_SIZE` controls files grouped per shard

Defaults today:

- threshold `256`
- shard size `4`

Use this for measurements and CI optimization, not as everyday defaults.

## Deep Dive

- [Multi-File Projects](features/projects.md)
- [CLI Reference](compiler/cli.md)
