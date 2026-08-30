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
            file: "/tmp/rulmee.log".to_string(),
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

    /// Scan environment variables matching `RULMEE_<SECTION>_<KEY>` (or legacy `LIDM_<SECTION>_<KEY>`) and apply them
    /// as overrides onto the current configuration. The naming convention is
    /// automatic: `RULMEE_STRINGS_F_POWEROFF=dsds` maps to `[strings] f_poweroff`.
    ///
    /// Values are auto-typed: `true`/`false` → bool, valid integers → integer,
    /// everything else → string.
    pub fn apply_env_overrides(&mut self) {
        let mut env_table = toml::Table::new();

        for (key, val) in std::env::vars() {
            if val.is_empty() {
                continue;
            }

            let rest = match key.strip_prefix("RULMEE_").or_else(|| key.strip_prefix("LIDM_")) {
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
        let known_sections = [
            "colors",
            "functions",
            "strings",
            "behavior",
            "logging",
            "auth",
        ];
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

            let (section_candidate, item_candidate) = if let Some(pos) = full_key.find(['_', '-']) {
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

    pub fn generate_cli_help() -> &'static str {
        static HELP: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        HELP.get_or_init(|| {
            let default_val = toml::Value::try_from(Config::default())
                .unwrap_or(toml::Value::Table(toml::Table::new()));
            let mut out = String::from(
                "Configuration Overrides:\n  Any setting can be overridden via --<section>-<key> <value> or RULMEE_<SECTION>_<KEY>=<value>.\n\n",
            );

            let section_order = ["behavior", "auth", "logging", "functions", "strings", "colors"];

            if let toml::Value::Table(sections) = default_val {
                for sec_name in section_order {
                    if let Some(toml::Value::Table(keys)) = sections.get(sec_name) {
                        out.push_str(&format!("  [{}]:\n", sec_name));
                        for (k, v) in keys {
                            let flag_name = format!("--{}-{}", sec_name, k.replace('_', "-"));
                            let default_display = match v {
                                toml::Value::String(s) => format!("\"{}\"", s),
                                toml::Value::Integer(i) => i.to_string(),
                                toml::Value::Float(f) => f.to_string(),
                                toml::Value::Boolean(b) => b.to_string(),
                                toml::Value::Array(a) => format!("{:?}", a),
                                _ => format!("{}", v),
                            };
                            out.push_str(&format!("    {:<38} [default: {}]\n", flag_name, default_display));
                        }
                        out.push('\n');
                    }
                }
            }
            out
        })
    }

    /// Resolve configuration file path with fallback to legacy path if the primary path is not found.
    pub fn resolve_config_path(primary: &str) -> (String, Option<String>) {
        Self::resolve_config_path_with_custom_fallback(
            primary,
            "/etc/rulmee/default.toml",
            "/etc/lidm/default.toml",
        )
    }

    /// Resolve configuration file path given an expected primary and fallback path.
    pub fn resolve_config_path_with_custom_fallback(
        primary: &str,
        expected_primary: &str,
        fallback_path: &str,
    ) -> (String, Option<String>) {
        if Path::new(primary).exists() {
            return (primary.to_string(), None);
        }

        if primary == expected_primary && Path::new(fallback_path).exists() {
            let parent_dir = Path::new(expected_primary)
                .parent()
                .unwrap_or(Path::new(""))
                .display();
            let warn_msg = format!(
                "Path '{}' not found; falling back to legacy '{}' (deprecated). Please migrate configuration to '{}'.",
                expected_primary, fallback_path, parent_dir
            );
            log::warn!("{}", warn_msg);
            return (fallback_path.to_string(), Some(warn_msg));
        }

        (primary.to_string(), None)
    }

    pub fn load_with_fallback(
        primary: &str,
        fallback: &str,
        cli_overrides: Option<toml::Table>,
    ) -> (Self, Option<String>) {
        let (resolved_path, fallback_warning) =
            Self::resolve_config_path_with_custom_fallback(primary, primary, fallback);
        let (mut config, err_msg) = if Path::new(&resolved_path).exists() {
            match Self::from_file(&resolved_path) {
                Ok(cfg) => (cfg, fallback_warning),
                Err(e) => {
                    let msg = format!(
                        "Failed to parse config from '{}': {}. Falling back to default configuration.",
                        resolved_path, e
                    );
                    eprintln!("{}", msg);
                    (Self::default(), Some(msg))
                }
            }
        } else {
            (Self::default(), fallback_warning)
        };

        config.apply_env_overrides();
        if let Some(overrides) = cli_overrides {
            config.apply_table_overrides(overrides);
        }

        (config, err_msg)
    }

    pub fn load_with_overrides(
        conf_path: &str,
        cli_overrides: Option<toml::Table>,
    ) -> (Self, Option<String>) {
        let (resolved_path, fallback_warning) = Self::resolve_config_path(conf_path);
        let (mut config, err_msg) = if Path::new(&resolved_path).exists() {
            match Self::from_file(&resolved_path) {
                Ok(cfg) => (cfg, fallback_warning),
                Err(e) => {
                    let msg = format!(
                        "Failed to parse config from '{}': {}. Falling back to default configuration.",
                        resolved_path, e
                    );
                    eprintln!("{}", msg);
                    (Self::default(), Some(msg))
                }
            }
        } else {
            (Self::default(), fallback_warning)
        };

        config.apply_env_overrides();
        if let Some(overrides) = cli_overrides {
            config.apply_table_overrides(overrides);
        }

        (config, err_msg)
    }

    pub fn load(args: &crate::Args, cli_overrides: Option<toml::Table>) -> (Self, Option<String>) {
        Self::load_with_overrides(&args.conf_path, cli_overrides)
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
            return std::path::PathBuf::from(xdg)
                .join("rulmee")
                .join("default.toml");
        }

        if let Ok(home) = std::env::var("HOME")
            && !home.trim().is_empty()
        {
            return std::path::PathBuf::from(home)
                .join(".config")
                .join("rulmee")
                .join("default.toml");
        }

        std::path::PathBuf::from("/etc/rulmee/default.toml")
    }

    pub fn execute_copy_config(
        dest: Option<&str>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let path = Self::resolve_default_copy_path(dest);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_toml = Self::generate_default_toml();
        let header = "# Default Configuration for Rulmee (Lightweight Display Manager)\n# All settings shown below with their default values.\n\n";
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
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_default_config_toml() {
        let default_toml = Config::generate_default_toml();
        let target_path = Path::new("themes/default.toml");
        let header = "# Default Configuration for Rulmee (Lightweight Display Manager)\n# All settings shown below with their default values.\n\n";
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
        assert_eq!(config.logging.file, "/tmp/rulmee.log");
        assert_eq!(config.logging.level, "debug");
        assert!(!config.logging.stdout);
        assert_eq!(config.auth.pam_service, "login");
    }

    #[test]
    fn test_config_automatic_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("RULMEE_LOGGING_LEVEL", "warn");
            std::env::set_var("RULMEE_LOGGING_STDOUT", "true");
            std::env::set_var("RULMEE_AUTH_PAM_SERVICE", "custom-pam");
            std::env::set_var("RULMEE_BEHAVIOR_REFRESH_RATE", "250");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.logging.level, "warn");
        assert!(config.logging.stdout);
        assert_eq!(config.auth.pam_service, "custom-pam");
        assert_eq!(config.behavior.refresh_rate, 250);

        unsafe {
            std::env::remove_var("RULMEE_LOGGING_LEVEL");
            std::env::remove_var("RULMEE_LOGGING_STDOUT");
            std::env::remove_var("RULMEE_AUTH_PAM_SERVICE");
            std::env::remove_var("RULMEE_BEHAVIOR_REFRESH_RATE");
        }
    }

    #[test]
    fn test_config_legacy_env_overrides_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("LIDM_LOGGING_LEVEL", "warn");
            std::env::set_var("LIDM_BEHAVIOR_REFRESH_RATE", "250");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.logging.level, "warn");
        assert_eq!(config.behavior.refresh_rate, 250);

        unsafe {
            std::env::remove_var("LIDM_LOGGING_LEVEL");
            std::env::remove_var("LIDM_BEHAVIOR_REFRESH_RATE");
        }
    }

    #[test]
    fn test_config_load_precedence() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rulmee_precedence.toml");
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
            std::env::set_var("RULMEE_LOGGING_LEVEL", "warn");
            std::env::set_var("RULMEE_BEHAVIOR_REFRESH_RATE", "300");
        }

        let raw_args = vec![
            "rulmee",
            "-c",
            config_path.to_str().unwrap(),
            "--logging-file",
            "/tmp/cli.log",
            "--logging-level",
            "error",
            "--behavior-refresh-rate",
            "450",
            "--behavior-box-type",
            "rounded",
        ];

        let (cli_overrides, _remaining) = Config::extract_cli_overrides(raw_args);
        let args = crate::Args {
            command: None,
            vt: None,
            conf_path: config_path.to_str().unwrap().to_string(),
        };

        let (config, err_opt) = Config::load(&args, Some(cli_overrides));
        assert!(err_opt.is_none());

        // CLI overrides TOML
        assert_eq!(config.logging.file, "/tmp/cli.log");

        // CLI overrides Env and TOML
        assert_eq!(config.logging.level, "error");

        // CLI overrides Env and TOML
        assert_eq!(config.behavior.refresh_rate, 450);
        assert_eq!(config.behavior.box_type, BoxType::Rounded);

        // TOML preserved when no Env or CLI set
        assert_eq!(config.auth.pam_service, "toml-pam");

        unsafe {
            std::env::remove_var("RULMEE_LOGGING_LEVEL");
            std::env::remove_var("RULMEE_BEHAVIOR_REFRESH_RATE");
        }
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_config_load_broken_toml_fallback_to_default() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rulmee_broken.toml");
        std::fs::write(&config_path, "invalid toml [[ [ {{ content").unwrap();

        let args = crate::Args {
            command: None,
            vt: None,
            conf_path: config_path.to_str().unwrap().to_string(),
        };

        let (config, err) = Config::load(&args, None);
        assert!(err.is_some());
        assert_eq!(config.logging.file, "/tmp/rulmee.log");

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_config_arbitrary_env_override_strings() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("RULMEE_STRINGS_F_POWEROFF", "dsds");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(config.strings.f_poweroff, "dsds");

        unsafe {
            std::env::remove_var("RULMEE_STRINGS_F_POWEROFF");
        }
    }

    #[test]
    fn test_box_type_deserialization() {
        let toml_plain: Config = toml::from_str(
            r#"[behavior]
box_type = "plain""#,
        )
        .unwrap();
        assert_eq!(toml_plain.behavior.box_type, BoxType::Border);

        let toml_default: Config = toml::from_str(
            r#"[behavior]
box_type = "default""#,
        )
        .unwrap();
        assert_eq!(toml_default.behavior.box_type, BoxType::Border);

        let toml_border: Config = toml::from_str(
            r#"[behavior]
box_type = "border""#,
        )
        .unwrap();
        assert_eq!(toml_border.behavior.box_type, BoxType::Border);

        let toml_none: Config = toml::from_str(
            r#"[behavior]
box_type = "none""#,
        )
        .unwrap();
        assert_eq!(toml_none.behavior.box_type, BoxType::None);

        let toml_rounded: Config = toml::from_str(
            r#"[behavior]
box_type = "rounded""#,
        )
        .unwrap();
        assert_eq!(toml_rounded.behavior.box_type, BoxType::Rounded);

        let toml_block: Config = toml::from_str(
            r#"[behavior]
box_type = "block""#,
        )
        .unwrap();
        assert_eq!(toml_block.behavior.box_type, BoxType::Block);
    }

    #[test]
    fn test_resolve_default_copy_path_custom() {
        let custom = "/tmp/custom_rulmee_config.toml";
        let path = Config::resolve_default_copy_path(Some(custom));
        assert_eq!(path, std::path::PathBuf::from(custom));
    }

    #[test]
    fn test_execute_copy_config_creates_file() {
        let temp_dir = std::env::temp_dir();
        let target = temp_dir.join("rulmee_test_copy_config/sub/default.toml");
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }

        let result = Config::execute_copy_config(target.to_str());
        assert!(result.is_ok());

        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("# Default Configuration for Rulmee"));
        assert!(content.contains("[logging]"));

        let _ = std::fs::remove_dir_all(temp_dir.join("rulmee_test_copy_config"));
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
            "rulmee".to_string(),
            "-c".to_string(),
            "/etc/rulmee/default.toml".to_string(),
            "--behavior-box-type".to_string(),
            "rounded".to_string(),
            "--behavior_refresh_rate=250".to_string(),
            "--behavior-show-console".to_string(),
            "--auth-pam-service".to_string(),
            "custom-pam".to_string(),
            "2".to_string(),
        ];

        let (overrides, remaining) = Config::extract_cli_overrides(raw_args);

        assert_eq!(remaining, vec!["rulmee", "-c", "/etc/rulmee/default.toml", "2"]);

        let behavior = overrides.get("behavior").unwrap().as_table().unwrap();
        assert_eq!(
            behavior.get("box_type").unwrap().as_str().unwrap(),
            "rounded"
        );
        assert_eq!(
            behavior.get("refresh_rate").unwrap().as_integer().unwrap(),
            250
        );
        assert!(behavior.get("show_console").unwrap().as_bool().unwrap());

        let auth = overrides.get("auth").unwrap().as_table().unwrap();
        assert_eq!(
            auth.get("pam_service").unwrap().as_str().unwrap(),
            "custom-pam"
        );
    }

    #[test]
    fn test_extract_cli_overrides_arrays_and_booleans() {
        let raw_args = vec![
            "rulmee".to_string(),
            "--behavior-source=/etc/profile,/etc/environment".to_string(),
            "--behavior-include-defshell".to_string(),
            "false".to_string(),
            "--logging-stdout".to_string(),
            "true".to_string(),
        ];

        let (overrides, remaining) = Config::extract_cli_overrides(raw_args);
        assert_eq!(remaining, vec!["rulmee"]);

        let behavior = overrides.get("behavior").unwrap().as_table().unwrap();
        let sources = behavior.get("source").unwrap().as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].as_str().unwrap(), "/etc/profile");
        assert_eq!(sources[1].as_str().unwrap(), "/etc/environment");
        assert!(!behavior.get("include_defshell").unwrap().as_bool().unwrap());

        let logging = overrides.get("logging").unwrap().as_table().unwrap();
        assert!(logging.get("stdout").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_apply_table_overrides() {
        let mut config = Config::default();
        let mut table = toml::Table::new();
        let mut behavior = toml::Table::new();
        behavior.insert("refresh_rate".to_string(), toml::Value::Integer(999));
        behavior.insert(
            "box_type".to_string(),
            toml::Value::String("block".to_string()),
        );
        table.insert("behavior".to_string(), toml::Value::Table(behavior));

        config.apply_table_overrides(table);
        assert_eq!(config.behavior.refresh_rate, 999);
        assert_eq!(config.behavior.box_type, BoxType::Block);
    }

    #[test]
    fn test_generate_cli_help_contains_options() {
        let help_text = Config::generate_cli_help();
        assert!(help_text.contains("Configuration Overrides:"));
        assert!(help_text.contains("--behavior-box-type"));
        assert!(help_text.contains("--behavior-refresh-rate"));
        assert!(help_text.contains("--auth-pam-service"));
        assert!(help_text.contains("[default: 100]"));
    }

    #[test]
    fn test_load_with_overrides_fallback_path() {
        let temp_dir = std::env::temp_dir();
        let legacy_dir = temp_dir.join("lidm_test_fallback");
        let _ = std::fs::create_dir_all(&legacy_dir);
        let legacy_conf = legacy_dir.join("default.toml");
        std::fs::write(&legacy_conf, "[behavior]\nshow_console = true\n").unwrap();

        let primary_path = "/nonexistent/path/rulmee/default.toml";
        let (cfg, warning) =
            Config::load_with_fallback(primary_path, legacy_conf.to_str().unwrap(), None);
        assert!(cfg.behavior.show_console);
        assert!(warning.is_some());
        let warn_msg = warning.unwrap();
        assert!(warn_msg.contains("deprecated"));
        assert!(warn_msg.contains("Please migrate"));

        let _ = std::fs::remove_dir_all(legacy_dir);
    }

    #[test]
    fn test_resolve_config_path_when_primary_exists() {
        let temp_dir = std::env::temp_dir();
        let primary = temp_dir.join("lidm_test_primary_exists.toml");
        std::fs::write(&primary, "[behavior]\nrefresh_rate = 50\n").unwrap();

        let (resolved, warning) = Config::resolve_config_path(primary.to_str().unwrap());
        assert_eq!(resolved, primary.to_str().unwrap());
        assert!(warning.is_none());

        let _ = std::fs::remove_file(primary);
    }

    #[test]
    fn test_resolve_config_path_custom_fallback() {
        let temp_dir = std::env::temp_dir();
        let fallback = temp_dir.join("lidm_test_custom_fallback.toml");
        std::fs::write(&fallback, "[behavior]\nrefresh_rate = 75\n").unwrap();

        let primary = "/nonexistent/custom/primary.toml";
        let (resolved, warning) = Config::resolve_config_path_with_custom_fallback(
            primary,
            primary,
            fallback.to_str().unwrap(),
        );
        assert_eq!(resolved, fallback.to_str().unwrap());
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("deprecated"));

        let _ = std::fs::remove_file(fallback);
    }

    #[test]
    fn test_resolve_config_path_neither_exists() {
        let primary = "/nonexistent/path/rulmee_custom_missing.toml";
        let (resolved, warning) = Config::resolve_config_path_with_custom_fallback(
            primary,
            primary,
            "/nonexistent/path/lidm_custom_missing.toml",
        );
        assert_eq!(resolved, primary);
        assert!(warning.is_none());
    }

    #[test]
    fn test_resolve_config_path_default_nonexistent() {
        let (resolved, _) = Config::resolve_config_path("/some/arbitrary/path.toml");
        assert_eq!(resolved, "/some/arbitrary/path.toml");
    }
}
