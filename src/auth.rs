use std::collections::HashMap;
use std::ffi::{CStr, CString};
use pam_client2::{Context, ConversationHandler, ErrorCode, Flag, SessionToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PamMessageType {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PamMessage {
    pub msg_type: PamMessageType,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthError {
    pub message: String,
    pub pam_messages: Vec<PamMessage>,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthError {}

pub struct CapturingConversation {
    pub username: String,
    pub password: String,
    pub messages: Vec<PamMessage>,
}

impl CapturingConversation {
    pub fn with_credentials(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            messages: Vec::new(),
        }
    }
}

impl ConversationHandler for CapturingConversation {
    fn init(&mut self, default_user: Option<&str>) {
        if let Some(user) = default_user {
            if self.username.is_empty() {
                self.username = user.to_string();
            }
        }
    }

    fn prompt_echo_on(&mut self, _prompt: &CStr) -> Result<CString, ErrorCode> {
        CString::new(self.username.clone()).map_err(|_| ErrorCode::CONV_ERR)
    }

    fn prompt_echo_off(&mut self, _prompt: &CStr) -> Result<CString, ErrorCode> {
        CString::new(self.password.clone()).map_err(|_| ErrorCode::CONV_ERR)
    }

    fn text_info(&mut self, msg: &CStr) {
        let text = msg.to_string_lossy().into_owned();
        self.messages.push(PamMessage {
            msg_type: PamMessageType::Info,
            message: text,
        });
    }

    fn error_msg(&mut self, msg: &CStr) {
        let text = msg.to_string_lossy().into_owned();
        self.messages.push(PamMessage {
            msg_type: PamMessageType::Error,
            message: text,
        });
    }
}

#[allow(dead_code)]
pub struct AuthSession {
    pub username: String,
    pub env: HashMap<String, String>,
    pub messages: Vec<PamMessage>,
    context: Option<Context<CapturingConversation>>,
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

pub fn authenticate(user: &str, password: &str, service: &str) -> Result<AuthSession, AuthError> {
    // 1. Initialize PAM context with capturing conversation handler
    let mut context = Context::new(
        service,
        Some(user),
        CapturingConversation::with_credentials(user, password),
    )
    .map_err(|e| AuthError {
        message: format!("pam_start failed: {}", e),
        pam_messages: Vec::new(),
    })?;

    // 2. Authenticate the user
    if let Err(e) = context.authenticate(Flag::NONE) {
        return Err(AuthError {
            message: format!("pam_authenticate failed: {}", e),
            pam_messages: context.conversation().messages.clone(),
        });
    }

    // 3. Account management (check for expired accounts, etc.)
    if let Err(e) = context.acct_mgmt(Flag::NONE) {
        let msg = format!("pam_acct_mgmt failed: {}", e);
        drop(e);
        return Err(AuthError {
            message: msg,
            pam_messages: context.conversation().messages.clone(),
        });
    }

    // 4. Open session and establish credentials
    let (token, open_err) = match context.open_session(Flag::NONE) {
        Ok(mut session) => {
            if let Err(e) = session.reinitialize_credentials(Flag::NONE) {
                let msg = format!("pam_setcred(REINITIALIZE) failed: {}", e);
                drop(session);
                (None, Some(msg))
            } else {
                (Some(session.leak()), None)
            }
        }
        Err(e) => (None, Some(format!("pam_open_session failed: {}", e))),
    };

    if let Some(msg) = open_err {
        let pam_messages = context.conversation().messages.clone();
        return Err(AuthError {
            message: msg,
            pam_messages,
        });
    }
    let token = token.unwrap();

    // 7. Extract environment variables
    let mut env = HashMap::new();
    for (key, value) in context.envlist().iter_tuples() {
        env.insert(
            key.to_string_lossy().into_owned(),
            value.to_string_lossy().into_owned(),
        );
    }

    let messages = context.conversation().messages.clone();

    Ok(AuthSession {
        username: user.to_string(),
        env,
        messages,
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
            messages: Vec::new(),
            context: None,
            token: None,
        };
        session.close();
        assert!(session.context.is_none());
        assert!(session.token.is_none());
    }

    #[test]
    fn test_capturing_conversation_records_messages() {
        let mut conv = CapturingConversation::with_credentials("testuser", "secret");
        let info_msg = CString::new("Welcome user").unwrap();
        let err_msg = CString::new("Password about to expire").unwrap();

        conv.text_info(&info_msg);
        conv.error_msg(&err_msg);

        assert_eq!(conv.messages.len(), 2);
        assert_eq!(
            conv.messages[0],
            PamMessage {
                msg_type: PamMessageType::Info,
                message: "Welcome user".to_string(),
            }
        );
        assert_eq!(
            conv.messages[1],
            PamMessage {
                msg_type: PamMessageType::Error,
                message: "Password about to expire".to_string(),
            }
        );
    }
}


