# Generics

Generics allow you to write flexible, reusable code that works with any type.

When you use multiple interface bounds on the same type parameter, overlapping methods must agree on the same signature. Arden now rejects ambiguous combinations like `T extends A, B` when `A.render()` and `B.render()` use different parameter or return types.

## Generic Functions

```arden
function identity<T>(x: T): T {
    return x;
}

val: Integer = identity<Integer>(5);
```

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

## Type Constraints

Generic parameters can be constrained with `extends` bounds. Bounds accept both local names and
qualified interface paths.

Bounds are interface-only. Using an unknown symbol or a class/enum as a bound is a type error.

```arden
function printAll<T extends Display>(item: T): None {
    return None;
}
```

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
