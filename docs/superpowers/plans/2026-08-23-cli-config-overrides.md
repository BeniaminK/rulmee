# Dynamic CLI Configuration Overrides Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement dynamic command-line configuration overrides (e.g. `--behavior-box-type`, `--behavior_refresh_rate`, `--auth-pam-service`, etc.) with zero struct annotations/repetition, hierarchical precedence, and auto-generated `--help` displaying all options and defaults.

**Architecture:** Add dynamic CLI override parsing in `src/config.rs` that extracts `--<section>-<key>` / `--<section>_<key>` flags and merges them into the TOML table (matching the environment variable pipeline). Generate dynamic `after_help` text for `clap` from `Config::default()`.

**Tech Stack:** Rust (edition 2024), `clap` 4.5, `serde` 1.0, `toml` 1.1.

## Global Constraints

- Do NOT add any struct annotations, attributes, or duplicate structs. `Behavior`, `AuthConfig`, etc. remain clean.
- Support both kebab-case (`--behavior-box-type`) and snake_case (`--behavior_box_type`), with `=` or space separation.
- Support boolean flags without explicit value (e.g. `--behavior-show-console` sets to `true`).
- Dynamic `--help` displays all sections, flags, and default values.
- Precedence: Defaults < TOML < Environment variables < CLI overrides.

---

### Task 1: Implement Dynamic CLI Override Parsing in `src/config.rs`

**Files:**
- Modify: `src/config.rs`

**Interfaces:**
- Produces:
  - `pub fn extract_cli_overrides<I, T>(args: I) -> (toml::Table, Vec<String>)`
  - `pub fn apply_table_overrides(&mut self, table: toml::Table)`

- [ ] **Step 1: Write failing unit tests for `extract_cli_overrides`**

In `src/config.rs` `tests` module:
```rust
#[test]
fn test_extract_cli_overrides_basic() {
    let raw_args = vec![
        "lidm".to_string(),
        "-c".to_string(),
        "/etc/lidm/default.toml".to_string(),
        "--behavior-box-type".to_string(),
        "rounded".to_string(),
        "--behavior_refresh_rate=250".to_string(),
        "--behavior-show-console".to_string(),
        "--auth-pam-service".to_string(),
        "custom-pam".to_string(),
        "2".to_string(),
    ];

    let (overrides, remaining) = Config::extract_cli_overrides(raw_args);

    assert_eq!(remaining, vec!["lidm", "-c", "/etc/lidm/default.toml", "2"]);

    let behavior = overrides.get("behavior").unwrap().as_table().unwrap();
    assert_eq!(behavior.get("box_type").unwrap().as_str().unwrap(), "rounded");
    assert_eq!(behavior.get("refresh_rate").unwrap().as_integer().unwrap(), 250);
    assert_eq!(behavior.get("show_console").unwrap().as_bool().unwrap(), true);

    let auth = overrides.get("auth").unwrap().as_table().unwrap();
    assert_eq!(auth.get("pam_service").unwrap().as_str().unwrap(), "custom-pam");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_extract_cli_overrides_basic`
Expected: FAIL (method does not exist).

- [ ] **Step 3: Implement `extract_cli_overrides` and `apply_table_overrides`**

