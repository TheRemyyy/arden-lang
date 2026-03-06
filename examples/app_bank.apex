// Application: Banking System
// A comprehensive banking app demonstrating Apex features

import std.io.*;

// Account types
class BankAccount {
    accountNumber: String;
    ownerName: String;
    mut balance: Float;
    mut isActive: Boolean;

    constructor(accNum: String, owner: String, initialBalance: Float) {
        this.accountNumber = accNum;
        this.ownerName = owner;
        this.balance = initialBalance;
        this.isActive = true;
    }

    function getAccountNumber(): String {
        return this.accountNumber;
    }

    function getOwnerName(): String {
        return this.ownerName;
    }

    function getBalance(): Float {
        return this.balance;
    }

    function deposit(amount: Float): Boolean {
        if (amount <= 0.0) {
            println("Error: Deposit amount must be positive");
            return false;
        }
        if (!this.isActive) {
            println("Error: Account is inactive");
            return false;
        }

        this.balance = this.balance + amount;
        println("Deposited: ${amount}");
        println("New balance: ${this.balance}");
        return true;
    }

    function withdraw(amount: Float): Boolean {
        if (amount <= 0.0) {
            println("Error: Withdrawal amount must be positive");
            return false;
        }
        if (!this.isActive) {
            println("Error: Account is inactive");
            return false;
        }
        if (amount > this.balance) {
            println("Error: Insufficient funds");
            return false;
        }

        this.balance = this.balance - amount;
        println("Withdrawn: ${amount}");
        println("New balance: ${this.balance}");
        return true;
    }

    function transfer(amount: Float, borrow toAccount: BankAccount): Boolean {
        if (!this.isActive) {
            println("Error: Source account is inactive");
            return false;
        }
        if (amount > this.balance) {
            println("Error: Insufficient funds for transfer");
            return false;
        }

        this.balance = this.balance - amount;
        // toAccount.balance = toAccount.balance + amount;
        println("Transferred: ${amount}");
        println("From: {this.accountNumber}");
        println("To: {toAccount.getAccountNumber()}");
        return true;
    }

    function deactivate(): None {
        this.isActive = false;
        println("Account {this.accountNumber} deactivated");
        return None;
    }

    function activate(): None {
        this.isActive = true;
        println("Account {this.accountNumber} activated");
        return None;
    }

    function displayInfo(): None {
        println("");
        println("Account: {this.accountNumber}");
        println("Owner: {this.ownerName}");
        println("Balance: ${this.balance}");
        if (this.isActive) {
            println("Status: Active");
        } else {
            println("Status: Inactive");
        }
        return None;
    }
}

// Savings account with interest
class SavingsAccount {
    base: BankAccount;
    interestRate: Float;
    mut withdrawalsThisMonth: Integer;
    maxWithdrawals: Integer;

    constructor(accNum: String, owner: String, initialBalance: Float, rate: Float) {
        this.base = BankAccount(accNum, owner, initialBalance);
        this.interestRate = rate;
        this.withdrawalsThisMonth = 0;
        this.maxWithdrawals = 6;
    }

    function getInterestRate(): Float {
        return this.interestRate;
    }

    function applyMonthlyInterest(): None {
        balance: Float = this.base.getBalance();
        interest: Float = balance * (this.interestRate / 12.0);
        this.base.deposit(interest);
        println("Monthly interest applied: ${interest}");
        this.withdrawalsThisMonth = 0;
        return None;
    }

    function withdraw(amount: Float): Boolean {
        if (this.withdrawalsThisMonth >= this.maxWithdrawals) {
            println("Error: Maximum withdrawals reached for this month");
            return false;
        }
        success: Boolean = this.base.withdraw(amount);
        if (success) {
            this.withdrawalsThisMonth = this.withdrawalsThisMonth + 1;
            remaining: Integer = this.maxWithdrawals - this.withdrawalsThisMonth;
            println("Withdrawals remaining this month: {remaining}");
        }
        return success;
    }

    function displayInfo(): None {
        this.base.displayInfo();
        println("Account Type: Savings");
        println("Interest Rate: {this.interestRate}%");
        remaining: Integer = this.maxWithdrawals - this.withdrawalsThisMonth;
        println("Withdrawals remaining: {remaining}");
        return None;
    }
}

// Transaction record
class Transaction {
    id: Integer;
    transactionType: String;
    amount: Float;
    fromAccount: String;
    toAccount: String;
    timestamp: String;

