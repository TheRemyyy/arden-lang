# Args Module

## Why This Matters

CLI arguments are the default input channel for scripts, automation, and tooling commands.
If you build command-driven apps, this is usually your first runtime boundary.

## Quick Mental Model

- `Args.count()` tells you how many argv entries exist
- `Args.get(i)` returns argv entry at index `i`
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

argc: Integer = Args.count();
println("argc={argc}");

if (argc > 1) {
    arg1: String = Args.get(1);
    println("first={arg1}");
}
```

## Safe Pattern

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

## Typed Function Value Usage

```arden
import std.args.*;

count_fn: () -> Integer = Args.count;
get_fn: (Integer) -> String = Args.get;
```

## Example In Repo

- [`22_args`](../../examples/single_file/stdlib_and_system/22_args/22_args.arden)
