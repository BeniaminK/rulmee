# `copy-config` Subcommand Design Spec

## Goal
Add a `copy-config` subcommand to the `lidm` CLI that copies default configuration values into a local configuration file (by default `~/.config/lidm/default.toml` or `$XDG_CONFIG_HOME/lidm/default.toml`, or a user-specified destination path).

## User Interface & Usage

```bash
# Copy default config to local user config directory (~/.config/lidm/default.toml)
lidm copy-config

# Copy default config to a specific target file
lidm copy-config /etc/lidm/default.toml
```

## Architecture & Data Flow

### 1. `clap` Subcommand Definition (`src/main.rs`)
Add `Commands` enum to `src/main.rs`:

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

Add `pub command: Option<Commands>` to `Args` struct in `src/main.rs`.

### 2. Destination Path Resolution (`src/config.rs`)
Define helper function `pub fn resolve_default_copy_path(dest: Option<&str>) -> std::path::PathBuf`:
- If `dest` is provided and non-empty, use `PathBuf::from(dest)`.
- If `dest` is `None` or empty:
  1. Check `$XDG_CONFIG_HOME`: if set, return `$XDG_CONFIG_HOME/lidm/default.toml`.
  2. Check `$HOME` (or `dirs` / environment): return `$HOME/.config/lidm/default.toml`.
  3. Fallback: `/etc/lidm/default.toml`.

### 3. Subcommand Execution (`src/config.rs` & `src/main.rs`)
Define `pub fn execute_copy_config(dest: Option<&str>) -> Result<(), Box<dyn std::error::Error>>`:
1. Resolve target path using `resolve_default_copy_path`.
2. Ensure parent directory exists (`std::fs::create_dir_all`).
3. Generate content using `Config::generate_default_toml()`.
4. Prepend explanatory header comment:
   ```toml
   # Default Configuration for LiDM (Lightweight Display Manager)
   # All settings shown below with their default values.

   ```
5. Write content to target file (`std::fs::write`).
6. Print confirmation: `Default configuration copied to <path>`.

In `src/main.rs`:
If `args.command` is `Some(Commands::CopyConfig { dest })`, run `execute_copy_config(dest.as_deref())` and exit with `0` (or `1` on write error).

## Testing Plan
- Unit tests in `src/config.rs`:
  - `test_resolve_default_copy_path_custom`: Verifies custom destination path resolution.
  - `test_execute_copy_config_tempfile`: Verifies `execute_copy_config` creates file with parent dirs and correct header/content.
- Run `cargo test` to ensure 100% pass rate.
