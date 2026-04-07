# Arden Examples

This directory is the executable tour of the language and toolchain.

The examples are split into three layers:

- focused single-file feature demos
- larger single-file app samples
- multi-file project examples

The point of this directory is not volume for its own sake. It exists so a reader can move from tiny syntax examples to application-shaped code without leaving the repository.

## Recommended First Pass

If you only want a few files to start with:

1. `01_hello.arden`
2. `10_ownership.arden`
3. `14_async.arden`
4. `24_test_attributes.arden`
5. `35_visibility_enforcement.arden`
6. `starter_project/`
7. `showcase_project/`

That sequence is deliberate:

- `01_hello` and `02_variables` establish the syntax shape
- `05_classes`, `08_modules`, and `09_generics` introduce the core structural features
- `10_ownership` and `13_error_handling` cover the parts people usually need to see before trusting a systems language
- `14_async` and `24_test_attributes` show that workflow-oriented features are already in repo
- the multi-file projects demonstrate how `arden.toml` based development fits together

## How To Use This Directory

Three good ways to work through the examples:

1. Read one example, run it, then change it locally.
2. Pair examples with the matching docs page.
3. Use the repository sweep scripts after making compiler changes.

Suggested doc pairings:

| Example | Companion docs |
| :--- | :--- |
| `05_classes.arden` | `docs/features/classes.md` |
| `08_modules.arden` | `docs/features/modules.md` |
| `09_generics.arden` | `docs/advanced/generics.md` |
| `10_ownership.arden` | `docs/advanced/ownership.md` |
| `14_async.arden` | `docs/advanced/async.md` |
| `24_test_attributes.arden` | `docs/features/testing.md` |
| `27_extern_c_interop.arden` | `docs/compiler/cli.md` and bindgen docs/examples |

## Feature Examples

| File | Covers |
| :--- | :--- |
| `01_hello.arden` | basic I/O and entrypoint |
| `02_variables.arden` | declarations, mutability, primitive types |
| `03_math.arden` | arithmetic and math stdlib |
| `04_control_flow.arden` | `if`, `while`, `for`, `break`, `continue` |
| `05_classes.arden` | classes, methods, constructors, visibility |
| `06_enums.arden` | enums and variant modeling |
| `07_interfaces.arden` | interface contracts |
| `08_modules.arden` | modules and namespaced calls |
| `09_generics.arden` | generics, containers, helper types |
| `10_ownership.arden` | ownership and borrowing |
| `11_lambdas.arden` | closures and higher-order code |
| `12_string_interp.arden` | interpolation |
| `13_error_handling.arden` | `Option`, `Result`, `?`, `require` |
| `14_async.arden` | async functions and await |
| `15_stdlib.arden` | stdlib overview usage |
| `16_pattern_matching.arden` | `match` patterns |
| `17_comprehensive.arden` | mixed end-to-end feature use |
| `18_file_io.arden` | file operations |
| `19_time.arden` | time APIs |
| `20_system.arden` | shell, cwd, env, OS |
| `21_conversions.arden` | numeric and string conversions |
| `22_args.arden` | command-line arguments |
| `23_str_utils.arden` | string helpers |
| `24_test_attributes.arden` | built-in test framework |
| `25_range_types.arden` | `Range<T>` iteration |
| `26_effect_system.arden` | effect attributes |
| `27_extern_c_interop.arden` | C interop |
| `28_async_runtime_control.arden` | `is_done`, `await_timeout`, `cancel` |
| `29_effect_inference_and_any.arden` | effect inference |
| `30_extern_variadic_printf.arden` | variadic extern calls |
| `31_extern_abi_link_name.arden` | ABI and symbol aliasing |
| `32_extern_safe_wrapper.arden` | wrapping raw extern calls |
| `33_extern_ptr_types.arden` | pointer-based extern signatures |
| `34_bindgen_workflow.arden` | generated extern declarations |
| `35_visibility_enforcement.arden` | `public` / `private` / `protected` |
| `36_inheritance_extends.arden` | class inheritance |
| `37_interfaces_contracts.arden` | interface implementation |
| `38_import_aliases.arden` | alias imports |
| `39_compound_assign.arden` | compound assignment |
| `40_borrow_scope_recovery.arden` | borrow lifetime edge cases |

## App-Style Examples

- `demo_banking.arden`
- `demo_calculator.arden`
- `demo_data_structures.arden`
- `demo_game.arden`
- `demo_notes.arden`
- `demo_todo.arden`

These are useful when you want something closer to application-shaped code instead of a small feature sample.

Recommended intent for each:

- `demo_calculator.arden` for control flow and user interaction
- `demo_todo.arden` for CRUD-shaped logic and collections
- `demo_banking.arden` for state transitions and validation
- `demo_data_structures.arden` for container-heavy examples
- `demo_notes.arden` for string/file oriented workflows
- `demo_game.arden` for longer single-file program structure

## Multi-File Projects

- [starter_project/README.md](starter_project/README.md)
- [nested_package_project/README.md](nested_package_project/README.md)
- [showcase_project/README.md](showcase_project/README.md)
- `minimal_project/`

These matter because they exercise the part of Arden that most language toy repos never reach: explicit project graphs, entrypoint configuration, and multi-file symbol resolution.

## Example Groups By Topic

### Syntax And Basics

`01_hello` through `04_control_flow`

### Type System And Structure

`05_classes` through `09_generics`

### Ownership, Errors, And Async

`10_ownership`, `13_error_handling`, `14_async`, `16_pattern_matching`

### Tooling And Workflow

`24_test_attributes`, `27_extern_c_interop`, `34_bindgen_workflow`

### Visibility, Modules, And Project Boundaries

`35_visibility_enforcement` through `40_borrow_scope_recovery`

## Running Examples

Run a single example:

```bash
arden run examples/01_hello.arden
```

Run a multi-file project:

```bash
cd examples/starter_project
arden run
```

Run the repository example sweep:

```bash
./scripts/examples_smoke_linux.sh
./scripts/examples_smoke_macos.sh
scripts\examples_smoke.bat
```

If you are changing parser, type-checker, borrow checker, project mode, or stdlib behavior, running the example sweep is the fastest way to catch accidental regressions in user-facing code.
