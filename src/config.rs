use crate::colors::Colors;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Functions {
    pub poweroff: Option<String>,
    pub reboot: Option<String>,
    pub fido: Option<String>,
    pub theme: Option<String>,
}

impl Default for Functions {
    fn default() -> Self {
        Self {
            poweroff: Some("F1".to_string()),
            reboot: Some("F2".to_string()),
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

impl Default for Behavior {
    fn default() -> Self {
        Self {
            box_type: BoxType::Border,
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
    pub functions: Functions,
    pub strings: Strings,
    pub behavior: Behavior,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Factory constructor: Parses a TOML string into a `Config`, warning on unknown fields.
    pub fn from_toml_str(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let deserializer = toml::Deserializer::parse(content)?;
        let parsed_config: Config = serde_ignored::deserialize(deserializer, |path| {
            log::warn!("Unknown configuration field ignored: {}", path);
        })?;
        Ok(parsed_config)
    }

    /// Factory constructor: Reads and parses a configuration file from a filesystem path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    pub fn apply_table_overrides(&mut self, overrides: toml::Table) {
        if overrides.is_empty() {
            return;
        }

        if let Ok(mut current_val) = toml::Value::try_from(&*self) {
            merge_toml_values(&mut current_val, toml::Value::Table(overrides));
            if let Ok(updated) = current_val.try_into::<Config>() {
                *self = updated;
            }
        }
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

        self.apply_table_overrides(env_table);
    }

    /// Extract configuration overrides matching `--<section>_<key>` or `--<section>-<key>`
    /// from an argument list, returning the parsed TOML table and the remaining arguments.
    pub fn extract_cli_overrides<I, T>(args: I) -> (toml::Table, Vec<String>)
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let known_sections = ["colors", "functions", "strings", "behavior", "logging", "auth"];
        let mut cli_table = toml::Table::new();
        let mut remaining = Vec::new();

        let raw_list: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        let mut i = 0;
        while i < raw_list.len() {
            let arg = &raw_list[i];

            if !arg.starts_with("--") || arg == "--" {
                remaining.push(arg.clone());
                i += 1;
                continue;
            }

            let flag_body = &arg[2..]; // strip "--"
            let (full_key, inline_val) = match flag_body.split_once('=') {
                Some((k, v)) => (k, Some(v.to_string())),
                None => (flag_body, None),
            };

            let (section_candidate, item_candidate) =
                if let Some(pos) = full_key.find(|c| c == '_' || c == '-') {
                    (&full_key[..pos], &full_key[pos + 1..])
                } else {
                    ("", "")
                };

            let section = section_candidate.to_lowercase();
            let item = item_candidate.to_lowercase().replace('-', "_");

            if known_sections.contains(&section.as_str()) && !item.is_empty() {
                let val_str = if let Some(v) = inline_val {
                    v
                } else if i + 1 < raw_list.len() && !raw_list[i + 1].starts_with('-') {
                    i += 1;
                    raw_list[i].clone()
                } else {
                    "true".to_string()
                };

                let toml_val = if val_str.eq_ignore_ascii_case("true") {
                    toml::Value::Boolean(true)
                } else if val_str.eq_ignore_ascii_case("false") {
                    toml::Value::Boolean(false)
                } else if let Ok(n) = val_str.parse::<i64>() {
                    toml::Value::Integer(n)
                } else if val_str.contains(',') && !val_str.starts_with('"') {
                    toml::Value::Array(
                        val_str
                            .split(',')
                            .map(|s| toml::Value::String(s.trim().to_string()))
                            .collect(),
                    )
                } else {
                    toml::Value::String(val_str)
                };

                let sec_entry = cli_table
                    .entry(section)
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                if let toml::Value::Table(table) = sec_entry {
                    table.insert(item, toml_val);
                }
            } else {
                remaining.push(arg.clone());
            }

            i += 1;
        }

        (cli_table, remaining)
    }

    pub fn load(args: &crate::Args) -> (Self, Option<String>) {
        let conf_path = &args.conf_path;
        let (mut config, err_msg) = if Path::new(conf_path).exists() {
            match Self::from_file(conf_path) {
                Ok(cfg) => (cfg, None),
                Err(e) => {
                    let msg = format!(
                        "Failed to parse config from '{}': {}. Falling back to default configuration.",
                        conf_path, e
                    );
                    eprintln!("{}", msg);
                    (Self::default(), Some(msg))
                }
            }
        } else {
            (Self::default(), None)
        };

        config.apply_env_overrides();
        config.apply_cli_overrides(args);

        (config, err_msg)
    }

    pub fn apply_cli_overrides(&mut self, args: &crate::Args) {
        if let Some(ref file) = args.logging_file
            && !file.is_empty()
        {
            self.logging.file = file.clone();
        }

        if let Some(ref level) = args.logging_level
            && !level.is_empty()
        {
            self.logging.level = level.clone();
        }
    }

    pub fn generate_default_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }

    pub fn resolve_default_copy_path(dest: Option<&str>) -> std::path::PathBuf {
        if let Some(d) = dest {
            let trimmed = d.trim();
            if !trimmed.is_empty() {
                return std::path::PathBuf::from(trimmed);
            }
        }

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.trim().is_empty()
        {
            return std::path::PathBuf::from(xdg).join("lidm").join("default.toml");
        }

        if let Ok(home) = std::env::var("HOME")
            && !home.trim().is_empty()
        {
            return std::path::PathBuf::from(home).join(".config").join("lidm").join("default.toml");
        }

        std::path::PathBuf::from("/etc/lidm/default.toml")
    }

    pub fn execute_copy_config(dest: Option<&str>) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let path = Self::resolve_default_copy_path(dest);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_toml = Self::generate_default_toml();
        let header = "# Default Configuration for LiDM (Lightweight Display Manager)\n# All settings shown below with their default values.\n\n";
        let full_content = format!("{}{}", header, default_toml);

        std::fs::write(&path, full_content)?;
        Ok(path)
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

[behavior]
box_type = "rounded"
"##;
        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML");

        // Explicitly defined fields in TOML
        assert_eq!(config.colors.fg.color.as_deref(), Some("#FFFFFF"));
        assert_eq!(config.behavior.box_type, BoxType::Rounded);

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
        assert!(!config.logging.stdout);
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
        assert!(config.logging.stdout);
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
            command: None,
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
            command: None,
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
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.strings.f_poweroff, "dsds");

        unsafe {
            std::env::remove_var("LIDM_STRINGS_F_POWEROFF");
        }
    }

