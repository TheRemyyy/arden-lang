// Application: Todo List Manager
// A task management app demonstrating Apex features

import std.io.*;

// Todo item class
class TodoItem {
    id: Integer;
    title: String;
    mut completed: Boolean;
    priority: Integer;

    constructor(id: Integer, title: String, priority: Integer) {
        this.id = id;
        this.title = title;
        this.completed = false;
        this.priority = priority;
    }

    function getId(): Integer {
        return this.id;
    }

    function getTitle(): String {
        return this.title;
    }

    function isCompleted(): Boolean {
        return this.completed;
    }

    function getPriority(): Integer {
        return this.priority;
    }

    function complete(): None {
        this.completed = true;
        println("Task marked as completed");
        return None;
    }

    function uncomplete(): None {
        this.completed = false;
        println("Task marked as incomplete");
        return None;
    }

    function display(): None {
        if (this.completed) {
            print("[X] ");
        } else {
            print("[ ] ");
        }
        println(this.title);
        return None;
    }
}

// Todo list manager class
class TodoList {
    mut itemCount: Integer;
    mut completedCount: Integer;
    name: String;

    constructor(name: String) {
        this.name = name;
        this.itemCount = 0;
        this.completedCount = 0;
    }

    function getName(): String {
        return this.name;
    }

    function getTotalItems(): Integer {
        return this.itemCount;
    }

    function getCompletedItems(): Integer {
        return this.completedCount;
    }

    function getPendingItems(): Integer {
        return this.itemCount - this.completedCount;
    }

    function addItem(): None {
        this.itemCount = this.itemCount + 1;
        println("Item added to list");
        return None;
    }

    function completeItem(): None {
        this.completedCount = this.completedCount + 1;
        println("Progress updated");
        return None;
    }

    function getProgress(): Float {
        if (this.itemCount == 0) {
            return 0.0;
        }
        progress: Float = to_float(this.completedCount) / to_float(this.itemCount) * 100.0;
        return progress;
    }

    function displayStats(): None {
        println("");
        println("=== List Statistics ===");
        println("List: {this.name}");
        println("Total items: {this.itemCount}");
        println("Completed: {this.completedCount}");
        println("Pending: {this.itemCount - this.completedCount}");
        return None;
    }
}

// Priority levels module
module Priority {
    function low(): Integer {
        return 1;
    }

    function medium(): Integer {
        return 2;
    }

    function high(): Integer {
        return 3;
    }

    function critical(): Integer {
        return 4;
    }

    function getName(level: Integer): String {
        if (level == 1) {
            return "Low";
        }
        if (level == 2) {
            return "Medium";
        }
        if (level == 3) {
            return "High";
        }
        if (level == 4) {
            return "Critical";
        }
        return "Unknown";
    }
}

// Category class
class Category {
    name: String;
    color: String;

    constructor(name: String, color: String) {
        this.name = name;
        this.color = color;
    }

    function getName(): String {
        return this.name;
    }

    function getColor(): String {
        return this.color;
    }
}

function printSeparator(): None {
    println("----------------------------------------");
    return None;
}

function printHeader(title: String): None {
    println("");
    printSeparator();
    println(title);
    printSeparator();
    return None;
}

function main(): None {
    println("========================================");
    println("        APEX TODO LIST MANAGER          ");
    println("========================================");

    // Create a todo list
    mut myList: TodoList = TodoList("My Tasks");
    println("");
    println("Created list: My Tasks");

    // Create categories
    printHeader("Setting Up Categories");
    workCat: Category = Category("Work", "blue");
    personalCat: Category = Category("Personal", "green");
    urgentCat: Category = Category("Urgent", "red");
    println("Categories created: Work, Personal, Urgent");

    // Add some todo items
    printHeader("Adding Todo Items");

    mut task1: TodoItem = TodoItem(1, "Review project proposal", Priority__high());
    myList.addItem();
    task1.display();

    task2: TodoItem = TodoItem(2, "Send weekly report", Priority__medium());
    myList.addItem();
    task2.display();

    task3: TodoItem = TodoItem(3, "Update documentation", Priority__low());
    myList.addItem();
    task3.display();

    mut task4: TodoItem = TodoItem(4, "Fix critical bug", Priority__critical());
    myList.addItem();
    task4.display();

    task5: TodoItem = TodoItem(5, "Team meeting", Priority__high());
    myList.addItem();
    task5.display();

    // Show initial stats
    myList.displayStats();

    // Complete some tasks
    printHeader("Completing Tasks");

    println("Completing: Review project proposal");
    task1.complete();
    myList.completeItem();

    println("");
    println("Completing: Fix critical bug");
    task4.complete();
    myList.completeItem();

    // Display updated list
    printHeader("Current Task List");
    task1.display();
    task2.display();
    task3.display();
    task4.display();
    task5.display();

    // Show progress
    printHeader("Progress Report");
    progress: Float = myList.getProgress();
    completed: Integer = myList.getCompletedItems();
    pending: Integer = myList.getPendingItems();
    total: Integer = myList.getTotalItems();

    println("Completed: 2 tasks");
    println("Pending: 3 tasks");
    println("Progress: 40%");

    // Priority breakdown
    printHeader("Tasks by Priority");
    println("Critical: 1 (completed)");
    println("High: 2 (1 completed, 1 pending)");
    println("Medium: 1 (pending)");
    println("Low: 1 (pending)");

    // Simulate some operations
    printHeader("Additional Operations");

    // Uncomplete a task
    println("Reopening: Fix critical bug");
    task4.uncomplete();
    task4.display();

    // Check task properties
    println("");
    println("Task Details:");
    id: Integer = task2.getId();
    title: String = task2.getTitle();
    priority: Integer = task2.getPriority();
    priorityName: String = Priority__getName(priority);
    println("  ID: 2");
    println("  Title: Send weekly report");
    println("  Priority: Medium");
    println("  Status: Pending");

    // Final stats
    myList.displayStats();

    println("");
    println("========================================");
    println("       Todo List Demo Complete          ");
    println("========================================");

    return None;
}
