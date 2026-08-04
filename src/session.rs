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
}

const SOURCES: &[(SessionType, &str)] = &[
    (SessionType::Xorg, "/usr/share/xsessions"),
    (SessionType::Xorg, "/usr/local/share/xsessions"),
    (SessionType::Wayland, "/usr/share/wayland-sessions"),
    (SessionType::Wayland, "/usr/local/share/wayland-sessions"),
];

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
                            // Basic parsing of exec string into args
                            let args: Vec<String> = exec.split_whitespace()
                                .map(|s| s.to_string())
                                .collect();
                            
                            if !args.is_empty() {
                                sessions.push(Session {
                                    name: name.to_string(),
                                    exec: ExecType::Desktop(args),
                                    session_type: *session_type,
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
