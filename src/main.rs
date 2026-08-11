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
