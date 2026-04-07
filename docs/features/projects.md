# Multi-File Projects

Arden supports organizing code into multi-file projects using a project configuration file.

## Project Structure

An Arden project consists of:

```
my_project/
├── arden.toml          # Project configuration
├── src/               # Source directory
│   ├── main.arden      # Entry point
│   ├── utils.arden     # Utility functions
│   └── lib.arden       # Library code
└── README.md          # Documentation
```

## Creating a New Project

```bash
arden new my_project
cd my_project
arden run
```

This creates:
- `arden.toml` - Project configuration
- `src/main.arden` - Entry point with `main()` function
- `README.md` - Project documentation

## Project Configuration (arden.toml)

```toml
name = "my_project"
version = "1.0.0"
entry = "src/main.arden"
files = [
    "src/utils.arden",
    "src/main.arden"
]
output = "my_project"
opt_level = "3"
output_kind = "bin"
link_libs = ["ssl"]
link_search = ["native/lib"]
link_args = ["-Wl,--as-needed"]
```

### Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Project name |
| `version` | Yes | Project version |
| `entry` | Yes | Entry point file (contains `main()`) |
| `files` | Yes | List of all source files to compile |
| `output` | No | Output binary name (default: project name) |
| `opt_level` | No | Final Clang optimization level: `0`, `1`, `2`, `3`, `s`, `z`, or `fast` (default: `3`) |
| `target` | No | Target triple (optional) |
| `output_kind` | No | Final artifact kind: `bin`, `shared`, or `static` (default: `bin`) |
| `link_libs` | No | Extra libraries passed to Clang as `-l<name>` |
| `link_search` | No | Extra library search paths passed as `-L<path>` |
| `link_args` | No | Extra raw linker arguments forwarded to Clang |

Path safety:
- `entry` and every path in `files` must resolve inside the project root.
- Paths that escape through `..` segments or symlinks are rejected during validation.

## Project Commands

### Build Project

```bash
arden build              # Debug build
arden build --release    # Optimized build
```

### Run Project

```bash
arden run                # Build and run
arden run --release      # Optimized build and run
arden run arg1 arg2      # Pass arguments
```

### Check Project

```bash
arden check              # Check entry point
arden check src/lib.arden # Check specific file
```

### Show Project Info

```bash
arden info
```

Output:
```
Project Information
  Name: my_project
  Version: 1.0.0
  Entry: src/main.arden
  Output: my_project
  Output Kind: Bin
  Opt Level: 3
  Target: native/default
  Root: /path/to/project

Source Files:
  - src/utils.arden
  - src/main.arden
```

## Optimization Behavior

- Project builds (`arden build`, `arden run` in a project) use `opt_level` from `arden.toml`.
- Valid values are: `0`, `1`, `2`, `3`, `s`, `z`, `fast`.
- If `opt_level` is missing, Arden defaults to `3`.
- If `opt_level` is present but invalid, Arden now rejects the config with a direct validation error instead of silently changing optimization behavior.
- If `target` is set in `arden.toml`, Arden forwards it to Clang as `--target <triple>`.
- When `target` is set, host-native tuning flags are skipped to keep target/toolchain compatibility.
- `output_kind = "shared"` emits a shared library, and `output_kind = "static"` emits a static archive.
- `link_libs`, `link_search`, and `link_args` let project builds declare native link requirements in `arden.toml`.
- Single-file mode (`arden compile file.arden`, `arden run file.arden`) defaults to maximum-performance settings.

## How It Works

1. **AST Build Pipeline**: All files listed in `files` are parsed and combined as declarations in one project AST.
   - Each path in `files` must be unique; duplicate entries are rejected during project validation.
