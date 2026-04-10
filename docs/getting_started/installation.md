# Installation

Arden can now be installed in two practical ways:

- download a portable bundle from the latest GitHub release
- build the compiler from source when you want the full repository workflow

If you only want a working compiler fast, use the portable bundle path first.

## Fastest Path: Portable Bundle

Latest stable releases publish portable archives for:

- Windows x64
- Linux x64
- macOS Apple Silicon
- macOS Intel

Each portable archive includes:

- the Arden compiler
- a bundled LLVM toolchain layout
- the linker helper binaries Arden expects on that platform
- a launcher script (`arden` or `arden.cmd`)

The simplest flow is:

1. Open the latest GitHub release.
2. Download the archive for your platform.
3. Extract it.
4. Run the included launcher.
5. Optional: run the bundled install script if you want Arden on PATH permanently.

Example verification:

```bash
./arden --version
./arden --help
```

On Windows:

```powershell
.\arden.cmd --version
.\arden.cmd --help
```

Optional install helpers shipped inside the portable bundles:

- Windows: `install.ps1`
- Linux / macOS: `install.sh`

Those scripts are convenience helpers for PATH setup. The launcher itself is intended to work directly from the extracted folder.

If you want the compiler on your shell PATH permanently, add the extracted bundle directory to PATH after verifying it runs.

## Build From Source

Use the source build path when you want to work inside the repository, hack on the compiler itself, or reproduce CI-style toolchain setup locally.

### Requirements

You need:

- Rust `1.85+`
- LLVM `22.1+`
- Clang on Linux/macOS for Cargo-driven compiler builds
- Git
- a supported linker setup

Linker policy is explicit:

- Linux: `mold`
- macOS: LLVM `lld`
- Windows: LLVM `lld`

### Clone The Repository

```bash
git clone https://github.com/TheRemyyy/arden-lang.git arden
cd arden
```

If the repository is renamed later, use the current upstream URL and keep the rest of the steps the same.

### Build The Compiler

```bash
cargo build --release
```

The resulting binary is:

- `target/release/arden`
- `target/release/arden.exe` on Windows

### First Verification

Before changing shell config or editor settings, make sure the freshly built binary actually runs:

```bash
./target/release/arden --version
./target/release/arden --help
```

On Windows:

```powershell
.\target\release\arden.exe --version
.\target\release\arden.exe --help
```

### Platform Notes

#### Fedora / Linux

Example package install:

```bash
sudo dnf install -y git clang mold cmake ninja-build make gcc gcc-c++ llvm llvm-devel llvm-libs
```

If `llvm-sys` does not auto-detect LLVM correctly, point it at the installed prefix:

```bash
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22
```

#### macOS

You need LLVM 22 tooling available and LLVM `lld` for final linking.

If LLVM is not on your shell path by default, export the appropriate prefix before building.

Typical things to verify:

- `clang --version`
- `ld.lld --version` or equivalent LLVM linker path
- the active Rust toolchain can build normal Rust crates successfully

#### Windows

Windows builds use LLVM `lld-link` together with the VC/UCRT/Windows SDK import libraries available from Visual Studio and the Windows SDK. CI also installs `libxml2` through `vcpkg`, which is a useful reference if you are reproducing the GitHub Actions environment locally.

The simplest path is usually:

- Rust with the MSVC target
- LLVM tools installed and reachable
- PowerShell session with the required toolchain paths available

### Add Arden To Your PATH

#### Linux / macOS

```bash
export PATH="$PATH:$(pwd)/target/release"
```

Add that line to your shell config if you want it permanently.

#### Windows PowerShell

```powershell
$env:PATH += ";$(pwd)\target\release"
```

### Verify The Installation

```bash
arden --version
arden --help
```

If those work, the next best check is to run a real file:

```bash
cat > hello.arden <<'EOF'
import std.io.*;

function main(): None {
    println("Hello, Arden!");
    return None;
}
EOF

arden run hello.arden
```

### Common Problems

- `cargo build --release` fails because LLVM headers/libs are missing
- linking fails because the expected linker (`mold` or `lld`) is not installed
- `arden` works only with `./target/release/arden` because PATH was not updated yet
- editor integration is attempted before the compiler itself is confirmed working

When in doubt, solve the compiler install first and only then move on to LSP/editor setup.

## Next Step

Continue with:

- [Quick Start](quick_start.md)
- [Editor Setup](editor_setup.md)
