# `BoxType` Enum & `CharsConfig` Removal Design Spec

## Goal
Remove `CharsConfig` (`config.chars`) and the `[chars]` section, and replace `box_type` string parsing in `Behavior` with a strongly-typed `BoxType` enum.

## Design

### 1. Enum `BoxType` in `src/config.rs`

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

### 2. Update `Behavior` Struct in `src/config.rs`

Replace `pub box_type: String` with `pub box_type: BoxType`:

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Behavior {
    pub box_type: BoxType,
    pub include_defshell: bool,
    pub show_console: bool,
    pub source: Vec<String>,
    pub user_source: Vec<String>,
    pub timefmt: String,
    pub refresh_rate: u64,
    pub bypass_shell_login: bool,
    pub show_theme: bool,
}
```

### 3. Remove `CharsConfig`

- Delete `pub struct CharsConfig` and `pub chars: CharsConfig` from `Config` struct.
- Remove `[chars]` table from `themes/default.toml`.

### 4. Simplified Border Rendering in `src/ui.rs`

In `src/ui.rs`:
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

## Testing Plan
- Update unit tests in `src/config.rs` and `src/ui.rs` to use `BoxType`.
- Run `cargo test` to ensure 100% pass rate.
