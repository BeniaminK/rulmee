mod auth;
mod colors;
mod config;
mod console;
mod exec;
mod logging;
mod session;
mod sys;
mod theme;
mod ui;
mod ui_adapter;
mod ui_state;
mod users;
mod vt;
mod launch_state;
mod signal_handler;

use crate::session::SessionType;
use crate::ui::{UI, UIResult};
use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
use std::ffi::c_int;
use uzers::os::unix::UserExt;

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Copy default configuration to local or specified config file")]
    CopyConfig {
        #[arg(help = "Destination path for the configuration file [default: ~/.config/lidm/default.toml]")]
        dest: Option<String>,
    },
}

#[derive(Parser, Debug)]
#[command(
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (git ",
        env!("VERGEN_GIT_DESCRIBE"),
        ", build date ",
        env!("VERGEN_BUILD_TIMESTAMP"),
        ", compiler ",
        env!("VERGEN_RUSTC_SEMVER"),
        ")"
    ),
    about = "LiDM: Lightweight Display Manager"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(help = "VT number to switch to")]
    pub vt: Option<c_int>,
    
    #[arg(
        short = 'c',
        long = "config",
        env = "LIDM_CONF",
        default_value = "/etc/lidm/default.toml",
        help = "Path to configuration file"
    )]
    pub conf_path: String,

    #[arg(long = "logging-file", help = "Path to log file")]
    pub logging_file: Option<String>,

    #[arg(long = "logging-level", help = "Log level filter")]
    pub logging_level: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSelection {
    pub username: String,
    pub shell: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSelection {
    pub name: String,
    pub exec_args: Vec<String>,
    pub is_xorg: bool,
    pub desktop_names: Option<String>,
}

pub fn resolve_user(users: &[users::LocalUser], user_idx: usize, custom_user: String) -> UserSelection {
    if user_idx < users.len() && custom_user.is_empty() {
        UserSelection {
            username: users[user_idx].username.clone(),
            shell: users[user_idx].shell.clone(),
        }
    } else {
        UserSelection {
            username: custom_user,
            shell: "/bin/bash".to_string(),
        }
    }
}

pub fn resolve_session(
    sessions: &[session::Session],
    session_idx: usize,
    custom_session: String,
    shell: &str,
) -> SessionSelection {
    if session_idx < sessions.len() && custom_session.is_empty() {
        let s = &sessions[session_idx];
        let args = match &s.exec {
            session::ExecType::Shell(sh) => vec![sh.clone()],
            session::ExecType::Desktop(args) => args.clone(),
        };
        SessionSelection {
            name: s.name.clone(),
            exec_args: args,
            is_xorg: s.session_type == SessionType::Xorg,
            desktop_names: s.desktop_names.clone(),
        }
    } else if session_idx == sessions.len() && custom_session.is_empty() {
        SessionSelection {
            name: shell.to_string(),
            exec_args: vec![shell.to_string()],
            is_xorg: false,
            desktop_names: None,
        }
    } else {
        SessionSelection {
            name: custom_session.clone(),
            exec_args: vec![custom_session],
            is_xorg: false,
            desktop_names: None,
        }
    }
}

pub fn handle_login(
    user_idx: usize,
    session_idx: usize,
    password: String,
    custom_session: String,
    custom_user: String,
    config: &config::Config,
    sessions: &[session::Session],
    users: &[users::LocalUser],
    vt: Option<c_int>,
    bypass_shell_login: bool,
) -> Result<(), auth::AuthError> {
    let user_sel = resolve_user(users, user_idx, custom_user);
    let session_sel = resolve_session(sessions, session_idx, custom_session, &user_sel.shell);

    let _ = launch_state::write_launch_state(&launch_state::LaunchState {
        username: user_sel.username.clone(),
        session_opt: session_sel.name.clone(),
    });

    let mut auth_session = auth::authenticate(&user_sel.username, &password, &config.auth.pam_service)?;

    let Some(u) = uzers::get_user_by_name(&user_sel.username) else {
        eprintln!("User not found in system: {}", user_sel.username);
        auth_session.close();
        return Ok(());
    };

    let home_dir = u.home_dir().to_string_lossy().into_owned();
    let uid = u.uid();
    let gid = u.primary_group_id();
    let session_type_str = if session_sel.is_xorg { "x11" } else { "wayland" };

    let env = exec::assemble_environment(
        &auth_session.env,
        &user_sel.username,
        &home_dir,
        &user_sel.shell,
        session_type_str,
        None,
        session_sel.desktop_names.as_deref(),
        &config.behavior.source,
        &config.behavior.user_source,
    );

    if let Err(e) = exec::launch_session(
        &user_sel.username,
        uid,
        gid,
        &env,
        &session_sel.exec_args,
        session_sel.is_xorg,
        vt,
        &user_sel.shell,
        bypass_shell_login,
    ) {
        eprintln!("Failed to launch session: {}", e);
    }

    auth_session.close();
    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Some(Commands::CopyConfig { ref dest }) = args.command {
        match config::Config::execute_copy_config(dest.as_deref()) {
            Ok(path) => {
                println!("Default configuration successfully copied to '{}'.", path.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error copying default configuration: {}", e);
                std::process::exit(1);
            }
        }
    }

    if let Err(e) = signal_handler::setup_signal_handler() {
        eprintln!("Warning: Failed to setup signal handler: {}", e);
    }

    let console_buffer: console::ConsoleBuffer = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::with_capacity(50)));

    let mut pam_messages = Vec::new();
    let mut auth_failed = false;

    loop {

        // Load config
        let conf_path = args.conf_path.clone();
        let (config, config_err) = config::Config::load(&args);

        let _log_guard = match logging::initialize_logging(&config.logging, Some(console_buffer.clone())) {
            Ok(guard) => Some(guard),
            Err(e) => {
                eprintln!("Failed to initialize logging: {}", e);
                None
            }
        };

        info!("Loading configuration from: {}", conf_path);
        if let Some(err) = config_err {
            error!("{}", err);
        }

        match args.vt {
            Some(vt) => match vt::chvt(vt) {
                Err(e) => {
                    warn!("Warning: Could not switch to VT {}: {}", vt, e);
                }
                _ => (),
            },
            None => (),
        }

        debug!("Config: {:?}", config);

        let sessions = session::get_available_sessions();

        debug!("Sessions: {:?}", sessions);

        let users = users::get_human_users();

        debug!("Users: {:?}", users);

        let initial_state = launch_state::read_launch_state();
        let (initial_user, initial_session) = match &initial_state {
            Some(state) => (Some(state.username.as_str()), Some(state.session_opt.as_str())),
            None => (None, None),
        };

        // Start console interceptor (always, to prevent TUI corruption)
        let _console_interceptor = match console::ConsoleInterceptor::intercept(console_buffer.clone()) {
            Ok(ci) => {
                info!("Console output intercepted");
                Some(ci)
            }
            Err(e) => {
                warn!("Could not intercept console: {} (continuing without)", e);
                None
            }
        };
        let bypass_shell_login = config.behavior.bypass_shell_login;

        let mut ui = UI::new(
            config.clone(),
            sessions.clone(),
            users.clone(),
            initial_user,
            initial_session,
            Some(console_buffer.clone()),
            pam_messages.clone(),
            auth_failed,
        );

        match ui.run() {
            Ok(UIResult::Login(session_idx, user_idx, password, custom_session, custom_user)) => {
                match handle_login(
                    user_idx,
                    session_idx,
                    password,
                    custom_session,
                    custom_user,
                    &config,
                    &sessions,
                    &users,
                    args.vt,
                    bypass_shell_login,
                ) {
                    Ok(()) => {
                        pam_messages.clear();
                        auth_failed = false;
                    }
                    Err(auth_err) => {
                        eprintln!("Auth failed: {}", auth_err);
                        auth_failed = true;
                        pam_messages = auth_err.pam_messages;
                        if pam_messages.is_empty() {
                            pam_messages.push(auth::PamMessage {
                                msg_type: auth::PamMessageType::Error,
                                message: auth_err.message.clone(),
                            });
                        }
                    }
                }
            }
            Ok(UIResult::Poweroff) => sys::poweroff(),
            Ok(UIResult::Reboot) => sys::reboot(),
            Ok(UIResult::Exit) => break,
            Err(e) => {
                eprintln!("UI Error: {}", e);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_user_indexed() {
        let users = vec![
            users::LocalUser {
                username: "alice".to_string(),
                display_name: "Alice".to_string(),
                shell: "/bin/zsh".to_string(),
            },
            users::LocalUser {
                username: "bob".to_string(),
                display_name: "Bob".to_string(),
                shell: "/bin/fish".to_string(),
            },
        ];
        let sel = resolve_user(&users, 1, "".to_string());
        assert_eq!(sel.username, "bob");
        assert_eq!(sel.shell, "/bin/fish");
    }

    #[test]
    fn test_resolve_user_custom() {
        let users = vec![users::LocalUser {
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            shell: "/bin/zsh".to_string(),
        }];
        let sel = resolve_user(&users, 0, "custom_user".to_string());
        assert_eq!(sel.username, "custom_user");
        assert_eq!(sel.shell, "/bin/bash");
    }

    #[test]
    fn test_resolve_user_out_of_bounds() {
        let users = vec![];
        let sel = resolve_user(&users, 5, "".to_string());
        assert_eq!(sel.username, "");
        assert_eq!(sel.shell, "/bin/bash");
    }

    #[test]
    fn test_resolve_session_indexed_shell() {
        let sessions = vec![session::Session {
            name: "Bash".to_string(),
            exec: session::ExecType::Shell("/bin/bash".to_string()),
            session_type: SessionType::Wayland,
            desktop_names: None,
        }];
        let sel = resolve_session(&sessions, 0, "".to_string(), "/bin/bash");
        assert_eq!(sel.name, "Bash");
        assert_eq!(sel.exec_args, vec!["/bin/bash"]);
        assert!(!sel.is_xorg);
        assert_eq!(sel.desktop_names, None);
    }

    #[test]
    fn test_resolve_session_indexed_desktop_xorg() {
        let sessions = vec![session::Session {
            name: "i3".to_string(),
            exec: session::ExecType::Desktop(vec!["i3".to_string()]),
            session_type: SessionType::Xorg,
            desktop_names: Some("i3".to_string()),
        }];
        let sel = resolve_session(&sessions, 0, "".to_string(), "/bin/bash");
        assert_eq!(sel.name, "i3");
        assert_eq!(sel.exec_args, vec!["i3"]);
        assert!(sel.is_xorg);
        assert_eq!(sel.desktop_names, Some("i3".to_string()));
    }

    #[test]
    fn test_resolve_session_default_shell_option() {
        let sessions = vec![session::Session {
            name: "i3".to_string(),
            exec: session::ExecType::Desktop(vec!["i3".to_string()]),
            session_type: SessionType::Xorg,
            desktop_names: None,
        }];
        // session_idx == sessions.len() represents default shell session
        let sel = resolve_session(&sessions, 1, "".to_string(), "/bin/zsh");
        assert_eq!(sel.name, "/bin/zsh");
        assert_eq!(sel.exec_args, vec!["/bin/zsh"]);
        assert!(!sel.is_xorg);
        assert_eq!(sel.desktop_names, None);
    }

    #[test]
    fn test_resolve_session_custom() {
        let sessions = vec![];
        let sel = resolve_session(&sessions, 0, "startxfce4".to_string(), "/bin/bash");
        assert_eq!(sel.name, "startxfce4");
        assert_eq!(sel.exec_args, vec!["startxfce4"]);
        assert!(!sel.is_xorg);
        assert_eq!(sel.desktop_names, None);
    }
}

