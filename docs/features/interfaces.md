# Interfaces

Interfaces define a contract that classes can implement.

## Definition

```apex
interface Printable {
    function print_me(): None;
}
```

## Implementation

Classes implement interfaces explicitly via `implements`. The compiler validates that required methods exist with compatible signatures.

```apex
class Book implements Printable {
    title: String;
    
    constructor(title: String) {
        this.title = title;
    }
    
    function print_me(): None {
        println("Book: {this.title}");
        return None;
    }
}
```

## Interface Inheritance

Interfaces can extend other interfaces.

```apex
interface Named extends Printable {
    function get_name(): String;
}
```

Generic interfaces can also be referenced directly in `implements` and `extends` clauses.

```apex
interface Reader<T> {
    function read(): T;
}

interface StringReader extends Reader<String> {}

class ConfigReader implements Reader<String> {
    function read(): String {
        return "ok";
    }
}
```

## Polymorphism

You can use interfaces as types.

```apex
function display(item: Printable): None {
    item.print_me();
    return None;
}
```

See `examples/37_interfaces_contracts.apex` for a full contract + interface-typed-parameter example.
