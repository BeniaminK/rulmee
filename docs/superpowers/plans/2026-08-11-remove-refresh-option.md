# Remove Refresh Option Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Completely remove the `refresh` option (F5 reload) from the configuration, UI hotkey bar, event handler, and main loop.

**Architecture:** Remove `refresh` and `f_refresh` fields from `Functions` and `Strings` structs in `src/config.rs`. Remove `HotkeyAction::Refresh` and `UIResult::Refresh` variants from `src/ui_adapter.rs`, `src/ui.rs`, and `src/main.rs`. Clean up `themes/default.toml` and `include/config.h`.

**Tech Stack:** Rust (edition 2021), TOML configuration, C header `config.h`.

## Global Constraints
- Preserve `config.behavior.refresh_rate` (UI polling loop interval).
- Codebase must compile cleanly with `cargo check`.
- All tests must pass with `cargo test`.

---

### Task 1: Remove `refresh` and `f_refresh` from configuration structs and defaults

**Files:**
- Modify: `src/config.rs`
- Modify: `themes/default.toml`
- Modify: `include/config.h`

**Interfaces:**
- Removes: `Functions.refresh`, `Strings.f_refresh`

- [ ] **Step 1: Remove `refresh` and `f_refresh` fields from `src/config.rs`**

Remove `pub refresh: Option<String>` from `Functions` and `pub f_refresh: String` from `Strings` in `src/config.rs`, along with their default values in `impl Default`.

- [ ] **Step 2: Update `themes/default.toml` and `include/config.h`**

Remove `refresh = "F5"` and `f_refresh = "refresh"` from `themes/default.toml` and `include/config.h`.

- [ ] **Step 3: Commit Task 1**

```bash
git add src/config.rs themes/default.toml include/config.h
git commit -m "refactor(config): remove refresh and f_refresh configuration fields"
```

---

### Task 2: Remove `Refresh` actions from UI adapter, UI, and main loop

**Files:**
- Modify: `src/ui_adapter.rs`
- Modify: `src/ui.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Removes: `HotkeyAction::Refresh`, `UIResult::Refresh`

- [ ] **Step 1: Remove `HotkeyAction::Refresh` and `UIResult::Refresh`**

Remove `Refresh` from `enum HotkeyAction` in `src/ui_adapter.rs` and `enum UIResult` in `src/ui.rs`. Remove `HotkeyAction::Refresh => Some(UIResult::Refresh)` in `src/ui.rs` and `is(&self.config.functions.refresh)` in `src/ui_adapter.rs`.

- [ ] **Step 2: Update hotkey rendering and main loop**

Remove `(&cfg.functions.refresh, &cfg.strings.f_refresh)` tuple from `src/ui.rs`. Remove `Ok(UIResult::Refresh) => continue,` from `src/main.rs`.

- [ ] **Step 3: Run `cargo check` and `cargo test`**

Run: `cargo check && cargo test`
Expected: PASS with 0 errors/warnings

- [ ] **Step 4: Commit Task 2**

```bash
git add src/ui_adapter.rs src/ui.rs src/main.rs
git commit -m "refactor(ui): remove refresh hotkey action and main loop match arm"
```
