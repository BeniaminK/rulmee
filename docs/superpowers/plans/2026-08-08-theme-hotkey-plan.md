# Theme Iteration Hotkey Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Theme Iteration Hotkey (default `F3`) to cycle through available themes live in memory and re-render the TUI immediately.

**Architecture:** A new module `src/theme.rs` discovers `.toml` and `.ini` theme files across standard paths (`/etc/lidm/themes/`, `/usr/share/lidm/themes/`, `./themes/`). `UIAdapter` and `UIState` manage theme state and advance theme selection on `HotkeyAction::Theme`, updating `config.colors` live.

**Tech Stack:** Rust 2021 edition, `ratatui`, `crossterm`, `serde`, `toml`.

## Global Constraints

- Must follow standard Rust idioms and pass `cargo check` and `cargo test` with 0 warnings and 0 errors.
- Default keybinding is `F3` (`functions.theme = "F3"`), default string label is `"theme"` (`strings.f_theme = "theme"`).
- Theme discovery scans `/etc/lidm/themes/`, `/usr/share/lidm/themes/`, and `./themes/` in order.
- Malformed theme files are skipped gracefully with a log warning.

---

### Task 1: Theme Discovery Module (`src/theme.rs`)

**Files:**
- Create: `src/theme.rs`
- Modify: `src/main.rs` (to register `mod theme;`)
- Test: `src/theme.rs`

**Interfaces:**
- Consumes: `Colors` struct from `src/colors.rs`
- Produces: `Theme` struct and `discover_themes(base_colors: &Colors) -> Vec<Theme>`

- [ ] **Step 1: Write the failing tests in `src/theme.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Colors;

    #[test]
    fn test_discover_themes_fallback() {
        let base_colors: Colors = toml::from_str("").unwrap();
        let themes = discover_themes(&base_colors);
        assert!(!themes.is_empty());
        assert_eq!(themes[0].name, "default");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test theme::tests::test_discover_themes_fallback`
Expected: FAIL (module `theme` does not exist)

- [ ] **Step 3: Write minimal implementation in `src/theme.rs` and register in `src/main.rs`**

In `src/main.rs`:
```rust
mod theme;
```

