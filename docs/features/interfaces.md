# Interfaces

Interfaces define a contract that classes can implement.

## Definition

```arden
interface Printable {
    function print_me(): None;
}
```

## Implementation

Classes implement interfaces explicitly via `implements`. The compiler validates that required methods exist with compatible signatures.

```arden
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

```arden
interface Named extends Printable {
    function get_name(): String;
}
```

Generic interfaces can also be referenced directly in `implements` and `extends` clauses.

```arden
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

Project builds also rewrite alias-qualified generic interface types in ordinary type positions, so declarations such as `reader: api.Reader<String>` and `reader: ReaderAlias<String>` work the same way as single-file `check`.

## Polymorphism

You can use interfaces as types.

```arden
function display(item: Printable): None {
    item.print_me();
    return None;
}
```

See `examples/37_interfaces_contracts.arden` for a full contract + interface-typed-parameter example.
