# Rulmee Rebrand & Configuration Path Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement configuration directory fallback lookup from `/etc/rulmee/default.toml` to `/etc/lidm/default.toml` with deprecation warnings, and update crate default paths to `/etc/rulmee/`.

**Architecture:** Update `Args::conf_path` default value in `src/main.rs` to `/etc/rulmee/default.toml` (env variable `RULMEE_CONF` with `LIDM_CONF` fallback), and update `Config::load_with_overrides` in `src/config.rs` to check for fallback path `/etc/lidm/default.toml` when `/etc/rulmee/default.toml` is not found, logging a deprecation warning.

**Tech Stack:** Rust (edition 2024), `clap`, `log`, `serde`, `toml`.

## Global Constraints

- Preserve complete backward compatibility when user system has `/etc/lidm/default.ini`.
- Issue clear `log::warn!` deprecation warnings when falling back to `/etc/lidm/` paths.
- Ensure all unit and integration tests pass cleanly with `cargo test`.

---

### Task 1: Add Fallback Configuration Path Resolution in `Config::load_with_overrides`

**Files:**
- Modify: `src/config.rs:347-375`
- Test: `src/config.rs` (unit tests at bottom)

**Interfaces:**
- Consumes: `conf_path: &str`, `cli_overrides: Option<toml::Table>`
- Produces: `Config::load_with_overrides` with fallback resolution logic

- [ ] **Step 1: Write failing unit test for fallback configuration path resolution**

```rust
#[test]
fn test_load_with_overrides_fallback_path() {
    let temp_dir = std::env::temp_dir();
    let legacy_dir = temp_dir.join("lidm_test_fallback");
    let _ = std::fs::create_dir_all(&legacy_dir);
    let legacy_conf = legacy_dir.join("default.toml");
    std::fs::write(&legacy_conf, "[behavior]\nshow_console = true\n").unwrap();

    let primary_path = "/nonexistent/path/rulmee/default.toml";
    // We expect loading from primary_path to fallback to legacy_conf if configured or fallback helper is invoked
    let (cfg, warning) = Config::load_with_fallback(primary_path, legacy_conf.to_str().unwrap(), None);
    assert!(cfg.behavior.show_console);
    assert!(warning.is_some());
    assert!(warning.unwrap().contains("deprecated"));

    let _ = std::fs::remove_dir_all(legacy_dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_load_with_overrides_fallback_path`
Expected: FAIL with "function `load_with_fallback` not found"

- [ ] **Step 3: Implement path resolution logic in `Config`**

Modify `src/config.rs`:
```rust
pub fn resolve_config_path(primary: &str) -> (String, Option<String>) {
    if Path::new(primary).exists() {
        return (primary.to_string(), None);
    }
    
    // Check fallback path if primary is default /etc/rulmee path
    if primary == "/etc/rulmee/default.toml" && Path::new("/etc/lidm/default.toml").exists() {
        let warn_msg = "Path '/etc/rulmee/default.toml' not found; falling back to legacy '/etc/lidm/default.toml'. Please migrate configuration to '/etc/rulmee/'.".to_string();
        log::warn!("{}", warn_msg);
        return ("/etc/lidm/default.ini".to_string(), Some(warn_msg));
    }

    (primary.to_string(), None)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_load_with_overrides_fallback_path`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): implement configuration path resolution with legacy fallback"
```

---

### Task 2: Update CLI Default Config Path and Environment Variable in `main.rs`

**Files:**
- Modify: `src/main.rs:55-63`
- Test: `src/main.rs` (unit tests at bottom)

**Interfaces:**
- Consumes: CLI args (`--config`, `RULMEE_CONF`, `LIDM_CONF`)
- Produces: `Args.conf_path` defaulted to `/etc/rulmee/default.toml`

- [ ] **Step 1: Write failing CLI args test for default config path**

```rust
#[test]
fn test_cli_args_default_config_path() {
    let args = Args::parse_from(["rulmee"]);
    assert_eq!(args.conf_path, "/etc/rulmee/default.toml");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_cli_args_default_config_path`
Expected: FAIL (returns `/etc/lidm/default.toml`)

- [ ] **Step 3: Update `Args` struct attributes in `src/main.rs`**

```rust
    #[arg(
        short = 'c',
        long = "config",
        env = "RULMEE_CONF",
        default_value = "/etc/rulmee/default.toml",
        help = "Path to configuration file"
    )]
    pub conf_path: String,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_cli_args_default_config_path`
Expected: PASS

- [ ] **Step 5: Run all test suite to ensure clean build**

Run: `cargo test`
Expected: PASS all tests

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): update default config path to /etc/rulmee/default.toml with RULMEE_CONF env var"
```
