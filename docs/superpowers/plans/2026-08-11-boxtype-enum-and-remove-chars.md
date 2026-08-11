# `BoxType` Enum & `CharsConfig` Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `CharsConfig` (`[chars]` section) and convert `box_type` in `Behavior` to a strongly-typed `BoxType` enum.

**Architecture:** `BoxType` enum in `src/config.rs` (`Border`, `None`, `Rounded`, `Block`). Simplify border rendering match in `src/ui.rs`. Remove `CharsConfig` struct and field.

**Tech Stack:** Rust 2024, serde, ratatui.

## Global Constraints
- Target files: `src/config.rs`, `src/ui.rs`, `themes/default.toml`.
- Enum variants for `BoxType`: `Border`, `None`, `Rounded`, `Block` with serde `rename_all = "lowercase"`.
- Must preserve 100% test pass rate.

---

### Task 1: `BoxType` Enum & `CharsConfig` Removal in `src/config.rs`

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs`

**Interfaces:**
- Produces: `pub enum BoxType`, updated `Behavior` struct, removed `CharsConfig`

- [ ] **Step 1: Define `BoxType` enum and update `Behavior` struct in `src/config.rs`**

In `src/config.rs`:
Add `BoxType` enum:
```rust
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BoxType {
    #[default]
    #[serde(alias = "plain", alias = "default")]
    Border,
    None,
    Rounded,
    Block,
}
```

In `Behavior` struct, change `pub box_type: String` to `pub box_type: BoxType`, and set default `box_type: BoxType::Border`.

Delete `pub struct CharsConfig` and `impl Default for CharsConfig`. Remove `pub chars: CharsConfig` from `Config`.

- [ ] **Step 2: Update unit tests in `src/config.rs`**

Update `test_partial_toml_merges_with_defaults` and `test_config_arbitrary_env_override_strings` to test `BoxType` and remove `chars` tests.

- [ ] **Step 3: Run `cargo check` to verify `src/config.rs` changes**

Run: `cargo check`

- [ ] **Step 4: Commit changes to `src/config.rs`**

```bash
git add src/config.rs
git commit -m "refactor(config): introduce BoxType enum and remove CharsConfig"
```

---

### Task 2: Simplify Border Rendering in `src/ui.rs` & Update `themes/default.toml`

**Files:**
- Modify: `src/ui.rs`
- Modify: `themes/default.toml`

**Interfaces:**
- Consumes: `BoxType` enum

- [ ] **Step 1: Update border rendering in `src/ui.rs`**

In `src/ui.rs`:
Import `BoxType` from `crate::config::BoxType`.

Replace border set rendering block (lines 237-260) with:
```rust
        let border = match cfg.behavior.box_type {
            BoxType::None => Block::default().borders(Borders::NONE),
            BoxType::Block => Block::default()
                .borders(Borders::ALL)
                .border_set(ratatui_core::symbols::border::FULL),
            BoxType::Rounded => Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
            BoxType::Border => Block::default().borders(Borders::ALL),
        }
        .style(Style::from(cfg.colors.e_box.clone()));
```

Update `test_custom_border_rendering` to test `BoxType::Border`, `BoxType::Rounded`, etc.

- [ ] **Step 2: Remove `[chars]` section from `themes/default.toml`**

Remove `[chars]` table from `themes/default.toml`.

- [ ] **Step 3: Run `cargo test` to verify all tests pass**

Run: `cargo test`
Expected: PASS (all tests pass, `test_sync_default_config_toml` updates `default.toml` if needed)

- [ ] **Step 4: Commit changes**

```bash
git add src/ui.rs themes/default.toml
git commit -m "refactor(ui): use BoxType enum for border rendering and remove chars section from theme"
```
