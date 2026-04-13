# Syntax

Arden syntax is intentionally familiar for C-family / Rust / TypeScript users, with explicit blocks and types.

## Why This Matters

Readable syntax is only useful if semantics stay obvious. Arden syntax favors explicitness where mistakes are costly (types, mutability, scopes).

## Comments

```arden
// Single-line comment

/*
 * Multi-line comment
 */
```

Nested block comments are not currently supported.

## Blocks and Scope

Blocks use `{}` and define lexical scope.

```arden
import std.io.*;

function main(): None {
    x: Integer = 10;

    {
        y: Integer = 20;
        println("{x} {y}");
    };

    // y is out of scope here
    return None;
}
```

## Statements and Semicolons

Semicolons are required after statements:

```arden
function main(): None {
    x: Integer = 5;
    return None;
}
```

Declarations like `function`, `class`, `if`, `while`, and `match` do not need `;` after closing brace.

## Top-Level Rule (Important)

At file top-level, Arden expects declarations only (`function`, `class`, `enum`,
`interface`, `module`, `import`, `package`).

Standalone executable statements/assignments must live inside a function
(usually `main()`).

## Assignment and Compound Assignment

```arden
function main(): None {
    mut x: Integer = 10;
    x = x + 1;
    x += 2;
    x -= 1;
    x *= 3;
    x /= 2;
    x %= 3;
    return None;
}
```

Compound assignment works with fields and indexes too:

```arden
function main(): None {
    mut numbers: List<Integer> = List<Integer>();
    numbers.push(10);
    numbers[0] -= 2;

    // field form uses the same operator shape:
    // obj.count += 1;
    return None;
}
```

## String Interpolation

```arden
import std.io.*;

function main(): None {
    println("count={10}, ok={true}, mark={'🚀'}");
    return None;
}
```

Interpolation supports scalar display types (`Integer`, `Float`, `Boolean`, `String`, `Char`, `None`).

## Parser Rules That Often Surprise People

### No trailing commas in many lists

Compiler currently rejects trailing commas in many syntax forms, including:

- function parameter lists
- argument lists
- lambda parameter lists
- function type parameters
- generic type argument lists and generic call type argument lists
- `implements` / `interface extends` lists
- extern option lists and extern parameter lists

### Match must have at least one arm

Empty `match` statements/expressions are rejected.

### `None` pattern form

Use `None` pattern without empty binding list `None()`.

### Import/package path shape

- path cannot start with `.`
- path cannot contain empty segment (`..`)
- package path cannot end with `.`
- `package ...;` declaration must appear at the beginning of file

### Lambda syntax

Pipe-style lambda syntax is not supported (`|x| ...`).
Use Arden lambda syntax from existing examples/docs.

### `main()` signature constraints

`main()` constraints enforced by compiler:

- no parameters
- no generic parameters
- not `async`
- not `extern`
- not variadic
- return type only `None` or `Integer`

## Identifiers

Identifier rule: start with letter/underscore, continue with letters/digits/underscore.

- valid: `name`, `_id`, `value2`
- invalid: `2name`, reserved keywords

## Style Conventions

- variables/functions: `camelCase`
- types/classes/interfaces: `PascalCase`
- constants: `SCREAMING_SNAKE_CASE`
