# Interfaces

## Why This Matters

Interfaces define behavior contracts without forcing callers to depend on one concrete class.
For beginners: an interface says "anything that implements these methods can be used here".

## Basic Interface

```arden
interface Named {
    function name(): String;
}
```

## Implementation + Usage (Runnable)

```arden
import std.io.*;

interface Named {
    function name(): String;
}

class User implements Named {
    value: String;

    constructor(value: String) {
        this.value = value;
    }

    function name(): String {
        return this.value;
    }
}

function printName(item: Named): None {
    println(item.name());
    return None;
}

function main(): None {
    user: User = User("Ada");
    printName(user);
    return None;
}
```

## Interface Inheritance and Default Implementations

Compiler surface includes:

- interface inheritance (`interface A extends B`)
- interface methods with inline bodies

Current behavior:

- `extends` contracts work and are enforced
- classes still must explicitly implement required interface methods
  even when interface method has an inline body

Use this to share default behavior while still enforcing capability contracts.

## Minimal Capability Pattern

Prefer small focused interfaces over one large "god interface".

Example shape:

- `Readable` with one read method
- `Writable` with one write method
- combined interface only where both are truly required

## Why It Helps

- decouples API from implementation details
- makes swapping implementations easier
- keeps function signatures stable at call sites

## Common Mistakes

- using interfaces with only one real implementation forever
- putting too many unrelated methods in one interface
- exposing concrete class types where interface would reduce coupling
- forcing interface abstraction too early in small code where class-only API is clearer

## Decision Rule

Use an interface when multiple types should satisfy the same capability.
If you have a single concrete model and no abstraction need, a class alone is simpler.

## Related

- [Classes](classes.md)
- [Generics](../advanced/generics.md)
- Example: [`37_interfaces_contracts`](../../examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden)
