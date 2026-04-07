# Java-Style Namespace Example

This example demonstrates Java-style package and import system.

## Project Structure

```
java_style_project/
├── arden.toml
└── src/
    ├── utils/
    │   ├── math.arden      # package utils.math
    │   └── strings.arden   # package utils.strings
    └── main.arden          # package main (entry)
```

## Package Declaration

Each file declares its package at the top:

```arden
// src/utils/math.arden
package utils.math;

function factorial(n: Integer): Integer {
    // ...
}
```

## Import Styles

### 1. Import Specific Functions

```arden
import utils.math.factorial;
import utils.math.power;
```

### 2. Wildcard Import

```arden
import utils.strings.*;
```

### 3. Usage

Imported functions are used directly by name:

```arden
result: Integer = factorial(5);
greeting: String = greet("World");
```

## How It Works

1. **Package = Namespace**: `package utils.math` creates namespace `utils.math`
2. **Function Mangling**: `utils.math.factorial` → `utils__math__factorial` in LLVM
3. **Imports**: Compiler resolves imports and generates correct function names
4. **Wildcard**: `import utils.strings.*` imports all exported functions

## Running

```bash
cd multi_file_depth_project
arden run
```

## Comparison with Java

| Java | Arden |
|------|------|
| `package com.example;` | `package utils.math;` |
| `import com.example.Utils;` | `import utils.math.factorial;` |
| `import com.example.*;` | `import utils.strings.*;` |
| `Utils.method()` | `method()` (direct) |

## Notes

- All functions are "public" by default (for now)
- No `mod.rs` needed (unlike Rust)
- Simple, predictable, Java-like behavior