2. **Import Checking**: Cross-file calls are validated by the import checker. Use explicit `import` statements when calling functions from other namespaces/modules.
3. **Deterministic Symbol Mangling**: Top-level function/class/module symbols are rewritten to namespace-qualified internal names during project build.
4. **Scope-Aware Rewrite**: Local bindings (parameters, `let` variables, loop vars, lambda params, match bindings) are never rewritten as imported/global symbols.
5. **Collision Safety**: Duplicate top-level function/class/enum/module names across namespaces are rejected during project analysis.
6. **Entry Point**: The `entry` file must contain the `main()` function.
7. **Compilation**: Project is compiled to a single binary.

## Best Practices

1. **Organize by Functionality**: Group related functions into files
   - `math.arden` - Mathematical functions
   - `string.arden` - String utilities
   - `io.arden` - Input/output operations

2. **Use src/ Directory**: Keep source files organized in a directory

3. **Entry Point**: Keep `main.arden` minimal, delegate to other modules

4. **Documentation**: Add README.md to explain project structure

## Example Projects

See `examples/multi_file_project/` for the minimal starter example.

See `examples/insane_showcase_project/` for a larger project-mode showcase that combines:
- cross-file packages
- nested modules
- interfaces and inheritance
- generics and enums
- async/await plus `await_timeout`
- file I/O and interpolation-heavy reporting

Project-mode name resolution now preserves async return types through package aliases and deep module chains too, so calls such as `await(analytics.Api.V2.score(10))` keep their declared inner type during codegen instead of degrading to the default numeric fallback. The same alias-aware resolution also applies to exact-typed first-class references like `f: (Integer) -> Task<Float> = analytics.Api.V2.score`.

Project-mode reporting paths also preserve merged mixed-numeric expression types during codegen, so inline expressions such as `"{if (flag) { 1 } else { 2.5 }}"` or equivalent `match` expressions now keep their `Float` display/phi lowering instead of degrading to the first arm's integer shape.

Expected-type propagation now also reaches `if` and `match` expression branches for first-class functions, so expression forms that return callables, including builtin values like `to_float`, behave the same way as direct assignments and returns.

That same expected-type propagation now extends through typed containers and constructors in project builds as well. `Option.some(...)`, `Result.ok(...)`, `Result.error(...)`, and constructor calls such as `Box<(Integer) -> Float>(to_float)` now preserve first-class function payload types instead of degrading into late `Unknown variable` codegen failures, even when the callable is pulled back out through a generic field like `box.value(1)`.

The same generic-function machinery now also stays correct for user-defined generic classes whose names overlap builtin containers. Project-mode flows like `class Box<T> { function map<U>(f: (T) -> U): Box<U> ... }` and `Box<Integer>(1).map<Float>(to_float)` now keep the user-defined `Box<U>` return type and the specialized `(Integer) -> Float` callback signature all the way through typechecking and codegen.

That fix now extends to builtin-shaped multi-parameter types too. User-defined classes such as `Result<T, E>` keep their owner-generic substitutions inside explicit generic method specializations, so methods like `map_ok<U>` can safely construct `Result<U, E>` and call builtins like `to_float` without drifting back into builtin `Result` lowering or mismatched runtime layouts.

Those owner-specialized rewrites now also derive their generic bindings from structured receiver expressions, not just direct constructor syntax. Project-mode calls such as `(if (flag) { Result<Integer, String>(1, "ok") } else { Result<Integer, String>(2, "ok") }).map_ok<Float>(to_float)` and the same pattern for user-defined `Map<K, V>` now keep the correct specialized receiver layout and runtime result.

Method-returned lambdas now also capture `this` correctly in project builds. That includes generic method flows where the lambda body reads instance state and calls a specialized callback, such as `Box<Integer>(7).mk<Float>(to_float)` returning a zero-arg closure that later evaluates `f(this.value)`.

Project-mode closure and async capture analysis now also follows nested expression branches like `if` expressions, so returned lambdas or async tasks can safely reference outer locals from inside branch tails without degrading into late `Unknown variable` codegen failures.

Async borrowed-capture diagnostics are now shadowing-aware too. A local name re-used inside the async block no longer gets mistaken for a capture of an outer borrowed reference with the same identifier.

