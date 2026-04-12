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

Think of this as a closed set of valid states.
If a value is `Status`, it must be exactly one of these variants.

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

## Design Rule

Keep one enum focused on one domain concept.
If variants describe unrelated concerns, split into multiple enums.

Example:

- good: `ConnectionState` only for connection lifecycle
- bad: one enum mixing connection state, user role, and payment status

## Common Mistakes

- using strings for state when enum variants are safer
- adding wildcard (`_`) too early and losing clarity on handled cases
- mixing unrelated concepts in one large enum
- creating "catch-all" variant too early instead of modeling real states explicitly

## Decision Rule

If the set of states is known and finite, start with an enum.
If the data needs evolving behavior and methods, consider a class.

## Related

- [Control Flow](../basics/control_flow.md)
- Example: [`06_enums`](../../examples/single_file/language_core/06_enums/06_enums.arden)
