# Generics

Generics let Arden code stay reusable without throwing away static type information.

They are already part of the language surface today, not a future feature proposal.

Use generics when you want one function, interface, or class shape to work across multiple concrete types while still being checked at compile time.

## What Generics Cover In Arden

Current generic surface documented and exercised in the repository includes:

- generic functions such as `identity<T>(value: T): T`
- generic classes such as `Box<T>`
- generic interfaces such as `Reader<T>`
- generic stdlib/container types such as `Option<T>`, `Result<T, E>`, `List<T>`, `Map<K, V>`, and `Set<T>`
- bounded type parameters via `extends`
- nested generic types such as `Option<List<Integer>>`
- explicit generic arguments and inferred generic arguments where the compiler has enough type information

When you use multiple interface bounds on the same type parameter, overlapping methods must agree on the same signature. Arden now rejects ambiguous combinations like `T extends A, B` when `A.render()` and `B.render()` use different parameter or return types.

## Generic Functions

```arden
function identity<T>(x: T): T {
    return x;
}

val: Integer = identity<Integer>(5);
```

In practice, explicit type arguments are useful when you want to make the specialization obvious:

```arden
function pairLeft<T>(left: T, right: T): T {
    return left;
}

picked: Integer = pairLeft<Integer>(1, 2);
```

When the surrounding types already force the answer, Arden can often infer the generic argument instead of needing it spelled out.

## Generic Classes

```arden
class Box<T> {
    value: T;
    
    constructor(value: T) {
        this.value = value;
    }
    
    function get(): T {
        return this.value;
    }
}
```

This is the pattern to use when a type owns or stores a value whose concrete type should stay flexible.

## Generic Interfaces

Interfaces can be generic too, and classes can implement them with concrete arguments.

```arden
interface Reader<T> {
    function read(): T;
}

class ConfigReader implements Reader<String> {
    function read(): String {
        return "ok";
    }
}
```

That matters because Arden's generic surface is not limited to containers. It also supports typed contracts and interface-driven APIs.

## Type Constraints

Generic parameters can be constrained with `extends` bounds. Bounds accept both local names and
qualified interface paths.

Bounds are interface-only. Using an unknown symbol or a class/enum as a bound is a type error.

```arden
function printAll<T extends Display>(item: T): None {
    return None;
}
```

The key rule is that bounds are interface-based capabilities, not arbitrary nominal inheritance.

Qualified and nested bounds are also valid:

```arden
function printAll<T extends util.Api.Named>(item: T): None {
    return None;
}

class Box<T extends util.Api.Named, util.Api.Serializable> {
    value: T;
}
```

Bounds are enforced for both explicit and inferred generic arguments:

```arden
interface Named { function name(): Integer; }

function read_name<T extends Named>(value: T): Integer {
    return value.name();
}
```

## Nested Generic Types

Generic types can be composed with each other:

```arden
scores: Map<String, List<Integer>> = Map<String, List<Integer>>();
maybeNames: Option<List<String>> = Option<List<String>>();
result: Result<Option<Integer>, String> = Result<Option<Integer>, String>();
```

This is the common shape for "optional value", "fallible result", "collection of X", and similar typed pipelines.

## Where To See More Than Toy Snippets

If you want runnable examples instead of isolated syntax fragments, use:

- [`../examples/09_generics.arden`](../../examples/09_generics.arden)
- [`../examples/showcase_project/README.md`](../../examples/showcase_project/README.md)
- [Interfaces](../features/interfaces.md)
- [Types](../basics/types.md)

## Current Practical Limits

The docs should describe the language as it exists today, so a few expectations should stay explicit:

- bounds currently use `extends`
- bounds are interface-only
- ambiguous overlapping bounds are rejected
- generic support should be validated against the current examples/tests when you are documenting edge cases

If you hit behavior that differs from this page, the correct fix is to update either the docs or the implementation so the two stop drifting apart.
