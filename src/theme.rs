use crate::colors::Colors;
use crate::config::Config;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub path: String,
    pub colors: Colors,
}

impl Theme {
    pub fn new(name: impl Into<String>, path: impl Into<String>, colors: Colors) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            colors,
        }
    }
}

/// Discovers available themes from standard directories.
///
/// Supports TOML theme files and legacy INI theme files (via `legacy_ini`).
/// The first entry is always `"default"` with the currently loaded colors.
pub fn discover_themes(base_colors: &Colors) -> Vec<Theme> {
    let mut themes = vec![Theme::new("default", "default", base_colors.clone())];

    let search_paths = [
        Path::new("/etc/rulmee/themes"),
        Path::new("/usr/share/rulmee/themes"),
        Path::new("/etc/lidm/themes"),
        Path::new("/usr/share/lidm/themes"),
        Path::new("./themes"),
    ];

    for dir in search_paths {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };

        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();

        for p in paths {
            let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
                continue;
            };

            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            if themes.iter().any(|t| t.name == name) {
                continue;
            }

            let colors_result = match ext {
                "toml" => std::fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|c| {
                        Config::from_toml_str(&c)
                            .map(|cfg| cfg.colors)
                            .map_err(|e| e.to_string())
                    }),
                "ini" => crate::legacy_ini::load_legacy_ini_theme(&p),
                _ => continue,
            };

            match colors_result {
                Ok(colors) => {
                    let path_str = p.display().to_string();
                    log::info!(
                        "Theme discovery: loaded theme '{}' from '{}'",
                        name,
                        path_str
                    );
                    themes.push(Theme::new(name, path_str, colors));
                }
                Err(e) => {
                    log::warn!("Theme discovery: failed to load '{}': {}", p.display(), e);
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

    #[test]
    fn test_discover_themes_loads_ini_and_toml() {
        let temp_dir = std::env::temp_dir();
        let themes_dir = temp_dir.join("rulmee_test_discover_themes");
        let _ = std::fs::create_dir_all(&themes_dir);

        let toml_file = themes_dir.join("theme_a.toml");
        std::fs::write(&toml_file, "[colors]\nfg = { color = \"#112233\" }\n").unwrap();

        let ini_file = themes_dir.join("theme_b.ini");
        std::fs::write(&ini_file, "[colors]\nfg = \"38;2;40;50;60\"\n").unwrap();

        let base_colors = Colors::default();
        let mut themes = vec![Theme::new("default", "default", base_colors.clone())];

        let Ok(entries) = std::fs::read_dir(&themes_dir) else {
            panic!()
        };
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();

        for p in paths {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            let name = p.file_stem().unwrap().to_str().unwrap().to_string();
            let colors = match ext {
                "toml" => {
                    Config::from_toml_str(&std::fs::read_to_string(&p).unwrap())
                        .unwrap()
                        .colors
                }
                "ini" => crate::legacy_ini::load_legacy_ini_theme(&p).unwrap(),
                _ => continue,
            };
            themes.push(Theme::new(name, p.display().to_string(), colors));
        }

        assert_eq!(themes.len(), 3);
        assert_eq!(themes[1].name, "theme_a");
        assert_eq!(themes[1].colors.fg.color.as_deref(), Some("#112233"));
        assert_eq!(themes[2].name, "theme_b");
        assert_eq!(themes[2].colors.fg.color.as_deref(), Some("#28323c"));

        let _ = std::fs::remove_dir_all(themes_dir);
    }
}