In `src/config.rs`:
```rust
impl Config {
    /// Extract configuration overrides matching `--<section>_<key>` or `--<section>-<key>`
    /// from an argument list, returning the parsed TOML table and the remaining arguments.
    pub fn extract_cli_overrides<I, T>(args: I) -> (toml::Table, Vec<String>)
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let known_sections = ["colors", "functions", "strings", "behavior", "logging", "auth"];
        let mut cli_table = toml::Table::new();
        let mut remaining = Vec::new();

        let raw_list: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        let mut i = 0;
        while i < raw_list.len() {
            let arg = &raw_list[i];

            // Don't treat standalone "-" or non-flag as override
            if !arg.starts_with("--") || arg == "--" {
                remaining.push(arg.clone());
                i += 1;
                continue;
            }

            let flag_body = &arg[2..]; // strip "--"
            let (full_key, inline_val) = match flag_body.split_once('=') {
                Some((k, v)) => (k, Some(v.to_string())),
                None => (flag_body, None),
            };

            // Split on first '_' or '-'
            let (section_candidate, item_candidate) = if let Some(pos) = full_key.find(|c| c == '_' || c == '-') {
                (&full_key[..pos], &full_key[pos + 1..])
            } else {
                ("", "")
            };

            let section = section_candidate.to_lowercase();
            // Convert '-' in item name to '_' (e.g. box-type -> box_type)
            let item = item_candidate.to_lowercase().replace('-', "_");

            if known_sections.contains(&section.as_str()) && !item.is_empty() {
                let val_str = if let Some(v) = inline_val {
                    v
                } else if i + 1 < raw_list.len() && !raw_list[i + 1].starts_with('-') {
                    i += 1;
                    raw_list[i].clone()
                } else {
                    // Boolean flag without value implies true
                    "true".to_string()
                };

                let toml_val = if val_str.eq_ignore_ascii_case("true") {
                    toml::Value::Boolean(true)
                } else if val_str.eq_ignore_ascii_case("false") {
                    toml::Value::Boolean(false)
                } else if let Ok(n) = val_str.parse::<i64>() {
                    toml::Value::Integer(n)
                } else if val_str.contains(',') && !val_str.starts_with('"') {
                    // Comma-separated list for Vec<String>
                    toml::Value::Array(
                        val_str
                            .split(',')
                            .map(|s| toml::Value::String(s.trim().to_string()))
                            .collect(),
                    )
                } else {
                    toml::Value::String(val_str)
                };

                let sec_entry = cli_table
                    .entry(section)
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let toml::Value::Table(table) = sec_entry {
                    table.insert(item, toml_val);
                }
            } else {
                remaining.push(arg.clone());
            }

            i += 1;
        }

        (cli_table, remaining)
    }

    pub fn apply_table_overrides(&mut self, overrides: toml::Table) {
        if overrides.is_empty() {
            return;
        }
        if let Ok(mut current_val) = toml::Value::try_from(&*self) {
            merge_toml_values(&mut current_val, toml::Value::Table(overrides));
            if let Ok(updated) = current_val.try_into::<Config>() {
                *self = updated;
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_extract_cli_overrides_basic`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): implement dynamic CLI override parser"
```

---

### Task 2: Implement Dynamic `--help` Documentation Generation

**Files:**
- Modify: `src/config.rs`

**Interfaces:**
- Produces: `pub fn generate_cli_help() -> &'static str` or `String`

- [ ] **Step 1: Write failing test for `generate_cli_help`**

In `src/config.rs`:
```rust
#[test]
fn test_generate_cli_help_contains_options() {
    let help_text = Config::generate_cli_help();
    assert!(help_text.contains("Configuration Overrides:"));
    assert!(help_text.contains("--behavior-box-type"));
    assert!(help_text.contains("--behavior-refresh-rate"));
    assert!(help_text.contains("--auth-pam-service"));
    assert!(help_text.contains("[default: 100]"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_generate_cli_help_contains_options`
Expected: FAIL.

- [ ] **Step 3: Implement `generate_cli_help`**

