use freedesktop::ApplicationEntry;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Xorg,
    Wayland,
    Shell,
}

#[derive(Debug, Clone)]
pub enum ExecType {
    Shell(String),
    Desktop(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub exec: ExecType,
    pub session_type: SessionType,
    pub desktop_names: Option<String>,
}

const SOURCES: &[(SessionType, &str)] = &[
    (SessionType::Xorg, "/usr/share/xsessions"),
    (SessionType::Xorg, "/usr/local/share/xsessions"),
    (SessionType::Wayland, "/usr/share/wayland-sessions"),
    (SessionType::Wayland, "/usr/local/share/wayland-sessions"),
];

/// Parses a desktop entry `Exec` command line into separate arguments,
/// handling single/double quotes, escape sequences, and stripping Freedesktop `%` field codes.
pub fn parse_exec_string(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = None;
    let mut has_token = false;
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        match (c, in_quote) {
            ('\\', None) => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    has_token = true;
                }
            }
            ('"' | '\'', None) => {
                in_quote = Some(c);
                has_token = true;
            }
            (q, Some(active)) if q == active => {
                in_quote = None;
            }
            (c, None) if c.is_whitespace() => {
                if has_token || !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            ('%', None) => match chars.peek() {
                Some('%') => {
                    chars.next();
                    current.push('%');
                    has_token = true;
                }
                Some(&next) if next.is_ascii_alphanumeric() => {
                    chars.next(); // Strip Freedesktop field code (%f, %u, etc.)
                }
                _ => {
                    current.push('%');
                    has_token = true;
                }
            },
            (c, _) => {
                current.push(c);
                has_token = true;
            }
        }
    }

    if has_token || !current.is_empty() {
        args.push(current);
    }

    args
}

fn get_desktop_names(app: &ApplicationEntry) -> Option<String> {
    app.get_string("DesktopNames")
        .or_else(|| app.get_vec("DesktopNames").map(|v| v.join(";")))
}

pub fn get_available_sessions() -> Vec<Session> {
    let mut sessions = Vec::new();

    for (session_type, dir) in SOURCES {
        let path = std::path::Path::new(dir);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };

        for entry in entries.flatten() {
            let fpath = entry.path();
            if fpath.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }

            let Ok(app) = ApplicationEntry::try_from_path(&fpath) else {
                continue;
            };

            if !app.should_show() {
                continue;
            }

            if let (Some(name), Some(exec)) = (app.name(), app.exec()) {
                let args = parse_exec_string(&exec);
                if !args.is_empty() {
                    sessions.push(Session {
                        name: name.to_string(),
                        exec: ExecType::Desktop(args),
                        session_type: *session_type,
                        desktop_names: get_desktop_names(&app),
                    });
                }
            }
        }
    }

    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_string_plain() {
        assert_eq!(
            parse_exec_string("sway --unsupported-gpu"),
            vec!["sway", "--unsupported-gpu"]
        );
    }

    #[test]
    fn test_parse_exec_string_quoted_spaces() {
        assert_eq!(
            parse_exec_string("gnome-session --session=\"gnome fallback\""),
            vec!["gnome-session", "--session=gnome fallback"]
        );
        assert_eq!(
            parse_exec_string("'my program' 'arg with spaces'"),
            vec!["my program", "arg with spaces"]
        );
    }

    #[test]
    fn test_parse_exec_string_escaped_spaces() {
        assert_eq!(parse_exec_string("my\\ app --arg"), vec!["my app", "--arg"]);
    }

    #[test]
    fn test_parse_exec_string_field_codes_stripped() {
        assert_eq!(
            parse_exec_string("gnome-terminal %u --dir %f"),
            vec!["gnome-terminal", "--dir"]
        );
        assert_eq!(
            parse_exec_string("app --title=%c %F"),
            vec!["app", "--title="]
        );
    }

    #[test]
    fn test_parse_exec_string_literal_percent() {
        assert_eq!(parse_exec_string("echo %%USER%%"), vec!["echo", "%USER%"]);
    }

    #[test]
    fn test_parse_exec_string_unclosed_quotes() {
        assert_eq!(
            parse_exec_string("sway --config \"/etc/sway/config"),
            vec!["sway", "--config", "/etc/sway/config"]
        );
    }

    #[test]
    fn test_parse_exec_string_percent_in_quotes_and_non_specifiers() {
        assert_eq!(
            parse_exec_string("app --title=\"100%\" --next"),
            vec!["app", "--title=100%", "--next"]
        );
        assert_eq!(parse_exec_string("echo % 100"), vec!["echo", "%", "100"]);
    }

    #[test]
    fn test_parse_exec_string_quotes_and_specifiers() {
        let input = "sway --config \"/etc/sway/config file\" %u %F";
        let args = parse_exec_string(input);
        assert_eq!(args, vec!["sway", "--config", "/etc/sway/config file"]);
    }

    #[test]
    fn test_get_available_sessions() {
        let sessions = get_available_sessions();
        for session in sessions {
            assert!(!session.name.is_empty());
            match &session.exec {
                ExecType::Desktop(args) => assert!(!args.is_empty()),
                ExecType::Shell(sh) => assert!(!sh.is_empty()),
            }
        }
    }
}
