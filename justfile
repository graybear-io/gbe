# GBE Development Tasks
# Run `just` to see all available commands

# List available recipes
default:
    @just --list

# Build all workspace packages
build:
    cargo build --workspace --all-targets

# Build in release mode
build-release:
    cargo build --workspace --all-targets --release

# Run all tests (unit + integration + e2e)
test:
    @echo "Building all packages first..."
    cargo build --workspace --all-targets
    @echo "Running tests..."
    cargo test --workspace --lib
    cargo test --workspace --test '*'
    cargo test --workspace -- --ignored --nocapture

# Run unit tests only
test-unit:
    cargo test --workspace --lib

# Run integration tests only
test-integration:
    cargo test --workspace --test '*'

# Run E2E tests only
test-e2e:
    cargo test --workspace -- --ignored --nocapture

# Run clippy linter
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Check code formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Run all lint checks (clippy + formatting)
lint: clippy fmt-check

# Build documentation
doc:
    cargo doc --workspace --no-deps

# Build and open documentation in browser
doc-open:
    cargo doc --workspace --no-deps --open

# Run full CI pipeline locally
ci: build test lint doc
    @echo "✓ All CI checks passed!"

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Watch and auto-run tests on file changes (requires cargo-watch)
watch:
    cargo watch -x "test --workspace"

# Run a specific component
run-router:
    cargo run --package gbe-router

run-adapter +ARGS:
    cargo run --package gbe-adapter -- {{ARGS}}

run-client +ARGS:
    cargo run --package gbe-client -- {{ARGS}}

# Quick E2E test with simple script
e2e-simple:
    ./test_simple.sh

# Interactive E2E test
e2e-interactive:
    ./test_interactive.sh

# Check project for common issues
check:
    cargo check --workspace --all-targets

# Update dependencies
update:
    cargo update

# Show outdated dependencies
outdated:
    cargo outdated

# Install development tools
install-tools:
    cargo install cargo-watch cargo-outdated
