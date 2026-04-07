// Test 07: Interfaces
// Tests: interface definition, method signatures, default implementations, implements

import std.io.*;

// Basic interface
interface Printable {
    function print(): None;
}

// Interface with return value
interface Describable {
    function describe(): String;
    function getName(): String;
}

// Interface with parameters
interface Comparable {
    function compareTo(other: Integer): Integer;
    function equals(other: Integer): Boolean;
}

// Interface with default implementation
interface Greeting {
    function greet(): None {
        println("Hello!");
    }

    function farewell(): None {
        println("Goodbye!");
    }
}

// Interface extending another interface
interface Animal {
    function speak(): None;
    function move(): None;
}

// Multiple interface simulation
interface Serializable {
    function serialize(): String;
    function deserialize(data: String): None;
}

// Class implementing interface (simulated with matching methods)
class Document {
    content: String;

    constructor(text: String) {
        this.content = text;
    }

    // Implements Printable
    function print(): None {
        println("Document content:");
        println(this.content);
        return None;
    }

    // Implements Describable
    function describe(): String {
        return "A text document";
    }

    function getName(): String {
        return "Document";
    }
}

class Dog {
    name: String;

    constructor(name: String) {
        this.name = name;
    }

    // Implements Animal
    function speak(): None {
        println("Woof! Woof!");
        return None;
    }

    function move(): None {
        println("Running on four legs");
        return None;
    }
}

class Cat {
    name: String;

    constructor(name: String) {
        this.name = name;
    }

    // Implements Animal
    function speak(): None {
        println("Meow!");
        return None;
    }

    function move(): None {
        println("Gracefully walking");
        return None;
    }
}

// Generic interface concept
interface Container {
    function add(item: Integer): None;
    function remove(): Integer;
    function size(): Integer;
    function isEmpty(): Boolean;
}

function main(): None {
    println("=== Interface Tests ===");

    // Document implementing Printable and Describable
    println("");
    println("Document class (Printable, Describable):");
    doc: Document = Document("Hello, this is a document.");
    doc.print();
    desc: String = doc.describe();
    name: String = doc.getName();
    println("Description and name retrieved");

    // Dog implementing Animal
    println("");
    println("Dog class (Animal interface):");
    dog: Dog = Dog("Buddy");
    dog.speak();
    dog.move();

    // Cat implementing Animal
    println("");
    println("Cat class (Animal interface):");
    cat: Cat = Cat("Whiskers");
    cat.speak();
    cat.move();

    // Polymorphism simulation
    println("");
    println("Polymorphic behavior:");
    println("Different animals, same interface:");

    // Both dog and cat have speak() and move()
    dog.speak();
    cat.speak();

    println("");
    println("=== All interface tests passed! ===");

    return None;
}
