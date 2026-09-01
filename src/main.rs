mod auth;
mod colors;
mod config;
mod console;
mod exec;
mod launch_state;
mod legacy_ini;
mod logging;
mod session;
mod signal_handler;
mod sys;
mod theme;
mod ui;
mod users;
mod vt;

use crate::session::SessionType;
use crate::ui::{LoginRequest, UI, UIContext, UIResult};
use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
use std::ffi::c_int;
use uzers::os::unix::UserExt;

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Copy default configuration to local or specified config file")]
    CopyConfig {
        #[arg(
            help = "Destination path for the configuration file [default: ~/.config/rulmee/default.toml]"
        )]
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
    about = "Rulmee: RUst Login ManagEEr",
    after_help = config::Config::generate_cli_help()
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(help = "VT number to switch to")]
    pub vt: Option<c_int>,

    #[arg(
        short = 'c',
        long = "config",
        env = "RULMEE_CONF",
        default_value = config::DEFAULT_CONFIG_PATH,
        help = "Path to configuration file"
    )]
    pub conf_path: String,
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

pub fn resolve_user(
    users: &[users::LocalUser],
    user_idx: usize,
    custom_user: String,
) -> UserSelection {
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

pub struct LoginContext<'a> {
    pub config: &'a config::Config,
    pub sessions: &'a [session::Session],
    pub users: &'a [users::LocalUser],
    pub vt: Option<c_int>,
    pub bypass_shell_login: bool,
}

pub fn handle_login(request: &LoginRequest, ctx: &LoginContext) -> Result<(), auth::AuthError> {
    let user_sel = resolve_user(ctx.users, request.user_idx, request.custom_user.clone());
    let session_sel = resolve_session(
        ctx.sessions,
        request.session_idx,
        request.custom_session.clone(),
        &user_sel.shell,
    );

    let _ = launch_state::write_launch_state(&launch_state::LaunchState {
        username: user_sel.username.clone(),
        session_opt: session_sel.name.clone(),
    });

    let mut auth_session = auth::authenticate(
        &user_sel.username,
        &request.password,
        &ctx.config.auth.pam_service,
    )?;

    let Some(u) = uzers::get_user_by_name(&user_sel.username) else {
        eprintln!("User not found in system: {}", user_sel.username);
        auth_session.close();
        return Ok(());
    };

    let home_dir = u.home_dir().to_string_lossy().into_owned();
    let uid = u.uid();
    let gid = u.primary_group_id();
    let session_type_str = if session_sel.is_xorg {
        "x11"
    } else {
        "wayland"
    };

    let env_opts = exec::EnvironmentOptions {
        pam_env: &auth_session.env,
        username: &user_sel.username,
        home_dir: &home_dir,
        shell: &user_sel.shell,
        session_type: session_type_str,
        display: None,
        desktop_names: session_sel.desktop_names.as_deref(),
        system_sources: &ctx.config.behavior.source,
        user_sources: &ctx.config.behavior.user_source,
    };
    let env = exec::assemble_environment(&env_opts);

    let launch_ctx = exec::LaunchContext {
        user: &user_sel.username,
        uid,
        gid,
        env: &env,
        exec_args: &session_sel.exec_args,
        is_xorg: session_sel.is_xorg,
        vt: ctx.vt,
        user_shell: &user_sel.shell,
        bypass_shell_login: ctx.bypass_shell_login,
    };

    if let Err(e) = exec::launch_session(&launch_ctx) {
        eprintln!("Failed to launch session: {}", e);
    }

    auth_session.close();
    Ok(())
}

