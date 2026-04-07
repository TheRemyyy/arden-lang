# Enums

Enums allow you to define a type by enumerating its possible variants. Arden enums are algebraic data types, meaning they can hold data.

## Basic Enums

```arden
enum Color {
    Red,
    Green,
    Blue
}
```

## Enums with Data

```arden
enum Message {
    Quit,
    Move(x: Integer, y: Integer),
    Write(String),
    ChangeColor(r: Integer, g: Integer, b: Integer)
}
```

## Pattern Matching with Enums

You use `match` to extract data from enums.

```arden
msg: Message = Message.Write("Hello");

match (msg) {
    Quit => { println("Quitting"); }
    Move(x, y) => { println("Moving to {x}, {y}"); }
    Write(s) => { println("Message: {s}"); }
    _ => { println("Other"); }
}
```
