# Installation

## Prerequisites

Before installing the Apex compiler, ensure you have the following dependencies installed on your system:

- **Rust**: Version 1.83 or later (stable).
- **LLVM**: Version 21.0 or later (21.1.7 is recommended).
- **Clang**: Required for IR/object compilation.
- **Linker**: `mold` is preferred for Apex builds on Unix-like systems. `lld` is supported as a fallback, and Windows keeps using LLVM's linker path.
- **Git**: To clone the repository.

## Installing from Source

Apex is currently available by building from source.

### 1. Clone the Repository

```bash
git clone https://github.com/TheRemyyy/apex-compiler.git
cd apex-compiler
```

### Fedora (LLVM 21)

Install toolchain dependencies first:

```bash
sudo dnf install -y git clang mold cmake ninja-build make gcc gcc-c++ llvm llvm-devel llvm-libs
```

Then point `llvm-sys` to the installed LLVM prefix (usually `/usr` on Fedora):

```bash
export LLVM_SYS_211_PREFIX=/usr
```

### 2. Build the Compiler

Use Cargo to build the project in release mode:

```bash
cargo build --release
```

If neither `mold` nor `lld` is available, Apex project builds will fail immediately with an explicit error.

The compiled binary will be located at `target/release/apex-compiler` (or `target/release/apex-compiler.exe` on Windows).

## Adding to PATH

To use `apex-compiler` from anywhere in your terminal, add the release directory to your system's `PATH`.

### Linux / macOS

Add the following line to your shell configuration file (`.bashrc`, `.zshrc`, etc.):

```bash
export PATH="$PATH:$(pwd)/target/release"
```

Then reload your shell:

```bash
source ~/.bashrc  # or ~/.zshrc
```

### Windows (PowerShell)

You can temporarily add it to the current session:

```powershell
$env:PATH += ";$(pwd)\target\release"
```

To make it permanent, search for "Edit the system environment variables" in the Start menu, click "Environment Variables", select "Path" under "User variables", and add the full path to `target\release`.

## Verifying Installation

To verify that everything is set up correctly, run:

```bash
apex-compiler --version
```

The executable exposes the `apex` CLI interface.
