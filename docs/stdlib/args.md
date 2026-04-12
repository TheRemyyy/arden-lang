# Args Module

## Why This Matters

CLI arguments are the default input channel for scripts, automation, and tooling commands.
If you build command-driven apps, this is usually your first runtime boundary.

## Quick Mental Model

- `Args.count()` -> number of argv entries
- `Args.get(i)` -> argument at index `i`
- index `0` is executable path
- user arguments start at index `1`

Import:

```arden
import std.args.*;
```

## Basic Usage

```arden
import std.args.*;
import std.io.*;

function main(): None {
    argc: Integer = Args.count();
    println("argc={argc}");

    if (argc > 1) {
        arg1: String = Args.get(1);
        println("first={arg1}");
    }
    return None;
}
```

## Safe Access Pattern

Always guard `Args.get(i)` by checking `Args.count()` first.

```arden
import std.args.*;

function maybe_get(index: Integer): Option<String> {
    if (Args.count() > index) {
        return Option.Some(Args.get(index));
    }
    return Option.None;
}
```

## Function Value Usage

```arden
import std.args.*;

function main(): None {
    count_fn: () -> Integer = Args.count;
    get_fn: (Integer) -> String = Args.get;
    _c: Integer = count_fn();
    return None;
}
```

## Effect Note

`Args.*` calls participate in effect checks and require `io` capability on caller context.
See [Effects](../advanced/effects.md).

## Common Mistakes

- treating `argv[0]` as first user argument
- calling `Args.get(i)` without bounds guard
- parsing numeric flags without validation/error branch

## Example In Repo

- [`22_args`](../../examples/single_file/stdlib_and_system/22_args/22_args.arden)
