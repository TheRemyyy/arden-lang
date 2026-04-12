# Enums

## Why This Matters

Enums model finite states explicitly and make impossible states harder to represent.
For beginners: use an enum instead of magic strings like `"active"`, `"inactive"`, `"pending"`.

## Basic Enum

```arden
enum Status {
    Active,
    Inactive,
    Pending
}
```

## Runnable Matching Example

```arden
import std.io.*;

enum Status {
    Active,
    Inactive,
    Pending
}

function describe(status: Status): String {
    match (status) {
        Status.Active => { return "active"; },
        Status.Inactive => { return "inactive"; },
        _ => { return "pending"; }
    }
}

function main(): None {
    s: Status = Status.Active;
    println(describe(s));
    return None;
}
```

## Payload Variants

```arden
enum Result<T, E> {
    Ok(T),
    Error(E)
}
```

Payload variants let each state carry data.
Example: successful value in `Ok`, error value in `Error`.

## Common Mistakes

- using strings for state when enum variants are safer
- adding wildcard (`_`) too early and losing clarity on handled cases
- mixing unrelated concepts in one large enum

## Decision Rule

If the set of states is known and finite, start with an enum.
If the data needs evolving behavior and methods, consider a class.

## Related

- [Control Flow](../basics/control_flow.md)
- Example: [`06_enums`](../../examples/single_file/language_core/06_enums/06_enums.arden)
