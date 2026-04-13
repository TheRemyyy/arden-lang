# Extern and FFI

## Why This Matters

`extern` lets Arden call native functions outside Arden code (typically C ABI).
This is the boundary where type safety meets platform ABI details, so precision matters.

## Basic Extern Declaration

```arden
extern function puts(msg: String): Integer;
```

Explicit ABI and link-name forms are supported:

```arden
extern(c) function puts(msg: String): Integer;
extern(system, "printf") function sys_printf(fmt: String, ...): Integer;
```

## Runnable Example

```arden
import std.io.*;

extern function strlen(msg: String): Integer;

function main(): None {
    n: Integer = strlen("hello");
    println("len={n}");
    return None;
}
```

## Compiler-Enforced Rules

- `extern` function cannot be `async`
- `extern` function generic parameters are not supported
- variadic extern must have at least one fixed parameter
- unsupported ABI names are rejected (supported: `c`, `system`)
- `main()` cannot be declared `extern`

## Extern Option Syntax Rules

`extern(...)` parser constraints:

- options cannot be empty (`extern()` is invalid)
- trailing comma in options is invalid
- accepts at most ABI and optional link name

## FFI-Safe Type Surface

Extern parameter/return types must be FFI-safe.
Compiler-accepted core FFI-safe set:

- `Integer`
- `Float`
- `Boolean`
- `Char`
- `String`
- `None`
- `Ptr<T>`

Types outside this set are rejected in extern signatures.

## `Ptr<T>` Notes

`Ptr<T>` is a low-level pointer type intended for FFI boundaries.
It is a type-level construct, not a general constructor workflow in normal Arden code.

Practical extern pattern (`malloc`/`free` style):

```arden
extern(c) function malloc(size: Integer): Ptr<None>;
extern(c) function free(ptr: Ptr<None>): None;
extern(c) function memset(dst: Ptr<None>, value: Integer, len: Integer): Ptr<None>;

function alloc_zeroed(size: Integer): Ptr<None> {
    mut p: Ptr<None> = malloc(size);
    p = memset(p, 0, size);
    return p;
}

function main(): None {
    p: Ptr<None> = alloc_zeroed(64);
    free(p);
    return None;
}
```

Why `p = memset(p, ...)`:

- pointer values are ownership-checked in normal flow
- passing `p` by value to extern consumes that value path
- using the returned pointer for the next step keeps ownership explicit

## First-Class Function Limitation

Extern functions cannot be used as first-class function values.
Keep extern calls direct or wrap them in normal Arden functions.

## Common Mistakes

- using non-FFI-safe custom types directly in extern signatures
- trying to pass extern functions around as lambdas/function values
- declaring variadic extern with zero fixed parameters
- malformed `extern(...)` option list

## Related

- [Functions](../features/functions.md)
- [Types](../basics/types.md)
- Examples:
  - [`27_extern_c_interop`](../../examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden)
  - [`30_extern_variadic_printf`](../../examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden)
  - [`31_extern_abi_link_name`](../../examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden)
  - [`32_extern_safe_wrapper`](../../examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden)
