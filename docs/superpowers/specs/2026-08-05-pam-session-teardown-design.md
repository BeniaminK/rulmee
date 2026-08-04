# Design Specification: PAM Session Teardown & Lifecycle Management

**Date:** 2026-08-05  
**Status:** Approved  
**Target Module:** `src/auth.rs`, `src/exec.rs`, `src/main.rs`, `TODO.md`  

---

## 1. Overview & Goal

Currently, the Rust rewrite of **LiDM** authenticates users and opens a PAM session via `pam_client2`, but calls `session.leak()` to release the PAM session handle without reclaiming or closing it after the desktop/shell session terminates.

This leaves `systemd-logind` sessions, UTMP/WTMP records, keyring locks, and PAM credentials hanging in memory indefinitely.

This design specification introduces proper **PAM Session Teardown & Lifecycle Management** using an RAII pattern in `AuthSession`, aligning LiDM's Rust implementation with Linux Display Manager standards (LightDM, GDM, SDDM, greetd) and restoring full feature parity with the C implementation.

---

## 2. Architecture & Control Flow

### 2.1 PAM Lifecycle Phases

```
[1. Authentication & Session Setup] (auth::authenticate)
  - pam_start(service, user)
  - pam_authenticate(flags)
  - pam_acct_mgmt(flags)
  - pam_open_session(flags)
  - pam_setcred(REINITIALIZE_CRED)
  - session.leak() -> SessionToken
  - Collect PAM environment variables
  - Wrap Context & SessionToken in AuthSession struct

[2. Session Execution] (exec::launch_session)
  - Child process forks, drops privileges (initgroups/setgid/setuid), execs session
  - Parent process blocks on waitpid(child_pid) holding AuthSession instance in scope

[3. Post-Session Teardown] (auth_session.close() / AuthSession::drop)
  - waitpid unblocks when session exits
  - context.unleak_session(SessionToken) -> Session
  - session.close(Flag::NONE) -> pam_close_session() & pam_setcred(PAM_DELETE_CRED)
  - context drop -> pam_end()
```

---

## 3. Data Structures & API Changes

### 3.1 `src/auth.rs`

Modify `AuthSession` struct to hold ownership of `Context` and `SessionToken`:

```rust
use pam_client2::{Context, Flag, SessionToken};
use pam_client2::conv_mock::Conversation;

pub struct AuthSession {
    pub username: String,
    pub env: HashMap<String, String>,
    context: Option<Context<Conversation>>,
    token: Option<SessionToken>,
}

impl AuthSession {
    /// Explicitly teardown the PAM session and release credentials.
    pub fn close(&mut self) {
        if let (Some(context), Some(token)) = (self.context.as_mut(), self.token.take()) {
            let session = context.unleak_session(token);
            let _ = session.close(Flag::NONE);
        }
        self.context = None;
    }
}

impl Drop for AuthSession {
    fn drop(&mut self) {
        self.close();
    }
}
```

Updating `auth::authenticate`:
- Store `context` and `token` in `AuthSession` upon successful authentication and session opening.

---

### 3.2 `src/main.rs`

Update the login handling loop in `main.rs` to maintain ownership of `auth_session` across `exec::launch_session`:

```rust
match auth::authenticate(&username, &password, &pam_service) {
    Ok(mut auth_session) => {
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

            // Teardown PAM session cleanly after session process finishes
            auth_session.close();
        } else {
            eprintln!("User not found in system: {}", username);
        }
    }
    Err(e) => {
        eprintln!("Auth failed: {}", e);
    }
}
```

---

## 4. Documentation & Roadmap Updates

Update [TODO.md](file:///home/beniamin/Apps/lidm/TODO.md):
- Mark `[x] **[P0] PAM Session Teardown**` as complete in Section 5.
- Update Section 3 (Comparison Matrix) and Section 4.1 to reflect full implementation of PAM session teardown in Rust.

---

## 5. Testing & Verification

1. Unit tests in `src/auth.rs` / `src/exec.rs` verifying `AuthSession` construction and safe drop behavior.
2. Run `cargo check` and `cargo test` to ensure zero regressions and clean compilation.
