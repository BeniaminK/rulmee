# CLI Configuration Overrides Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement full command-line configuration overrides (such as `--behavior-box-type`, `--behavior-refresh-rate`, etc.) using a single struct definition and native `clap` `--help` with automatic default and enum display.

**Architecture:** Utilize `clap-serde-derive` on `Config` and its sub-structs (`Behavior`, `AuthConfig`, `LoggingConfig`, `Functions`, `Strings`, `Colors`). In `src/main.rs`, flatten `<Config as ClapSerde>::Opt` into `Args`, and merge CLI options on top of environment variables and TOML configuration in `Config::load`.

**Tech Stack:** Rust (edition 2024), `clap` 4.5, `clap-serde-derive` 0.2, `serde` 1.0, `toml` 1.1.

## Global Constraints

- Define configuration structs once without duplication.
- CLI flags for the behavior section must use the prefix `--behavior-<field>` (e.g. `--behavior-box-type`, `--behavior-refresh-rate`).
- Inferred enum parsing from `BoxType: clap::ValueEnum` without manual `value_enum` attributes.
- Native `--help` displays default values and valid enum choices.
- Hierarchical precedence: Defaults < TOML file < Environment variables (`LIDM_...`) < CLI flags.

---

### Task 1: Add Dependency and Configure `Cargo.toml`

**Files:**
- Modify: `Cargo.toml:20-25`

**Interfaces:**
- Produces: `clap-serde-derive` crate available for deriving `ClapSerde`.

- [ ] **Step 1: Update Cargo.toml dependencies**

Add `clap-serde-derive = "0.2"` to `Cargo.toml` under `[dependencies]`.

```toml
clap = { version = "4.5", features = ["derive", "env"] }
clap-serde-derive = "0.2"
```

- [ ] **Step 2: Verify dependency resolution**

Run: `cargo check`
Expected: PASS (resolves `clap-serde-derive` and compiles without errors).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add clap-serde-derive dependency"
```

---

### Task 2: Annotate Configuration Structs in `src/config.rs`

**Files:**
- Modify: `src/config.rs:1-150`

**Interfaces:**
- Consumes: `clap_serde_derive::ClapSerde`, `clap::ValueEnum`
- Produces: `Config`, `Behavior`, `AuthConfig`, `LoggingConfig`, `Functions`, `Strings`, `Colors` with `ClapSerde` implementations and `<Config as ClapSerde>::Opt`.

- [ ] **Step 1: Write failing unit test for `ClapSerde` parsing of `Behavior`**

In `src/config.rs` `tests` module:
```rust
#[test]
fn test_behavior_clap_serde_override() {
    use clap_serde_derive::ClapSerde;
    let mut config = Config::default();
    let opt = <Config as ClapSerde>::Opt::default();
    // Verify default Opt has None for fields before override
    assert!(opt.behavior.refresh_rate.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_behavior_clap_serde_override`
Expected: FAIL (ClapSerde not yet implemented on Config).

- [ ] **Step 3: Implement `ClapSerde` on `Config` and Sub-structs**

In `src/config.rs`:
```rust
use clap_serde_derive::ClapSerde;

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum BoxType {
    #[default]
    #[serde(alias = "plain", alias = "default")]
    Border,
    None,
    Rounded,
    Block,
}

#[derive(ClapSerde, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Behavior {
    /// Border style
    #[default(BoxType::Border)]
    #[arg(long = "behavior-box-type")]
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

#[derive(ClapSerde, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Functions {
    #[arg(long = "functions-poweroff")]
    pub poweroff: Option<String>,
    #[arg(long = "functions-reboot")]
    pub reboot: Option<String>,
    #[arg(long = "functions-fido")]
    pub fido: Option<String>,
    #[arg(long = "functions-theme")]
    pub theme: Option<String>,
}

#[derive(ClapSerde, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Strings {
    #[default("poweroff".to_string())]
    #[arg(long = "strings-f-poweroff")]
    pub f_poweroff: String,
    #[default("reboot".to_string())]
    #[arg(long = "strings-f-reboot")]
    pub f_reboot: String,
    #[arg(long = "strings-f-fido")]
    pub f_fido: Option<String>,
    #[arg(long = "strings-f-theme")]
    pub f_theme: Option<String>,
    #[default("user".to_string())]
    #[arg(long = "strings-e-user")]
    pub e_user: String,
    #[default("password".to_string())]
    #[arg(long = "strings-e-passwd")]
    pub e_passwd: String,
    #[default("wayland".to_string())]
    #[arg(long = "strings-s-wayland")]
    pub s_wayland: String,
    #[default("xorg".to_string())]
    #[arg(long = "strings-s-xorg")]
    pub s_xorg: String,
    #[default("shell".to_string())]
    #[arg(long = "strings-s-shell")]
    pub s_shell: String,
    #[default("< ".to_string())]
    #[arg(long = "strings-opts-pre")]
    pub opts_pre: String,
    #[default(" >".to_string())]
    #[arg(long = "strings-opts-post")]
    pub opts_post: String,
    #[default("…".to_string())]
    #[arg(long = "strings-ellipsis")]
    pub ellipsis: String,
}

