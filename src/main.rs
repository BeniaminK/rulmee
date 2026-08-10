mod auth;
mod colors;
mod config;
mod console;
mod exec;
mod logging;
mod macros;
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
use clap::Parser;
use log::{debug, error, info, warn};
use std::ffi::c_int;
use uzers::os::unix::UserExt;

#[derive(Parser, Debug)]
#[command(
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (git ",
        env!("LIDM_GIT_REV"),
        ", build date ",
        env!("LIDM_BUILD_TS"),
        ", compiler ",
        env!("LIDM_COMPILER_VER"),
        ")"
    ),
    about = "LiDM: Lightweight Display Manager"
)]
pub struct Args {
    #[arg(help = "VT number to switch to", env = "LIDM_VT")]
    pub vt: Option<c_int>,

    #[arg(long = "logging-file", help = "Path to log file", env = "LIDM_LOGGING_FILE")]
    pub logging_file: Option<String>,

    #[arg(long = "logging-level", help = "Log level filter", env = "LIDM_LOGGING_LEVEL")]
    pub logging_level: Option<String>,

    #[arg(long = "logging-stdout", help = "Enable stdout logging", env = "LIDM_LOGGING_STDOUT")]
    pub logging_stdout: bool,

    #[arg(long = "auth-pam-service", help = "PAM service name", env = "LIDM_AUTH_PAM_SERVICE")]
    pub auth_pam_service: Option<String>,

    #[arg(
        short = 'c',
        long = "config",
        env = "LIDM_CONF",
        default_value = "/etc/lidm.ini",
        help = "Path to configuration file"
    )]
    pub conf_path: String,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = signal_handler::setup_signal_handler() {
        eprintln!("Warning: Failed to setup signal handler: {}", e);
    }

    let console_buffer: console::ConsoleBuffer = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::with_capacity(50)));

    let initial_config = match config::Config::load(&args) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading config from {}: {}", args.conf_path, e);
            std::process::exit(1);
        }
    };

    let _log_guard = match logging::initialize_logging(&initial_config.logging, Some(console_buffer.clone())) {
        Ok(guard) => Some(guard),
        Err(e) => {
            eprintln!("Failed to initialize logging: {}", e);
            None
        }
    };

    match args.vt {
        Some(vt) => match vt::chvt(vt) {
            Err(e) => {
                warn!("Warning: Could not switch to VT {}: {}", vt, e);
            }
            _ => (),
        },
        None => (),
    }

    let mut pam_messages = Vec::new();
    let mut auth_failed = false;

    loop {
        // Load config
        info!("Loading configuration from: {}", args.conf_path);
        let config = match config::Config::load(&args) {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Error loading config from {}: {}", args.conf_path, e);
                std::process::exit(1);
            }
        };

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
        let ui_console_buffer = if config.behavior.show_console {
            Some(console_buffer.clone())
        } else {
            None
        };

        let mut ui = UI::new(
            config.clone(),
            sessions.clone(),
            users.clone(),
            initial_user,
            initial_session,
            ui_console_buffer,
            pam_messages.clone(),
            auth_failed,
        );

        let ui_result = ui.run();

        let login_data = match &ui_result {
            Ok(UIResult::Login(s, u, p, cs, cu)) => Some((*s, *u, p.clone(), cs.clone(), cu.clone())),
            _ => None,
        };

        if let Some((session_idx, user_idx, password, custom_session, custom_user)) = login_data {
            let (username, shell) = if user_idx < users.len() && custom_user.is_empty() {
                    (
                        users[user_idx].username.clone(),
                        users[user_idx].shell.clone(),
                    )
                } else {
                    (custom_user, "/bin/bash".to_string())
                };

                let (session_name, exec_args, is_xorg, desktop_names) =
                    if session_idx < sessions.len() && custom_session.is_empty() {
                        let s = &sessions[session_idx];
                        let args = match &s.exec {
                            session::ExecType::Shell(sh) => vec![sh.clone()],
                            session::ExecType::Desktop(args) => args.clone(),
                        };
                        (
                            s.name.clone(),
                            args,
                            s.session_type == SessionType::Xorg,
                            s.desktop_names.clone(),
                        )
                    } else if session_idx == sessions.len() && custom_session.is_empty() {
                        (shell.clone(), vec![shell.clone()], false, None)
                    } else {
                        (custom_session.clone(), vec![custom_session], false, None)
                    };

                let _ = launch_state::write_launch_state(&launch_state::LaunchState {
                    username: username.clone(),
                    session_opt: session_name.clone(),
                });

                // Perform authentication
                match auth::authenticate(&username, &password, &config.auth.pam_service) {
                    Ok(mut auth_session) => {
                        pam_messages.clear();
                        auth_failed = false;
                        if let Some(u) = uzers::get_user_by_name(&username) {
                            let home_dir = u.home_dir().to_string_lossy().into_owned();
                            let uid = u.uid();
                            let gid = u.primary_group_id();
                            let session_type_str = if is_xorg { "x11" } else { "wayland" };

                            let env = exec::assemble_environment(
                                &auth_session.env,
                                &username,
                                &home_dir,
                                &shell,
                                session_type_str,
                                None,
                                desktop_names.as_deref(),
                                &config.behavior.source,
                                &config.behavior.user_source,
                            );

                            if let Err(e) = exec::launch_session(
                                &username,
                                uid,
                                gid,
                                &env,
                                &exec_args,
                                is_xorg,
                                args.vt,
                                &shell,
                                bypass_shell_login,
                            ) {
                                eprintln!("Failed to launch session: {}", e);
                            }

                            // Teardown PAM session cleanly after session process completes
                            auth_session.close();
                        } else {
                            eprintln!("User not found in system: {}", username);
                        }
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
        } else {
            match ui_result {
                Ok(UIResult::Poweroff) => {
                    unsafe {
                        libc::reboot(libc::RB_POWER_OFF);
                    }
                    std::process::exit(0);
                }
                Ok(UIResult::Reboot) => {
                    unsafe {
                        libc::reboot(libc::RB_AUTOBOOT);
                    }
                    std::process::exit(0);
                }
                Ok(UIResult::Refresh) => {
                    continue; // Reload everything
                }
                Ok(UIResult::Exit) => break,
                Err(e) => {
                    eprintln!("UI Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }
}
