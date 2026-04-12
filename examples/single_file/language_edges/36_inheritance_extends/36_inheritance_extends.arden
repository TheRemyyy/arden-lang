// Test 36: Inheritance with extends
// Tests: inherited methods and fields

import std.io.*;

class Animal {
    public name: String;

    constructor(name: String) {
        this.name = name;
    }

    public function speak(): String {
        return "unknown";
    }

    public function describe(): String {
        return "Animal({this.name})";
    }
}

class Dog extends Animal {
    constructor(name: String) {
        this.name = name;
    }

    public function speak(): String {
        return "woof";
    }
}

function main(): None {
    println("=== Inheritance (extends) ===");
    d: Dog = Dog("Buddy");
    println("{d.describe()}");
    println("sound={d.speak()}");
    return None;
}
