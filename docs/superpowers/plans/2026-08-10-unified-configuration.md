# Unified Compound Configuration System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Centralize all application configuration into a single unified `Config` system in `src/config.rs` with automatic `LIDM_<SECTION>_<KEY>` environment variable mapping and zero scattered `std::env::var()` calls across the codebase.

**Architecture:** `clap` parses CLI args and environment variables into `Args`. `Config::load(&args)` applies layered resolution (Defaults < `/etc/lidm.ini` TOML < `LIDM_<SECTION>_<KEY>` Env Vars < CLI Args). Runtime reloads (F5 refresh & theme changes) re-read the configuration dynamically while preserving process startup overrides.

**Tech Stack:** Rust 2024, serde, toml, clap 4.5.

## Global Constraints
- Target files: `src/config.rs`, `src/main.rs`, `src/logging.rs`, `src/auth.rs`, `themes/default.ini`.
- Precedence: CLI Flags > Environment Variables (`LIDM_<SECTION>_<KEY>`) > `/etc/lidm.ini` > Built-in Defaults.
- Zero direct `std::env::var()` calls outside `clap` and `Config::load()`.

---

### Task 1: Schema Extensions in `src/config.rs`

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs`

**Interfaces:**
- Produces: `LoggingConfig` (`file: String`, `level: String`), `AuthConfig` (`pam_service: String`), updated `Config` struct.

- [ ] **Step 1: Write failing unit test for `LoggingConfig` and `AuthConfig` defaults**

```rust
#[test]
fn test_logging_and_auth_config_defaults() {
    let config = Config::default();
    assert_eq!(config.logging.file, "/tmp/lidm.log");
    assert_eq!(config.logging.level, "debug");
    assert_eq!(config.auth.pam_service, "login");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_logging_and_auth_config_defaults`  
Expected: FAIL with "no field `logging` on type `Config`"

- [ ] **Step 3: Add `LoggingConfig` and `AuthConfig` structs to `src/config.rs`**

```rust
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingConfig {
    pub file: String,
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            file: "/tmp/lidm.log".to_string(),
            level: "debug".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    pub pam_service: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            pam_service: "login".to_string(),
        }
    }
}
```

Add `pub logging: LoggingConfig` and `pub auth: AuthConfig` to `Config`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_logging_and_auth_config_defaults`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add LoggingConfig and AuthConfig schema sections to Config"
```

---

### Task 2: Automatic `LIDM_<SECTION>_<KEY>` Environment Mapping & `Config::load`

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs`

**Interfaces:**
- Consumes: `Args` (from `main.rs`)
- Produces: `Config::load(args: &Args) -> Result<Config, Box<dyn std::error::Error>>`

- [ ] **Step 1: Write failing unit test for `LIDM_<SECTION>_<KEY>` env overrides and precedence**

```rust
#[test]
fn test_config_automatic_env_overrides() {
    std::env::set_var("LIDM_LOGGING_LEVEL", "warn");
    std::env::set_var("LIDM_AUTH_PAM_SERVICE", "custom-pam");
    std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "250");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert_eq!(config.logging.level, "warn");
    assert_eq!(config.auth.pam_service, "custom-pam");
    assert_eq!(config.behavior.refresh_rate, 250);

    std::env::remove_var("LIDM_LOGGING_LEVEL");
    std::env::remove_var("LIDM_AUTH_PAM_SERVICE");
    std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_config_automatic_env_overrides`  
Expected: FAIL with "no method `apply_env_overrides`"

- [ ] **Step 3: Implement `apply_env_overrides` and `Config::load` in `src/config.rs`**

```rust
impl Config {
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("LIDM_LOGGING_FILE").or_else(|_| std::env::var("LIDM_LOG")) {
            if !val.is_empty() { self.logging.file = val; }
        }
        if let Ok(val) = std::env::var("LIDM_LOGGING_LEVEL").or_else(|_| std::env::var("LIDM_LOGLEVEL")) {
            if !val.is_empty() { self.logging.level = val; }
        }
        if let Ok(val) = std::env::var("LIDM_AUTH_PAM_SERVICE").or_else(|_| std::env::var("LIDM_PAM_SERVICE")) {
            if !val.is_empty() { self.auth.pam_service = val; }
        }

        for (key, val) in std::env::vars() {
            if let Some(rest) = key.strip_prefix("LIDM_") {
                if let Some((section, item)) = rest.split_once('_') {
                    let section = section.to_lowercase();
                    let item = item.to_lowercase();
                    match (section.as_str(), item.as_str()) {
                        ("behavior", "show_console") => {
                            if let Ok(b) = val.parse::<bool>() { self.behavior.show_console = b; }
                        }
                        ("behavior", "refresh_rate") => {
                            if let Ok(u) = val.parse::<u64>() { self.behavior.refresh_rate = u; }
                        }
                        ("behavior", "bypass_shell_login") => {
                            if let Ok(b) = val.parse::<bool>() { self.behavior.bypass_shell_login = b; }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_config_automatic_env_overrides`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): implement automatic LIDM_SECTION_KEY env overrides"
