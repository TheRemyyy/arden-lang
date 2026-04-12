# Classes

## Why This Matters

Classes are Arden's nominal state+behavior type. Use them when data and methods belong together.

## Basic Class

```arden
class User {
    mut age: Integer;

    constructor(age: Integer) {
        this.age = age;
    }

    function birthday(): None {
        this.age += 1;
        return None;
    }
}
```

## Access and Mutation

- fields can be mutable (`mut`) or immutable
- method calls participate in borrow/mutability analysis

## Constructors

Use constructors to ensure instances start valid.

## Best Practices

- keep constructor invariants explicit
- avoid exposing mutable fields unnecessarily
- prefer methods for controlled mutation

## Related

- [Interfaces](interfaces.md)
- [Ownership](../advanced/ownership.md)
- Example: [`05_classes`](../../examples/single_file/language_core/05_classes/05_classes.arden)
