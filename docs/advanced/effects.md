# Effects

## Why This Matters

Effects tell the compiler what side effects a function is allowed to perform.
This turns hidden runtime behavior into explicit compile-time contracts.

## Attribute Reference

- `@Pure`: function must remain side-effect free
- `@Io`: allows I/O side effects
- `@Thread`: allows thread/time side effects
- `@Net`: marks network-effect capability
- `@Alloc`: marks allocation-effect capability
- `@Unsafe`: marks unsafe-effect capability
- `@Any`: allow any effects (escape hatch)

## Minimal Example

```arden
import std.io.*;

@Pure
function add(a: Integer, b: Integer): Integer {
    return a + b;
}

@Io
function logLine(msg: String): None {
    println(msg);
    return None;
}
```

## Compiler Rules

### `@Pure`

- cannot call effectful functions
- cannot be combined with explicit effects (`@Io`, `@Thread`, ...)
- cannot be combined with `@Any`

### Explicit effect enforcement

If a function calls another function requiring an effect, caller must declare that effect (or use `@Any`).

```arden
import std.io.*;

@Io
function writeLog(): None {
    println("log");
    return None;
}
```

Without `@Io`, this call is rejected during type checking.

### Inference

If you omit effect attributes, compiler infers effects from function body and call graph.

Practical recommendation:

- explicit effects on public APIs
- inference for internal helpers when that improves readability

## Built-in Calls and Required Effects

Current built-in behavior in compiler checks:

- `println`, `print`, `read_line`, `File.*`, `System.*`, `Args.*` -> require `io`
- `Time.sleep`, `Time.now`, `Time.unix` -> require `thread`

## `@Any` Usage Rule

`@Any` is useful for boundary/orchestrator functions that intentionally mix effect categories.
Do not overuse it in core logic, or you lose effect-level guarantees.

## Common Mistakes

- adding `@Pure` and then calling `println`
- forgetting to propagate required effect to caller
- overusing `@Any` where narrow effects would be clearer

## Related

- [Async / Await](async.md)
- [Error Handling](error_handling.md)
- Examples:
  - [`26_effect_system`](../../examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden)
  - [`29_effect_inference_and_any`](../../examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden)
  - [`41_effect_attributes_reference`](../../examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden)
