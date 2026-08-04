use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeStyle {
    pub color: Option<String>,
    pub bg: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

impl ThemeStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.color = Some(color.to_string());
        self
    }

    pub fn with_bg(mut self, bg: &str) -> Self {
        self.bg = Some(bg.to_string());
        self
    }

    pub fn with_modifiers(mut self, modifiers: &[&str]) -> Self {
        self.modifiers = modifiers.iter().map(|&s| s.to_string()).collect();
        self
    }
}

static MODIFIER_MAP: LazyLock<HashMap<&'static str, Modifier>> = LazyLock::new(|| {
    HashMap::from([
        ("bold", Modifier::BOLD),
        ("dim", Modifier::DIM),
        ("italic", Modifier::ITALIC),
        ("underline", Modifier::UNDERLINED),
        ("underlined", Modifier::UNDERLINED),
        ("slow_blink", Modifier::SLOW_BLINK),
        ("rapid_blink", Modifier::RAPID_BLINK),
        ("reversed", Modifier::REVERSED),
        ("reverse", Modifier::REVERSED),
        ("hidden", Modifier::HIDDEN),
        ("crossed_out", Modifier::CROSSED_OUT),
        ("strikethrough", Modifier::CROSSED_OUT),
    ])
});

fn parse_modifier(m: &str) -> Option<Modifier> {
    MODIFIER_MAP.get(m.to_lowercase().as_str()).copied()
}

impl From<ThemeStyle> for Style {
    fn from(s: ThemeStyle) -> Self {
        let mut style = Style::default();

        if let Some(c) = s.color {
            match Color::from_str(&c) {
                Ok(color) => style = style.fg(color),
                Err(_) => log::warn!("Unrecognized color format: {}", c),
            }
        }

        if let Some(bg) = s.bg {
            match Color::from_str(&bg) {
                Ok(color) => style = style.bg(color),
                Err(_) => log::warn!("Unrecognized color format: {}", bg),
            }
        }

        for m in s.modifiers {
            match parse_modifier(&m) {
                Some(modifier) => style = style.add_modifier(modifier),
                None => log::warn!("Unrecognized modifier: {}", m),
            }
        }

        style
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Colors {
    pub bg: ThemeStyle,
    pub fg: ThemeStyle,
    pub err: ThemeStyle,
    pub warn: ThemeStyle,
    pub s_wayland: ThemeStyle,
    pub s_xorg: ThemeStyle,
    pub s_shell: ThemeStyle,
    pub e_hostname: ThemeStyle,
    pub e_date: ThemeStyle,
    pub e_box: ThemeStyle,
    pub e_header: ThemeStyle,
    pub e_user: ThemeStyle,
    pub e_passwd: ThemeStyle,
    pub e_badpasswd: ThemeStyle,
    pub e_key: ThemeStyle,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            bg: ThemeStyle::new().with_bg("#261c1c"),
            fg: ThemeStyle::new().with_color("#f5f5f5"),
            err: ThemeStyle::new()
                .with_color("red")
                .with_modifiers(&["bold"]),
            warn: ThemeStyle::new().with_color("yellow"),
            s_wayland: ThemeStyle::new().with_color("#ffae42"),
            s_xorg: ThemeStyle::new().with_color("#25afff"),
            s_shell: ThemeStyle::new().with_color("green"),
            e_hostname: ThemeStyle::new().with_color("lightred"),
            e_date: ThemeStyle::new().with_color("darkgray"),
            e_box: ThemeStyle::new().with_color("gray"),
            e_header: ThemeStyle::new()
                .with_color("green")
                .with_modifiers(&["underlined"]),
            e_user: ThemeStyle::new().with_color("cyan"),
            e_passwd: ThemeStyle::new()
                .with_color("#f5f5cd")
                .with_modifiers(&["underlined"]),
            e_badpasswd: ThemeStyle::new()
                .with_color("red")
                .with_modifiers(&["italic", "underlined"]),
            e_key: ThemeStyle::new().with_color("#ffae42"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_exhaustiveness() {
        let mut mapped_modifiers = Modifier::empty();
        for &modifier in MODIFIER_MAP.values() {
            mapped_modifiers.insert(modifier);
        }

        // Assert that our supported strings generate every single bitflag modifier known to ratatui
        assert_eq!(
            mapped_modifiers,
            Modifier::all(),
            "Ratatui has added new modifiers that are not supported in MODIFIER_MAP!"
        );
    }
}
