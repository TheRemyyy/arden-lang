# Contributing to Arden

Thank you for your interest in contributing to Arden! We are building a modern, safe, and efficient systems programming language, and we need your help.

## Development Setup

See the [Installation Guide](docs/getting_started/installation.md) for prerequisites.

1. **Fork the repository** on GitHub.
2. **Clone your fork**:

   ```bash
   git clone https://github.com/theremyyy/arden.git
   cd arden
   ```

3. **Build the project**:

   ```bash
   cargo build
   ```

## Workflow

### 1. Create a Branch

Always work on a new branch for your changes:

```bash
git checkout -b feat/my-new-feature
# or
git checkout -b fix/bug-description
```

### 2. Make Changes

- **Code Style**: We follow standard Rust formatting. Run `cargo fmt` before committing.
- **Linting**: Ensure your code passes `cargo clippy` without warnings.
- **Tests**: Add unit tests for new logic and integration tests (in `examples/`) for new language features.

### 3. Run Tests

Ensure the compiler is working correctly:

```bash
cargo test
```

To run a specific Arden example check:

```bash
cargo run -- check examples/01_hello.arden
```

### 4. Submit a Pull Request

Push your changes to your fork and open a Pull Request against the `main` branch of the official repository.

## Project Structure

- **`src/lexer.rs`**: Tokenizes source code.
- **`src/parser.rs`**: Parses tokens into an AST.
- **`src/typeck.rs`**: Validates types.
- **`src/borrowck.rs`**: Enforces ownership rules.
- **`src/codegen.rs`**: Generates LLVM IR.

See internal docs in [Compiler Architecture](docs/compiler/architecture.md) for more details.

## Adding Language Features

If you are adding a new syntax feature (e.g., a new keyword):

1. **Lexer**: Add the new token variant in `Token` enum and matching logic in `lexer.rs`.
2. **AST**: Add the new node structure in `ast.rs`.
3. **Parser**: Update `parser.rs` to handle the new syntax.
4. **Type Checking**: Implement validation logic in `typeck.rs`.
5. **Codegen**: Implement LLVM IR generation in `codegen.rs`.
6. **Docs**: Update relevant documentation in `docs/` and add an example in `examples/`.

## Code of Conduct

Be kind and respectful. Harassment or abusive behavior will not be tolerated.
