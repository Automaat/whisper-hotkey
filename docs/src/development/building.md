# Building from Source

Build Whisper Hotkey from source code.

## Prerequisites

### Required

- **macOS** (M1/M2/M3 or Intel)
- **Rust 1.84+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Xcode Command Line Tools**: `xcode-select --install`

### Optional

- **mise**: `curl https://mise.run | sh` (for tool management)
- **cargo-watch**: `cargo install cargo-watch` (for auto-rebuild)
- **cargo-flamegraph**: `cargo install flamegraph` (for profiling)

## Quick Build

```bash
# Clone repository
git clone https://github.com/Automaat/whisper-hotkey.git
cd whisper-hotkey

# Install tools (if using mise)
mise install

# Build debug
cargo build

# OR build release
cargo build --release

# Run
cargo run --release
```

## Using mise (Recommended)

mise manages tool versions automatically.

```bash
# Install tools from .mise.toml
mise install

# Run with mise
mise exec -- cargo build --release
mise exec -- cargo run --release

# Or activate mise in shell
mise activate bash  # or zsh, fish
cargo build --release
```

## Build Variants

### Debug Build

```bash
cargo build
```

**Characteristics:**
- Includes debug symbols
- No optimizations
- Faster compilation (~30s)
- Slower runtime (3-5x)
- Binary: `target/debug/whisper-hotkey`

**Use for:** Development, debugging

### Release Build

```bash
cargo build --release
```

**Characteristics:**
- Optimizations enabled
- No debug symbols (unless RUST_BACKTRACE=1)
- Slower compilation (~2min first time, ~30s incremental)
- Fast runtime (production speed)
- Binary: `target/release/whisper-hotkey`

**Use for:** Production, testing, distribution

### Development Build (with logging)

```bash
RUST_LOG=debug cargo build --release
```

**Use for:** Performance testing with logs

## Running

### From Source

```bash
# Debug
cargo run

# Release
cargo run --release

# With logging
RUST_LOG=debug cargo run --release
RUST_LOG=trace cargo run --release
```

### Direct Binary

```bash
# Debug
./target/debug/whisper-hotkey

# Release
./target/release/whisper-hotkey
```

## Installation

### Binary Installation

```bash
# Build release
cargo build --release

# Install to /usr/local/bin
./scripts/install.sh
```

### App Bundle Installation

```bash
# Build and create .app bundle
./scripts/install.sh app
```

Installs to `/Applications/WhisperHotkey.app`

## Development Workflow

### Watch Mode

Auto-rebuild on file changes:

```bash
cargo install cargo-watch
cargo watch -x 'run --release'
```

### Format Before Commit

```bash
cargo fmt --all
```

### Lint Before Commit

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Run All Checks

```bash
mise exec -- mise run check
```

Runs: `cargo fmt --check`, `cargo clippy`, `shellcheck`, `actionlint`

## Testing

```bash
# Unit tests
cargo test

# Integration tests (requires hardware)
cargo test -- --ignored

# With coverage
cargo install cargo-llvm-cov
cargo llvm-cov
cargo llvm-cov --html && open target/llvm-cov/html/index.html
```

See [Testing](./testing.md) for details.

## Troubleshooting Build Issues

### Linker Errors

**Error:** `ld: library not found`

**Solution:**
```bash
xcode-select --install
```

### Rust Version Too Old

**Error:** `edition2024 not supported`

**Solution:**
```bash
rustup update
cargo clean
cargo build
```

### Missing Dependencies

**Error:** `could not find crate`

**Solution:**
```bash
cargo clean
cargo build
```

### Build Hangs

**Symptom:** Build stuck at "Compiling whisper-rs"

**Cause:** whisper-rs builds C++ code (takes time)

**Solution:** Wait 5-10 minutes on first build

## Cross-Compilation

### Intel from Apple Silicon

```bash
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
```

Binary: `target/x86_64-apple-darwin/release/whisper-hotkey`

### Apple Silicon from Intel

```bash
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

Binary: `target/aarch64-apple-darwin/release/whisper-hotkey`

## Optimizing Build Times

### Use cargo-chef (CI)

```bash
cargo install cargo-chef
```

### Use sccache (Local)

```bash
brew install sccache
export RUSTC_WRAPPER=sccache
```

### Incremental Compilation

Already enabled by default in debug mode.

**For release:**
```toml
# .cargo/config.toml
[build]
incremental = true
```

## Binary Size

### Default Release

```bash
cargo build --release
ls -lh target/release/whisper-hotkey
# ~3-5MB
```

### Stripped Binary

```bash
cargo build --release
strip target/release/whisper-hotkey
ls -lh target/release/whisper-hotkey
# ~2-3MB
```

### Optimized for Size

```toml
# Cargo.toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

## Next Steps

- Run [Tests](./testing.md) to verify your build
- Read [Contributing](./contributing.md) guidelines
- See [Project Structure](./structure.md)
