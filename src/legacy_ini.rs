use crate::colors::{Colors, ThemeStyle};
use std::path::Path;

pub const LEGACY_THEME_SEARCH_PATHS: &[&str] = &["/etc/lidm/themes", "/usr/share/lidm/themes"];

/// Parses a legacy ANSI SGR escape code string (e.g. `"1;4;38;2;255;174;66"`) into a `ThemeStyle`.
pub fn parse_ansi_style(sgr: &str) -> ThemeStyle {
    let sgr = sgr.trim().trim_matches('"').trim_matches('\'');
    let tokens: Vec<&str> = sgr
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut style = ThemeStyle::default();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            "1" => {
                if !style.modifiers.iter().any(|m| m == "bold") {
                    style.modifiers.push("bold".to_string());
                }
                i += 1;
            }
            "2" => {
                if !style.modifiers.iter().any(|m| m == "dim") {
                    style.modifiers.push("dim".to_string());
                }
                i += 1;
            }
            "3" => {
                if !style.modifiers.iter().any(|m| m == "italic") {
                    style.modifiers.push("italic".to_string());
                }
                i += 1;
            }
            "4" => {
                if !style.modifiers.iter().any(|m| m == "underlined") {
                    style.modifiers.push("underlined".to_string());
                }
                i += 1;
            }
            "5" => {
                if !style.modifiers.iter().any(|m| m == "slow_blink") {
                    style.modifiers.push("slow_blink".to_string());
                }
                i += 1;
            }
            "6" => {
                if !style.modifiers.iter().any(|m| m == "rapid_blink") {
                    style.modifiers.push("rapid_blink".to_string());
                }
                i += 1;
            }
            "7" => {
                if !style.modifiers.iter().any(|m| m == "reversed") {
                    style.modifiers.push("reversed".to_string());
                }
                i += 1;
            }
            "8" => {
                if !style.modifiers.iter().any(|m| m == "hidden") {
                    style.modifiers.push("hidden".to_string());
                }
                i += 1;
            }
            "9" => {
                if !style.modifiers.iter().any(|m| m == "crossed_out") {
                    style.modifiers.push("crossed_out".to_string());
                }
                i += 1;
            }
            "38" if i + 4 < tokens.len() && tokens[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    tokens[i + 2].parse::<u8>(),
                    tokens[i + 3].parse::<u8>(),
                    tokens[i + 4].parse::<u8>(),
                ) {
                    style.color = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
                }
                i += 5;
            }
            "48" if i + 4 < tokens.len() && tokens[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    tokens[i + 2].parse::<u8>(),
                    tokens[i + 3].parse::<u8>(),
                    tokens[i + 4].parse::<u8>(),
                ) {
                    style.bg = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
                }
                i += 5;
            }
            "38" if i + 2 < tokens.len() && tokens[i + 1] == "5" => {
                style.color = Some(tokens[i + 2].to_string());
                i += 3;
            }
            "48" if i + 2 < tokens.len() && tokens[i + 1] == "5" => {
                style.bg = Some(tokens[i + 2].to_string());
                i += 3;
            }
            "30" => {
                style.color = Some("black".to_string());
                i += 1;
            }
            "31" => {
                style.color = Some("red".to_string());
                i += 1;
            }
            "32" => {
                style.color = Some("green".to_string());
                i += 1;
            }
            "33" => {
                style.color = Some("yellow".to_string());
                i += 1;
            }
            "34" => {
                style.color = Some("blue".to_string());
                i += 1;
            }
            "35" => {
                style.color = Some("magenta".to_string());
                i += 1;
            }
            "36" => {
                style.color = Some("cyan".to_string());
                i += 1;
            }
            "37" => {
                style.color = Some("white".to_string());
                i += 1;
            }
            "40" => {
                style.bg = Some("black".to_string());
                i += 1;
            }
            "41" => {
                style.bg = Some("red".to_string());
                i += 1;
            }
            "42" => {
                style.bg = Some("green".to_string());
                i += 1;
            }
            "43" => {
                style.bg = Some("yellow".to_string());
                i += 1;
            }
            "44" => {
                style.bg = Some("blue".to_string());
                i += 1;
            }
            "45" => {
                style.bg = Some("magenta".to_string());
                i += 1;
            }
            "46" => {
                style.bg = Some("cyan".to_string());
                i += 1;
            }
            "47" => {
                style.bg = Some("white".to_string());
                i += 1;
            }
            "90" => {
                style.color = Some("darkgray".to_string());
                i += 1;
            }
            "91" => {
                style.color = Some("lightred".to_string());
                i += 1;
            }
            "92" => {
                style.color = Some("lightgreen".to_string());
                i += 1;
            }
            "93" => {
                style.color = Some("lightyellow".to_string());
                i += 1;
            }
            "94" => {
                style.color = Some("lightblue".to_string());
                i += 1;
            }
            "95" => {
                style.color = Some("lightmagenta".to_string());
                i += 1;
            }
            "96" => {
                style.color = Some("lightcyan".to_string());
                i += 1;
            }
            "97" => {
                style.color = Some("white".to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    style
}

/// Parses a legacy `.ini` theme file into a `Colors` structure, logging a deprecation warning.
pub fn load_legacy_ini_theme(path: &Path) -> Result<Colors, String> {
    log::warn!(
        "Theme '{}' uses deprecated INI format; please migrate to TOML",
        path.display()
    );

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

    let mut colors = Colors::default();
    let mut in_colors_section = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_colors_section = line.eq_ignore_ascii_case("[colors]");
            continue;
        }

        if in_colors_section && let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let style = parse_ansi_style(val);

            match key.as_str() {
                "bg" => colors.bg = style,
                "fg" => colors.fg = style,
                "err" => colors.err = style,
                "warn" => colors.warn = style,
                "s_wayland" => colors.s_wayland = style,
                "s_xorg" => colors.s_xorg = style,
                "s_shell" => colors.s_shell = style,
                "e_hostname" => colors.e_hostname = style,
                "e_date" => colors.e_date = style,
                "e_box" => colors.e_box = style,
                "e_header" => colors.e_header = style,
                "e_user" => colors.e_user = style,
                "e_passwd" => colors.e_passwd = style,
                "e_badpasswd" => colors.e_badpasswd = style,
                "e_key" => colors.e_key = style,
                _ => {}
            }
        }
    }

    Ok(colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ansi_style_rgb() {
        let style = parse_ansi_style("48;2;77;33;55");
        assert_eq!(style.bg.as_deref(), Some("#4d2137"));

        let style_fg = parse_ansi_style("22;3;24;38;2;245;245;245");
        assert_eq!(style_fg.color.as_deref(), Some("#f5f5f5"));
        assert!(style_fg.modifiers.iter().any(|m| m == "italic"));
    }

    #[test]
    fn test_parse_ansi_style_standard_colors_and_modifiers() {
        let style = parse_ansi_style("1;4;31");
        assert_eq!(style.color.as_deref(), Some("red"));
        assert!(style.modifiers.iter().any(|m| m == "bold"));
        assert!(style.modifiers.iter().any(|m| m == "underlined"));
    }

    #[test]
    fn test_load_legacy_ini_theme() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_legacy_theme.ini");
        let ini_content = r#"
[colors]
bg = "48;2;30;30;46"
fg = "38;2;205;214;244"
e_hostname = "1;31"
"#;
        std::fs::write(&path, ini_content).unwrap();

        let colors = load_legacy_ini_theme(&path).unwrap();
        assert_eq!(colors.bg.bg.as_deref(), Some("#1e1e2e"));
        assert_eq!(colors.fg.color.as_deref(), Some("#cdd6f4"));
        assert_eq!(colors.e_hostname.color.as_deref(), Some("red"));
        assert!(colors.e_hostname.modifiers.iter().any(|m| m == "bold"));

        let _ = std::fs::remove_file(path);
    }
}
