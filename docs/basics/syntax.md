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
function main(): None {
    x: Integer = 10;

    {
        y: Integer = 20;
        println("{x} {y}");
    }

    // y is out of scope here
    return None;
}
```

## Statements and Semicolons

Semicolons are required after statements:

```arden
x: Integer = 5;
return None;
```

Declarations like `function`, `class`, `if`, `while`, and `match` do not need `;` after their closing brace.

## Assignment and Compound Assignment

```arden
mut x: Integer = 10;
x = x + 1;
x += 2;
x -= 1;
x *= 3;
x /= 2;
x %= 3;
```

Compound assignment works with fields and indexes too:

```arden
obj.count += 1;
arr[i] -= 2;
```

## String Interpolation

```arden
println("count={10}, ok={true}, mark={'🚀'}");
```

Interpolation currently supports scalar display types (`Integer`, `Float`, `Boolean`, `String`, `Char`, `None`).

## Identifiers

Identifier rule: start with letter/underscore, continue with letters/digits/underscore.

- valid: `name`, `_id`, `value2`
- invalid: `2name`, reserved keywords

## Style Conventions

- variables/functions: `camelCase`
- types/classes/interfaces: `PascalCase`
- constants: `SCREAMING_SNAKE_CASE`

