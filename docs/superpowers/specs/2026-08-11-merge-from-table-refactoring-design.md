# Refactoring `merge_from_table` in `src/config.rs`

## Problem
In `src/config.rs`, `Config::apply_env_overrides` uses a custom `merge_from_table` function and a helper macro `merge_fields!` (defined in `src/macros.rs`) to copy individual fields from an environment-variable configuration table into the main `Config` struct.

This requires:
- Exhaustive string matching on section names ("logging", "auth", "behavior", "strings", "functions", "chars").
- Explicitly registering every field key in `merge_fields!`.
- Extra maintenance overhead whenever fields are added or modified in `Config`.

## Solution: Generic `toml::Value` Deep Merge
Replace `merge_from_table` and `merge_fields!` with a recursive TOML table merge function (`merge_toml_values`).

### Mechanism
1. Convert current `Config` instance (`*self`) to a `toml::Value` table via `toml::Value::try_from(&*self)`.
2. Construct `env_table: toml::Table` from matching `LIDM_<SECTION>_<KEY>` environment variables.
3. Deep-merge `env_table` into `current_val` using `merge_toml_values`.
4. Deserialize the merged `toml::Value` back into `Config` (`current_val.try_into::<Config>()`).
5. Remove `merge_from_table` from `src/config.rs`.
6. Remove `merge_fields!` from `src/macros.rs` (or remove `src/macros.rs` entirely if unused elsewhere).

### Algorithms & Code

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

In `Config::apply_env_overrides`:
```rust
if let Ok(mut current_val) = toml::Value::try_from(&*self) {
    merge_toml_values(&mut current_val, toml::Value::Table(env_table));
    if let Ok(updated) = current_val.try_into::<Config>() {
        *self = updated;
    }
}
```

## Testing Plan
- Existing unit tests in `src/config.rs` (`test_config_automatic_env_overrides`, `test_config_arbitrary_env_override_strings`, `test_config_load_precedence`) verify env override merging works properly.
- Run `cargo test` to ensure 100% test pass rate.
