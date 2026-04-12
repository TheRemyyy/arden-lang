# Interfaces

## Why This Matters

Interfaces let you define capability contracts without coupling callers to one concrete class.

## Basic Interface

```arden
interface Named {
    function name(): String;
}
```

## Implementation

```arden
class User implements Named {
    value: String;

    constructor(value: String) {
        this.value = value;
    }

    function name(): String {
        return this.value;
    }
}
```

## Usage Through Interface Types

```arden
function printName(item: Named): None {
    println(item.name());
    return None;
}
```

## Related

- [Classes](classes.md)
- [Generics](../advanced/generics.md)
- Example: [`37_interfaces_contracts`](../../examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden)