The same shadowing-aware async analysis now applies to `match` pattern bindings as well, so enum payload names introduced by an arm safely shadow unrelated outer borrowed locals with the same identifier.

Project-mode type errors now also render demangled module paths in diagnostics. That now covers generic bound failures, branch mismatches, assignment mismatches, unknown-field errors, non-function-call errors, bad condition/index/await operand errors, pattern and enum-variant mismatches, return/call-site type mismatches, unknown declared types in function/interface/extern/enum signatures, and other type-driven diagnostics, so reported types use source-style names like `lib.Plain`, `lib.Box<lib.Named>`, `u.Api.Missing`, or `lib.Api.Person` instead of internal rewritten symbols. Qualified enum patterns like `lib.Choice.Left` are also parsed as real variant patterns even without payload bindings, so invalid variants fail predictably during project analysis.

The same qualified-path handling now also reaches codegen-side enum value inference for unit variants. Project and single-file flows like `match (Kind.A) { ... }`, `match (util.E.A) { ... }`, and `match (u.E.A) { ... }` now keep the real enum type through codegen instead of degrading the scrutinee to the default integer fallback, which previously made every qualified unit-variant arm miss and could corrupt mixed-numeric `match` expression runtime results.

That enum-alias fix also now covers exact imported unit variants on both the value side and the pattern side. Flows like `import E.A as A; value: E = A;` and `match (E.B) { A => ... }` now keep `A` as an enum variant alias all the way through typechecking, exhaustiveness checks, closure-capture analysis, and codegen instead of degrading into `Unknown variable` failures or accidental catch-all bindings.

Single-file import validation now understands those same local type roots as valid import targets too. Exact alias imports such as `import E.A as First` no longer get rejected before typechecking/codegen, and payload alias patterns like `First(v)` now bind `v` through codegen the same way as spelled-out variants like `E.A(v)`.

That alias-aware type resolution now also covers imported type names themselves, not just enum variants. Exact aliases such as `import Box as B`, `import M.Box as B`, or generic forms built on those names now flow through annotations and constructor expressions consistently, so `B(2)` and `b: B` resolve to the imported class instead of falling back to `Unknown type`.

The same single-file alias path now stays intact once generics and enum-root constructors enter the picture too. Exact imported generic type aliases like `import Box as B; B<Integer>(2)` and nested forms like `import M.Box as B; B<Integer>(2)` now trigger the right generic specialization during codegen instead of failing as `Unknown type: B<Integer>`, and imported enum type aliases such as `import E as Alias; Alias.A(2)` now behave as real enum roots in both value construction and `match` patterns.

Namespace aliases now reach those same constructor-style single-file paths too. Flows like `import U as u; u.M.Box<Integer>(2)`, `u.E.A(2)`, and `u.M.E.A(2)` now resolve through typechecking, specialization discovery, enum-value inference, and codegen as real type/enum roots instead of degrading into variable lookups on `u`.

The same explicit-generic pipeline now also finishes the second half of imported generic free-function aliases that return generic classes. Single-file and project flows like `import M.mk as mk; mk<Integer>(2).get()` now re-run generic class specialization after emitting the function `__spec__`, so the generated body constructs and calls the specialized class layout instead of mixing a specialized function wrapper with the unspecialized class ABI.

```toml
# arden.toml
name = "multi_file_demo"
version = "1.0.0"
entry = "src/main.arden"
files = [
    "src/math_utils.arden",
    "src/string_utils.arden",
    "src/main.arden"
]
output = "multi_file_demo"
output_kind = "bin"
```

```arden
// src/math_utils.arden
function factorial(n: Integer): Integer {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

// src/main.arden
import std.io.*;

function main(): None {
    result: Integer = factorial(5);
    println("5! = " + to_string(result));
    return None;
}
```

## Single-File Mode

You can still compile single files without a project:

```bash
arden compile file.arden
arden run file.arden
```

Note: When in a project directory, the compiler will warn you to use `arden build` instead.
