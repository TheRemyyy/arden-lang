# Multi-File Projects

Arden supports explicit multi-file projects through `arden.toml`.

Project mode is the right fit when you want:

- more than one source file
- stable build outputs
- explicit file lists
- project-aware `build`, `run`, `check`, `test`, `fmt`, and `info`
- build cache reuse across runs

## Why Project Mode Exists

Single-file commands are useful for experiments, but they stop being enough as soon as you care about structure, repeatability, and larger code organization.

Project mode gives Arden a clear source of truth:

- which files belong to the build
- which file is the entrypoint
- what artifact gets produced
- which optimization level and output kind to use

That explicitness is also what allows better validation and cache reuse.

## Typical Layout

```text
my_project/
├── arden.toml
├── src/
│   ├── main.arden
│   ├── math.arden
│   └── strings.arden
└── README.md
```

## Create A Project

```bash
arden new my_project
cd my_project
arden run
arden info
```

The generated scaffold is intentionally small, but it already contains everything needed for project-aware commands.

## `arden.toml`

Example:

```toml
name = "my_project"
version = "1.0.0"
entry = "src/main.arden"
files = [
    "src/math.arden",
    "src/strings.arden",
    "src/main.arden"
]
output = "my_project"
opt_level = "3"
output_kind = "bin"
```

Optional linking fields:

```toml
target = "x86_64-unknown-linux-gnu"
link_libs = ["ssl"]
link_search = ["native/lib"]
link_args = ["-Wl,--as-needed"]
```

## Config Fields

| Field | Required | Meaning |
| :--- | :--- | :--- |
| `name` | yes | Project name |
| `version` | yes | Project version |
| `entry` | yes | Entry source file |
| `files` | yes | Complete file list for the project |
| `output` | no | Final output path/name |
| `opt_level` | no | `0`, `1`, `2`, `3`, `s`, `z`, or `fast` |
| `target` | no | Optional target triple |
| `output_kind` | no | `bin`, `shared`, or `static` |
| `link_libs` | no | Extra libraries passed to the native linker backend |
| `link_search` | no | Extra library search paths |
| `link_args` | no | Extra raw linker args |

## Add A New Source File

When your project grows beyond `src/main.arden`, the normal flow is:

1. create the new file under `src/`
2. add it to `files = [...]`
3. import or reference it from the rest of the project
4. run `arden check` or `arden run`

Example:

```toml
files = [
    "src/math.arden",
    "src/strings.arden",
    "src/main.arden"
]
```

Arden intentionally does not treat the file list as optional metadata. It is part of build configuration.

## Validation Rules

Project config is not just advisory; Arden validates it.

Important rules:

- `entry` and every path in `files` must stay inside the project root
- duplicate file entries are rejected
- invalid `opt_level` values are rejected
- output paths cannot escape outside the project root
- `arden run` requires `output_kind = "bin"`

These checks exist so project builds stay predictable instead of silently doing the wrong thing.

## Project Commands

```bash
arden info
arden check
arden build
arden build --release
arden build --timings
arden run
arden test
arden fmt
```

Useful mental model:

- `info` tells you what Arden resolved
- `check` validates the configured graph
- `build` produces the artifact
- `run` builds and executes
- `test` discovers project tests
- `fmt` formats project sources

## Build Behavior

Project mode uses:

- the explicit file list from `arden.toml`
- dependency-aware project analysis
- namespace/package-aware rewriting
- project build cache in `.ardencache/`

If nothing relevant changed, Arden can exit early using cached build state.

Use:

```bash
arden build --timings
```

to inspect phase timings and reuse/rebuild counts.

## Common Mistakes

- adding a new file under `src/` but forgetting to add it to `files`
- changing output settings and assuming old local artifacts still match
- using `output_kind = "shared"` or `static` and then expecting `arden run` to execute it
- debugging project layout without checking `arden info`

## Examples

Starter example:

- [examples/starter_project/README.md](../../examples/starter_project/README.md)

Nested package example:

- [examples/nested_package_project/README.md](../../examples/nested_package_project/README.md)

Larger showcase:

- [examples/showcase_project/README.md](../../examples/showcase_project/README.md)
