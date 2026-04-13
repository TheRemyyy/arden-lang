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
- `borrow mut`: borrow-mut mode (caller-side exclusivity contract)

Current compiler behavior notes:

- calling `borrow mut` parameters requires mutable caller binding
- inside callee, parameter can be read and reassigned (`println(x)`, `x += 1`)
- caller-visible mutation propagation is type-dependent in current behavior
- if you need explicit and predictable caller-visible in-place mutation semantics, use `&mut T`

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

## `mut`, `&`, `&mut` At Function Boundaries

Quick rule for beginners:

- parameter `x: T` receives owned value semantics
- parameter `x: &T` receives read-only reference
- parameter `x: &mut T` receives mutable reference (in-place update path)

```arden
import std.io.*;

function show(x: &Integer): None {
    println("x={*x}");
    return None;
}

function bump(x: &mut Integer): None {
    *x += 1;
    return None;
}

function main(): None {
    mut n: Integer = 10;
    show(&n);
    bump(&mut n);
    println("after={n}");
    return None;
}
```

Use this as default API design:

- read-only helper -> `&T`
- mutating helper -> `&mut T`
- ownership transfer intentionally required -> `owned T`

## Return Style

Use explicit `return` for clarity, especially in beginner-facing code and non-trivial branches.

## `main()` Entry Constraints

Current compiler rules for `main()`:

- no parameters
- no generic parameters
- cannot be `async`
- cannot be `extern`
- cannot be variadic
- return type must be `None` or `Integer`

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
- using `borrow mut` where `&mut T` parameter would make mutation path clearer
- returning sentinel values instead of explicit `Option`/`Result` style

## Related

- [Generics](../advanced/generics.md)
- [Ownership](../advanced/ownership.md)
- borrow-mut behavior example:
  [`43_borrow_mut_semantics`](../../examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden)
- FFI examples:
  - [`27_extern_c_interop`](../../examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden)
  - [`30_extern_variadic_printf`](../../examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden)
  - [`31_extern_abi_link_name`](../../examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden)
  - [`32_extern_safe_wrapper`](../../examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden)
