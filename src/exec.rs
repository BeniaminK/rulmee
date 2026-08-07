use std::ffi::c_int;
use std::os::unix::io::AsRawFd;
use nix::unistd::{fork, ForkResult, setuid, setgid, initgroups, setpgid, getpid, Uid, Gid, close, read};
use std::ffi::CString;
use std::collections::HashMap;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicI32, Ordering};

pub static ACTIVE_CHILD_PGID: AtomicI32 = AtomicI32::new(0);

pub fn get_active_child_pgid() -> i32 {
    ACTIVE_CHILD_PGID.load(Ordering::SeqCst)
}

pub fn set_active_child_pgid(pgid: i32) {
    ACTIVE_CHILD_PGID.store(pgid, Ordering::SeqCst);
}

pub fn drop_privileges(user: &str, uid: u32, gid: u32) -> Result<(), String> {
    let user_cstr = CString::new(user).map_err(|e| format!("Invalid username string: {}", e))?;
    initgroups(&user_cstr, Gid::from_raw(gid)).map_err(|e| format!("initgroups failed: {}", e))?;
    setgid(Gid::from_raw(gid)).map_err(|e| format!("setgid failed: {}", e))?;
    setuid(Uid::from_raw(uid)).map_err(|e| format!("setuid failed: {}", e))?;
    Ok(())
}

pub fn assemble_environment(
    pam_env: &HashMap<String, String>,
    username: &str,
    home_dir: &str,
    shell: &str,
    session_type: &str,
    display: Option<&str>,
    desktop_names: Option<&str>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 1. POSIX Credential Defaults
    env.insert("USER".to_string(), username.to_string());
    env.insert("LOGNAME".to_string(), username.to_string());
    env.insert("HOME".to_string(), home_dir.to_string());
    env.insert("SHELL".to_string(), shell.to_string());
    env.insert(
        "PATH".to_string(),
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
    );

    // 2. Freedesktop / XDG Standards
    env.insert("XDG_SESSION_TYPE".to_string(), session_type.to_string());
    env.insert("XDG_SESSION_CLASS".to_string(), "user".to_string());
    if let Some(names) = desktop_names {
        env.insert("XDG_CURRENT_DESKTOP".to_string(), names.to_string());
    }

    // 3. Merged PAM Environment
    for (k, v) in pam_env {
        env.insert(k.clone(), v.clone());
    }

    // 4. Optional Display Variable
    if let Some(disp) = display {
        env.insert("DISPLAY".to_string(), disp.to_string());
    }

    env
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn build_exec_command(
    exec_args: &[String],
    user_shell: &str,
    bypass_shell_login: bool,
) -> (String, Vec<String>) {
    let shell = if user_shell.is_empty() { "/bin/bash" } else { user_shell };

    if exec_args.is_empty() {
        return (shell.to_string(), vec!["-l".to_string()]);
    }

    if bypass_shell_login {
        (exec_args[0].clone(), exec_args[1..].to_vec())
    } else {
        let quoted_args: Vec<String> = exec_args.iter().map(|arg| shell_quote(arg)).collect();
        let full_cmd = quoted_args.join(" ");
        (
            shell.to_string(),
            vec!["-l".to_string(), "-c".to_string(), format!("exec {}", full_cmd)],
        )
    }
}

pub fn launch_session(
    user: &str,
    uid: u32,
    gid: u32,
    env: &HashMap<String, String>,
    exec_args: &[String],
    is_xorg: bool,
    vt: Option<c_int>,
    user_shell: &str,
    bypass_shell_login: bool,
) -> Result<(), String> {
    if is_xorg {
        launch_xorg(user, uid, gid, env, exec_args, vt, user_shell, bypass_shell_login)
    } else {
        launch_direct(user, uid, gid, env, exec_args, user_shell, bypass_shell_login)
    }
}

fn launch_direct(
    user: &str,
    uid: u32,
    gid: u32,
    env: &HashMap<String, String>,
    exec_args: &[String],
    user_shell: &str,
    bypass_shell_login: bool,
) -> Result<(), String> {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let pid = getpid();
            let _ = setpgid(pid, pid);
            if let Err(e) = drop_privileges(user, uid, gid) {
                eprintln!("Failed to drop privileges: {}", e);
                std::process::exit(1);
            }

            let (prog, args) = build_exec_command(exec_args, user_shell, bypass_shell_login);

            let mut cmd = Command::new(&prog);
            cmd.args(&args);
            cmd.envs(env);
            
            let err = cmd.exec();
            eprintln!("Failed to exec: {}", err);
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child }) => {
            let _ = setpgid(child, child);
            set_active_child_pgid(child.as_raw());
            let mut status: i32 = 0;
            unsafe { libc::waitpid(child.as_raw(), &mut status, 0) };
            set_active_child_pgid(0);
            Ok(())
        }
        Err(e) => Err(format!("Fork failed: {}", e)),
    }
}

