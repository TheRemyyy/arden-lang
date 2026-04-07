# Multi-File Project Example

This is the smallest good starter for Arden project mode.

It shows:

- `arden.toml`
- explicit file lists
- a project entrypoint
- cross-file organization
- project-aware `run`, `check`, and `info`

If someone asks "what is the smallest non-trivial Arden project?", this is the answer.

## Run It

```bash
cd examples/starter_project
arden info
arden check
arden run
```

## Layout

```text
starter_project/
├── arden.toml
└── src/
    ├── main.arden
    ├── math.arden
    └── strings.arden
```

## What The Files Do

- `src/main.arden` wires the demo together and prints the results
- `src/math.arden` contains simple numeric helpers like `factorial`, `power`, and `is_prime`
- `src/strings.arden` contains small string helpers like `repeat`, `pad_left`, and `greet`

## What To Notice

- project files are declared explicitly in `arden.toml`
- the entry file is `src/main.arden`
- `arden info` shows the resolved project configuration
- `main.arden` calls helpers that live in separate source files without collapsing everything into one module

## Why This Example Matters

This project is intentionally plain. It does not try to show every language feature at once.

Its job is to teach:

- how Arden project layout feels
- how to split source across files
- how to validate the project with `check`
- how to inspect config with `info`

After this example, the next logical step is:

- [../nested_package_project/README.md](../nested_package_project/README.md) for nested package imports
- [../showcase_project/README.md](../showcase_project/README.md) for a denser project that combines more features
