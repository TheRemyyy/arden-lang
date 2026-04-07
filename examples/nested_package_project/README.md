# Nested Package Project Example

This example focuses on package declarations and deeper import paths.

It is useful when you want to see:

- `package ...;`
- nested namespaces
- explicit imports
- wildcard imports
- multi-file symbol organization

## Run It

```bash
cd examples/nested_package_project
arden info
arden check
arden run
```

## Layout

```text
nested_package_project/
├── arden.toml
└── src/
    ├── main.arden
    └── utils/
        ├── math.arden
        └── strings.arden
```

## Example Import Style

```arden
import utils.math.factorial;
import utils.strings.*;
```

## What To Notice

- `main.arden` declares a package and imports both specific symbols and wildcard exports
- utility files live under `src/utils/` and map naturally to nested import paths
- project mode still relies on `arden.toml`, even though the directory layout is deeper

## Why This Example Matters

The smaller `starter_project/` example proves that Arden can split logic across files.

This example goes one step further and shows how package-style naming and nested paths work when the codebase stops being flat.

It is the right bridge between:

- [../starter_project/README.md](../starter_project/README.md)
- [../showcase_project/README.md](../showcase_project/README.md)
