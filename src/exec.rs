use std::ffi::c_int;
use std::os::unix::io::AsRawFd;
use nix::unistd::{fork, ForkResult, setuid, setgid, initgroups, Uid, Gid, close, read};
use std::ffi::CString;
use std::collections::HashMap;
use std::process::Command;
use std::os::unix::process::CommandExt;

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
