use std::ffi::c_int;
use std::os::unix::io::AsRawFd;
use nix::unistd::{fork, ForkResult, setuid, setgid, initgroups, Uid, Gid, close, read};
use std::ffi::CString;
use std::collections::HashMap;
use std::process::Command;
use std::os::unix::process::CommandExt;

pub fn assemble_environment(
    pam_env: &HashMap<String, String>,
    username: &str,
    home_dir: &str,
    shell: &str,
    session_type: &str,
    display: Option<&str>,
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

pub fn build_exec_command(
    exec_args: &[String],
    user_shell: &str,
    bypass_shell_login: bool,
) -> (String, Vec<String>) {
    if exec_args.is_empty() {
        return (user_shell.to_string(), vec!["-l".to_string()]);
    }

    if bypass_shell_login {
        (exec_args[0].clone(), exec_args[1..].to_vec())
    } else {
        let full_cmd = exec_args.join(" ");
        let shell = if user_shell.is_empty() { "/bin/bash" } else { user_shell };
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
) -> Result<(), String> {
    if is_xorg {
        launch_xorg(user, uid, gid, env, exec_args, vt)
    } else {
        launch_direct(user, uid, gid, env, exec_args)
    }
}

fn launch_direct(
    user: &str,
    uid: u32,
    gid: u32,
    env: &HashMap<String, String>,
    exec_args: &[String],
) -> Result<(), String> {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Drop privileges
            let user_cstr = CString::new(user).unwrap();
            initgroups(&user_cstr, Gid::from_raw(gid)).unwrap();
            setgid(Gid::from_raw(gid)).unwrap();
            setuid(Uid::from_raw(uid)).unwrap();

            let mut cmd = Command::new(&exec_args[0]);
            cmd.args(&exec_args[1..]);
            cmd.envs(env);
            
            let err = cmd.exec();
            eprintln!("Failed to exec: {}", err);
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child }) => {
            let mut status: i32 = 0;
            unsafe { libc::waitpid(child.as_raw(), &mut status, 0) };
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
) -> Result<(), String> {
    let vt = vt.ok_or_else(|| "Xorg requires a VT number (none provided)".to_string())?;

    // Create pipe for displayfd
    let (pipe_read, pipe_write) = nix::unistd::pipe().map_err(|e| format!("Pipe failed: {}", e))?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Xorg server child
            close(pipe_read.as_raw_fd()).unwrap();
            let display_fd = pipe_write.as_raw_fd();
            
            // Xorg needs to run as root usually, or has its own setuid
            // but lidm C version forks and then runs start_xorg_server
            // start_xorg_server does NOT drop privileges before execle(xorg_path, ...)
            
            let mut cmd = Command::new("Xorg");
            cmd.arg("-displayfd").arg(display_fd.to_string());
            cmd.arg(format!("vt{}", vt));
            
            let err = cmd.exec();
            eprintln!("Failed to exec Xorg: {}", err);
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child: xorg_pid }) => {
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
                    let user_cstr = CString::new(user).unwrap();
                    initgroups(&user_cstr, Gid::from_raw(gid)).unwrap();
                    setgid(Gid::from_raw(gid)).unwrap();
                    setuid(Uid::from_raw(uid)).unwrap();

                    let mut cmd = Command::new(&exec_args[0]);
                    cmd.args(&exec_args[1..]);
                    cmd.envs(session_env);
                    
                    let err = cmd.exec();
                    eprintln!("Failed to exec session: {}", err);
                    std::process::exit(1);
                }
                Ok(ForkResult::Parent { child: session_pid }) => {
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

        let env = assemble_environment(&pam_env, "alice", "/home/alice", "/bin/zsh", "wayland", None);

        assert_eq!(env.get("USER").map(|s| s.as_str()), Some("alice"));
        assert_eq!(env.get("LOGNAME").map(|s| s.as_str()), Some("alice"));
        assert_eq!(env.get("HOME").map(|s| s.as_str()), Some("/home/alice"));
        assert_eq!(env.get("SHELL").map(|s| s.as_str()), Some("/bin/zsh"));
        assert_eq!(env.get("XDG_SESSION_TYPE").map(|s| s.as_str()), Some("wayland"));
        assert_eq!(env.get("XDG_SESSION_CLASS").map(|s| s.as_str()), Some("user"));
        assert_eq!(env.get("PAM_VAR").map(|s| s.as_str()), Some("pam_val"));
        assert_eq!(env.get("PATH").map(|s| s.as_str()), Some("/custom/path"));
    }

    #[test]
    fn test_build_exec_command_bypass_login_false() {
        let exec_args = vec!["sway".to_string(), "--unsupported-gpu".to_string()];
        let (prog, args) = build_exec_command(&exec_args, "/bin/bash", false);

        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-l", "-c", "exec sway --unsupported-gpu"]);
    }

    #[test]
    fn test_build_exec_command_bypass_login_true() {
        let exec_args = vec!["sway".to_string(), "--unsupported-gpu".to_string()];
        let (prog, args) = build_exec_command(&exec_args, "/bin/bash", true);

        assert_eq!(prog, "sway");
        assert_eq!(args, vec!["--unsupported-gpu"]);
    }
}

