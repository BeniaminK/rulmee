use crate::colors::Colors;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
}

pub fn discover_themes(base_colors: &Colors) -> Vec<Theme> {
    let mut themes = Vec::new();
    themes.push(Theme {
        name: "default".to_string(),
        colors: base_colors.clone(),
    });

    let search_paths = [
        Path::new("/etc/lidm/themes"),
        Path::new("/usr/share/lidm/themes"),
        Path::new("./themes"),
    ];

    for path in search_paths {
        if !path.exists() || !path.is_dir() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut file_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            file_entries.sort_by_key(|e| e.path());

            for entry in file_entries {
                let p = entry.path();
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext != "toml" && ext != "ini" {
                    continue;
                }

                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(colors) = toml::from_str::<Colors>(&content) {
                        let name = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if !themes.iter().any(|t| t.name == name) {
                            themes.push(Theme { name, colors });
                        }
                    } else {
                        log::warn!("Failed to parse theme file: {}", p.display());
                    }
                }
            }
        }
    }

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
