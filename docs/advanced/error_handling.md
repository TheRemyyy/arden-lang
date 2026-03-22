# Error Handling

Apex provides robust error handling using the `Option<T>` and `Result<T, E>` types.

## Option<T>

Represents a value that might be present or missing.

```apex
enum Option<T> {
    Some(T),
    None
}
```

Usage:

```apex
function findParams(id: Integer): Option<String> {
    if (id == 0) {
        return Option.None;
    }
    return Option.Some("Param");
}
```

### Option Methods

- `is_some(): Boolean` - Returns `true` if the option is a `Some` value.
- `is_none(): Boolean` - Returns `true` if the option is a `None` value.
- `unwrap(): T` - Returns the inner value or panics if `None`.

## Result<T, E>

Represents a success (`Ok`) or failure (`Error`).

```apex
enum Result<T, E> {
    Ok(T),
    Error(E)
}
```

Usage:

```apex
function divide(a: Integer, b: Integer): Result<Integer, String> {
    if (b == 0) {
        return Result.Error("Division by zero");
    }
    return Result.Ok(a / b);
}
```

### Result Methods

- `is_ok(): Boolean` - Returns `true` if the result is `Ok`.
- `is_error(): Boolean` - Returns `true` if the result is `Error`.
- `unwrap(): T` - Returns the success value or panics if `Error`.

## The `?` Operator

The `?` operator simplifies error propagation. If a Result is `Error`, it returns early.

```apex
function computation(): Result<Integer, String> {
    val: Integer = divide(10, 2)?; // Unwraps or returns Error
    val2: Integer = divide(val, 0)?; // Returns Error("Division by zero")
    return Result.Ok(val2);
}
```

`apex check` validates `?` at the function boundary:
- `Option<T>?` requires the enclosing function to return `Option<...>`
- `Result<T, E>?` requires the enclosing function to return `Result<..., E-compatible>`
- nested lambda bodies are checked against their own lambda return context, not the outer function's `Option`/`Result` return type
