# Modules

## Why This Matters

Modules create clear symbol boundaries and scale code organization beyond single files.

## Basic Module

```arden
module MathUtil {
    function twice(x: Integer): Integer {
        return x * 2;
    }
}

value: Integer = MathUtil.twice(10);
```

## Practical Guidance

- use modules to group related functionality
- prefer explicit imports over implicit global coupling
- avoid deep nesting unless it improves clarity

## Related

- [Projects](projects.md)
- Example: [`08_modules`](../../examples/single_file/language_core/08_modules/08_modules.arden)