fn main() {
    let (cli_overrides, remaining_args) = config::Config::extract_cli_overrides(std::env::args());
    let args = Args::parse_from(remaining_args);

    if let Some(Commands::CopyConfig { ref dest }) = args.command {
        match config::Config::execute_copy_config(dest.as_deref()) {
            Ok(path) => {
                println!(
                    "Default configuration successfully copied to '{}'.",
                    path.display()
                );
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

    let console_buffer: console::ConsoleBuffer = std::sync::Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::with_capacity(50),
    ));

    let mut pam_messages = Vec::new();
    let mut auth_failed = false;

    loop {
        // Load config
        let conf_path = args.conf_path.clone();
        let (config, config_err) = config::Config::load(&args, Some(cli_overrides.clone()));

        let _log_guard =
            match logging::initialize_logging(&config.logging, Some(console_buffer.clone())) {
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

        if let Some(vt) = args.vt
            && let Err(e) = vt::chvt(vt)
        {
            warn!("Warning: Could not switch to VT {}: {}", vt, e);
        }

        debug!("Config: {:?}", config);

        let sessions = session::get_available_sessions();

        debug!("Sessions: {:?}", sessions);

        let users = users::get_human_users();

        debug!("Users: {:?}", users);

        let initial_state = launch_state::read_launch_state();
        let (initial_user, initial_session) = match &initial_state {
            Some(state) => (
                Some(state.username.as_str()),
                Some(state.session_opt.as_str()),
            ),
            None => (None, None),
        };

        // Start console interceptor (always, to prevent TUI corruption)
        let _console_interceptor =
            match console::ConsoleInterceptor::intercept(console_buffer.clone()) {
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

        let mut ui = UI::new(UIContext {
            config: config.clone(),
            sessions: sessions.clone(),
            users: users.clone(),
            initial_user,
            initial_session,
            console_buffer: Some(console_buffer.clone()),
            pam_messages: pam_messages.clone(),
            auth_error: auth_failed,
        });

        match ui.run() {
            Ok(UIResult::Login(login_req)) => {
                let login_ctx = LoginContext {
                    config: &config,
                    sessions: &sessions,
                    users: &users,
                    vt: args.vt,
                    bypass_shell_login,
                };
                match handle_login(&login_req, &login_ctx) {
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

    #[test]
    fn test_login_request_and_context_creation() {
        let req = LoginRequest {
            session_idx: 0,
            user_idx: 1,
            password: "secret".to_string(),
            custom_session: "".to_string(),
            custom_user: "".to_string(),
        };
        let cfg = config::Config::default();
        let sessions = vec![];
        let users = vec![];
        let ctx = LoginContext {
            config: &cfg,
            sessions: &sessions,
            users: &users,
            vt: Some(7),
            bypass_shell_login: false,
        };

        assert_eq!(req.session_idx, 0);
        assert_eq!(req.user_idx, 1);
        assert_eq!(ctx.vt, Some(7));
        assert!(!ctx.bypass_shell_login);
    }

    #[test]
    fn test_full_cli_args_parsing_and_config_apply() {
        let input_args = vec![
            "rulmee",
            "--behavior-box-type",
            "block",
            "--behavior-refresh-rate",
            "450",
            "--behavior_bypass_shell_login",
            "true",
            "--behavior-show-console",
            "--logging-level",
            "warn",
            "--auth-pam-service",
            "test-pam",
            "--strings-f-poweroff",
            "shutdown",
            "-c",
            "/nonexistent.toml",
            "3",
        ];

        let (overrides, remaining) = config::Config::extract_cli_overrides(input_args);
        let parsed_args = Args::try_parse_from(remaining).unwrap();

        assert_eq!(parsed_args.vt, Some(3));
        assert_eq!(parsed_args.conf_path, "/nonexistent.toml");

        let (config, _) = config::Config::load(&parsed_args, Some(overrides));
        assert_eq!(config.behavior.box_type, config::BoxType::Block);
        assert_eq!(config.behavior.refresh_rate, 450);
        assert!(config.behavior.bypass_shell_login);
        assert!(config.behavior.show_console);
        assert_eq!(config.logging.level, "warn");
        assert_eq!(config.auth.pam_service, "test-pam");
        assert_eq!(config.strings.f_poweroff, "shutdown");
    }

    #[test]
    fn test_cli_help_rendering() {
        use clap::CommandFactory;
        let mut cmd = Args::command();
        let mut help_buf = Vec::new();
        cmd.write_help(&mut help_buf).unwrap();
        let help_str = String::from_utf8(help_buf).unwrap();

        assert!(help_str.contains("Configuration Overrides:"));
        assert!(help_str.contains("--behavior-box-type"));
        assert!(help_str.contains("--behavior-refresh-rate"));
        assert!(help_str.contains("--behavior-show-console"));
        assert!(help_str.contains("--auth-pam-service"));
        assert!(help_str.contains("--logging-file"));
        assert!(help_str.contains("[default: 100]"));
    }

    #[test]
    fn test_cli_args_default_config_path() {
        let _guard = config::ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("RULMEE_CONF");
        }
        let args = Args::parse_from(["rulmee"]);
        assert_eq!(args.conf_path, "/etc/rulmee/default.toml");
    }

    #[test]
    fn test_cli_args_rulmee_conf_env() {
        let _guard = config::ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("RULMEE_CONF", "/custom/rulmee.toml");
        }
        let args = Args::parse_from(["rulmee"]);
        assert_eq!(args.conf_path, "/custom/rulmee.toml");
        unsafe {
            std::env::remove_var("RULMEE_CONF");
        }
    }

    #[test]
    fn test_cli_args_flag_precedence_over_env() {
        let _guard = config::ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("RULMEE_CONF", "/custom/rulmee.toml");
        }
        let args = Args::parse_from(["rulmee", "-c", "/cli/config.toml"]);
        assert_eq!(args.conf_path, "/cli/config.toml");
        unsafe {
            std::env::remove_var("RULMEE_CONF");
        }
    }
}
