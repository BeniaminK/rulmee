use crate::colors::Colors;
use crate::config::Config;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
}

/// Discovers available themes from standard directories.
///
/// Theme files must be valid TOML files matching the `Config` structure
/// (i.e., with a `[colors]` section using `ThemeStyle` objects).
/// Legacy `.ini` files using C-era ANSI escape code strings are skipped.
///
/// The first entry is always `"default"` with the currently loaded colors.
pub fn discover_themes(base_colors: &Colors) -> Vec<Theme> {
    let mut themes = Vec::new();
    themes.push(Theme {
        name: "default".to_string(),
        colors: base_colors.clone(),
    });
    log::debug!("Theme discovery: added 'default' theme from current config");

    let search_paths = [
        Path::new("/etc/lidm/themes"),
        Path::new("/usr/share/lidm/themes"),
        Path::new("./themes"),
    ];

    for path in search_paths {
        if !path.exists() || !path.is_dir() {
            log::debug!("Theme discovery: skipping '{}' (not found or not a directory)", path.display());
            continue;
        }

        log::debug!("Theme discovery: scanning '{}'", path.display());

        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Theme discovery: failed to read directory '{}': {}", path.display(), e);
                continue;
            }
        };

        let mut file_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        file_entries.sort_by_key(|e| e.path());

        for entry in file_entries {
            let p = entry.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");

            // Only process .toml files — .ini files use the C-era ANSI
            // escape code format which is incompatible with ThemeStyle.
            if ext != "toml" {
                if ext == "ini" {
                    log::debug!(
                        "Theme discovery: skipping '{}' (INI format not supported, use TOML)",
                        p.display()
                    );
                }
                continue;
            }

            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            if themes.iter().any(|t| t.name == name) {
                log::debug!("Theme discovery: skipping duplicate theme '{}'", name);
                continue;
            }

            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Theme discovery: failed to read '{}': {}", p.display(), e);
                    continue;
                }
            };

            // Theme files use the same Config format (with [colors] section).
            // Parse as Config to extract just the colors.
            match toml::from_str::<Config>(&content) {
                Ok(parsed_config) => {
                    log::info!(
                        "Theme discovery: loaded theme '{}' from '{}'",
                        name,
                        p.display()
                    );
                    themes.push(Theme {
                        name,
                        colors: parsed_config.colors,
                    });
                }
                Err(e) => {
                    log::warn!(
                        "Theme discovery: failed to parse '{}': {}",
                        p.display(),
                        e
                    );
                }
            }
        }
    }

    log::info!(
        "Theme discovery: found {} theme(s): [{}]",
        themes.len(),
        themes
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    themes
}

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