#[derive(ClapSerde, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingConfig {
    #[default("/tmp/lidm.log".to_string())]
    #[arg(long = "logging-file")]
    pub file: String,
    #[default("debug".to_string())]
    #[arg(long = "logging-level")]
    pub level: String,
    #[default(false)]
    #[arg(long = "logging-stdout", action = clap::ArgAction::Set)]
    pub stdout: bool,
}

#[derive(ClapSerde, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    #[default("login".to_string())]
    #[arg(long = "auth-pam-service")]
    pub pam_service: String,
}

#[derive(ClapSerde, Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    #[clap_serde]
    #[command(flatten)]
    pub colors: Colors,

    #[clap_serde]
    #[command(flatten)]
    pub functions: Functions,

    #[clap_serde]
    #[command(flatten)]
    pub strings: Strings,

    #[clap_serde]
    #[command(flatten)]
    pub behavior: Behavior,

    #[clap_serde]
    #[command(flatten)]
    pub logging: LoggingConfig,

    #[clap_serde]
    #[command(flatten)]
    pub auth: AuthConfig,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_behavior_clap_serde_override`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/colors.rs
git commit -m "feat(config): derive ClapSerde on Config and sub-structs"
```

---

### Task 3: Update `Args` and `Config::load`

**Files:**
- Modify: `src/main.rs:33-69`
- Modify: `src/config.rs:225-265`

**Interfaces:**
- Consumes: `<Config as ClapSerde>::Opt`
- Produces: `Args` with flattened CLI config flags; `Config::load(&Args)` returning configuration merged with CLI overrides.

- [ ] **Step 1: Write failing test for full precedence (Defaults < TOML < Env < CLI)**

In `src/config.rs`:
```rust
#[test]
fn test_config_full_precedence_with_cli() {
    use clap_serde_derive::ClapSerde;
    let _guard = ENV_LOCK.lock().unwrap();

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_precedence_cli.toml");
    let toml_content = r#"
[behavior]
refresh_rate = 200
box_type = "rounded"
"#;
    std::fs::write(&config_path, toml_content).unwrap();

    unsafe {
        std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "300");
    }

    let mut opt = <Config as ClapSerde>::Opt::default();
    opt.behavior.refresh_rate = Some(400); // CLI overrides Env and TOML

    let args = crate::Args {
        command: None,
        vt: None,
        conf_path: config_path.to_str().unwrap().to_string(),
        config: opt,
    };

    let (cfg, _) = Config::load(&args);

    assert_eq!(cfg.behavior.refresh_rate, 400); // CLI won
    assert_eq!(cfg.behavior.box_type, BoxType::Rounded); // TOML preserved

    unsafe {
        std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
    }
    let _ = std::fs::remove_file(config_path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_config_full_precedence_with_cli`
Expected: FAIL (Args and Config::load not yet updated).

- [ ] **Step 3: Update `Args` and `Config::load`**

In `src/main.rs`:
```rust
use clap_serde_derive::ClapSerde;

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

In `src/config.rs`:
```rust
pub fn load(args: &crate::Args) -> (Self, Option<String>) {
    let conf_path = &args.conf_path;
    let (mut config, err_msg) = if Path::new(conf_path).exists() {
        match Self::from_file(conf_path) {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                let msg = format!(
                    "Failed to parse config from '{}': {}. Falling back to default configuration.",
                    conf_path, e
                );
                eprintln!("{}", msg);
                (Self::default(), Some(msg))
            }
        }
    } else {
        (Self::default(), None)
    };

    config.apply_env_overrides();
    config.merge_opts(args.config.clone());

    (config, err_msg)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_config_full_precedence_with_cli`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/config.rs
git commit -m "feat(cli): integrate ClapSerde Opt flattening in Args and Config::load"
```

---

### Task 4: CLI Help Output & Behavior Flag Integration Tests

**Files:**
- Modify: `src/config.rs` (tests)
- Modify: `src/main.rs` (tests)

- [ ] **Step 1: Write tests for CLI argument parsing and `--help` output**

```rust
#[test]
fn test_cli_help_contains_behavior_flags() {
    use clap::CommandFactory;
    let mut cmd = Args::command();
    let mut help_buf = Vec::new();
    cmd.write_help(&mut help_buf).unwrap();
    let help_str = String::from_utf8(help_buf).unwrap();

    assert!(help_str.contains("--behavior-box-type"));
    assert!(help_str.contains("--behavior-refresh-rate"));
    assert!(help_str.contains("--behavior-include-defshell"));
    assert!(help_str.contains("--behavior-bypass-shell-login"));
    assert!(help_str.contains("[default: border]"));
    assert!(help_str.contains("[default: 100]"));
}

#[test]
fn test_cli_parse_from_args_behavior_flags() {
    let args = Args::try_parse_from([
        "lidm",
        "--behavior-box-type", "rounded",
        "--behavior-refresh-rate", "350",
        "--behavior-bypass-shell-login", "true",
    ]).unwrap();

    let (config, _) = Config::load(&args);
    assert_eq!(config.behavior.box_type, BoxType::Rounded);
    assert_eq!(config.behavior.refresh_rate, 350);
    assert!(config.behavior.bypass_shell_login);
}
```

- [ ] **Step 2: Run all tests to verify**

Run: `cargo test`
Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "test(cli): add tests for --help rendering and CLI flag parsing"
```
