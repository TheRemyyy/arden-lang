// Test 35: Visibility enforcement
// Tests: public/private/protected behavior

import std.io.*;

class Account {
    private balance: Integer;
    protected ownerId: Integer;
    public label: String;

    constructor(label: String, initialBalance: Integer, ownerId: Integer) {
        this.label = label;
        this.balance = initialBalance;
        this.ownerId = ownerId;
    }

    public function deposit(amount: Integer): None {
        this.balance = this.balance + amount;
        return None;
    }

    public function getBalance(): Integer {
        return this.balance;
    }
}

class PremiumAccount extends Account {
    constructor(label: String, initialBalance: Integer, ownerId: Integer) {
        this.label = label;
        this.ownerId = ownerId;
        this.deposit(initialBalance);
    }

    public function ownerCode(): Integer {
        // protected field access from subclass is allowed
        return this.ownerId;
    }
}

function main(): None {
    println("=== Visibility Enforcement ===");
    mut a: Account = Account("Standard", 100, 42);
    a.deposit(50);
    println("Account: {a.label}, balance={a.getBalance()}");

    p: PremiumAccount = PremiumAccount("Premium", 250, 77);
    println("Premium owner code={p.ownerCode()}");

    println("Private/protected access violations are compile-time errors.");
    return None;
}