fn launch_xorg(
    user: &str,
    uid: u32,
    gid: u32,
    env: &HashMap<String, String>,
    exec_args: &[String],
    vt: Option<c_int>,
    user_shell: &str,
    bypass_shell_login: bool,
) -> Result<(), String> {
    let vt = vt.ok_or_else(|| "Xorg requires a VT number (none provided)".to_string())?;

    // Create pipe for displayfd
    let (pipe_read, pipe_write) = nix::unistd::pipe().map_err(|e| format!("Pipe failed: {}", e))?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Xorg server child
            close(pipe_read.as_raw_fd()).unwrap();
            let display_fd = pipe_write.as_raw_fd();
            let pid = getpid();
            let _ = setpgid(pid, pid);
            
            let mut cmd = Command::new("Xorg");
            cmd.arg("-displayfd").arg(display_fd.to_string());
            cmd.arg(format!("vt{}", vt));
            
            let err = cmd.exec();
            eprintln!("Failed to exec Xorg: {}", err);
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child: xorg_pid }) => {
            let _ = setpgid(xorg_pid, xorg_pid);
            close(pipe_write.as_raw_fd()).unwrap();
            let mut display_buf = [0u8; 16];
            let n = read(pipe_read.as_raw_fd(), &mut display_buf).map_err(|e| format!("Read pipe failed: {}", e))?;
            let display_str = std::str::from_utf8(&display_buf[..n]).unwrap().trim();
            let display = format!(":{}", display_str);

            let mut session_env = env.clone();
            session_env.insert("DISPLAY".to_string(), display);

            match unsafe { fork() } {
                Ok(ForkResult::Child) => {
                    // Session child
                    let pid = getpid();
                    let _ = setpgid(pid, pid);
                    if let Err(e) = drop_privileges(user, uid, gid) {
                        eprintln!("Failed to drop privileges: {}", e);
                        std::process::exit(1);
                    }

                    let (prog, args) = build_exec_command(exec_args, user_shell, bypass_shell_login);

                    let mut cmd = Command::new(&prog);
                    cmd.args(&args);
                    cmd.envs(session_env);
                    
                    let err = cmd.exec();
                    eprintln!("Failed to exec session: {}", err);
                    std::process::exit(1);
                }
                Ok(ForkResult::Parent { child: session_pid }) => {
                    let _ = setpgid(session_pid, session_pid);
                    set_active_child_pgid(session_pid.as_raw());
                    // Wait for either Xorg or Session to die
                    let mut status: i32 = 0;
                    loop {
                        let pid = unsafe { libc::waitpid(-1, &mut status, 0) };
                        if pid == xorg_pid.as_raw() || pid == session_pid.as_raw() {
                            let to_kill = if pid == xorg_pid.as_raw() { session_pid } else { xorg_pid };
                            let _ = nix::sys::signal::kill(to_kill, nix::sys::signal::Signal::SIGTERM);
                            unsafe { libc::waitpid(to_kill.as_raw(), &mut status, 0) };
                            break;
                        }
                    }
                    set_active_child_pgid(0);
                    Ok(())
                }
                Err(e) => Err(format!("Fork failed: {}", e)),
            }
        }
        Err(e) => Err(format!("Fork failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_assemble_environment_merging() {
        let mut pam_env = HashMap::new();
        pam_env.insert("PAM_VAR".to_string(), "pam_val".to_string());
        pam_env.insert("PATH".to_string(), "/custom/path".to_string());

        let env = assemble_environment(&pam_env, "alice", "/home/alice", "/bin/zsh", "wayland", None, None);

        assert_eq!(env.get("USER").map(|s| s.as_str()), Some("alice"));
        assert_eq!(env.get("LOGNAME").map(|s| s.as_str()), Some("alice"));
        assert_eq!(env.get("HOME").map(|s| s.as_str()), Some("/home/alice"));
        assert_eq!(env.get("SHELL").map(|s| s.as_str()), Some("/bin/zsh"));
        assert_eq!(env.get("XDG_SESSION_TYPE").map(|s| s.as_str()), Some("wayland"));
        assert_eq!(env.get("XDG_SESSION_CLASS").map(|s| s.as_str()), Some("user"));
        assert_eq!(env.get("PAM_VAR").map(|s| s.as_str()), Some("pam_val"));
        assert_eq!(env.get("PATH").map(|s| s.as_str()), Some("/custom/path"));
        assert_eq!(env.get("XDG_CURRENT_DESKTOP"), None);
    }

    #[test]
    fn test_assemble_environment_xdg_current_desktop() {
        let pam_env = HashMap::new();
        let env = assemble_environment(
            &pam_env,
            "bob",
            "/home/bob",
            "/bin/bash",
            "wayland",
            None,
            Some("Sway:Wayland"),
        );
        assert_eq!(env.get("XDG_CURRENT_DESKTOP").map(|s| s.as_str()), Some("Sway:Wayland"));
    }

    #[test]
    fn test_build_exec_command_bypass_login_false() {
        let exec_args = vec!["sway".to_string(), "--unsupported-gpu".to_string()];
        let (prog, args) = build_exec_command(&exec_args, "/bin/bash", false);

        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-l", "-c", "exec 'sway' '--unsupported-gpu'"]);
    }

    #[test]
    fn test_build_exec_command_bypass_login_true() {
        let exec_args = vec!["sway".to_string(), "--unsupported-gpu".to_string()];
        let (prog, args) = build_exec_command(&exec_args, "/bin/bash", true);

        assert_eq!(prog, "sway");
        assert_eq!(args, vec!["--unsupported-gpu"]);
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote("sway"), "'sway'");
        assert_eq!(shell_quote("my config.conf"), "'my config.conf'");
        assert_eq!(shell_quote("don't stop"), "'don'\\''t stop'");
    }

    #[test]
    fn test_build_exec_command_quoting_and_empty_shell() {
        let exec_args = vec!["sway".to_string(), "--config".to_string(), "my config.conf".to_string()];
        let (prog, args) = build_exec_command(&exec_args, "", false);

        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-l", "-c", "exec 'sway' '--config' 'my config.conf'"]);
    }

    #[test]
    fn test_active_child_pgid_state() {
        assert_eq!(get_active_child_pgid(), 0);
        set_active_child_pgid(1234);
        assert_eq!(get_active_child_pgid(), 1234);
        set_active_child_pgid(0);
        assert_eq!(get_active_child_pgid(), 0);
    }

    #[test]
    fn test_drop_privileges_invalid_user_nul_byte() {
        let result = drop_privileges("user\0invalid", 1000, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid username string"));
    }
}

