# Quick Start

This guide walks through the fastest ways to get useful output from Arden.

The goal is not just to run one file, but to understand the normal workflow shape:

1. run a single file
2. inspect it with `check`, `fmt`, and `profile`
3. create a project
4. move into project-aware `info`, `test`, and `build`

## 1. Run A Single File

Create `hello.arden`:

```arden
import std.io.*;

function main(): None {
    println("Hello, Arden!");
    return None;
}
```

Run it:

```bash
arden run hello.arden
```

Useful companion commands:

```bash
arden check hello.arden
arden compile hello.arden
arden fmt hello.arden
arden lint hello.arden
arden profile hello.arden
```

What each one is for:

- `run` builds and executes the file
- `check` stops before native code generation
- `compile` emits a native artifact without running it
- `fmt` normalizes formatting
- `lint` reports static findings
- `profile` gives a one-run timing summary

## 2. Look At A Slightly Larger Program

Once the hello-world path works, try something with a loop and state:

```arden
import std.io.*;

function main(): None {
    name: String = "Arden";
    mut counter: Integer = 0;

    println("Starting count for {name}...");

    while (counter < 5) {
        counter += 1;
        println("Count: {counter}");
    }

    println("Done!");
    return None;
}
```

Run it:

```bash
arden run program.arden
```

If you want more than this, jump straight to:

- `examples/04_control_flow.arden`
- `examples/10_ownership.arden`
- `examples/14_async.arden`

Reference index:

- [Examples](../../examples/README.md)

## 3. Create A Project

```bash
arden new my_project
cd my_project
arden run
```

That generates:

- `arden.toml`
- `src/main.arden`
- `README.md`

Inspect the resolved config:

```bash
arden info
```

At that point you are using project mode rather than ad-hoc single-file compilation.

## 4. Understand `arden.toml`

The generated project config is the source of truth for:

- project name and version
- entry file
- full source file list
- output file name
- optimization level
- output kind

Minimal example:

```toml
name = "my_project"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "my_project"
opt_level = "3"
output_kind = "bin"
```

As you add files, update `files = [...]` explicitly. Arden project mode does not rely on vague directory scanning.

Reference:

- [Projects](../features/projects.md)

## 5. Pass Program Arguments

`arden run` forwards trailing arguments to the compiled program.

```bash
arden run hello.arden one two three
```

Inside Arden, read them with:

- `Args.count()`
- `Args.get(index)`

Reference:

- [Stdlib: Args](../stdlib/args.md)
- [Examples: Args](../../examples/22_args.arden)

## 6. Use The Built-In Tooling

Arden bundles several common workflows:

```bash
arden fmt .
arden fix src/main.arden
arden test
arden bench hello.arden --iterations 5
arden bindgen sample.h -o bindings.arden
```

That matters because once you are in a project you do not need separate unrelated wrappers for formatting, testing, and simple performance checks.

Reference:

- [Compiler CLI](../compiler/cli.md)
- [Testing](../features/testing.md)

## 7. Use A Shebang Script

On Unix-like systems:

```arden
#!/usr/bin/env arden
import std.io.*;

function main(): None {
    println("hello");
    return None;
}
```

This is useful for quick tooling scripts where creating a whole project would be overkill.

## 8. Common First-Day Workflow

A realistic first session often looks like this:

```bash
arden new scratchpad
cd scratchpad
arden info
arden run
arden check --timings
arden test
arden build --release
```

That sequence exercises the main project commands without requiring any extra repo-specific setup.

## If Something Fails Early

The most common setup problems are toolchain related:

- LLVM or Clang missing
- linker mismatch (`mold` on Linux, `lld` on macOS/Windows)
- `arden` not on your shell path yet

Use:

```bash
arden --help
arden info
```

and compare your machine against:

- [Installation](installation.md)

## Next Steps

- [Syntax](../basics/syntax.md)
- [Types](../basics/types.md)
- [Projects](../features/projects.md)
- [Testing](../features/testing.md)
- [Examples](../../examples/README.md)
