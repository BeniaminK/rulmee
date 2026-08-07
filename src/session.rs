use std::fs;
use freedesktop_entry_parser::parse_entry;

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

pub fn parse_exec_string(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut escaped = false;
    let mut saw_quotes_for_arg = false;

    let chars: Vec<char> = exec.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escaped {
            current_arg.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        if c == '\\' && !in_single_quote {
            escaped = true;
            i += 1;
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            saw_quotes_for_arg = true;
            i += 1;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            saw_quotes_for_arg = true;
            i += 1;
            continue;
        }

        if (c == ' ' || c == '\t' || c == '\n') && !in_double_quote && !in_single_quote {
            if !current_arg.is_empty() || saw_quotes_for_arg {
                args.push(std::mem::take(&mut current_arg));
                saw_quotes_for_arg = false;
            }
            i += 1;
            continue;
        }

        if c == '%' && !in_double_quote && !in_single_quote {
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if next == '%' {
                    current_arg.push('%');
                    i += 2;
                    continue;
                } else if next.is_ascii_alphanumeric() {
                    // Drop field code (%f, %F, %u, %U, %i, %c, %k, etc.)
                    i += 2;
                    continue;
                } else {
                    // Non-specifier character (e.g. space, quote) after %: treat % as literal %
                    current_arg.push('%');
                    i += 1;
                    continue;
                }
            } else {
                // Trailing % at EOF: treat % as literal %
                current_arg.push('%');
                i += 1;
                continue;
            }
        }

        current_arg.push(c);
        i += 1;
    }

    if !current_arg.is_empty() || saw_quotes_for_arg {
        args.push(current_arg);
    }

    args
}

pub fn get_available_sessions() -> Vec<Session> {
    let mut sessions = Vec::new();

    for (session_type, dir) in SOURCES {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let fpath = entry.path();
                if fpath.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(desktop_entry) = parse_entry(&fpath) {
                        let section = desktop_entry.section("Desktop Entry");
                        let name = section.attr("Name");
                        let exec = section.attr("Exec");
                        
                        if let (Some(name), Some(exec)) = (name, exec) {
                            let args = parse_exec_string(exec);
                            
                            if !args.is_empty() {
                                sessions.push(Session {
                                    name: name.to_string(),
                                    exec: ExecType::Desktop(args),
                                    session_type: *session_type,
                                    desktop_names: None,
                                });
                            }
                        }
                    }
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
        assert_eq!(
            parse_exec_string("my\\ app --arg"),
            vec!["my app", "--arg"]
        );
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
        assert_eq!(
            parse_exec_string("echo %%USER%%"),
            vec!["echo", "%USER%"]
        );
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
        assert_eq!(
            parse_exec_string("echo % 100"),
            vec!["echo", "%", "100"]
        );
    }
}

