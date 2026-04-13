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

## Interface Inheritance and Inline Method Bodies

Compiler surface includes:

- interface inheritance (`interface A extends B`)
- interface methods with inline bodies

Current behavior:

- `extends` contracts work and are enforced
- classes still must explicitly implement required interface methods
  even when interface method has an inline body

Practical implication:

- do not assume Java/C#-style automatic default-method inheritance into classes
- write explicit class methods for required interface members
- treat inline interface bodies as contract-adjacent behavior definition, not implicit class implementation

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
- Example: [`45_interface_inline_body_rules`](../../examples/single_file/language_edges/45_interface_inline_body_rules/45_interface_inline_body_rules.arden)
