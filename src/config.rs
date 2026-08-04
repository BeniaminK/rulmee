use crate::colors::Colors;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Functions {
    pub poweroff: Option<String>,
    pub reboot: Option<String>,
    pub refresh: Option<String>,
}

impl Default for Functions {
    fn default() -> Self {
        Self {
            poweroff: Some("F1".to_string()),
            reboot: Some("F2".to_string()),
            refresh: Some("F5".to_string()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Strings {
    pub f_poweroff: String,
    pub f_reboot: String,
    pub f_refresh: String,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Behavior {
    pub box_type: String,
    pub include_defshell: bool,
    pub show_console: bool,
    pub source: Vec<String>,
    pub user_source: Vec<String>,
    pub timefmt: String,
    pub refresh_rate: u64,
    pub bypass_shell_login: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            box_type: "border".to_string(),
            include_defshell: true,
            show_console: false,
            source: Vec::new(),
            user_source: Vec::new(),
            timefmt: "%Y-%m-%d %H:%M:%S".to_string(),
            refresh_rate: 100,
            bypass_shell_login: false,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub colors: Colors,
    pub functions: Functions,
    pub strings: Strings,
    pub behavior: Behavior,
}

impl Config {
    pub fn parse<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        if !path.as_ref().exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)?;
        let deserializer = toml::Deserializer::parse(&content)?;
        let parsed_config: Config = serde_ignored::deserialize(deserializer, |path| {
            log::warn!("Unknown configuration field ignored: {}", path);
        })?;
        *self = parsed_config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_toml_merges_with_defaults() {
        let toml_str = r##"
[colors]
fg = { color = "#FFFFFF" }

[chars]
hb = "="
"##;
        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML");

        // Explicitly defined fields in TOML
        assert_eq!(config.colors.fg.color.as_deref(), Some("#FFFFFF"));

        // Missing fields should perfectly retain their rich defaults from Config::default()
        let default_config = Config::default();

        // Colors that weren't defined at all in the subset
        assert_eq!(
            config.colors.bg.bg.as_deref(),
            default_config.colors.bg.bg.as_deref()
        ); // "#261c1c"
        assert_eq!(
            config.colors.err.color.as_deref(),
            default_config.colors.err.color.as_deref()
        ); // "red"

        // Other sections that were partially defined or entirely missing
        assert_eq!(config.chars.vb, default_config.chars.vb); // "│"
        assert_eq!(
            config.behavior.refresh_rate,
            default_config.behavior.refresh_rate
        ); // 100
    }
}
