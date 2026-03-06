# Ownership and Borrowing

Apex uses an ownership system inspired by Rust to ensure memory safety without a garbage collector.

## Ownership Rules

1. Each value in Apex has a variable that's called its **owner**.
2. There can only be one owner at a time.
3. When the owner goes out of scope, the value will be dropped.

## Move Semantics

When you assign a value to another variable or pass it to a function, ownership is transferred (moved).

```apex
s1: String = "hello";
s2: String = s1; // s1 is moved to s2
// println("{s1}"); // Error: s1 is invalid
```

## Borrowing

You can allow other code to access data without taking ownership by using **references**.

### Immutable References

Create an immutable reference with `&`. You can have multiple immutable references.

```apex
function len(s: &String): Integer {
    return strlen(*s); // Dereference might be implicit
}

s1: String = "hello";
leng: Integer = len(&s1); // Pass reference
println("{s1}"); // s1 is still valid
```

### Mutable References

Create a mutable reference with `&mut`. You can only have **one** mutable reference at a time.

```apex
function append(s: &mut String): None {
    // ... modify s
    return None;
}

mut s: String = "hello";
append(&mut s);
```

## Lifetimes

(Advanced)
Apex tracks lifetimes to ensure references do not outlive the data they refer to. This is currently handled implicitly by the compiler.

## Borrow Checker Edge Behavior

The compiler enforces these edge cases explicitly:

- **use-after-move** is rejected
- **move while borrowed** is rejected
- **double mutable borrow** is rejected
- immutable borrow state is released after scope exit, so the value is movable again
- lambda captures participate in move/borrow analysis for outer variables
- compound assignment on a currently borrowed variable is rejected
