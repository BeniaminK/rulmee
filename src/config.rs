use crate::colors::Colors;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct CharsConfig {
    pub hb: String,
    pub vb: String,
    pub ctl: String,
    pub ctr: String,
    pub cbl: String,
    pub cbr: String,
}

impl Default for CharsConfig {
    fn default() -> Self {
        Self {
            hb: "─".to_string(),
            vb: "│".to_string(),
            ctl: "┌".to_string(),
            ctr: "┐".to_string(),
            cbl: "└".to_string(),
            cbr: "┘".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    pub show_theme: bool,
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
            show_theme: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingConfig {
    pub file: String,
    pub level: String,
    pub stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            file: "/tmp/lidm.log".to_string(),
            level: "debug".to_string(),
            stdout: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct AuthConfig {
    pub pam_service: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            pam_service: "login".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub colors: Colors,
    pub chars: CharsConfig,
    pub functions: Functions,
    pub strings: Strings,
    pub behavior: Behavior,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
}

impl Config {
    fn parse<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {

        let content = std::fs::read_to_string(path)?;
        let deserializer = toml::Deserializer::parse(&content)?;
        let parsed_config: Config = serde_ignored::deserialize(deserializer, |path| {
            log::warn!("Unknown configuration field ignored: {}", path);
        })?; 
        *self = parsed_config;
        Ok(())
    }

    /// Scan environment variables matching `LIDM_<SECTION>_<KEY>` and apply them
    /// as overrides onto the current configuration. The naming convention is
    /// automatic: `LIDM_STRINGS_F_POWEROFF=dsds` maps to `[strings] f_poweroff`.
    ///
    /// Values are auto-typed: `true`/`false` → bool, valid integers → integer,
    /// everything else → string.
    pub fn apply_env_overrides(&mut self) {
        let mut env_table = toml::Table::new();

        for (key, val) in std::env::vars() {
            if val.is_empty() {
                continue;
            }

            let rest = match key.strip_prefix("LIDM_") {
                Some(r) => r,
                None => continue,
            };

            let (section_str, item_str) = match rest.split_once('_') {
                Some(pair) => pair,
                None => continue,
            };

            let section = section_str.to_lowercase();
            let item = item_str.to_lowercase();

            // Skip CLAP-owned top-level args (handled by clap, not config sections)
            if section == "conf" || section == "vt" {
                continue;
            }

            let toml_val = if val.eq_ignore_ascii_case("true") {
                toml::Value::Boolean(true)
            } else if val.eq_ignore_ascii_case("false") {
                toml::Value::Boolean(false)
            } else if let Ok(i) = val.parse::<i64>() {
                toml::Value::Integer(i)
            } else {
                toml::Value::String(val)
            };

            let sec_entry = env_table
                .entry(section)
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let toml::Value::Table(table) = sec_entry {
                table.insert(item, toml_val);
            }
        }

        if env_table.is_empty() {
            return;
        }

        if let Ok(mut current_val) = toml::Value::try_from(&*self) {
            merge_toml_values(&mut current_val, toml::Value::Table(env_table));
            if let Ok(updated) = current_val.try_into::<Config>() {
                *self = updated;
            }
        }
    }

    pub fn load(args: &crate::Args) -> (Self, Option<String>) {
        let mut config = Config::default();
        let mut err_msg = None;

        let conf_path = &args.conf_path;
        if Path::new(conf_path).exists() {
            if let Err(e) = config.parse(conf_path) {
                let msg = format!(
                    "Failed to parse config from '{}': {}. Falling back to default configuration.",
                    conf_path, e
                );
                eprintln!("{}", msg);
                err_msg = Some(msg);
                config = Config::default();
            }
        }

        config.apply_env_overrides();
        config.apply_cli_overrides(args);

        (config, err_msg)
    }

    pub fn apply_cli_overrides(&mut self, args: &crate::Args) {
        if let Some(ref file) = args.logging_file {
            if !file.is_empty() {
                self.logging.file = file.clone();
            }
        }

        if let Some(ref level) = args.logging_level {
            if !level.is_empty() {
                self.logging.level = level.clone();
            }
        }
    }

    pub fn generate_default_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

fn merge_toml_values(dest: &mut toml::Value, source: toml::Value) {
    match (dest, source) {
        (toml::Value::Table(dest_map), toml::Value::Table(source_map)) => {
            for (key, val) in source_map {
                merge_toml_values(
                    dest_map
                        .entry(key)
                        .or_insert_with(|| toml::Value::Table(toml::Table::new())),
                    val,
                );
            }
        }
        (dest, source) => *dest = source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_sync_default_config_toml() {
        let default_toml = Config::generate_default_toml();
        let target_path = Path::new("themes/default.toml");
        let header = "# Default Configuration for LiDM (Lightweight Display Manager)\n# All settings shown below with their default values.\n\n";
        let full_content = format!("{}{}", header, default_toml);

        let needs_write = if target_path.exists() {
            std::fs::read_to_string(target_path).unwrap_or_default() != full_content
        } else {
            true
        };

        if needs_write {
            std::fs::write(target_path, &full_content).unwrap();
        }
    }

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
        assert_eq!(config.chars.hb, "=");
        assert_eq!(config.chars.vb, "│");
        assert_eq!(config.chars.ctl, "┌");
        assert_eq!(config.chars.ctr, "┐");
        assert_eq!(config.chars.cbl, "└");
        assert_eq!(config.chars.cbr, "┘");

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
        assert_eq!(
            config.behavior.refresh_rate,
            default_config.behavior.refresh_rate
        ); // 100
    }

    #[test]
    fn test_fido_config_defaults_and_parsing() {
        let default_config = Config::default();
        assert_eq!(default_config.functions.fido, None);
        assert_eq!(default_config.strings.f_fido, Some("fido".to_string()));

        let toml_str = r##"
[functions]
fido = "F3"

[strings]
f_fido = "yubikey"
"##;
        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.functions.fido.as_deref(), Some("F3"));
        assert_eq!(config.strings.f_fido.as_deref(), Some("yubikey"));
    }

    #[test]
    fn test_theme_config_defaults() {
        let config = Config::default();
        assert_eq!(config.functions.theme.as_deref(), Some("F3"));
        assert_eq!(config.strings.f_theme.as_deref(), Some("theme"));
    }

    #[test]
    fn test_logging_and_auth_config_defaults() {
        let config = Config::default();
        assert_eq!(config.logging.file, "/tmp/lidm.log");
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.stdout, false);
        assert_eq!(config.auth.pam_service, "login");
    }

    #[test]
    fn test_config_automatic_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("LIDM_LOGGING_LEVEL", "warn");
            std::env::set_var("LIDM_LOGGING_STDOUT", "true");
            std::env::set_var("LIDM_AUTH_PAM_SERVICE", "custom-pam");
            std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "250");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.logging.level, "warn");
        assert_eq!(config.logging.stdout, true);
        assert_eq!(config.auth.pam_service, "custom-pam");
        assert_eq!(config.behavior.refresh_rate, 250);

        unsafe {
            std::env::remove_var("LIDM_LOGGING_LEVEL");
            std::env::remove_var("LIDM_LOGGING_STDOUT");
            std::env::remove_var("LIDM_AUTH_PAM_SERVICE");
            std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
        }
    }

