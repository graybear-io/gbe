# Commit Workflow

## Automated: Pre-commit Hook (Preferred)

The repository has a pre-commit hook that **automatically** runs quality gates:

```bash
git add <files>
git commit -m "message"
# → Hook runs: cargo fmt --check, clippy, test
# → Commit proceeds only if all pass
```

**To bypass** (emergency only):
```bash
git commit --no-verify -m "message"
```

## Manual: Quality Gates Checklist

If you need to run checks manually:

```bash
# 1. Format
cargo fmt

# 2. Lint
cargo clippy --workspace

# 3. Test
cargo test --workspace

# 4. Commit (hook will verify)
git add <files>
git commit -m "message"
```

## Future: `/commit` Skill

**Planned skill for guided workflow:**

```bash
/commit "feat: add new feature"
```

**What it does:**
1. Runs `cargo fmt` (auto-fix)
2. Runs `cargo clippy --workspace`
3. Runs `cargo test --workspace`
4. Prompts for file selection
5. Creates commit with message
6. Shows git status

**Implementation:** See `~/.claude/skills/commit/` (TBD)

## Troubleshooting

**"Formatting check failed"**
```bash
cargo fmt
git add <files>
git commit -m "message"
```

**"Clippy check failed"**
```bash
cargo clippy --workspace --fix --allow-dirty
git add <files>
git commit -m "message"
```

**"Tests failed"**
- Fix the failing tests first
- Or commit with `--no-verify` if intentionally committing broken tests (rare)

## Hook Installation

Pre-commit hook is at `.git/hooks/pre-commit`.

**To disable:**
```bash
rm .git/hooks/pre-commit
```

**To enable:**
```bash
chmod +x .git/hooks/pre-commit
```

**Not in git?** Hooks are local-only (`.git/hooks/` not tracked). Document setup in README or project setup guide.
