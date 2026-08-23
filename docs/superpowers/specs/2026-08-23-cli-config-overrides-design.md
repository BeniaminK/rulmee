# Design Spec: Unified CLI Configuration Overrides

**Date:** 2026-08-23  
**Status:** Approved

## Overview
Currently, `lidm` supports configuration via a TOML file and automatic environment variables (`LIDM_<SECTION>_<KEY>`). CLI argument overrides are limited to a small subset (e.g. logging options). 

This design introduces comprehensive CLI argument overrides (such as `--behavior-box-type`, `--behavior-refresh-rate`, etc.) using `clap-serde-derive`. This provides:
1. **Single struct definition**: Configuration structs (`Behavior`, `AuthConfig`, etc.) are defined once with no duplicate structs.
2. **Native `clap` `--help` support**: All flags, docstrings, and types appear automatically in `lidm --help`.
3. **Hierarchical Precedence**: Defaults < TOML file < Environment variables < CLI flags.

---

## Architecture & Struct Design

### Dependencies
Add `clap-serde-derive` to `Cargo.toml`:
```toml
[dependencies]
clap-serde-derive = "0.2"
```

### Struct Annotations in `src/config.rs`
The configuration structures derive `ClapSerde` alongside `Serialize`, `Deserialize`, and `Default`:

```rust
use clap_serde_derive::ClapSerde;

#[derive(ClapSerde, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Behavior {
    /// Border style: border, rounded, block, none
    #[default(BoxType::Border)]
    #[arg(long = "behavior-box-type", value_enum)]
    pub box_type: BoxType,

    /// Include default shell option in session list
    #[default(true)]
    #[arg(long = "behavior-include-defshell", action = clap::ArgAction::Set)]
    pub include_defshell: bool,

    /// Show intercepted console messages in TUI
    #[default(false)]
    #[arg(long = "behavior-show-console", action = clap::ArgAction::Set)]
    pub show_console: bool,

    /// System environment source files to load
    #[default(Vec::new())]
    #[arg(long = "behavior-source", value_delimiter = ',')]
    pub source: Vec<String>,

    /// User environment source files to load
    #[default(Vec::new())]
    #[arg(long = "behavior-user-source", value_delimiter = ',')]
    pub user_source: Vec<String>,

    /// Time format string for header clock
    #[default("%Y-%m-%d %H:%M:%S".to_string())]
    #[arg(long = "behavior-timefmt")]
    pub timefmt: String,

    /// UI refresh rate in milliseconds
    #[default(100)]
    #[arg(long = "behavior-refresh-rate")]
    pub refresh_rate: u64,

    /// Bypass login shell when launching user session
    #[default(false)]
    #[arg(long = "behavior-bypass-shell-login", action = clap::ArgAction::Set)]
    pub bypass_shell_login: bool,

    /// Show active theme in footer
    #[default(false)]
    #[arg(long = "behavior-show-theme", action = clap::ArgAction::Set)]
    pub show_theme: bool,
}
```

### Top-Level CLI Flattening in `src/main.rs`
`Args` flattens the config arguments using `<Config as ClapSerde>::Opt`:

```rust
#[derive(Parser, Debug)]
#[command(about = "LiDM: Lightweight Display Manager")]
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

    #[clap_serde]
    #[command(flatten)]
    pub config: <Config as ClapSerde>::Opt,
}
```

---

## Data Flow & Precedence

1. **Defaults**: Initial state is constructed from `Config::default()`.
2. **TOML File**: If `-c / --config <path>` exists, it is parsed via serde/toml and replaces/merges into `Config`.
3. **Environment Variables**: `config.apply_env_overrides()` inspects `LIDM_<SECTION>_<KEY>` and applies overrides.
4. **CLI Flags**: `config = config.merge_opts(args.config)` applies any explicitly provided CLI flags, overriding both TOML and environment variables.

---

## Error Handling & Validation

- `clap` validates argument types (integers, enum variants for `box_type`, etc.) during CLI parsing, printing clear help/usage messages on error.
- Corrupted or invalid TOML files trigger a fallback to default values while preserving environment and CLI overrides.

---

## Testing Plan

1. **Unit Tests**:
   - `test_cli_overrides_precedence`: Verify `Defaults < TOML < Env < CLI`.
   - `test_behavior_cli_flags`: Verify parsing and merging of all `Behavior` fields (`box_type`, `refresh_rate`, `bypass_shell_login`, etc.).
   - `test_help_output`: Verify all `--behavior-...` flags appear in CLI help generation.
2. **Automated Verification**:
   - Run `cargo test` and ensure all tests pass.
