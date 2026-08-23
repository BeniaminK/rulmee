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

pub fn parse_env_file<P: AsRef<std::path::Path>>(path: P) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return pairs;
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = if line.starts_with("export")
            && (line.chars().nth(6) == Some(' ') || line.chars().nth(6) == Some('\t'))
        {
            line[6..].trim_start()
        } else {
            line
        };

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let mut val = val.trim();

            if key.is_empty() {
                continue;
            }

            if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
                || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
            {
                val = &val[1..val.len() - 1];
            }

            pairs.push((key.to_string(), val.to_string()));
        }
    }

    pairs
}

pub fn source_environment_files(
    env: &mut HashMap<String, String>,
    system_sources: &[String],
    home_dir: &str,
    user_sources: &[String],
) {
    for path_str in system_sources {
        let path = std::path::Path::new(path_str);
        for (k, v) in parse_env_file(path) {
            env.insert(k, v);
        }
    }

    if !home_dir.is_empty() {
        let home_path = std::path::Path::new(home_dir);
        for rel_path in user_sources {
            let path = home_path.join(rel_path);
            for (k, v) in parse_env_file(path) {
                env.insert(k, v);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentOptions<'a> {
    pub pam_env: &'a HashMap<String, String>,
    pub username: &'a str,
    pub home_dir: &'a str,
    pub shell: &'a str,
    pub session_type: &'a str,
    pub display: Option<&'a str>,
    pub desktop_names: Option<&'a str>,
    pub system_sources: &'a [String],
    pub user_sources: &'a [String],
}

pub fn assemble_environment(opts: &EnvironmentOptions) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 1. POSIX Credential Defaults
    env.insert("USER".to_string(), opts.username.to_string());
    env.insert("LOGNAME".to_string(), opts.username.to_string());
    env.insert("HOME".to_string(), opts.home_dir.to_string());
    env.insert("SHELL".to_string(), opts.shell.to_string());
    env.insert(
        "PATH".to_string(),
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
    );

    // 2. Freedesktop / XDG Standards
    env.insert("XDG_SESSION_TYPE".to_string(), opts.session_type.to_string());
    env.insert("XDG_SESSION_CLASS".to_string(), "user".to_string());
    if let Some(names) = opts.desktop_names {
        env.insert("XDG_CURRENT_DESKTOP".to_string(), names.to_string());
    }

    // 3. Merged PAM Environment
    for (k, v) in opts.pam_env {
        env.insert(k.clone(), v.clone());
    }

    // 4. Environment Profile Sourcing
    source_environment_files(&mut env, opts.system_sources, opts.home_dir, opts.user_sources);

    // 5. Optional Display Variable
    if let Some(disp) = opts.display {
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

#[derive(Debug)]
pub struct LaunchContext<'a> {
    pub user: &'a str,
    pub uid: u32,
    pub gid: u32,
    pub env: &'a HashMap<String, String>,
    pub exec_args: &'a [String],
    pub is_xorg: bool,
    pub vt: Option<c_int>,
    pub user_shell: &'a str,
    pub bypass_shell_login: bool,
}

pub fn launch_session(ctx: &LaunchContext) -> Result<(), String> {
    if ctx.is_xorg {
        launch_xorg(ctx)
    } else {
        launch_direct(ctx)
    }
}

fn launch_direct(ctx: &LaunchContext) -> Result<(), String> {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let pid = getpid();
            let _ = setpgid(pid, pid);
            if let Err(e) = drop_privileges(ctx.user, ctx.uid, ctx.gid) {
                eprintln!("Failed to drop privileges: {}", e);
                std::process::exit(1);
            }

            let (prog, args) = build_exec_command(ctx.exec_args, ctx.user_shell, ctx.bypass_shell_login);

            let mut cmd = Command::new(&prog);
            cmd.args(&args);
            cmd.envs(ctx.env);
            
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

fn launch_xorg(ctx: &LaunchContext) -> Result<(), String> {
    let vt = ctx.vt.ok_or_else(|| "Xorg requires a VT number (none provided)".to_string())?;

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

            let mut session_env = ctx.env.clone();
            session_env.insert("DISPLAY".to_string(), display);

            match unsafe { fork() } {
                Ok(ForkResult::Child) => {
                    // Session child
                    let pid = getpid();
                    let _ = setpgid(pid, pid);
                    if let Err(e) = drop_privileges(ctx.user, ctx.uid, ctx.gid) {
                        eprintln!("Failed to drop privileges: {}", e);
                        std::process::exit(1);
                    }

                    let (prog, args) = build_exec_command(ctx.exec_args, ctx.user_shell, ctx.bypass_shell_login);

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

        let opts = EnvironmentOptions {
            pam_env: &pam_env,
            username: "alice",
            home_dir: "/home/alice",
            shell: "/bin/zsh",
            session_type: "wayland",
            display: None,
            desktop_names: None,
            system_sources: &[],
            user_sources: &[],
        };
        let env = assemble_environment(&opts);

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
        let opts = EnvironmentOptions {
            pam_env: &pam_env,
            username: "bob",
            home_dir: "/home/bob",
            shell: "/bin/bash",
            session_type: "wayland",
            display: None,
            desktop_names: Some("Sway:Wayland"),
            system_sources: &[],
            user_sources: &[],
        };
        let env = assemble_environment(&opts);
        assert_eq!(env.get("XDG_CURRENT_DESKTOP").map(|s| s.as_str()), Some("Sway:Wayland"));
    }

    #[test]
    fn test_parse_env_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("lidm_test_env_parse");
        let content = r#"
# Comment line
FOO=bar
export BAR="quoted value"
export  BAZ='single quoted'
  SPACED = trimmed  
INVALID_LINE_NO_EQUALS
"#;
        std::fs::write(&file_path, content).unwrap();

        let pairs = parse_env_file(&file_path);
        let env_map: HashMap<String, String> = pairs.into_iter().collect();

        assert_eq!(env_map.get("FOO").map(|s| s.as_str()), Some("bar"));
        assert_eq!(env_map.get("BAR").map(|s| s.as_str()), Some("quoted value"));
        assert_eq!(env_map.get("BAZ").map(|s| s.as_str()), Some("single quoted"));
        assert_eq!(env_map.get("SPACED").map(|s| s.as_str()), Some("trimmed"));
        assert_eq!(env_map.get("INVALID_LINE_NO_EQUALS"), None);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_source_environment_files_order_and_precedence() {
        let temp_dir = std::env::temp_dir();
        let sys_file = temp_dir.join("lidm_test_sys_profile");
        let user_dir = temp_dir.join("lidm_test_user_home");
        let _ = std::fs::create_dir_all(&user_dir);
        let user_file = user_dir.join(".xprofile");

        std::fs::write(&sys_file, "SYS_VAR=system\nOVERRIDE_VAR=system_val\n").unwrap();
        std::fs::write(&user_file, "USER_VAR=user\nOVERRIDE_VAR=user_val\n").unwrap();

        let mut env = HashMap::new();
        source_environment_files(
            &mut env,
            &[sys_file.to_string_lossy().to_string()],
            &user_dir.to_string_lossy(),
            &[".xprofile".to_string()],
        );

        assert_eq!(env.get("SYS_VAR").map(|s| s.as_str()), Some("system"));
        assert_eq!(env.get("USER_VAR").map(|s| s.as_str()), Some("user"));
        assert_eq!(env.get("OVERRIDE_VAR").map(|s| s.as_str()), Some("user_val"));

        let _ = std::fs::remove_file(sys_file);
        let _ = std::fs::remove_file(user_file);
        let _ = std::fs::remove_dir(user_dir);
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

