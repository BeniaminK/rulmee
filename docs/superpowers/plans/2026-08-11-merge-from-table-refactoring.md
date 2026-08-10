# Refactor `merge_from_table` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `src/config.rs` to replace `merge_from_table` and `merge_fields!` macro with a generic `toml::Value` deep merge mechanism.

**Architecture:** `Config::apply_env_overrides` serializes current `*self` into `toml::Value`, recursively merges `env_table` into it via `merge_toml_values`, and deserializes the merged `toml::Value` back into `Config`.

**Tech Stack:** Rust 2024, toml 1.1, serde 1.0.

## Global Constraints
- Target files: `src/config.rs`, `src/macros.rs`.
- Must preserve 100% of existing behavior and test coverage for `LIDM_<SECTION>_<KEY>` environment variable overrides.
- No macro required; delete `merge_fields!` and `src/macros.rs` if `merge_fields!` is the only macro defined.

---

### Task 1: Replace `merge_from_table` with `merge_toml_values` in `src/config.rs`

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs`

**Interfaces:**
- Produces: `fn merge_toml_values(dest: &mut toml::Value, source: toml::Value)`
- Modifies: `Config::apply_env_overrides` to use `merge_toml_values` and removes `merge_from_table`

- [ ] **Step 1: Implement `merge_toml_values` and update `apply_env_overrides` in `src/config.rs`**

In `src/config.rs`:
```rust
fn merge_toml_values(dest: &mut toml::Value, source: toml::Value) {
    match (dest, source) {
        (toml::Value::Table(dest_map), toml::Value::Table(source_map)) => {
            for (key, val) in source_map {
                merge_toml_values(
                    dest_map
                        .entry(key)
                        .or_insert_with(|| toml::Value::Table(toml::Table::new())),
                    val,
                );
            }
        }
        (dest, source) => *dest = source,
    }
}
```

In `Config::apply_env_overrides`, replace line 241:
```rust
        // Walk the env_table keys and copy only those fields from env_config → self.
        Self::merge_from_table(self, &env_config, &env_table);
```
and lines 231-238 (the `env_config` serialization step) with:
```rust
        if let Ok(mut current_val) = toml::Value::try_from(&*self) {
            merge_toml_values(&mut current_val, toml::Value::Table(env_table));
            if let Ok(updated) = current_val.try_into::<Config>() {
                *self = updated;
            }
        }
```
Remove `merge_from_table` function completely from `src/config.rs`.

- [ ] **Step 2: Run unit tests to verify behavior and test pass rate**

Run: `cargo test`
Expected: All 51 tests PASS.

- [ ] **Step 3: Commit changes**

```bash
git add src/config.rs
git commit -m "refactor(config): replace merge_from_table with recursive toml::Value deep merge"
```

---

### Task 2: Remove obsolete `src/macros.rs`

**Files:**
- Modify: `src/main.rs`
- Delete: `src/macros.rs`

**Interfaces:**
- Removes: `merge_fields!` macro

- [ ] **Step 1: Check if `src/macros.rs` is used anywhere else**

Check codebase for any other invocations of `merge_fields!` or other macros in `src/macros.rs`.
If `src/macros.rs` only contained `merge_fields!`, remove `mod macros;` from `src/main.rs` and delete `src/macros.rs`.

- [ ] **Step 2: Run `cargo test`**

Run: `cargo test`
Expected: PASS with 0 warnings/errors regarding `macros`.

- [ ] **Step 3: Commit removal**

```bash
git rm src/macros.rs
git add src/main.rs
git commit -m "refactor(macros): remove src/macros.rs after replacing merge_fields! macro"
```
