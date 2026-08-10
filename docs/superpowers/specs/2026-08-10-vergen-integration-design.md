# Vergen Build Metadata Integration Design

## Overview
Replace custom build script logic in `build.rs` with `vergen` and `vergen-gitcl` crates. This automates build-time environment variable generation for git description, build timestamp, and compiler versioning.

## Design Details

### 1. Build Dependencies (`Cargo.toml`)
Add `vergen` and `vergen-gitcl` under `[build-dependencies]`:
```toml
[build-dependencies]
vergen = { version = "9.1", features = ["build", "rustc"] }
vergen-gitcl = "9.1"
```

### 2. Build Script (`build.rs`)
Remove custom process execution functions (`get_git_rev`, `get_build_ts`, `get_compiler_version`) and rewrite `main()` to use `vergen::Emitter`:
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

### 3. Application CLI Version String (`src/main.rs`)
Update `clap`'s `version` macro string to consume standard `VERGEN_*` environment variables:
- `VERGEN_GIT_DESCRIBE` (git version/commit)
- `VERGEN_BUILD_TIMESTAMP` (ISO 8601 build timestamp)
- `VERGEN_RUSTC_SEMVER` (Rust compiler version)

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

## Verification
- Run `cargo check` and `cargo build` to ensure `build.rs` compiles and emits `VERGEN_*` environment variables.
- Run `cargo run -- --version` to verify output matches expected CLI version string formatting.