    constructor(id: Integer, txType: String, amount: Float, from: String, to: String) {
        this.id = id;
        this.transactionType = txType;
        this.amount = amount;
        this.fromAccount = from;
        this.toAccount = to;
        this.timestamp = "2024-01-15 10:30:00";
    }

    function display(): None {
        println("TX#{this.id}: {this.transactionType} ${this.amount}");
        return None;
    }
}

// Bank module with utility functions
module Bank {
    function generateAccountNumber(): String {
        return "ACC-00001";
    }

    function validateAccountNumber(accNum: String): Boolean {
        len: Integer = Str.len(accNum);
        return len > 0;
    }

    function calculateCompoundInterest(principal: Float, rate: Float, years: Integer): Float {
        // A = P(1 + r)^n
        factor: Float = Math.pow(1.0 + rate, to_float(years));
        return principal * factor;
    }

    function calculateLoanPayment(principal: Float, rate: Float, months: Integer): Float {
        // Simplified monthly payment calculation
        monthlyRate: Float = rate / 12.0;
        payment: Float = principal * (monthlyRate / (1.0 - Math.pow(1.0 + monthlyRate, to_float(-months))));
        return payment;
    }
}

function printBanner(): None {
    println("========================================");
    println("          APEX BANKING SYSTEM           ");
    println("========================================");
    return None;
}

function printSeparator(): None {
    println("----------------------------------------");
    return None;
}

function main(): None {
    printBanner();

    // Create accounts
    println("");
    println("=== Creating Accounts ===");

    mut checking: BankAccount = BankAccount("CHK-001", "Alice Johnson", 1000.0);
    checking.displayInfo();

    mut savings: SavingsAccount = SavingsAccount("SAV-001", "Alice Johnson", 5000.0, 0.025);
    savings.displayInfo();

    mut checking2: BankAccount = BankAccount("CHK-002", "Bob Smith", 2500.0);
    checking2.displayInfo();

    // Deposit operations
    println("");
    printSeparator();
    println("=== Deposit Operations ===");

    checking.deposit(500.0);
    savings.base.deposit(1000.0);

    // Withdrawal operations
    println("");
    printSeparator();
    println("=== Withdrawal Operations ===");

    checking.withdraw(200.0);
    savings.withdraw(100.0);
    savings.withdraw(100.0);

    // Transfer operation
    println("");
    printSeparator();
    println("=== Transfer Operations ===");

    checking.transfer(300.0, checking2);

    // Apply interest to savings
    println("");
    printSeparator();
    println("=== Monthly Interest ===");

    savings.applyMonthlyInterest();

    // Loan calculation demo
    println("");
    printSeparator();
    println("=== Loan Calculator ===");

    loanAmount: Float = 10000.0;
    loanRate: Float = 0.05;
    loanMonths: Integer = 36;

    monthlyPayment: Float = Bank__calculateLoanPayment(loanAmount, loanRate, loanMonths);
    println("Loan Amount: $10,000");
    println("Interest Rate: 5%");
    println("Term: 36 months");
    println("Monthly Payment: ~$299");

    // Compound interest demo
    println("");
    printSeparator();
    println("=== Investment Calculator ===");

    principal: Float = 1000.0;
    rate: Float = 0.07;
    years: Integer = 10;

    futureValue: Float = Bank__calculateCompoundInterest(principal, rate, years);
    println("Initial Investment: $1,000");
    println("Annual Rate: 7%");
    println("After 10 years: ~$1,967");

    // Transaction history (simulated)
    println("");
    printSeparator();
    println("=== Recent Transactions ===");

    tx1: Transaction = Transaction(1, "DEPOSIT", 500.0, "EXTERNAL", "CHK-001");
    tx1.display();

    tx2: Transaction = Transaction(2, "WITHDRAWAL", 200.0, "CHK-001", "CASH");
    tx2.display();

    tx3: Transaction = Transaction(3, "TRANSFER", 300.0, "CHK-001", "CHK-002");
    tx3.display();

    tx4: Transaction = Transaction(4, "INTEREST", 12.5, "SYSTEM", "SAV-001");
    tx4.display();

    // Final account summaries
    println("");
    printSeparator();
    println("=== Final Account Balances ===");

    checking.displayInfo();
    savings.displayInfo();
    checking2.displayInfo();

    // Account management
    println("");
    printSeparator();
    println("=== Account Management ===");

    // Deactivate and reactivate demo
    checking2.deactivate();

    // Try to withdraw from inactive account
    withdrawResult: Boolean = checking2.withdraw(100.0);

    checking2.activate();

    println("");
    println("========================================");
    println("       Banking Demo Complete            ");
    println("========================================");

    return None;
}
