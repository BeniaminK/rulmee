# Design Specification: Theme Iteration Hotkey

## Overview
This document specifies the technical design for adding a Theme Iteration Hotkey to **LiDM** (Lightweight Display Manager in Rust). Pressing the designated hotkey (`F3` by default) cycles through available themes live in memory, updating the UI styling immediately without restarting the application.

---

## 1. Requirements & User Stories
- **Hotkey Trigger**: Pressing `F3` (or key configured in `functions.theme`) cycles to the next theme.
- **Configurability**:
  - Hotkey can be configured or disabled via `[functions]` (`theme = "F3"` or `theme = ""`).
  - Shortcut label in UI footer configurable via `[strings]` (`f_theme = "theme"`).
- **Automatic Theme Discovery**:
  - Automatically scans `/etc/lidm/themes/`, `/usr/share/lidm/themes/`, and `./themes/` for `.toml` and `.ini` theme files.
  - Gracefully falls back to default theme if no external theme files exist.
- **Live Updating**:
  - Immediately updates `config.colors` in memory and re-renders the TUI upon hotkey press.

---

## 2. Architecture & Component Changes

### 2.1 Configuration Extensions (`src/config.rs`)
1. **`Functions` struct**:
   - Add `pub theme: Option<String>` with default value `Some("F3".to_string())`.
2. **`Strings` struct**:
   - Add `pub f_theme: Option<String>` with default value `Some("theme".to_string())`.

### 2.2 Theme Discovery Module (`src/theme.rs`)
Create a new module `src/theme.rs` responsible for discovering and loading themes:
- **`Theme` Struct**:
  ```rust
  pub struct Theme {
      pub name: String,
      pub colors: Colors,
  }
  ```
- **`discover_themes` Function**:
  - Checks `/etc/lidm/themes/`, `/usr/share/lidm/themes/`, and `./themes/`.
  - Iterates over files ending with `.toml` or `.ini`.
  - Deserializes theme color configurations into `Colors`.
  - Returns `Vec<Theme>` sorted by theme name.
  - Ensures default config colors are included as the base entry.

### 2.3 Hotkey Detection & State Management (`src/ui_adapter.rs` & `src/ui_state.rs`)
1. **`HotkeyAction` Enum**:
   - Add `HotkeyAction::Theme` variant.
2. **`UIAdapter::check_hotkey`**:
   - Checks if key event matches `config.functions.theme`. Returns `HotkeyAction::Theme`.
3. **`UIState` Modifications**:
   - Add `pub themes: Vec<Theme>` and `pub current_theme_idx: usize`.
   - Add `pub fn cycle_theme(&mut self) -> Option<&Colors>`.
4. **`UIAdapter::cycle_theme`**:
   - Advances `current_theme_idx` to `(current_theme_idx + 1) % themes.len()`.
   - Updates `self.config.colors` with the newly active theme colors.

### 2.4 Event Handling & Live Re-render (`src/ui.rs`)
- In `UI::handle_key_event`:
  - When `HotkeyAction::Theme` is returned:
    - Calls `self.adapter.cycle_theme()`.
    - Triggers immediate terminal draw with the updated theme colors.
- Footer rendering dynamically displays `F3 theme` alongside existing hotkey hints.

---

## 3. Error Handling & Edge Cases
- **No theme files found**: Falling back to single default theme entry; pressing `F3` performs a safe no-op.
- **Malformed theme file**: Logs warning and skips malformed file without crashing.
- **Unconfigured hotkey (`theme = None` or `""`)**: F3 detection and footer hint are hidden.

---

## 4. Testing & Verification Plan
1. **Unit Tests**:
   - `test_theme_discovery_fallback`: Verify default theme presence when directory is missing or empty.
   - `test_theme_cycling`: Verify `cycle_theme` correctly wraps around `themes.len()`.
   - `test_theme_hotkey_detection`: Verify key press matching `functions.theme` triggers `HotkeyAction::Theme`.
2. **Integration Verification**:
   - Run `cargo check` and `cargo test`.
