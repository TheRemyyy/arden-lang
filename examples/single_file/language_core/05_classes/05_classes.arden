// Test 05: Classes
// Tests: class, constructor, destructor, fields, methods, visibility, this

import std.io.*;
import std.math.*;

// Simple class with fields only
class Point {
    x: Integer;
    y: Integer;

    constructor(x: Integer, y: Integer) {
        this.x = x;
        this.y = y;
    }

    function getX(): Integer {
        return this.x;
    }

    function getY(): Integer {
        return this.y;
    }

    function distance(): Float {
        dist: Float = Math.sqrt(to_float(this.x * this.x + this.y * this.y));
        return dist;
    }
}

// Class with mutable fields
class Counter {
    mut value: Integer;

    constructor(initial: Integer) {
        this.value = initial;
    }

    function increment(): None {
        this.value = this.value + 1;
        return None;
    }

    function decrement(): None {
        this.value = this.value - 1;
        return None;
    }

    function getValue(): Integer {
        return this.value;
    }

    function reset(): None {
        this.value = 0;
        return None;
    }
}

// Class with visibility modifiers
class Person {
    public name: String;
    private age: Integer;
    protected id: Integer;

    constructor(name: String, age: Integer) {
        this.name = name;
        this.age = age;
        this.id = 12345;
    }

    public function getName(): String {
        return this.name;
    }

    private function getAge(): Integer {
        return this.age;
    }

    public function describe(): None {
        println("Person created");
        return None;
    }
}

// Class with destructor
class Resource {
    mut data: Integer;

    constructor(value: Integer) {
        this.data = value;
        println("Resource acquired");
    }

    destructor() {
        println("Resource released");
    }

    function getData(): Integer {
        return this.data;
    }
}

// Complex class example
class Rectangle {
    width: Integer;
    height: Integer;

    constructor(w: Integer, h: Integer) {
        this.width = w;
        this.height = h;
    }

    function area(): Integer {
        return this.width * this.height;
    }

    function perimeter(): Integer {
        return 2 * (this.width + this.height);
    }

    function isSquare(): Boolean {
        return this.width == this.height;
    }

    function scale(factor: Integer): None {
        // Note: would need mutable fields to actually modify
        println("Scaling rectangle");
        return None;
    }
}

function main(): None {
    println("=== Class Tests ===");

    // Point class
    println("Creating Point(3, 4)...");
    p: Point = Point(3, 4);
    px: Integer = p.getX();
    py: Integer = p.getY();
    println("Point created successfully");

    // Counter class
    println("");
    println("Creating Counter...");
    mut c: Counter = Counter(0);
    c.increment();
    c.increment();
    c.increment();
    v1: Integer = c.getValue();
    c.decrement();
    v2: Integer = c.getValue();
    c.reset();
    v3: Integer = c.getValue();
    println("Counter operations complete");

    // Person class
    println("");
    println("Creating Person...");
    person: Person = Person("Alice", 30);
    name: String = person.getName();
    person.describe();
    println("Person created");

    // Resource class (with destructor)
    println("");
    println("Testing Resource with destructor...");
    res: Resource = Resource(42);
    d: Integer = res.getData();
    println("Resource test complete");

    // Rectangle class
    println("");
    println("Creating Rectangle(5, 3)...");
    rect: Rectangle = Rectangle(5, 3);
    area: Integer = rect.area();
    perim: Integer = rect.perimeter();
    is_sq: Boolean = rect.isSquare();
    println("Rectangle area calculated");
    println("Rectangle perimeter calculated");

    // Square test
    println("");
    println("Creating Square(4, 4)...");
    square: Rectangle = Rectangle(4, 4);
    is_square: Boolean = square.isSquare();
    println("Square check complete");

    println("");
    println("=== All class tests passed! ===");

    return None;
}
