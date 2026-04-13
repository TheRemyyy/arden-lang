# Language Edges

## Why This Matters

Beyond core syntax, Arden already supports project-scale language features that shape architecture and API boundaries.
This page collects them in one place.

## Visibility Modifiers

Supported visibilities:

- `public`
- `protected`
- `private`

Use visibility to enforce boundaries at compile time.

Current parser restrictions:

- visibility modifiers are supported on class members
- visibility modifiers are rejected on `module` declarations
- visibility modifiers are rejected on `import` declarations
- visibility modifiers are rejected on `package` declarations
- visibility modifiers are rejected on constructors

Example: [`35_visibility_enforcement`](../../examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden)

## Inheritance (`extends`)

Classes can extend base classes and override methods.

Example: [`36_inheritance_extends`](../../examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden)

## Import Aliases

Alias imports reduce long namespace paths and improve readability:

```arden
import std.math as math;
import std.string as str;
```

Example: [`38_import_aliases`](../../examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden)

## Exact Import Members

Arden also supports importing specific members (including values) with alias:

```arden
import std.args.count as ArgCount;
import std.system.cwd as CurrentDir;
import Option.None as Empty;
```

This is useful when you want explicit dependency wiring without pulling full namespaces.

Example: [`44_exact_import_values`](../../examples/single_file/language_edges/44_exact_import_values/44_exact_import_values.arden)

## Package Namespaces

In project mode, files can declare namespace with `package ...;` and import symbols across packages.

Example project: [nested_package_project](../../examples/nested_package_project/README.md)

## Compound Assignment

Arden supports compound assignment on variables/fields/indexes: `+=`, `-=`, `*=`, `/=`, `%=`.

Example: [`39_compound_assign`](../../examples/single_file/language_edges/39_compound_assign/39_compound_assign.arden)

## Destructors

Classes can define `destructor()` for teardown-time logic.
A class can define at most one destructor.

## Common Mistakes

- exposing internals as `public` by default
- deep inheritance where composition would be simpler
- broad wildcard imports that hide symbol origin
- relying on implicit package assumptions after file moves
- expecting `public module ...` / `private import ...` style declarations to compile

## Related

- [Classes](classes.md)
- [Modules](modules.md)
- [Packages and Imports](packages_imports.md)
- [Projects](projects.md)
