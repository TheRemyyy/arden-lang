# Quick Start

## Why This Matters

This guide gets you from zero to a working compile/check/run/test loop fast.

## 1. Run Your First Program

```bash
arden run examples/single_file/basics/01_hello/01_hello.arden
```

Then inspect:

- `examples/single_file/basics/02_variables/02_variables.arden`
- `examples/single_file/basics/04_control_flow/04_control_flow.arden`

## 2. Learn Safety Early

Run ownership and async examples next:

```bash
arden run examples/single_file/safety_and_async/10_ownership/10_ownership.arden
arden run examples/single_file/safety_and_async/14_async/14_async.arden
```

## 3. Learn Effects and Testing

```bash
arden run examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden
arden test --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

## 4. Use `check` During Development

```bash
arden check examples/single_file/safety_and_async/10_ownership/10_ownership.arden
```

`check` is the fastest way to validate syntax + types + borrow rules without full final artifact flow.

## 5. Start A Project

```bash
arden new my_project
cd my_project
arden run
```

Inspect project config:

```bash
arden info
```

## 6. Add Quality Commands

```bash
arden test
arden fmt
arden lint
```

## Next Docs

- [Syntax](../basics/syntax.md)
- [Types](../basics/types.md)
- [Functions](../features/functions.md)
- [Packages and Imports](../features/packages_imports.md)
- [Language Edges](../features/language_edges.md)
- [Ownership](../advanced/ownership.md)
- [Effects](../advanced/effects.md)
- [Extern and FFI](../advanced/ffi.md)
- [CLI Reference](../compiler/cli.md)

## Full Example Index

- [examples/README](../../examples/README.md)
- [Args example](../../examples/single_file/stdlib_and_system/22_args/22_args.arden)
