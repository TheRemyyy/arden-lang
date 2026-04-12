# Multi-File Projects

## Why This Matters

Project mode is the production workflow: explicit file graph, deterministic builds, and better team-scale reliability.

## `arden.toml` Essentials

A project declares at least:

- project metadata (`name`, `version`)
- entrypoint (`entry`)
- source file list (`files`)

## Typical Project Flow

```bash
arden new my_project
cd my_project
arden info
arden check
arden run
```

## Key Behavior

- commands resolve through project config
- import usage is checked across files
- cache data in `.ardencache/` speeds repeated builds

## Optimization

`opt_level` controls final binary optimization (`0/1/2/3/s/z/fast`, default `3`).

## Related

- [Projects summary](../projects.md)
- [CLI reference](../compiler/cli.md)
- Project examples:
  - [starter_project](../../examples/starter_project/README.md)
  - [nested_package_project](../../examples/nested_package_project/README.md)
  - [showcase_project](../../examples/showcase_project/README.md)
