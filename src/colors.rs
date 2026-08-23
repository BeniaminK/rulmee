use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ThemeStyle {
    pub color: Option<String>,
    pub bg: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

impl ThemeStyle {
    pub fn color(color: &str) -> Self {
        Self {
            color: Some(color.to_string()),
            bg: None,
            modifiers: Vec::new(),
        }
    }

    pub fn bg(bg: &str) -> Self {
        Self {
            color: None,
            bg: Some(bg.to_string()),
            modifiers: Vec::new(),
        }
    }

    pub fn styled(color: &str, modifiers: &[&str]) -> Self {
        Self {
            color: Some(color.to_string()),
            bg: None,
            modifiers: modifiers.iter().map(|&s| s.to_string()).collect(),
        }
    }
}

pub fn parse_modifier(m: &str) -> Option<Modifier> {
    match m.to_ascii_lowercase().as_str() {
        "bold" => Some(Modifier::BOLD),
        "dim" => Some(Modifier::DIM),
        "italic" => Some(Modifier::ITALIC),
        "underline" | "underlined" => Some(Modifier::UNDERLINED),
        "slow_blink" => Some(Modifier::SLOW_BLINK),
        "rapid_blink" => Some(Modifier::RAPID_BLINK),
        "reversed" | "reverse" => Some(Modifier::REVERSED),
        "hidden" => Some(Modifier::HIDDEN),
        "crossed_out" | "strikethrough" => Some(Modifier::CROSSED_OUT),
        _ => None,
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
            bg: ThemeStyle::bg("#261c1c"),
            fg: ThemeStyle::color("#f5f5f5"),
            err: ThemeStyle::styled("red", &["bold"]),
            warn: ThemeStyle::color("yellow"),
            s_wayland: ThemeStyle::color("#ffae42"),
            s_xorg: ThemeStyle::color("#25afff"),
            s_shell: ThemeStyle::color("green"),
            e_hostname: ThemeStyle::color("lightred"),
            e_date: ThemeStyle::color("darkgray"),
            e_box: ThemeStyle::color("gray"),
            e_header: ThemeStyle::styled("green", &["underlined"]),
            e_user: ThemeStyle::color("cyan"),
            e_passwd: ThemeStyle::styled("#f5f5cd", &["underlined"]),
            e_badpasswd: ThemeStyle::styled("red", &["italic", "underlined"]),
            e_key: ThemeStyle::color("#ffae42"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_exhaustiveness() {
        let modifier_names = [
            "bold", "dim", "italic", "underline", "slow_blink", "rapid_blink",
            "reversed", "hidden", "crossed_out",
        ];
        let mut mapped = Modifier::empty();
        for name in modifier_names {
            if let Some(m) = parse_modifier(name) {
                mapped.insert(m);
            }
        }
        assert_eq!(
            mapped,
            Modifier::all(),
            "All Ratatui modifier bitflags should be parseable!"
        );
    }
}
