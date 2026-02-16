# Security Guardrails

Design-time conventions that prevent common security findings across GBE subprojects.

## 1. Validated Newtypes

Never accept raw `String` or `PathBuf` for domain identifiers or filesystem paths at API boundaries.

```rust
pub struct HostId(String);

impl HostId {
    pub fn new(raw: &str) -> Result<Self, ValidationError> {
        // reject empty, control chars, path separators, etc.
        if raw.is_empty() || raw.len() > 128 || raw.contains(|c: char| !c.is_alphanumeric() && c != '-') {
            return Err(ValidationError::InvalidHostId(raw.to_string()));
        }
        Ok(Self(raw.to_string()))
    }
}
```

```rust
pub struct SafePath(PathBuf);

impl SafePath {
    pub fn new(base: &Path, relative: &str) -> Result<Self, ValidationError> {
        let resolved = base.join(relative).canonicalize()?;
        if !resolved.starts_with(base) {
            return Err(ValidationError::PathTraversal);
        }
        Ok(Self(resolved))
    }
}
```

### Rule
Functions that accept host IDs, VM CIDs, image paths, or any external identifier must take the validated newtype, not the raw primitive. Construction is the only place validation happens — callers cannot bypass it.

### Prevents
- Path traversal via config fields (`image_dir`, `kernel_path`, `base_image`)
- Injection via `host_id`, `vm_cid`, tool names, subject strings

## 2. Deserialization Policy

### Rules
- **No `serde_json::Value` at API boundaries.** Always deserialize into a concrete struct with known fields. `Value` defers validation to every consumer.
- **Size limits on untrusted input.** Wrap deserialization with a length check before parsing.
- **Prefer self-describing formats** (JSON, MessagePack) over length-prefixed formats (bincode) for data crossing trust boundaries. bincode is fine for internal IPC with matching versions.

```rust
// Bad: accepts anything
fn handle(payload: serde_json::Value) { ... }

// Good: rejects unknown shapes at the boundary
#[derive(Deserialize)]
struct ToolRequest {
    tool: String,
    params: HashMap<String, String>,
}

fn handle(raw: &[u8]) -> Result<ToolRequest, Error> {
    if raw.len() > MAX_PAYLOAD_SIZE {
        return Err(Error::PayloadTooLarge);
    }
    serde_json::from_slice(raw)
}
```

### Prevents
- Arbitrary type confusion via `serde_json::Value`
- DoS via unbounded deserialization (bincode, JSON)
- Schema drift between producer and consumer

## 3. Error Display Separation

### Rules
- `Display` is for external consumers: include error category and safe context only.
- `Debug` is for internal logs: may include full context but never secrets (tokens, keys, passwords).
- Never log at `Debug` level in production-facing code paths.

```rust
#[derive(Debug)]
pub enum SentinelError {
    ConfigInvalid { path: PathBuf, reason: String },
    VmFailed { vm_id: String, exit_code: i32 },
}

impl fmt::Display for SentinelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigInvalid { reason, .. } => write!(f, "invalid config: {reason}"),
            Self::VmFailed { vm_id, .. } => write!(f, "vm {vm_id} failed"),
        }
    }
}
```

### Prevents
- Leaking filesystem paths, internal IDs, or stack traces to external callers
- Sensitive data appearing in user-facing error responses

## 4. Concurrency Defaults

### Rules
- Shared mutable state uses `Arc<Mutex<T>>` or `Arc<RwLock<T>>`. No `RefCell`, no `unsafe` interior mutability.
- Counters and flags use `AtomicU32` / `AtomicBool` with explicit `Ordering` (prefer `Acquire`/`Release` pair).
- If a struct will be shared across tasks, design it that way from the start — don't bolt on `Send + Sync` later.

```rust
// SlotTracker: safe by construction
pub struct SlotTracker {
    max: u32,
    used: AtomicU32,
}

impl SlotTracker {
    pub fn try_claim(&self) -> bool {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                if current < self.max { Some(current + 1) } else { None }
            })
            .is_ok()
    }

    pub fn release(&self) {
        self.used.fetch_sub(1, Ordering::Release);
    }
}
```

### Prevents
- Race conditions in shared counters (slot tracking, connection pools)
- Data races from incorrect `unsafe` usage
- Deadlocks from inconsistent lock ordering

## 5. CI Gates

These checks run automatically on every PR. Failures block merge.

### Required checks
| Check | Tool | What it catches |
|-------|------|-----------------|
| Dependency audit | `cargo audit` | Known CVEs in transitive deps |
| Dep alignment | custom lint | Crate deps diverging from workspace definitions |
| Unused deps | `cargo machete` | Unnecessary attack surface |
| Format | `cargo fmt --check` | Already enforced |
| Lint | `cargo clippy` | Already enforced |

### Recommended additions
- `cargo deny` for license compliance and duplicate dep detection
- Pinned transitive dep review: flag any `=x.y.z` pins in `Cargo.lock` that differ from latest patch

### Prevents
- Known vulnerabilities shipping to production
- Workspace dependency drift
- Bloated binaries with unused crates

## 6. Process Spawning

Relevant to gbe-sentinel (Firecracker, tap devices, overlay filesystems).

### Rules
- Validate executable paths with `SafePath` before spawning.
- Never interpolate user input into shell commands. Use `Command::new()` with `.arg()`, never `sh -c`.
- Drop privileges after binding privileged resources (tap devices, sockets).
- Every `spawn()` must have a corresponding cleanup path (kill process, remove socket, destroy tap).

```rust
// Bad
Command::new("sh").arg("-c").arg(format!("ip tuntap add {} mode tap", name));

// Good
Command::new("ip")
    .args(["tuntap", "add", name.as_validated_str(), "mode", "tap"])
    .spawn()?;
```

### Prevents
- Command injection via unsanitized arguments
- Privilege escalation via unvalidated binary paths
- Resource leaks (zombie processes, orphaned sockets, stale tap devices)

## Checklist for New Code

Before submitting a PR, verify:

- [ ] External identifiers use validated newtypes, not raw strings
- [ ] Deserialization targets concrete types with size limits
- [ ] Error `Display` impls don't leak internal paths or secrets
- [ ] Shared mutable state uses `Arc<Mutex<T>>` or atomics
- [ ] Process spawning uses `Command::arg()`, not shell interpolation
- [ ] `cargo audit` passes
