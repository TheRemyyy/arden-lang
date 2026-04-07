# Multi-File Project Example

This example demonstrates how to organize an Arden project with multiple source files.

## Project Structure

```
.
├── arden.toml          # Project configuration
├── src/
│   ├── math_utils.arden    # Mathematical utilities
│   ├── string_utils.arden  # String manipulation utilities
│   └── main.arden          # Entry point with main function
└── README.md
```

## Configuration (arden.toml)

```toml
name = "multi_file_demo"
version = "1.0.0"
entry = "src/main.arden"        # Entry point file
files = [                       # All source files
    "src/math_utils.arden",
    "src/string_utils.arden",
    "src/main.arden"
]
output = "multi_file_demo"     # Output binary name
opt_level = "3"                # Optimization level
```

## Commands

```bash
# Build the project
arden build

# Build and run
arden run

# Show project info
arden info

# Check for errors
arden check
```

## Notes

- All files listed in `files` are compiled together
- The `entry` file must contain the `main()` function
- Functions from all files are available globally (no explicit imports needed yet)
