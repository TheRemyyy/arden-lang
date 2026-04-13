# Error Handling

## Why This Matters

Arden makes recoverable failure explicit in types, so callers cannot ignore error paths accidentally.

## Choose the Right Result Shape

- `Option<T>`: value may be absent, absence is expected and not exceptional
- `Result<T, E>`: operation can fail and caller needs failure context

## `Option<T>`

Built-in shape (conceptually):

- `Some(T)`
- `None`

Construction styles accepted by current compiler:

- enum-variant style: `Option.Some(v)`, `Option.None`
- helper style: `Option.some(v)`, `Option.none()`
- zero-arg generic constructor: `Option<T>()` (initializes to `None`)

## `Result<T, E>`

Built-in shape (conceptually):

- `Ok(T)`
- `Error(E)`

Construction styles accepted by current compiler:

- enum-variant style: `Result.Ok(v)`, `Result.Error(e)`
- helper style: `Result.ok(v)`, `Result.error(e)`
- zero-arg generic constructor: `Result<T, E>()` (initializes to `Error(default(E))`)

Display note for zero-arg `Result<T, E>()`:

- with `E = Integer`, default often appears as `Error(0)`
- with `E = String`, default often appears as `Error()`

Practical recommendation: use explicit `ok/error` (or enum variants) in normal
code so intent is obvious at call sites.

## `?` Operator

`?` unwraps success and returns early on failure.

```arden
function divide(a: Integer, b: Integer): Result<Integer, String> {
    if (b == 0) {
        return Result.Error("division by zero");
    }
    return Result.Ok(a / b);
}

function compute(): Result<Integer, String> {
    x: Integer = divide(10, 2)?;
    y: Integer = divide(x, 5)?;
    return Result.Ok(y);
}
```

## Compiler Validation

`arden check` enforces that `?` is only used in compatible return contexts.
If surrounding function/lambda cannot propagate that error kind, it fails type checking.

## Match-Based Handling

Use `match` when you want explicit branch behavior:

```arden
import std.io.*;

function report(value: Result<Integer, String>): None {
    match (value) {
        Result.Ok(v) => { println("ok={v}"); },
        Result.Error(e) => { println("error={e}"); }
    }
    return None;
}
```

## Practical Guidance

- use `Option<T>` for expected missing data
- use `Result<T, E>` when caller needs diagnostics/recovery decisions
- avoid `unwrap()` outside tests or proven invariants
- keep error values structured and meaningful (avoid opaque text blobs)

## Common Mistakes

- using `Option<T>` where failure reason is needed
- converting every failure into `String` too early
- deep `match` pyramids where `?` would keep flow cleaner

## Related

- [Control Flow](../basics/control_flow.md)
- [Async / Await](async.md)
- Example: [`13_error_handling`](../../examples/single_file/safety_and_async/13_error_handling/13_error_handling.arden)
