# Enums

## Why This Matters

Enums model finite states explicitly and pair naturally with exhaustive `match` logic.

## Basic Enum

```arden
enum Status {
    Active,
    Inactive,
    Pending
}
```

## Payload Variants

```arden
enum Result<T, E> {
    Ok(T),
    Error(E)
}
```

## Pattern Matching

```arden
match (status) {
    Status.Active => { println("active"); },
    Status.Inactive => { println("inactive"); },
    _ => { println("other"); }
}
```

## Best Practices

- use enums instead of magic strings/ints for state machines
- prefer exhaustive `match` to long `if` chains

## Related

- [Control Flow](../basics/control_flow.md)
- Example: [`06_enums`](../../examples/single_file/language_core/06_enums/06_enums.arden)
