# GBE Testing Strategy

## Philosophy

GBE tests Unix process composition - routers, adapters, clients spawning and communicating via Unix sockets. Tests must be:
- **Isolated**: No resource conflicts between parallel tests
- **Deterministic**: Same result every run, any environment
- **Fast**: Minimal overhead, no unnecessary containers
- **Simple**: Easy to write, debug, and maintain

## Why Not Containers?

**Initial assumption**: Test flakiness meant we needed containerized test environments (Docker, etc.)

**Reality**: The flakiness was caused by poor test isolation patterns, not environment differences.

**Problems containers don't solve:**
1. **Test design issues**: Socket path collisions, resource cleanup, race conditions
2. **Developer friction**: Slow inner loop (rebuild images), extra dependencies
3. **Overhead**: Startup time, disk space, complexity
4. **False confidence**: Masks real issues instead of fixing root causes

**When containers ARE useful:**
- Matching exact CI environment (we already do: CircleCI uses `cimg/rust:1.93`)
- Complex external dependencies (databases, services) - not needed in Phase 1
- Multi-platform testing - handled by CI matrix instead

**Decision**: Use proper test isolation patterns. Add optional containerized testing later if needed (Phase 2+).

## Test Harness Approach

### The Problem

Integration tests spawn real processes (router, adapter, client) that need:
- Unique Unix socket paths (avoid collisions in parallel tests)
- Proper cleanup (remove sockets, kill processes)
- Deterministic timing (wait for socket creation, handle startup)
- Clear error messages (what failed, why)

**Without a harness:**
```rust
// ❌ Brittle, collision-prone
let router = Command::new("gbe-router")
    .args(["--socket", "/tmp/gbe-router.sock"])  // Fixed path!
    .spawn()?;
// No cleanup, no coordination, manual socket management
```

**With a harness:**
```rust
// ✅ Isolated, reliable, clean
let env = TestEnv::new();
env.start_router()?;
env.start_adapter("seq 1 10")?;
// Automatic cleanup, unique paths, proper coordination
```

### Design Principles

1. **RAII cleanup**: Processes and sockets cleaned up automatically via Drop
2. **Unique resources**: Each test gets isolated socket paths (PID + counter)
3. **Builder pattern**: Fluent API for test setup
4. **Clear errors**: Meaningful messages when things fail
5. **No global state**: Tests can run in any order, any parallelism

### Implementation Plan

**Phase 1: Basic harness** (`router/tests/test_harness.rs`)
- `TestEnv`: Manages test lifecycle
- Unique socket path generation
- Process spawning helpers
- RAII cleanup

**Phase 2: Utilities** (as needed)
- `TestRouter`, `TestAdapter`, `TestClient` wrappers
- Timeout helpers
- Log capture
- Assertion helpers

**Phase 3: Shared harness** (when needed)
- Move to workspace `test-utils` crate
- Share across package tests
- Common patterns extracted

## Test Types

### Unit Tests
- **Location**: `src/` alongside code (`#[cfg(test)] mod tests`)
- **Scope**: Single function/module logic
- **Dependencies**: None (pure functions, mocked I/O)
- **Run**: `cargo test --workspace --lib`

### Integration Tests
- **Location**: `tests/*.rs` in each package
- **Scope**: Component interactions (protocol, router, adapter)
- **Dependencies**: Pre-built binaries from `target/debug/`
- **Run**: `cargo test --workspace --test '*'`
- **Harness**: Uses `test_harness.rs` for process management

### E2E Tests
- **Location**: `router/tests/e2e_*.rs` (orchestration tests)
- **Scope**: Full stack (router + adapter + client)
- **Dependencies**: All binaries, real commands (seq, grep, etc.)
- **Run**: `cargo test --workspace -- --ignored`
- **Harness**: Uses `test_harness.rs` + shell commands

## Test Execution

### Local Development
```bash
just test           # All tests (unit + integration + e2e)
just test-unit      # Fast feedback loop
just test-e2e       # Full stack validation
```

### CI (CircleCI)
```bash
just build-bins     # Pre-build all binaries
just test           # Run all tests with pre-built bins
just lint           # Clippy + fmt
```

**Key insight**: `build-bins` eliminates compilation during test execution, fixing timeout issues.

## Common Patterns

### Starting a Test Router
```rust
let env = TestEnv::new();
let router = env.start_router()?;
let socket = router.socket_path();
```

### Connecting a Client
```rust
let client = env.connect_client()?;
client.send(&ControlMessage::Connect { capabilities: vec![] })?;
```

### Running Commands
```rust
let adapter = env.start_adapter(&["seq", "1", "10"])?;
let output = adapter.wait_for_output()?;
```

## Debugging Tests

### Run Single Test
```bash
cargo test --package gbe-router --test integration_test test_connect -- --nocapture
```

### Check Binary Paths
```bash
ls -la target/debug/gbe-*
echo $CARGO_BIN_EXE_gbe_router
```

### Run Test Binary Directly
```bash
# Find test binary
cargo test --no-run --package gbe-router --test integration_test

# Run it directly
./target/debug/deps/integration_test-* --ignored --nocapture
```

### Check Socket State
```bash
# List active sockets
ls -la /tmp/gbe-*.sock

# Check if process listening
lsof /tmp/gbe-router.sock
```

## Future Considerations

### When to Add Containers
- **Phase 2+**: Tool composition might need complex test scenarios
- **External services**: If we add Redis, databases, etc.
- **Multi-platform**: Testing platform-specific behavior
- **CI only**: Optional, not mandatory for local dev

### When to Extract Shared Harness
- **Multiple packages**: When adapter, client, buffer all need test utils
- **Pattern repetition**: Same setup/teardown in 3+ places
- **Version 1.0**: Before external users write tests

### Test Data Management
- **Fixtures**: Use `tests/fixtures/` for sample files
- **Generated data**: Create in temp dirs, clean up automatically
- **Snapshots**: Consider insta for protocol message testing

## References

- Cargo Book: https://doc.rust-lang.org/cargo/guide/tests.html
- Test harness patterns: https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html
- RAII cleanup: https://doc.rust-lang.org/rust-by-example/scope/raii.html
