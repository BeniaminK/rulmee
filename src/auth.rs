use std::collections::HashMap;
use pam_client2::{Context, Flag, SessionToken};
use pam_client2::conv_mock::Conversation;

pub struct AuthSession {
    pub username: String,
    pub env: HashMap<String, String>,
    context: Option<Context<Conversation>>,
    token: Option<SessionToken>,
}

impl AuthSession {
    pub fn close(&mut self) {
        if let (Some(mut context), Some(token)) = (self.context.take(), self.token.take()) {
            let session = context.unleak_session(token);
            let _ = session.close(Flag::NONE);
        }
    }
}

impl Drop for AuthSession {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn authenticate(user: &str, password: &str, service: &str) -> Result<AuthSession, String> {
    // 1. Initialize PAM context with a non-interactive conversation handler
    let mut context = Context::new(
        service,
        Some(user),
        Conversation::with_credentials(user, password),
    ).map_err(|e| format!("pam_start failed: {}", e))?;

    // 2. Authenticate the user
    context.authenticate(Flag::NONE)
        .map_err(|e| format!("pam_authenticate failed: {}", e))?;

    // 3. Account management (check for expired accounts, etc.)
    context.acct_mgmt(Flag::NONE)
        .map_err(|e| format!("pam_acct_mgmt failed: {}", e))?;

    // 4. Open session and establish credentials
    // Note: open_session in pam-client2 automatically calls pam_setcred(PAM_ESTABLISH_CRED)
    let mut session = context.open_session(Flag::NONE)
        .map_err(|e| format!("pam_open_session failed: {}", e))?;

    session.reinitialize_credentials(Flag::NONE)
        .map_err(|e| format!("pam_setcred(REINITIALIZE) failed: {}", e))?;

    // 6. Leak the session to keep it open
    // In display managers, the session usually stays open until the child process (desktop session) terminates.
    // The C version calls pam_end in the parent AFTER waitpid.
    // By leaking the session, we prevent it from being closed when this function returns.
    // This also ends the mutable borrow of the context.
    let token = session.leak();

    // 7. Extract environment variables
    let mut env = HashMap::new();
    for (key, value) in context.envlist().iter_tuples() {
        env.insert(
            key.to_string_lossy().into_owned(),
            value.to_string_lossy().into_owned(),
        );
    }

    Ok(AuthSession {
        username: user.to_string(),
        env,
        context: Some(context),
        token: Some(token),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_session_teardown_safely_handles_none() {
        let mut session = AuthSession {
            username: "testuser".to_string(),
            env: HashMap::new(),
            context: None,
            token: None,
        };
        session.close();
        assert!(session.context.is_none());
        assert!(session.token.is_none());
    }
}

