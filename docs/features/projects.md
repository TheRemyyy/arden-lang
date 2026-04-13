# Multi-File Projects

## Why This Matters

Project mode is Arden's production workflow.
It gives deterministic builds and clear boundaries that scale to teams and CI.

## `arden.toml` Essentials

A project should explicitly declare:

- metadata (`name`, `version`)
- entrypoint (`entry`)
- source graph (`files`)

Minimal example:

```toml
name = "my_project"
version = "0.1.0"
entry = "src/main.arden"
files = [
  "src/main.arden",
  "src/math.arden"
]
```

## Typical Project Flow

```bash
arden new my_project
cd my_project
arden info
arden check
arden run
arden test
```

Pass args to your project binary:

```bash
arden run -- --mode dev --verbose
```

## Import + Graph Behavior

- import usage is validated across all declared files
- unresolved imports fail in `arden check`
- unlisted files are not part of compile graph

## Cache Behavior

Arden stores cache artifacts in `.ardencache/` to speed repeated checks/builds.
In CI, keeping cache between runs reduces no-op latency significantly.

## Optimization Settings

`opt_level` controls final binary optimization: `0/1/2/3/s/z/fast` (default `3`).

Use lower levels while iterating locally if compile speed matters more than peak runtime.

## Build Throughput Diagnostics

When project builds become slow:

```bash
arden check --timings
arden build --timings
```

For advanced large-project tuning only:

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

## Common Mistakes

- partial file lists drifting from real codebase
- mixing generated and source files without clear ownership
- not pinning entrypoint and then debugging the wrong startup file

## Common Diagnostics You Will Hit

- `No arden.toml found ...`
  - you are outside project root; `cd` into directory with `arden.toml`
- invalid `opt_level` in config
  - use one of `0`, `1`, `2`, `3`, `s`, `z`, `fast`
- file graph errors after adding new file
  - add file path into `files = [...]` and rerun `arden check`

## Related

- [Projects summary](../projects.md)
- [CLI reference](../compiler/cli.md)
- Project examples:
  - [starter_project](../../examples/starter_project/README.md)
  - [nested_package_project](../../examples/nested_package_project/README.md)
  - [showcase_project](../../examples/showcase_project/README.md)
