# Error Handling

## Why This Matters

Arden makes recoverable failure explicit in types, so callers cannot ignore error paths accidentally.

## Choose the Right Result Shape

- `Option<T>`: value may be absent, absence is expected and not exceptional
- `Result<T, E>`: operation can fail and caller needs failure context

## `Option<T>`

```arden
enum Option<T> {
    Some(T),
    None
}
```

## `Result<T, E>`

```arden
enum Result<T, E> {
    Ok(T),
    Error(E)
}
```

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
