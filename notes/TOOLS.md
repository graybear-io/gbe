# Development Tools

## Rust Toolchain

Installed via Homebrew:

```bash
brew install rust
```

**Installed tools:**
- **cargo** - Rust package manager and build tool
- **rustc** - Rust compiler
- **rustdoc** - Rust documentation generator
- **rustfmt** - Rust code formatter (included)
- **clippy** - Rust linter (included)

**Version:** 1.93.0

**Installation includes dependencies:**
- libssh2 (1.11.1_1) - SSH2 library
- libgit2 (1.9.2) - Git library
- z3 (4.15.4) - Theorem prover
- llvm (21.1.8) - Compiler infrastructure
- pkgconf (2.5.1) - Package config tool

## Verification

```bash
cargo --version   # cargo 1.93.0
rustc --version   # rustc 1.93.0
```

## Usage

### Create new project
```bash
cargo new project-name           # Binary crate
cargo new --lib library-name     # Library crate
```

### Build and run
```bash
cargo build          # Debug build
cargo build --release # Release build
cargo run            # Build and run
cargo test           # Run tests
cargo check          # Fast compile check
```

### Code quality
```bash
cargo fmt            # Format code
cargo clippy         # Run linter
```

## Additional Tools (Future)

Tools to install as needed:
- **cargo-watch** - Auto-rebuild on file changes
- **cargo-edit** - Add/remove dependencies from CLI
- **cargo-outdated** - Check for outdated dependencies
- **cargo-audit** - Security vulnerability scanning

```bash
cargo install cargo-watch cargo-edit cargo-outdated cargo-audit
```
