# Error Handling

## Why This Matters

Arden makes recoverable failure explicit in types, so callers cannot ignore error paths accidentally.

## `Option<T>`

Use for optional presence.

```arden
enum Option<T> {
    Some(T),
    None
}
```

## `Result<T, E>`

Use for success/error outcomes.

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

## Compile-Time Validation

`arden check` enforces that `?` matches surrounding function/lambda return kind.

## Practical Guidance

- `Option<T>` for expected absence
- `Result<T, E>` for actionable failure context
- avoid `unwrap()` outside tests/proven invariants
