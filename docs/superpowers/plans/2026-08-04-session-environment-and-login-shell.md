# Session Environment & Login Shell Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement full session environment assembly (PAM + POSIX + XDG + DISPLAY) and login shell execution delegation (`$SHELL -l -c "exec <cmd>"`) based on `config.behavior.bypass_shell_login`.

**Architecture:** Environment assembly and login shell vector generation will be placed in `src/exec.rs` and `src/auth.rs`. `auth::authenticate` extracts PAM environment and user credentials, `exec::assemble_environment` merges POSIX/XDG/PAM variables, and `exec::build_exec_command` constructs the shell execution vector before privilege dropping.

**Tech Stack:** Rust 2021, `pam-client2`, `nix`, `uzers`, `libc`, `std::process::Command`.

## Global Constraints

- Preserve PAM environment variables and merge them with POSIX defaults (`USER`, `LOGNAME`, `HOME`, `SHELL`, `PATH`).
- When `config.behavior.bypass_shell_login` is `false` (default), session processes must execute via `[user_shell, "-l", "-c", "exec <cmd>"]`.
- When `config.behavior.bypass_shell_login` is `true`, session processes execute directly (`exec_args`).
- All tests must pass via `cargo test`.

---

### Task 1: Environment Assembly and Shell Command Builder in `src/exec.rs`

**Files:**
- Modify: `src/exec.rs`
- Test: `src/exec.rs` (unit tests module)

**Interfaces:**
- Consumes: `user`, `uid`, `gid`, `pam_env: HashMap<String, String>`, `exec_args: &[String]`, `bypass_shell_login: bool`, `session_type: &str`, `display: Option<&str>`
- Produces: 
  - `pub fn assemble_environment(pam_env: &HashMap<String, String>, username: &str, home_dir: &str, shell: &str, session_type: &str, display: Option<&str>) -> HashMap<String, String>`
  - `pub fn build_exec_command(exec_args: &[String], user_shell: &str, bypass_shell_login: bool) -> (String, Vec<String>)`

- [ ] **Step 1: Write failing unit tests for `assemble_environment` and `build_exec_command`**

Add at the bottom of `src/exec.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test exec::tests`
Expected: Compilation failure because `assemble_environment` and `build_exec_command` are missing.

- [ ] **Step 3: Implement `assemble_environment` and `build_exec_command`**

Implement in `src/exec.rs`:

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test exec::tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/exec.rs
git commit -m "feat: add assemble_environment and build_exec_command in exec.rs"
```

---

### Task 2: Integrate Login Shell Execution in `exec.rs` and `main.rs`

**Files:**
- Modify: `src/exec.rs:1-134`
- Modify: `src/main.rs:145-168`

**Interfaces:**
- Consumes: `bypass_shell_login: bool`, `user_shell: &str`, `session_type: &str`
- Updates: `launch_session`, `launch_direct`, `launch_xorg` in `src/exec.rs` to accept `user_shell` and `bypass_shell_login`.

- [ ] **Step 1: Update signatures and implementation of `launch_session`, `launch_direct`, and `launch_xorg`**

Update `src/exec.rs`:

```rust
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
            let user_cstr = CString::new(user).unwrap();
            initgroups(&user_cstr, Gid::from_raw(gid)).unwrap();
            setgid(Gid::from_raw(gid)).unwrap();
            setuid(Uid::from_raw(uid)).unwrap();

            let (prog, args) = build_exec_command(exec_args, user_shell, bypass_shell_login);

            let mut cmd = Command::new(&prog);
            cmd.args(&args);
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
    user_shell: &str,
    bypass_shell_login: bool,
) -> Result<(), String> {
    let vt = vt.ok_or_else(|| "Xorg requires a VT number (none provided)".to_string())?;

    let (pipe_read, pipe_write) = nix::unistd::pipe().map_err(|e| format!("Pipe failed: {}", e))?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            close(pipe_read.as_raw_fd()).unwrap();
            let display_fd = pipe_write.as_raw_fd();

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
                    let user_cstr = CString::new(user).unwrap();
                    initgroups(&user_cstr, Gid::from_raw(gid)).unwrap();
                    setgid(Gid::from_raw(gid)).unwrap();
                    setuid(Uid::from_raw(uid)).unwrap();

                    let (prog, args) = build_exec_command(exec_args, user_shell, bypass_shell_login);

                    let mut cmd = Command::new(&prog);
                    cmd.args(&args);
                    cmd.envs(session_env);

                    let err = cmd.exec();
                    eprintln!("Failed to exec session: {}", err);
                    std::process::exit(1);
                }
                Ok(ForkResult::Parent { child: session_pid }) => {
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
```

- [ ] **Step 2: Update `main.rs` to call `assemble_environment` and pass `bypass_shell_login`**

Update `src/main.rs` inside the login block:

```rust
                let pam_service =
                    std::env::var("LIDM_PAM_SERVICE").unwrap_or_else(|_| "login".to_string());
                match auth::authenticate(&username, &password, &pam_service) {
                    Ok(auth_session) => {
                        let home_dir = uzers::get_user_by_name(&username)
                            .map(|u| u.home_dir().to_string_lossy().into_owned())
                            .unwrap_or_else(|| format!("/home/{}", username));

                        let uid = uzers::get_user_by_name(&username).map(|u| u.uid()).unwrap_or(1000);
                        let gid = uzers::get_user_by_name(&username).map(|u| u.primary_group_id()).unwrap_or(1000);

                        let session_type_str = if is_xorg { "x11" } else { "wayland" };

                        let env = exec::assemble_environment(
                            &auth_session.env,
                            &username,
                            &home_dir,
                            &shell,
                            session_type_str,
                            None,
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
                            config.behavior.bypass_shell_login,
                        ) {
                            eprintln!("Failed to launch session: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Auth failed: {}", e);
                    }
                }
```

- [ ] **Step 3: Run `cargo check` and `cargo test` to verify build and tests pass**

Run: `cargo test`
Expected: All unit tests pass cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/exec.rs src/main.rs
git commit -m "feat: assemble session environment and delegate session launch to login shell"
```
