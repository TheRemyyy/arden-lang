// Example 26: Iterator Pattern
// Demonstrates iterator patterns using Apex features

import std.io.*;

// Manual iterator implementation pattern
struct RangeIterator {
    current: Integer,
    end: Integer,
    step: Integer,
}

function RangeIterator_new(start: Integer, end: Integer, step: Integer): RangeIterator {
    return RangeIterator {
        current: start,
        end: end,
        step: step,
    };
}

function RangeIterator_has_next(self: &RangeIterator): Boolean {
    if self.step > 0 {
        return self.current < self.end;
    } else {
        return self.current > self.end;
    }
}

function RangeIterator_next(self: &mut RangeIterator): Integer {
    value: Integer = self.current;
    self.current = self.current + self.step;
    return value;
}

// Filter iterator pattern
struct FilterIterator<T> {
    source: [T],
    index: Integer,
}

// Map operation using array comprehension pattern
function map_array(arr: [Integer], f: function(Integer): Integer): [Integer] {
    // In real implementation, this would create a new mapped array
    // For now, demonstrating the pattern
    result: [Integer] = [];
    for item in arr {
        // Would append f(item) to result
    }
    return result;
}

// Filter operation
function filter_array(arr: [Integer], predicate: function(Integer): Boolean): [Integer] {
    result: [Integer] = [];
    for item in arr {
        if predicate(item) {
            // Would append item to result
        }
    }
    return result;
}

// Predicate functions
function is_positive(n: Integer): Boolean {
    return n > 0;
}

function is_less_than_ten(n: Integer): Boolean {
    return n < 10;
}

function double(n: Integer): Integer {
    return n * 2;
}

function main(): None {
    println("Iterator Patterns in Apex");
    println("=========================\n");
    
    // Basic range iteration
    println("Range 0 to 5:");
    mut i: Integer = 0;
    while i < 5 {
        println("  " + to_string(i));
        i = i + 1;
    }
    
    // Array iteration
    println("\nArray iteration:");
    numbers: [Integer] = [1, 2, 3, 4, 5];
    for num in numbers {
        println("  Number: " + to_string(num));
    }
    
    // With index
    println("\nWith index:");
    mut idx: Integer = 0;
    for num in numbers {
        println("  [" + to_string(idx) + "] = " + to_string(num));
        idx = idx + 1;
    }
    
    // Filter pattern
    println("\nFilter (even numbers):");
    for num in numbers {
        if num % 2 == 0 {
            println("  " + to_string(num));
        }
    }
    
    // Map pattern
    println("\nMap (double):");
    for num in numbers {
        doubled: Integer = num * 2;
        println("  " + to_string(num) + " * 2 = " + to_string(doubled));
    }
    
    // Reduce pattern (sum)
    println("\nReduce (sum):");
    mut sum: Integer = 0;
    for num in numbers {
        sum = sum + num;
    }
    println("  Sum = " + to_string(sum));
    
    // Chain operations pattern
    println("\nChain operations:");
    println("  Filter even, then double:");
    for num in numbers {
        if num % 2 == 0 {
            doubled: Integer = num * 2;
            println("    " + to_string(doubled));
        }
    }
    
    return None;
}
