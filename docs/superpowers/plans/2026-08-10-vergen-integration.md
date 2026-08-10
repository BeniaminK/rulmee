# Vergen Build Metadata Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace custom process execution logic in `build.rs` with `vergen` and `vergen-gitcl` crates and update `src/main.rs` to use standard `VERGEN_*` environment variables.

**Architecture:** Add `vergen` and `vergen-gitcl` to `[build-dependencies]`. In `build.rs`, initialize `vergen::Emitter` with `BuildBuilder`, `GitclBuilder`, and `RustcBuilder` to automatically emit build environment variables. In `src/main.rs`, update `clap` version command to consume `VERGEN_GIT_DESCRIBE`, `VERGEN_BUILD_TIMESTAMP`, and `VERGEN_RUSTC_SEMVER`.

**Tech Stack:** Rust edition 2024, `vergen 9.1`, `vergen-gitcl 9.1`, `clap 4.5`.

## Global Constraints
- `Cargo.toml`: Add `vergen = { version = "9.1", features = ["build", "rustc"] }` and `vergen-gitcl = "9.1"` under `[build-dependencies]`.
- `build.rs`: Use `vergen::Emitter` with `BuildBuilder`, `GitclBuilder`, and `RustcBuilder`.
- `src/main.rs`: Reference `VERGEN_GIT_DESCRIBE`, `VERGEN_BUILD_TIMESTAMP`, `VERGEN_RUSTC_SEMVER`.

---

### Task 1: Update `Cargo.toml` build dependencies

**Files:**
- Modify: `Cargo.toml:37`

- [ ] **Step 1: Update `Cargo.toml` with `[build-dependencies]`**

Modify `Cargo.toml` to add `[build-dependencies]`:
```toml
[build-dependencies]
vergen = { version = "9.1", features = ["build", "rustc"] }
vergen-gitcl = "9.1"
```

- [ ] **Step 2: Verify `Cargo.toml` parses correctly**

Run: `cargo check`
Expected: Resolution/download of `vergen` crates and successful cargo check.

- [ ] **Step 3: Commit `Cargo.toml` changes**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add vergen build dependencies"
```

---

### Task 2: Rewrite `build.rs` to use `vergen::Emitter`

**Files:**
- Modify: `build.rs:1-88`

- [ ] **Step 1: Replace contents of `build.rs`**

Replace `build.rs` content with:
```rust
use vergen::Emitter;
use vergen_gitcl::{BuildBuilder, GitclBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&GitclBuilder::all_git()?)?
        .add_instructions(&RustcBuilder::all_rustc()?)?
        .emit()?;
    Ok(())
}
```

- [ ] **Step 2: Commit `build.rs` updates**

```bash
git add build.rs
git commit -m "build: rewrite build.rs using vergen emitter"
```

---

### Task 3: Update `src/main.rs` version macro and verify CLI version output

**Files:**
- Modify: `src/main.rs:27-40`

- [ ] **Step 1: Update `clap` version command macro in `src/main.rs`**

Update lines 28-40 in `src/main.rs`:
```rust
#[derive(Parser, Debug)]
#[command(
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (git ",
        env!("VERGEN_GIT_DESCRIBE"),
        ", build date ",
        env!("VERGEN_BUILD_TIMESTAMP"),
        ", compiler ",
        env!("VERGEN_RUSTC_SEMVER"),
        ")"
    ),
    about = "LiDM: Lightweight Display Manager"
)]
```

- [ ] **Step 2: Run `cargo check` and `cargo build`**

Run: `cargo check && cargo build`
Expected: PASS clean build without missing env var errors.

- [ ] **Step 3: Test `--version` output**

Run: `cargo run -- --version`
Expected: Prints `lidm <version> (git <git-describe>, build date <timestamp>, compiler <rustc-semver>)`

- [ ] **Step 4: Commit `src/main.rs` changes**

```bash
git add src/main.rs
git commit -m "feat: consume vergen environment variables in CLI version flag"
```