In `src/theme.rs`:
```rust
use crate::colors::Colors;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
}

pub fn discover_themes(base_colors: &Colors) -> Vec<Theme> {
    let mut themes = Vec::new();
    themes.push(Theme {
        name: "default".to_string(),
        colors: base_colors.clone(),
    });

    let search_paths = [
        Path::new("/etc/lidm/themes"),
        Path::new("/usr/share/lidm/themes"),
        Path::new("./themes"),
    ];

    for path in search_paths {
        if !path.exists() || !path.is_dir() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut file_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            file_entries.sort_by_key(|e| e.path());

            for entry in file_entries {
                let p = entry.path();
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext != "toml" && ext != "ini" {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(colors) = toml::from_str::<Colors>(&content) {
                        let name = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if !themes.iter().any(|t| t.name == name) {
                            themes.push(Theme { name, colors });
                        }
                    } else {
                        log::warn!("Failed to parse theme file: {}", p.display());
                    }
                }
            }
        }
    }

    themes
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test theme::tests::test_discover_themes_fallback`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/theme.rs src/main.rs
git commit -m "feat: add theme discovery module"
```

---

### Task 2: Config Extensions (`src/config.rs`)

**Files:**
- Modify: `src/config.rs:30-85`
- Test: `src/config.rs`

**Interfaces:**
- Consumes: TOML configuration
- Produces: `Functions::theme: Option<String>` and `Strings::f_theme: Option<String>`

- [ ] **Step 1: Write failing unit test in `src/config.rs`**

```rust
#[test]
fn test_theme_config_defaults() {
    let config = Config::default();
    assert_eq!(config.functions.theme.as_deref(), Some("F3"));
    assert_eq!(config.strings.f_theme.as_deref(), Some("theme"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests::test_theme_config_defaults`
Expected: FAIL (`theme` and `f_theme` fields missing)

- [ ] **Step 3: Update `Functions` and `Strings` structs in `src/config.rs`**

In `Functions`:
```rust
pub struct Functions {
    pub poweroff: Option<String>,
    pub reboot: Option<String>,
    pub refresh: Option<String>,
    pub fido: Option<String>,
    pub theme: Option<String>,
}

impl Default for Functions {
    fn default() -> Self {
        Self {
            poweroff: Some("F1".to_string()),
            reboot: Some("F2".to_string()),
            refresh: Some("F5".to_string()),
            fido: None,
            theme: Some("F3".to_string()),
        }
    }
}
```

In `Strings`:
```rust
pub struct Strings {
    pub f_poweroff: String,
    pub f_reboot: String,
    pub f_refresh: String,
    pub f_fido: Option<String>,
    pub f_theme: Option<String>,
    pub e_user: String,
    pub e_passwd: String,
    pub s_wayland: String,
    pub s_xorg: String,
    pub s_shell: String,
    pub opts_pre: String,
    pub opts_post: String,
    pub ellipsis: String,
}

impl Default for Strings {
    fn default() -> Self {
        Self {
            f_poweroff: "poweroff".to_string(),
            f_reboot: "reboot".to_string(),
            f_refresh: "refresh".to_string(),
            f_fido: Some("fido".to_string()),
            f_theme: Some("theme".to_string()),
            e_user: "user".to_string(),
            e_passwd: "password".to_string(),
            s_wayland: "wayland".to_string(),
            s_xorg: "xorg".to_string(),
            s_shell: "shell".to_string(),
            opts_pre: "< ".to_string(),
            opts_post: " >".to_string(),
            ellipsis: "…".to_string(),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test config::tests::test_theme_config_defaults`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add theme hotkey configuration fields"
```

---

### Task 3: Hotkey Detection & UI State Management (`src/ui_adapter.rs` & `src/ui_state.rs`)

**Files:**
- Modify: `src/ui_state.rs`
- Modify: `src/ui_adapter.rs`
- Test: `src/ui_adapter.rs`

**Interfaces:**
- Consumes: `Theme` from `src/theme.rs`, `Functions::theme` from `src/config.rs`
- Produces: `HotkeyAction::Theme`, `UIAdapter::cycle_theme(&mut self)`

- [ ] **Step 1: Write failing unit test in `src/ui_adapter.rs`**

```rust
#[test]
fn test_theme_hotkey_detection_and_cycling() {
    let config = Config::default();
    let adapter = UIAdapter::new(
        config,
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
        Vec::new(),
        false,
    );

    let key_event = KeyEvent::new(KeyCode::F(3), crossterm::event::KeyModifiers::NONE);
    assert_eq!(adapter.check_hotkey(key_event), Some(HotkeyAction::Theme));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui_adapter::tests::test_theme_hotkey_detection_and_cycling`
Expected: FAIL (`HotkeyAction::Theme` variant missing)

- [ ] **Step 3: Implement `HotkeyAction::Theme` and theme cycling state**

In `src/ui_adapter.rs`:
Add `Theme` to `HotkeyAction` enum:
```rust
pub enum HotkeyAction {
    Poweroff,
    Reboot,
    Refresh,
    Fido,
    Theme,
}
```

Update `check_hotkey` in `src/ui_adapter.rs`:
```rust
if match_key(&self.config.functions.theme) {
    return Some(HotkeyAction::Theme);
}
```

In `src/ui_state.rs`:
Add `pub themes: Vec<Theme>` and `pub current_theme_idx: usize` to `UIState`:
```rust
use crate::theme::Theme;

pub struct UIState {
    // ...
    pub themes: Vec<Theme>,
    pub current_theme_idx: usize,
}
```

In `src/ui_adapter.rs`:
In `UIAdapter::new`, populate `themes` using `crate::theme::discover_themes(&config.colors)`:
```rust
let themes = crate::theme::discover_themes(&config.colors);
```

Add `pub fn cycle_theme(&mut self)` to `UIAdapter`:
```rust
pub fn cycle_theme(&mut self) {
    if self.state.themes.is_empty() {
        return;
    }
    self.state.current_theme_idx = (self.state.current_theme_idx + 1) % self.state.themes.len();
    let new_colors = self.state.themes[self.state.current_theme_idx].colors.clone();
    self.config.colors = new_colors;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui_adapter::tests::test_theme_hotkey_detection_and_cycling`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui_state.rs src/ui_adapter.rs
git commit -m "feat: implement HotkeyAction::Theme and theme cycling state"
```

---

### Task 4: UI Event Loop & Footer Rendering (`src/ui.rs`)

**Files:**
- Modify: `src/ui.rs:88-300`
- Test: `src/ui.rs`

**Interfaces:**
- Consumes: `HotkeyAction::Theme`, `UIAdapter::cycle_theme()`
- Produces: Live theme re-rendering and F3 hotkey footer hint

- [ ] **Step 1: Write failing unit test in `src/ui.rs`**

```rust
#[test]
fn test_theme_hotkey_footer_hint() {
    let config = Config::default();
    let adapter = UIAdapter::new(
        config,
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
        Vec::new(),
        false,
    );
    let ui = UI::new(adapter);
    let buffer = ui.render_to_buffer(80, 24);
    assert!(buffer.contains_string("F3 theme"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui::tests::test_theme_hotkey_footer_hint`
Expected: FAIL ("F3 theme" hint not in footer)

- [ ] **Step 3: Update `src/ui.rs` event handling and footer rendering**

In `UI::handle_key_event`:
```rust
Some(HotkeyAction::Theme) => {
    self.adapter.cycle_theme();
    return Ok(None);
}
```

In `UI::render` (footer hints rendering block):
Add footer shortcut item for theme:
```rust
if let (Some(k), Some(l)) = (&cfg.functions.theme, &cfg.strings.f_theme) {
    if !k.is_empty() {
        hints.push(Span::styled(format!("{} ", k), style_key));
        hints.push(Span::styled(format!("{} ", l), style_label));
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui::tests::test_theme_hotkey_footer_hint`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs
git commit -m "feat: add theme hotkey handler and footer hint"
```
