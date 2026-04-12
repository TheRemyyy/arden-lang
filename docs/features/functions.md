# Functions

## Why This Matters

Functions are your primary API boundary for data ownership, effects, and behavior.
If your function signatures are clear, the rest of the codebase stays predictable.

## Basic Shape

```arden
function add(a: Integer, b: Integer): Integer {
    return a + b;
}
```

## Ownership Modes In Parameters

- `owned` (default): function takes ownership
- `borrow`: read-only borrow
- `borrow mut`: mutable borrow

```arden
import std.io.*;

function consume(owned s: String): None {
    println("consumed={s}");
    return None;
}

function readName(borrow s: String): None {
    println("read={s}");
    return None;
}
```

## Return Style

Use explicit `return` for clarity, especially in beginner-facing code and non-trivial branches.

## Higher-Order Functions

Functions are first-class values and can be passed around.

```arden
function apply(x: Integer, f: (Integer) -> Integer): Integer {
    return f(x);
}
```

## Runnable Example

```arden
import std.io.*;

function square(x: Integer): Integer {
    return x * x;
}

function apply(x: Integer, f: (Integer) -> Integer): Integer {
    return f(x);
}

function main(): None {
    println("result={apply(7, square)}");
    return None;
}
```

## Common Mistakes

- oversized functions mixing validation, logic, and side effects
- unclear ownership in signatures when borrowing is intended
- returning sentinel values instead of explicit `Option`/`Result` style

## Related

- [Generics](../advanced/generics.md)
- [Ownership](../advanced/ownership.md)
- FFI examples:
  - [`27_extern_c_interop`](../../examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden)
  - [`30_extern_variadic_printf`](../../examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden)
  - [`31_extern_abi_link_name`](../../examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden)
  - [`32_extern_safe_wrapper`](../../examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden)
