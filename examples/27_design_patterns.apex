// Example 27: Design Patterns
// Common design patterns implemented in Apex

import std.io.*;

// ============== SINGLETON PATTERN ==============
// Using module-level state (simulated with static-like pattern)
struct Config {
    database_url: String,
    port: Integer,
    debug_mode: Boolean,
}

// In real implementation, this would be a true singleton
function Config_default(): Config {
    return Config {
        database_url: "localhost:5432",
        port: 8080,
        debug_mode: false,
    };
}

// ============== BUILDER PATTERN ==============
struct HttpRequest {
    method: String,
    url: String,
    headers: [String],
    body: Option<String>,
    timeout: Integer,
}

function HttpRequest_new(url: String): HttpRequest {
    return HttpRequest {
        method: "GET",
        url: url,
        headers: [],
        body: None,
        timeout: 30,
    };
}

function HttpRequest_with_method(self: HttpRequest, method: String): HttpRequest {
    self.method = method;
    return self;
}

function HttpRequest_with_header(self: HttpRequest, key: String, value: String): HttpRequest {
    // Would add header
    return self;
}

function HttpRequest_with_body(self: HttpRequest, body: String): HttpRequest {
    self.body = Some(body);
    return self;
}

function HttpRequest_with_timeout(self: HttpRequest, seconds: Integer): HttpRequest {
    self.timeout = seconds;
    return self;
}

// ============== STRATEGY PATTERN ==============
trait PaymentStrategy {
    function pay(amount: Float): Result<String, String>;
}

struct CreditCardPayment {
    card_number: String,
    cvv: String,
}

function CreditCardPayment_pay(self: &CreditCardPayment, amount: Float): Result<String, String> {
    println("Paying " + to_string(amount) + " with credit card");
    return Ok("Payment successful");
}

struct PayPalPayment {
    email: String,
}

function PayPalPayment_pay(self: &PayPalPayment, amount: Float): Result<String, String> {
    println("Paying " + to_string(amount) + " with PayPal");
    return Ok("Payment successful");
}

struct PaymentProcessor {
    strategy: Option<CreditCardPayment>, // Simplified - would use trait
}

function PaymentProcessor_process(self: &PaymentProcessor, amount: Float): Result<String, String> {
    // Would use strategy pattern
    return Ok("Processed");
}

// ============== OBSERVER PATTERN ==============
trait Observer {
    function update(event: String): None;
}

struct EmailNotifier {
    email: String,
}

function EmailNotifier_update(self: &EmailNotifier, event: String): None {
    println("Email to " + self.email + ": " + event);
}

struct SmsNotifier {
    phone: String,
}

function SmsNotifier_update(self: &SmsNotifier, event: String): None {
    println("SMS to " + self.phone + ": " + event);
}

struct EventManager {
    observers: [String], // Simplified
}

function EventManager_new(): EventManager {
    return EventManager {
        observers: [],
    };
}

function EventManager_subscribe(self: &mut EventManager, observer: String): None {
    // Would add observer
}

function EventManager_notify(self: &EventManager, event: String): None {
    println("Notifying observers: " + event);
}

// ============== FACTORY PATTERN ==============
trait Animal {
    function speak(): String;
    function get_type(): String;
}

struct Dog {
    name: String,
}

function Dog_speak(self: &Dog): String {
    return self.name + " says: Woof!";
}

function Dog_get_type(self: &Dog): String {
    return "Dog";
}

struct Cat {
    name: String,
}

function Cat_speak(self: &Cat): String {
    return self.name + " says: Meow!";
}

function Cat_get_type(self: &Cat): String {
    return "Cat";
}

enum AnimalType {
    Dog,
    Cat,
}

function AnimalFactory_create(animal_type: AnimalType, name: String): String {
    match animal_type {
        AnimalType::Dog => {
            dog: Dog = Dog { name: name };
            return Dog_speak(&dog);
        },
        AnimalType::Cat => {
            cat: Cat = Cat { name: name };
            return Cat_speak(&cat);
        },
    };
    return "";
}

// ============== DECORATOR PATTERN ==============
struct Coffee {
    cost: Float,
    description: String,
}

function Coffee_new(): Coffee {
    return Coffee {
        cost: 2.0,
        description: "Simple coffee",
    };
}

function Coffee_with_milk(self: Coffee): Coffee {
    self.cost = self.cost + 0.5;
    self.description = self.description + ", milk";
    return self;
}

function Coffee_with_sugar(self: Coffee): Coffee {
    self.cost = self.cost + 0.2;
    self.description = self.description + ", sugar";
    return self;
}

function Coffee_with_whipped_cream(self: Coffee): Coffee {
    self.cost = self.cost + 0.7;
    self.description = self.description + ", whipped cream";
    return self;
}

// ============== MAIN ==============
function main(): None {
    println("Design Patterns in Apex");
    println("=======================\n");
    
    // Builder pattern demo
    println("Builder Pattern - HTTP Request:");
    request: HttpRequest = HttpRequest_new("https://api.example.com")
        .HttpRequest_with_method("POST")
        .HttpRequest_with_body("{\"key\": \"value\"}")
        .HttpRequest_with_timeout(60);
    println("  Method: " + request.method);
    println("  URL: " + request.url);
    println("  Timeout: " + to_string(request.timeout));
    
    // Factory pattern demo
    println("\nFactory Pattern - Animals:");
    dog_sound: String = AnimalFactory_create(AnimalType::Dog, "Buddy");
    println("  " + dog_sound);
    cat_sound: String = AnimalFactory_create(AnimalType::Cat, "Whiskers");
    println("  " + cat_sound);
    
    // Decorator pattern demo
    println("\nDecorator Pattern - Coffee:");
    coffee: Coffee = Coffee_new()
        .Coffee_with_milk()
        .Coffee_with_sugar()
        .Coffee_with_whipped_cream();
    println("  Description: " + coffee.description);
    println("  Cost: $" + to_string(coffee.cost));
    
    // Observer pattern demo
    println("\nObserver Pattern - Event Manager:");
    manager: EventManager = EventManager_new();
    EventManager_notify(&manager, "User logged in");
    
    return None;
}