    #[test]
    fn test_box_type_deserialization() {
        let toml_plain: Config = toml::from_str(r#"[behavior]
box_type = "plain""#).unwrap();
        assert_eq!(toml_plain.behavior.box_type, BoxType::Border);

        let toml_default: Config = toml::from_str(r#"[behavior]
box_type = "default""#).unwrap();
        assert_eq!(toml_default.behavior.box_type, BoxType::Border);

        let toml_border: Config = toml::from_str(r#"[behavior]
box_type = "border""#).unwrap();
        assert_eq!(toml_border.behavior.box_type, BoxType::Border);

        let toml_none: Config = toml::from_str(r#"[behavior]
box_type = "none""#).unwrap();
        assert_eq!(toml_none.behavior.box_type, BoxType::None);

        let toml_rounded: Config = toml::from_str(r#"[behavior]
box_type = "rounded""#).unwrap();
        assert_eq!(toml_rounded.behavior.box_type, BoxType::Rounded);

        let toml_block: Config = toml::from_str(r#"[behavior]
box_type = "block""#).unwrap();
        assert_eq!(toml_block.behavior.box_type, BoxType::Block);
    }

    #[test]
    fn test_resolve_default_copy_path_custom() {
        let custom = "/tmp/custom_lidm_config.toml";
        let path = Config::resolve_default_copy_path(Some(custom));
        assert_eq!(path, std::path::PathBuf::from(custom));
    }

    #[test]
    fn test_execute_copy_config_creates_file() {
        let temp_dir = std::env::temp_dir();
        let target = temp_dir.join("lidm_test_copy_config/sub/default.toml");
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }

        let result = Config::execute_copy_config(target.to_str());
        assert!(result.is_ok());

        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("# Default Configuration for LiDM"));
        assert!(content.contains("[logging]"));

        let _ = std::fs::remove_dir_all(temp_dir.join("lidm_test_copy_config"));
    }

    #[test]
    fn test_config_from_toml_str_factory() {
        let toml_str = r#"
[behavior]
box_type = "block"
refresh_rate = 300

[auth]
pam_service = "gdm"
"#;
        let cfg = Config::from_toml_str(toml_str).unwrap();
        assert_eq!(cfg.behavior.box_type, BoxType::Block);
        assert_eq!(cfg.behavior.refresh_rate, 300);
        assert_eq!(cfg.auth.pam_service, "gdm");
    }

    #[test]
    fn test_extract_cli_overrides_basic() {
        let raw_args = vec![
            "lidm".to_string(),
            "-c".to_string(),
            "/etc/lidm/default.toml".to_string(),
            "--behavior-box-type".to_string(),
            "rounded".to_string(),
            "--behavior_refresh_rate=250".to_string(),
            "--behavior-show-console".to_string(),
            "--auth-pam-service".to_string(),
            "custom-pam".to_string(),
            "2".to_string(),
        ];

        let (overrides, remaining) = Config::extract_cli_overrides(raw_args);

        assert_eq!(remaining, vec!["lidm", "-c", "/etc/lidm/default.toml", "2"]);

        let behavior = overrides.get("behavior").unwrap().as_table().unwrap();
        assert_eq!(behavior.get("box_type").unwrap().as_str().unwrap(), "rounded");
        assert_eq!(behavior.get("refresh_rate").unwrap().as_integer().unwrap(), 250);
        assert_eq!(behavior.get("show_console").unwrap().as_bool().unwrap(), true);

        let auth = overrides.get("auth").unwrap().as_table().unwrap();
        assert_eq!(auth.get("pam_service").unwrap().as_str().unwrap(), "custom-pam");
    }

    #[test]
    fn test_extract_cli_overrides_arrays_and_booleans() {
        let raw_args = vec![
            "lidm".to_string(),
            "--behavior-source=/etc/profile,/etc/environment".to_string(),
            "--behavior-include-defshell".to_string(),
            "false".to_string(),
            "--logging-stdout".to_string(),
            "true".to_string(),
        ];

        let (overrides, remaining) = Config::extract_cli_overrides(raw_args);
        assert_eq!(remaining, vec!["lidm"]);

        let behavior = overrides.get("behavior").unwrap().as_table().unwrap();
        let sources = behavior.get("source").unwrap().as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].as_str().unwrap(), "/etc/profile");
        assert_eq!(sources[1].as_str().unwrap(), "/etc/environment");
        assert_eq!(behavior.get("include_defshell").unwrap().as_bool().unwrap(), false);

        let logging = overrides.get("logging").unwrap().as_table().unwrap();
        assert_eq!(logging.get("stdout").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_apply_table_overrides() {
        let mut config = Config::default();
        let mut table = toml::Table::new();
        let mut behavior = toml::Table::new();
        behavior.insert("refresh_rate".to_string(), toml::Value::Integer(999));
        behavior.insert("box_type".to_string(), toml::Value::String("block".to_string()));
        table.insert("behavior".to_string(), toml::Value::Table(behavior));

        config.apply_table_overrides(table);
        assert_eq!(config.behavior.refresh_rate, 999);
        assert_eq!(config.behavior.box_type, BoxType::Block);
    }
}



