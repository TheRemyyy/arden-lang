# Projects Guide Index

## Why This Matters

Use this page as the quick jump table for project-mode docs.
If you are moving from single-file experiments to maintainable multi-file code, start here.

## Read In This Order

1. [Multi-File Projects](../projects.md)
2. [Projects summary](../../projects.md)
3. [CLI reference](../../compiler/cli.md)

## Core Commands

```bash
arden new my_project
cd my_project
arden info
arden check
arden run
arden test
```

## What To Verify First In New Project

- `arden.toml` has correct `entry`
- all source files are listed in `files`
- `arden check` passes before first feature work

## Example Projects

- [starter_project](../../../examples/starter_project/README.md)
- [nested_package_project](../../../examples/nested_package_project/README.md)
- [showcase_project](../../../examples/showcase_project/README.md)