In `src/config.rs`:
```rust
impl Config {
    pub fn generate_cli_help() -> String {
        let default_val = toml::Value::try_from(&Config::default()).unwrap_or(toml::Value::Table(toml::Table::new()));
        let mut out = String::from(
            "Configuration Overrides:\n  Any setting can be overridden via --<section>-<key> <value> or LIDM_<SECTION>_<KEY>=<value>\n\n",
        );

        if let toml::Value::Table(sections) = default_val {
            for (sec_name, sec_val) in sections {
                if let toml::Value::Table(keys) = sec_val {
                    out.push_str(&format!("  [{}]:\n", sec_name));
                    for (k, v) in keys {
                        let flag_name = format!("--{}-{}", sec_name, k.replace('_', "-"));
                        let default_display = match v {
                            toml::Value::String(s) => format!("\"{}\"", s),
                            toml::Value::Integer(i) => i.to_string(),
                            toml::Value::Float(f) => f.to_string(),
                            toml::Value::Boolean(b) => b.to_string(),
                            toml::Value::Array(a) => format!("{:?}", a),
                            _ => format!("{}", v),
                        };
                        out.push_str(&format!("    {:<35} [default: {}]\n", flag_name, default_display));
                    }
                    out.push('\n');
                }
            }
        }
        out
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_generate_cli_help_contains_options`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): generate dynamic configuration help text"
```

---

### Task 3: Integrate with `main.rs` & `Config::load`

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

**Interfaces:**
- Updates `main()` to extract CLI overrides, pass remaining args to Clap with dynamic `after_help`, and pass overrides to `Config::load`.

- [ ] **Step 1: Write failing test for full precedence (Defaults < TOML < Env < CLI)**

In `src/config.rs`:
```rust
#[test]
fn test_config_load_precedence_full() {
    let _guard = ENV_LOCK.lock().unwrap();
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_lidm_dyn_precedence.toml");
    let toml_content = r#"
[behavior]
refresh_rate = 150
box_type = "rounded"

[auth]
pam_service = "toml-pam"
"#;
    std::fs::write(&config_path, toml_content).unwrap();

    unsafe {
        std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "300");
    }

    let mut cli_overrides = toml::Table::new();
    let mut behavior_overrides = toml::Table::new();
    behavior_overrides.insert("refresh_rate".to_string(), toml::Value::Integer(500));
    cli_overrides.insert("behavior".to_string(), toml::Value::Table(behavior_overrides));

    let (config, _) = Config::load_with_overrides(config_path.to_str().unwrap(), Some(cli_overrides));

    // CLI overrides Env (500 > 300)
    assert_eq!(config.behavior.refresh_rate, 500);
    // TOML preserved
    assert_eq!(config.behavior.box_type, BoxType::Rounded);
    assert_eq!(config.auth.pam_service, "toml-pam");

    unsafe {
        std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
    }
    let _ = std::fs::remove_file(config_path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_config_load_precedence_full`
Expected: FAIL.

- [ ] **Step 3: Update `Config::load` and `main.rs`**

In `src/config.rs`:
```rust
pub fn load_with_overrides(conf_path: &str, cli_overrides: Option<toml::Table>) -> (Self, Option<String>) {
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
    if let Some(overrides) = cli_overrides {
        config.apply_table_overrides(overrides);
    }

    (config, err_msg)
}
```

In `src/main.rs`:
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
    about = "LiDM: Lightweight Display Manager",
    after_help = config::Config::generate_cli_help()
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

fn main() {
    let (cli_overrides, remaining_args) = config::Config::extract_cli_overrides(std::env::args());
    let args = Args::parse_from(remaining_args);

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

    // ... Load config
    let (config, config_err) = config::Config::load_with_overrides(&args.conf_path, Some(cli_overrides));
    // ...
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/config.rs
git commit -m "feat: integrate dynamic CLI overrides into main and Config::load"
```

---

### Task 4: Comprehensive End-to-End Tests

**Files:**
- Modify: `src/config.rs` (tests)
- Modify: `src/main.rs` (tests)

- [ ] **Step 1: Add integration tests for CLI flags and `--help`**

```rust
#[test]
fn test_full_cli_args_parsing_and_config_apply() {
    let input_args = vec![
        "lidm",
        "--behavior-box-type", "block",
        "--behavior-refresh-rate", "450",
        "--behavior-bypass-shell-login", "true",
        "--logging-level", "warn",
        "-c", "/nonexistent.toml",
    ];

    let (overrides, remaining) = Config::extract_cli_overrides(input_args);
    let parsed_args = Args::try_parse_from(remaining).unwrap();

    let (config, _) = Config::load_with_overrides(&parsed_args.conf_path, Some(overrides));
    assert_eq!(config.behavior.box_type, BoxType::Block);
    assert_eq!(config.behavior.refresh_rate, 450);
    assert!(config.behavior.bypass_shell_login);
    assert_eq!(config.logging.level, "warn");
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "test: add full integration tests for dynamic CLI overrides"
```
