# `copy-config` Subcommand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `lidm copy-config [dest]` CLI subcommand to copy default configuration values to `~/.config/lidm/default.toml` or a specified destination path.

**Architecture:** `clap::Subcommand` enum `Commands::CopyConfig` in `src/main.rs`. `Config::resolve_default_copy_path` and `Config::execute_copy_config` in `src/config.rs`.

**Tech Stack:** Rust 2024, clap 4.5 (`derive`, `Subcommand`), serde, toml.

## Global Constraints
- Target files: `src/main.rs`, `src/config.rs`.
- Subcommand signature: `lidm copy-config [dest]`.
- Default target location: `$XDG_CONFIG_HOME/lidm/default.toml` or `$HOME/.config/lidm/default.toml` (fallback: `/etc/lidm/default.toml`).
- Auto-creates parent directories if missing (`std::fs::create_dir_all`).
- Must exit cleanly with code `0` on success and print confirmation to stdout.

---

### Task 1: Path Resolution & Implementation in `src/config.rs`

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs`

**Interfaces:**
- Produces:
  - `pub fn resolve_default_copy_path(dest: Option<&str>) -> std::path::PathBuf`
  - `pub fn execute_copy_config(dest: Option<&str>) -> Result<std::path::PathBuf, Box<dyn std::error::Error>>`

- [ ] **Step 1: Write unit tests for path resolution and file copying in `src/config.rs`**

Add unit tests to `src/config.rs`:
```rust
#[test]
fn test_resolve_default_copy_path_custom() {
    let custom = "/tmp/custom_lidm_config.toml";
    let path = Config::resolve_default_copy_path(Some(custom));
    assert_eq!(path, std::path::PathBuf::from(custom));
}

#[test]
fn test_execute_copy_config_creates_file() {
    let temp_dir = std::env::temp_dir();
    let target = temp_dir.join("lidm_test_copy_config/sub/default.toml");
    if target.exists() {
        let _ = std::fs::remove_file(&target);
    }

    let result = Config::execute_copy_config(target.to_str());
    assert!(result.is_ok());

    assert!(target.exists());
    let content = std::fs::read_to_string(&target).unwrap();
    assert!(content.contains("# Default Configuration for LiDM"));
    assert!(content.contains("[logging]"));

    let _ = std::fs::remove_dir_all(temp_dir.join("lidm_test_copy_config"));
}
```

- [ ] **Step 2: Run `cargo test` to verify tests fail before implementation**

Run: `cargo test test_resolve_default_copy_path_custom`
Expected: FAIL (functions not defined)

- [ ] **Step 3: Implement `resolve_default_copy_path` and `execute_copy_config` in `src/config.rs`**

In `src/config.rs`:
```rust
impl Config {
    pub fn resolve_default_copy_path(dest: Option<&str>) -> std::path::PathBuf {
        if let Some(d) = dest {
            let trimmed = d.trim();
            if !trimmed.is_empty() {
                return std::path::PathBuf::from(trimmed);
            }
        }

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.trim().is_empty() {
                return std::path::PathBuf::from(xdg).join("lidm").join("default.toml");
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            if !home.trim().is_empty() {
                return std::path::PathBuf::from(home).join(".config").join("lidm").join("default.toml");
            }
        }

        std::path::PathBuf::from("/etc/lidm/default.toml")
    }

    pub fn execute_copy_config(dest: Option<&str>) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let path = Self::resolve_default_copy_path(dest);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_toml = Self::generate_default_toml();
        let header = "# Default Configuration for LiDM (Lightweight Display Manager)\n# All settings shown below with their default values.\n\n";
        let full_content = format!("{}{}", header, default_toml);

        std::fs::write(&path, full_content)?;
        Ok(path)
    }
}
```

- [ ] **Step 4: Run `cargo test` to verify implementation passes**

Run: `cargo test`
Expected: PASS (all tests pass)

- [ ] **Step 5: Commit changes**

```bash
git add src/config.rs
git commit -m "feat(config): implement resolve_default_copy_path and execute_copy_config"
```

---

### Task 2: Subcommand Definition & CLI Dispatch in `src/main.rs`

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

**Interfaces:**
- Consumes: `clap::Subcommand`, `Config::execute_copy_config`

- [ ] **Step 1: Update `src/main.rs` with `Commands` enum and dispatch logic**

In `src/main.rs`:
Update imports:
```rust
use clap::{Parser, Subcommand};
```

Add `Commands` enum:
```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Copy default configuration to local or specified config file")]
    CopyConfig {
        #[arg(help = "Destination path for the configuration file [default: ~/.config/lidm/default.toml]")]
        dest: Option<String>,
    },
}
```

Add field to `Args` struct:
```rust
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(help = "VT number to switch to")]
    pub vt: Option<c_int>,
    ...
```

At the top of `fn main()`:
```rust
    let args = Args::parse();

    if let Some(Commands::CopyConfig { ref dest }) = args.command {
        match config::Config::execute_copy_config(dest.as_deref()) {
            Ok(path) => {
                println!("Default configuration successfully copied to '{}'.", path.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error copying default configuration: {}", e);
                std::process::exit(1);
            }
        }
    }
```

- [ ] **Step 2: Run `cargo test` and manual binary execution check**

Run: `cargo test`
Expected: PASS

Run: `cargo run -- copy-config /tmp/test_copy_config.toml`
Expected: Prints `Default configuration successfully copied to '/tmp/test_copy_config.toml'.` and file `/tmp/test_copy_config.toml` exists.

- [ ] **Step 3: Commit changes**

```bash
git add src/main.rs
git commit -m "feat(cli): add copy-config subcommand to clap CLI"
```

---

### Task 3: Update Documentation in `README.md`

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add `copy-config` subcommand section to `README.md`**

In `README.md` under `# Usage` / `### Arguments` or a new `### Commands` section:
```markdown
### Commands

- `lidm copy-config [DEST]`: Copy the default configuration to your user config directory (`~/.config/lidm/default.toml`) or a specified destination path.

```bash
# Copy to default user config location (~/.config/lidm/default.toml)
lidm copy-config

# Copy to custom path
lidm copy-config /etc/lidm/default.toml
```
```

- [ ] **Step 2: Commit documentation update**

```bash
git add README.md
git commit -m "docs(readme): document copy-config subcommand"
```
