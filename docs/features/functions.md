# Functions

Functions are the building blocks of Apex programs.

## Definition

A function is defined using the `function` keyword.

```apex
function name(param1: Type1, param2: Type2): ReturnType {
    // body...
    return value;
}
```

Example:

```apex
function add(a: Integer, b: Integer): Integer {
    return a + b;
}
```

## Return Values

If a function does not return a meaningful value, it should return `None` and the return type should be `None`.

```apex
function greet(): None {
    println("Hello");
    return None;
}
```

## Lambdas (Anonymous Functions)

Apex supports lambda expressions for concise function definition.

Type: `(ParamTypes) -> ReturnType`

```apex
// Implicit return
square: (Integer) -> Integer = (x: Integer) => x * x;

// Explicit block
complex: (Integer) -> Integer = (x: Integer) => {
    y: Integer = x * 2;
    return y + 1;
};
```

## Higher-Order Functions

Functions can take other functions as arguments or return them.

```apex
function callTwice(f: (Integer) -> None, val: Integer): None {
    f(val);
    f(val);
    return None;
}
```

## Closures

Lambdas can capture variables from their enclosing environment.

```apex
offset: Integer = 10;
adder: (Integer) -> Integer = (x: Integer) => x + offset;
```

## Extern Functions (C Interop)

Use `extern function` to declare C ABI symbols and call native libraries.

```apex
extern function puts(msg: String): Integer;

function main(): None {
    puts("hello from C");
    return None;
}
```

Reference example: `examples/27_extern_c_interop.apex`.

Variadic C signatures are supported:

```apex
extern function printf(fmt: String, ...): Integer;
```

Reference example: `examples/30_extern_variadic_printf.apex`.

You can also specify ABI and link name explicitly:

```apex
extern(c, "puts") function c_puts(msg: String): Integer;
extern(system, "printf") function sys_printf(fmt: String, ...): Integer;
```

Reference example: `examples/31_extern_abi_link_name.apex`.

Current extern FFI-safe signature types are:
- `Integer`
- `Float`
- `Boolean`
- `Char`
- `String` (C string pointer interop)
- `Ptr<T>` (raw pointer interop)
- `None`

For robust integrations, prefer a safe Apex wrapper around raw extern calls.
Reference example: `examples/32_extern_safe_wrapper.apex`.

## Effect Attributes

You can annotate functions with effect attributes:

- `@Pure`
- `@Io`
- `@Net`
- `@Alloc`
- `@Unsafe`
- `@Thread`
- `@Any`

`@Pure` forbids effectful calls. If a function declares effect attributes, calls requiring
missing effects produce type-check errors.

For functions without explicit effect attributes, Apex infers required effects from the call graph.
Use `@Any` to explicitly opt into permissive mode for integration-heavy code.

Reference example: `examples/26_effect_system.apex`.
