# Remove Refresh Option Design

## Objective
Remove the `refresh` hotkey option (F5 reload) completely from the `lidm` codebase. The UI refresh action is not useful for end users at the login screen and adds visual clutter to the hotkey bar.

## Changes Overview

### 1. `src/config.rs` & `themes/default.toml`
- Remove `pub refresh: Option<String>` from `struct Functions`.
- Remove `pub f_refresh: String` from `struct Strings`.
- Update `Functions::default()` to omit `refresh`.
- Update `Strings::default()` to omit `f_refresh`.
- Remove `refresh = "F5"` and `f_refresh = "refresh"` from `themes/default.toml`.
- Note: Keep `behavior.refresh_rate` (UI polling loop timer) untouched.

### 2. `src/ui.rs` & `src/ui_adapter.rs`
- Remove `HotkeyAction::Refresh` variant from `src/ui_adapter.rs`.
- Remove `UIResult::Refresh` variant from `src/ui.rs`.
- Remove `HotkeyAction::Refresh => Some(UIResult::Refresh)` matching in `src/ui.rs`.
- Remove `if is(&self.config.functions.refresh)` matching in `check_hotkey()` in `src/ui_adapter.rs`.
- Remove `(&cfg.functions.refresh, &cfg.strings.f_refresh)` tuple from hotkey rendering list in `src/ui.rs`.

### 3. `src/main.rs`
- Remove `Ok(UIResult::Refresh) => continue,` match arm from the `main` loop.

### 4. C Header / Config Files (if applicable)
- Remove `refresh` and `f_refresh` from `include/config.h`.

## Verification
- Run `cargo check` and `cargo test` to confirm compilation and zero test breakages.
- Verify hotkey bar no longer displays `F5 refresh`.
