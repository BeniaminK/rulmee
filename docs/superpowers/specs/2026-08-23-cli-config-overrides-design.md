# Design Spec: Dynamic CLI Configuration Overrides (Zero Struct Duplication)

**Date:** 2026-08-23  
**Status:** Approved

## Overview
Currently, `lidm` supports configuration via a TOML file and automatic environment variables (`LIDM_<SECTION>_<KEY>`). 

This design introduces dynamic command-line configuration overrides (e.g. `--behavior-box-type rounded`, `--behavior_refresh_rate 250`, `--auth-pam-service custom`) following the exact same dynamic pattern as environment variables:
1. **Zero Struct Annotations / Repetition**: Configuration structs (`Behavior`, `AuthConfig`, `LoggingConfig`, etc.) remain 100% pure Rust/Serde structs with zero macro attributes or duplicate field definitions.
2. **Dynamic Flag Resolution**: CLI arguments matching `--<section>_<key>` or `--<section>-<key>` are dynamically mapped to configuration fields.
3. **Comprehensive `--help` Listing**: A dynamic help formatter inspects `Config::default()` and appends a complete table of all available configuration overrides and their default values to `lidm --help`.
4. **Hierarchical Precedence**: Defaults < TOML file < Environment variables (`LIDM_...`) < CLI flags.

---

## Architecture & Data Flow

### 1. Pure Configuration Structs (`src/config.rs`)
Structs remain completely clean without any `clap` or `clap-serde-derive` annotations:

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Behavior {
    pub box_type: BoxType,
    pub include_defshell: bool,
    pub show_console: bool,
    pub source: Vec<String>,
    pub user_source: Vec<String>,
    pub timefmt: String,
    pub refresh_rate: u64,
    pub bypass_shell_login: bool,
    pub show_theme: bool,
}
```

### 2. Dynamic CLI Argument Extraction
A parser extracts configuration override flags before passing top-level arguments (such as `-c / --config`, `vt`, subcommands) to Clap:

```rust
pub fn extract_cli_overrides<I, T>(args: I) -> (toml::Table, Vec<String>)
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
```

- Supports `--<section>_<key>=<value>`, `--<section>-<key>=<value>`, `--<section>_<key> <value>`, `--<section>-<key> <value>`.
- Supports boolean flags without explicit value (e.g., `--behavior-show-console` implies `true`).
- Auto-types values (`true`/`false` -> bool, numbers -> integer, comma-separated -> arrays, others -> string).
- Merges into the TOML table using the same pipeline as environment variables.

### 3. Dynamic `--help` Generation
`Config::generate_cli_help()` serializes `Config::default()` into a structured representation and formats an `after_help` section:

```rust
#[derive(Parser, Debug)]
#[command(
    about = "LiDM: Lightweight Display Manager",
    after_help = crate::config::Config::generate_cli_help()
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(help = "VT number to switch to")]
    pub vt: Option<c_int>,

    #[arg(
        short = 'c',
        long = "config",
        env = "LIDM_CONF",
        default_value = "/etc/lidm/default.toml",
        help = "Path to configuration file"
    )]
    pub conf_path: String,
}
```

---

## Configuration Loading Pipeline

1. **Defaults**: Initial configuration from `Config::default()`.
2. **TOML File**: Load from `-c / --config` path if present.
3. **Environment Variables**: Apply `LIDM_<SECTION>_<KEY>` overrides.
4. **CLI Overrides**: Apply extracted `--<section>-<key>` / `--<section>_<key>` overrides (highest priority).

---

## Testing Plan

1. **Unit Tests**:
   - `test_extract_cli_overrides_various_formats`: Test `--behavior-box-type rounded`, `--behavior_refresh_rate=250`, `--behavior-show-console`, etc.
   - `test_cli_overrides_precedence`: Verify `Defaults < TOML < Env < CLI`.
   - `test_generate_cli_help`: Verify `--help` contains all sections and default values.
2. **Automated Verification**:
   - Run `cargo test` and verify all tests pass.
