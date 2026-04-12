# Functions

## Why This Matters

Functions are your primary API boundary for types, ownership, effects, and error behavior.

## Function Shape

```arden
function add(a: Integer, b: Integer): Integer {
    return a + b;
}
```

## Parameter Ownership Modes

- `owned` (default): takes ownership
- `borrow`: read-only borrow
- `borrow mut`: mutable borrow

```arden
function consume(owned s: String): None { return None; }
function read(borrow s: String): None { println(s); return None; }
```

## Return Style

A function can return explicitly or use an expression tail when type-compatible.

## Higher-Order Functions

Function values are first-class and can be typed.

```arden
function apply(x: Integer, f: (Integer) -> Integer): Integer {
    return f(x);
}
```

## Extern and FFI Surfaces

For C interop patterns, see:

- [`27_extern_c_interop`](../../examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden)
- [`30_extern_variadic_printf`](../../examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden)
- [`31_extern_abi_link_name`](../../examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden)
- [`32_extern_safe_wrapper`](../../examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden)

## Related

- [Generics](../advanced/generics.md)
- [Ownership](../advanced/ownership.md)
