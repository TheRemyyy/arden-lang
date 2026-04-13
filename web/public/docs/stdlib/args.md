# Args Module

## Why This Matters

CLI arguments are the default input channel for scripts, automation, and tooling commands.
If you build command-driven apps, this is usually your first runtime boundary.

## Quick Mental Model

- `Args.count()` -> number of argv entries
- `Args.get(i)` -> argument at index `i` (`String`)
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

function require_arg(index: Integer): Result<String, String> {
    if (index < 0) {
        return Result.error("argument index must be non-negative");
    }
    if (Args.count() > index) {
        return Result.ok(Args.get(index));
    }
    return Result.error("missing argument at index " + to_string(index));
}
```

Compiler note:

- `Args.get()` requires `Integer` index
- statically-known negative index (for example `Args.get(-1)`) is rejected
- return type is `String` (not `Option<String>`)

Runtime note:

- out-of-range access (for example `Args.get(999)`) fails at runtime with
  `Args.get() index out of bounds`
- runtime negative index values (for example `i = -1; Args.get(i)`) fail with
  `Args.get() index cannot be negative`

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
- calling `Args.get(i)` without bounds guard (runtime fail on out-of-range)
- parsing numeric flags without validation/error branch

## Example In Repo

- [`22_args`](../../examples/single_file/stdlib_and_system/22_args/22_args.arden)
