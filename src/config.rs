use crate::colors::Colors;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, Deserialize)]
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

    pub fn apply_env_overrides(&mut self) {
        // Handle legacy alias mappings first
        if let Ok(val) = std::env::var("LIDM_LOG") {
            if !val.is_empty() { self.logging.file = val; }
        }
        if let Ok(val) = std::env::var("LIDM_LOGLEVEL") {
            if !val.is_empty() { self.logging.level = val; }
        }
        if let Ok(val) = std::env::var("LIDM_LOG_STDOUT") {
            if !val.is_empty() { self.logging.stdout = val == "1" || val.eq_ignore_ascii_case("true"); }
        }
        if let Ok(val) = std::env::var("LIDM_PAM_SERVICE") {
            if !val.is_empty() { self.auth.pam_service = val; }
        }

        let mut env_table = toml::Table::new();

        for (key, val) in std::env::vars() {
            if val.is_empty() {
                continue;
            }

            if let Some(rest) = key.strip_prefix("LIDM_") {
                if let Some((section_str, item_str)) = rest.split_once('_') {
                    let section = section_str.to_lowercase();
                    let item = item_str.to_lowercase();

                    if section == "conf" || section == "vt" {
                        continue;
                    }

                    let toml_val = if val.eq_ignore_ascii_case("true") {
                        toml::Value::Boolean(true)
                    } else if val.eq_ignore_ascii_case("false") {
                        toml::Value::Boolean(false)
                    } else if let Ok(i) = val.parse::<i64>() {
                        toml::Value::Integer(i)
                    } else if let Ok(f) = val.parse::<f64>() {
                        toml::Value::Float(f)
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
            }
        }

        if !env_table.is_empty() {
            if let Ok(toml_str) = toml::to_string(&env_table) {
                if let Ok(override_config) = toml::from_str::<Config>(&toml_str) {
                    for (section, sec_val) in env_table {
                        if let toml::Value::Table(items) = sec_val {
                            for (item, _) in items {
                                match (section.as_str(), item.as_str()) {
                                    ("logging", "file") => self.logging.file = override_config.logging.file.clone(),
                                    ("logging", "level") => self.logging.level = override_config.logging.level.clone(),
                                    ("logging", "stdout") => self.logging.stdout = override_config.logging.stdout,
                                    ("auth", "pam_service") => self.auth.pam_service = override_config.auth.pam_service.clone(),
                                    ("behavior", "box_type") => self.behavior.box_type = override_config.behavior.box_type.clone(),
                                    ("behavior", "include_defshell") => self.behavior.include_defshell = override_config.behavior.include_defshell,
                                    ("behavior", "show_console") => self.behavior.show_console = override_config.behavior.show_console,
                                    ("behavior", "timefmt") => self.behavior.timefmt = override_config.behavior.timefmt.clone(),
                                    ("behavior", "refresh_rate") => self.behavior.refresh_rate = override_config.behavior.refresh_rate,
                                    ("behavior", "bypass_shell_login") => self.behavior.bypass_shell_login = override_config.behavior.bypass_shell_login,
                                    ("strings", "f_poweroff") => self.strings.f_poweroff = override_config.strings.f_poweroff.clone(),
                                    ("strings", "f_reboot") => self.strings.f_reboot = override_config.strings.f_reboot.clone(),
                                    ("strings", "f_refresh") => self.strings.f_refresh = override_config.strings.f_refresh.clone(),
                                    ("strings", "f_fido") => self.strings.f_fido = override_config.strings.f_fido.clone(),
                                    ("strings", "f_theme") => self.strings.f_theme = override_config.strings.f_theme.clone(),
                                    ("strings", "e_user") => self.strings.e_user = override_config.strings.e_user.clone(),
                                    ("strings", "e_passwd") => self.strings.e_passwd = override_config.strings.e_passwd.clone(),
                                    ("strings", "s_wayland") => self.strings.s_wayland = override_config.strings.s_wayland.clone(),
                                    ("strings", "s_xorg") => self.strings.s_xorg = override_config.strings.s_xorg.clone(),
                                    ("strings", "s_shell") => self.strings.s_shell = override_config.strings.s_shell.clone(),
                                    ("strings", "opts_pre") => self.strings.opts_pre = override_config.strings.opts_pre.clone(),
                                    ("strings", "opts_post") => self.strings.opts_post = override_config.strings.opts_post.clone(),
                                    ("strings", "ellipsis") => self.strings.ellipsis = override_config.strings.ellipsis.clone(),
                                    ("functions", "poweroff") => self.functions.poweroff = override_config.functions.poweroff.clone(),
                                    ("functions", "reboot") => self.functions.reboot = override_config.functions.reboot.clone(),
                                    ("functions", "refresh") => self.functions.refresh = override_config.functions.refresh.clone(),
                                    ("functions", "fido") => self.functions.fido = override_config.functions.fido.clone(),
                                    ("functions", "theme") => self.functions.theme = override_config.functions.theme.clone(),
                                    ("chars", "hb") => self.chars.hb = override_config.chars.hb.clone(),
                                    ("chars", "vb") => self.chars.vb = override_config.chars.vb.clone(),
                                    ("chars", "ctl") => self.chars.ctl = override_config.chars.ctl.clone(),
                                    ("chars", "ctr") => self.chars.ctr = override_config.chars.ctr.clone(),
                                    ("chars", "cbl") => self.chars.cbl = override_config.chars.cbl.clone(),
                                    ("chars", "cbr") => self.chars.cbr = override_config.chars.cbr.clone(),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn load(args: &crate::Args) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::default();

        if Path::new(&args.conf_path).exists() {
            config.parse(&args.conf_path)?;
        }

        config.apply_env_overrides();

        if let Some(ref file) = args.logging_file {
            if !file.is_empty() {
                config.logging.file = file.clone();
            }
        }
        if let Some(ref level) = args.logging_level {
            if !level.is_empty() {
                config.logging.level = level.clone();
            }
        }
        if args.logging_stdout {
            config.logging.stdout = true;
        }
        if let Some(ref pam_service) = args.auth_pam_service {
            if !pam_service.is_empty() {
                config.auth.pam_service = pam_service.clone();
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
            logging_stdout: true,
            auth_pam_service: None,
            conf_path: config_path.to_str().unwrap().to_string(),
        };

        let config = Config::load(&args).unwrap();

        // CLI overrides TOML
        assert_eq!(config.logging.file, "/tmp/cli.log");

        // CLI overrides Env and TOML
        assert_eq!(config.logging.level, "error");

        // CLI flag set
        assert_eq!(config.logging.stdout, true);

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