    #[test]
    fn test_config_load_precedence() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_lidm_precedence.toml");
        let toml_content = r#"
[logging]
level = "info"
file = "/tmp/toml.log"

[auth]
pam_service = "toml-pam"

[behavior]
refresh_rate = 150
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        unsafe {
            std::env::set_var("LIDM_LOGGING_LEVEL", "warn");
            std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "300");
        }

        let args = crate::Args {
            vt: None,
            logging_file: Some("/tmp/cli.log".to_string()),
            logging_level: Some("error".to_string()),
            conf_path: config_path.to_str().unwrap().to_string(),
        };

        let (config, err_opt) = Config::load(&args);
        assert!(err_opt.is_none());

        // CLI overrides TOML
        assert_eq!(config.logging.file, "/tmp/cli.log");

        // CLI overrides Env and TOML
        assert_eq!(config.logging.level, "error");

        // TOML preserved when no Env or CLI set
        assert_eq!(config.auth.pam_service, "toml-pam");

        // Env overrides TOML
        assert_eq!(config.behavior.refresh_rate, 300);

        unsafe {
            std::env::remove_var("LIDM_LOGGING_LEVEL");
            std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
        }
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_config_load_broken_toml_fallback_to_default() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_lidm_broken.toml");
        std::fs::write(&config_path, "invalid toml [[ [ {{ content").unwrap();

        let args = crate::Args {
            vt: None,
            logging_file: None,
            logging_level: None,
            conf_path: config_path.to_str().unwrap().to_string(),
        };

        let (config, err) = Config::load(&args);
        assert!(err.is_some());
        assert_eq!(config.logging.file, "/tmp/lidm.log");

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_config_arbitrary_env_override_strings() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("LIDM_STRINGS_F_POWEROFF", "dsds");
            std::env::set_var("LIDM_CHARS_HB", "==");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.strings.f_poweroff, "dsds");
        assert_eq!(config.chars.hb, "==");

        unsafe {
            std::env::remove_var("LIDM_STRINGS_F_POWEROFF");
            std::env::remove_var("LIDM_CHARS_HB");
        }
    }
}
