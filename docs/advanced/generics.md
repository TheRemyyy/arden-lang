# Generics

Generics allow you to write flexible, reusable code that works with any type.

## Generic Functions

```apex
function identity<T>(x: T): T {
    return x;
}

val: Integer = identity<Integer>(5);
```

## Generic Classes

```apex
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

```apex
function printAll<T extends Display>(item: T): None {
    return None;
}
```

Qualified and nested bounds are also valid:

```apex
function printAll<T extends util.Api.Named>(item: T): None {
    return None;
}

class Box<T extends util.Api.Named, util.Api.Serializable> {
    value: T;
}
```
