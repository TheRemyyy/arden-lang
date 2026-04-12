# Variables

Variables in Arden are explicit and static: each binding has a declared type and clear mutability.

## Why This Matters

Most production bugs around state come from hidden mutation and unclear ownership.
Arden forces you to answer two questions at declaration time:

- what type is this value?
- can this binding be reassigned (`mut`) or not?

That keeps state transitions visible in code review.

## Declaration Syntax

Use `name: Type = value;`.

```arden
function main(): None {
    age: Integer = 30;
    name: String = "Alice";
    return None;
}
```

`let` is optional and equivalent:

```arden
function main(): None {
    let score: Integer = 10;
    level: Integer = 10;
    return None;
}
```

## Mutability (`mut`)

Bindings are immutable by default.

```arden
function main(): None {
    x: Integer = 10;
    // x = 20; // Error: immutable variable
    return None;
}
```

Mark the binding as mutable when reassignment is intentional:

```arden
function main(): None {
    mut count: Integer = 0;
    count = count + 1;
    count += 1;
    return None;
}
```

### Quick Rule

- use immutable bindings by default
- add `mut` only when the variable models real changing state

## Shadowing

You can create a new binding with the same name in the same scope.

```arden
function main(): None {
    input: String = "100";
    input: Integer = to_int(input); // new binding, new type
    return None;
}
```

Use this for staged transformations, not as a replacement for clear naming.

## Variables vs References

A variable owns its value by default. References borrow access:

- `&T` read-only borrow
- `&mut T` mutable borrow

```arden
function main(): None {
    mut n: Integer = 5;
    read: &Integer = &n;
    write: &mut Integer = &mut n;
    *write = 9;
    return None;
}
```

See [Ownership and Borrowing](../advanced/ownership.md) for full rules.

## Common Mistakes

- trying to reassign a non-`mut` binding
- treating shadowing as mutation
- borrowing mutably from an immutable binding
