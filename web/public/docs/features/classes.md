# Classes

Arden supports Object-Oriented Programming (OOP) via classes.

## Definition

```arden
class Point {
    // Fields
    public x: Integer;
    public y: Integer;
    
    // Constructor
    constructor(x: Integer, y: Integer) {
        this.x = x;
        this.y = y;
    }
    
    // Methods
    public function move(dx: Integer, dy: Integer): None {
        this.x = this.x + dx;
        this.y = this.y + dy;
        return None;
    }
}
```

## Visibility

- `public` (default): Accessible from anywhere.
- `private`: Accessible only within the declaring class.
- `protected`: Accessible within the declaring class and subclasses.

Visibility is now enforced by the type checker. Invalid access is a compile-time error.

```arden
class Account {
    private balance: Integer;
    
    constructor() {
        this.balance = 0;
    }
}
```

## Inheritance

Classes can inherit members from a base class using `extends`.

```arden
class Animal {
    public name: String;

    constructor(name: String) {
        this.name = name;
    }

    public function describe(): String {
        return "Animal({this.name})";
    }
}

class Dog extends Animal {
    constructor(name: String) {
        this.name = name;
    }
}
```

## Objects

Objects are instances of classes.

```arden
p: Point = Point(10, 20);
p.move(5, 5);
```

## Destructors

You can define a `destructor` to run code when an object is destroyed (goes out of scope).

```arden
class FileHandler {
    destructor() {
        println("File closed");
    }
}
```
