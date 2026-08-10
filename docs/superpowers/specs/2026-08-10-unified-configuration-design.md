# Unified Compound Configuration System

**Date:** 2026-08-10  
**Status:** Approved  
**Target Component:** `src/config.rs`, `src/main.rs`, `src/logging.rs`, `src/auth.rs`, `themes/default.ini`

---

## 1. Overview & Goals

LiDM currently accesses environment variables (such as `LIDM_LOG`, `LIDM_LOGLEVEL`, and `LIDM_PAM_SERVICE`) via scattered `std::env::var()` calls in multiple modules (`logging.rs`, `main.rs`).

This specification introduces a **Unified Compound Configuration System** modeled after layered configuration systems (e.g. Spring Boot). All application configuration is centralized into a single `Config` struct in `src/config.rs`. Configuration settings follow a strict, automatic $1:1$ naming convention between TOML sections/keys and environment variables:

$$\text{Environment Variable} = \text{\texttt{LIDM\_}} + \text{\texttt{SECTION}} + \text{\texttt{\_}} + \text{\texttt{KEY}}$$

All environment variables and CLI arguments are bound once at startup via `clap` and resolved dynamically in `Config::load(&args)`.

---

## 2. Order of Precedence

Configuration values are resolved in the following strict hierarchy (highest priority wins):

1. **Command Line Arguments** (`--logging-file`, `--logging-level`, `--auth-pam-service`, `-c`/`--config`, `vt`)
2. **Environment Variables** (`LIDM_<SECTION>_<KEY>`, e.g., `LIDM_LOGGING_LEVEL`, `LIDM_AUTH_PAM_SERVICE`, `LIDM_BEHAVIOR_REFRESH_RATE`)
3. **Configuration File (`/etc/lidm.ini` or custom path)** (`[logging]`, `[auth]`, `[behavior]`, etc.)
4. **Built-in Defaults** (`Config::default()`)

---

## 3. Automatic 1:1 Naming Convention & Legacy Aliases

Every section and key in `lidm.ini` corresponds to an environment variable by converting section and key to uppercase:

| TOML Section | TOML Key | Canonical Env Variable | Backward Compatibility Alias | CLI Flag |
| :--- | :--- | :--- | :--- | :--- |
| `[logging]` | `file` | `LIDM_LOGGING_FILE` | `LIDM_LOG` | `--logging-file` |
| `[logging]` | `level` | `LIDM_LOGGING_LEVEL` | `LIDM_LOGLEVEL` | `--logging-level` |
| `[auth]` | `pam_service` | `LIDM_AUTH_PAM_SERVICE` | `LIDM_PAM_SERVICE` | `--auth-pam-service` |
| `[behavior]` | `show_console` | `LIDM_BEHAVIOR_SHOW_CONSOLE` | — | `--behavior-show-console` |
| `[behavior]` | `refresh_rate` | `LIDM_BEHAVIOR_REFRESH_RATE` | — | `--behavior-refresh-rate` |
| `[behavior]` | `bypass_shell_login` | `LIDM_BEHAVIOR_BYPASS_SHELL_LOGIN` | — | `--behavior-bypass-shell-login` |
| (top-level) | `conf_path` | `LIDM_CONF` | — | `-c`, `--config` |

---

## 4. Config Struct Schema (`src/config.rs`)

`Config` is extended with `LoggingConfig` (`[logging]`) and `AuthConfig` (`[auth]`):

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

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub colors: Colors,
    pub chars: CharsConfig,
    pub functions: Functions,
    pub strings: Strings,
    pub behavior: Behavior,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
}
```

---

## 5. Compound Resolver & Reload Lifecycle (`Config::load`)

`Config::load(args: &Args)` performs the unified resolution:

```rust
impl Config {
    pub fn load(args: &Args) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::default();

        // 1. Read TOML file if present
        if Path::new(&args.conf_path).exists() {
            config.parse(&args.conf_path)?;
        }

        // 2. Apply automatic LIDM_<SECTION>_<KEY> environment variables
        config.apply_env_overrides();

        // 3. Apply explicit CLI flag overrides (highest precedence)
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

    fn apply_env_overrides(&mut self) {
        // Inspect std::env::vars() for LIDM_<SECTION>_<KEY> and legacy aliases
        if let Ok(val) = std::env::var("LIDM_LOGGING_FILE").or_else(|_| std::env::var("LIDM_LOG")) {
            if !val.is_empty() { self.logging.file = val; }
        }
        if let Ok(val) = std::env::var("LIDM_LOGGING_LEVEL").or_else(|_| std::env::var("LIDM_LOGLEVEL")) {
            if !val.is_empty() { self.logging.level = val; }
        }
        if let Ok(val) = std::env::var("LIDM_AUTH_PAM_SERVICE").or_else(|_| std::env::var("LIDM_PAM_SERVICE")) {
            if !val.is_empty() { self.auth.pam_service = val; }
        }

        // Automatic mapping for any other LIDM_<SECTION>_<KEY> env vars
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

### Runtime Reload Lifecycle
In `main.rs`, the main execution loop calls `Config::load(&args)` on every iteration (including UI refresh / F5 and theme cycling). This ensures:
- Changing `/etc/lidm.ini` at runtime takes effect upon F5 refresh.
- Environment variables and CLI arguments passed at process startup remain enforced as high-precedence overrides.

---

## 6. Codebase Clean-up

All direct `std::env::var(...)` calls across `src/logging.rs`, `src/main.rs`, and `src/auth.rs` are removed:
- `logging::initialize_logging(&config.logging, ...)` receives `&LoggingConfig` directly.
- `auth::authenticate(&username, &password, &config.auth.pam_service)` consumes `config.auth.pam_service`.

---

## 7. Testing & Verification

1. **Precedence Tests**: Unit tests in `src/config.rs` testing Defaults < TOML < Env < CLI override order.
2. **Automatic Env Mapping Tests**: Unit tests verifying `LIDM_LOGGING_LEVEL`, `LIDM_AUTH_PAM_SERVICE`, `LIDM_BEHAVIOR_REFRESH_RATE`, and legacy aliases (`LIDM_LOG`).
3. **Build & Test Suite**: Run `cargo test` and `cargo build` to ensure clean build with zero warnings.