```

---

### Task 3: CLAP Integration & Main Loop Wiring in `src/main.rs`

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

**Interfaces:**
- Consumes: `clap::Parser`, `Config::load`
- Produces: Updated `Args` struct with `--logging-file`, `--logging-level`, `--auth-pam-service`, `--config`, and `vt`. Unified config loading in main loop.

- [ ] **Step 1: Update `Args` struct in `src/main.rs`**

```rust
#[derive(Parser, Debug)]
#[command(
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (git ",
        env!("LIDM_GIT_REV"),
        ", build date ",
        env!("LIDM_BUILD_TS"),
        ", compiler ",
        env!("LIDM_COMPILER_VER"),
        ")"
    ),
    about = "LiDM: Lightweight Display Manager"
)]
pub struct Args {
    #[arg(help = "VT number to switch to", env = "LIDM_VT")]
    pub vt: Option<c_int>,

    #[arg(long = "logging-file", help = "Path to log file", env = "LIDM_LOGGING_FILE")]
    pub logging_file: Option<String>,

    #[arg(long = "logging-level", help = "Log level filter", env = "LIDM_LOGGING_LEVEL")]
    pub logging_level: Option<String>,

    #[arg(long = "auth-pam-service", help = "PAM service name", env = "LIDM_AUTH_PAM_SERVICE")]
    pub auth_pam_service: Option<String>,

    #[arg(
        short = 'c',
        long = "config",
        env = "LIDM_CONF",
        default_value = "/etc/lidm.ini",
        help = "Path to configuration file"
    )]
    pub conf_path: String,
}
```

- [ ] **Step 2: Connect `Config::load(&args)` in `src/config.rs` and `src/main.rs`**

Add `Config::load` method to `src/config.rs`:
```rust
pub fn load(args: &crate::Args) -> Result<Self, Box<dyn std::error::Error>> {
    let mut config = Config::default();

    if Path::new(&args.conf_path).exists() {
        config.parse(&args.conf_path)?;
    }

    config.apply_env_overrides();

    if let Some(ref file) = args.logging_file {
        config.logging.file = file.clone();
    }
    if let Some(ref level) = args.logging_level {
        config.logging.level = level.clone();
    }
    if let Some(ref pam_service) = args.auth_pam_service {
        config.auth.pam_service = pam_service.clone();
    }

    Ok(config)
}
```

In `src/main.rs`, update configuration loading inside the main loop:
```rust
let config = match config::Config::load(&args) {
    Ok(cfg) => cfg,
    Err(e) => {
        error!("Error loading config from {}: {}", args.conf_path, e);
        std::process::exit(1);
    }
};
```

- [ ] **Step 3: Run `cargo check` to verify main loop wiring**

Run: `cargo check`  
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/config.rs
git commit -m "feat(main): wire CLAP args and Config::load in main loop"
```

---

### Task 4: Clean Up `src/logging.rs` and `src/auth.rs`

**Files:**
- Modify: `src/logging.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `&LoggingConfig`, `&config.auth.pam_service`
- Produces: Logging and authentication initialized directly from `Config`. No `std::env::var()` calls.

- [ ] **Step 1: Update `resolve_log_path` and `initialize_logging` in `src/logging.rs`**

Update `resolve_log_path` to accept optional CLI override:
```rust
pub fn resolve_log_path(cli_log_path: Option<&str>) -> String {
    if let Some(path) = cli_log_path {
        if !path.is_empty() {
            return path.to_string();
        }
    }
    "/tmp/lidm.log".to_string()
}
```

Update `initialize_logging` to accept `&LoggingConfig`:
```rust
pub fn initialize_logging(
    log_cfg: &crate::config::LoggingConfig,
    console_buffer: Option<ConsoleBuffer>,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_new(&log_cfg.level)
        .unwrap_or_else(|_| EnvFilter::new("debug"));
    ...
```

- [ ] **Step 2: Update `main.rs` to pass `&config.logging` and `&config.auth.pam_service`**

In `main.rs`:
```rust
let _log_guard = match logging::initialize_logging(&config.logging, Some(console_buffer.clone())) { ... };
```
and:
```rust
match auth::authenticate(&username, &password, &config.auth.pam_service) { ... }
```

- [ ] **Step 3: Run `cargo test` to verify build and tests pass**

Run: `cargo test`  
Expected: PASS (0 errors, 0 warnings)

- [ ] **Step 4: Commit**

```bash
git add src/logging.rs src/main.rs
git commit -m "refactor(logging,auth): remove scattered std::env::var calls in favor of unified Config"
```

---

### Task 5: Documentation & End-to-End Verification

**Files:**
- Modify: `themes/default.ini`

- [ ] **Step 1: Update `themes/default.ini` with `[logging]` and `[auth]` documented tables**

```ini
[logging]
# file = "/tmp/lidm.log"
# level = "debug"

[auth]
# pam_service = "login"
```

- [ ] **Step 2: Run full test suite and verify build**

Run: `cargo test`  
Expected: PASS

Run: `cargo build --release`  
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add themes/default.ini
git commit -m "docs(config): document [logging] and [auth] sections in default.ini"
```
