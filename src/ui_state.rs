use strum::{EnumCount, VariantArray};
use tui_input::Input;

use crate::theme::Theme;

pub use crate::auth::PamMessage;

#[derive(
    Debug, PartialEq, Eq, Clone, Copy, strum_macros::VariantArray, strum_macros::EnumCount,
)]
pub enum Field {
    Session,
    User,
    Password,
}

impl Field {
    pub fn next(self) -> Self {
        Self::VARIANTS[(self as usize + 1) % Self::COUNT]
    }

    pub fn prev(self) -> Self {
        Self::VARIANTS[(self as usize + Self::COUNT - 1) % Self::COUNT]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginRequest {
    pub session_idx: usize,
    pub user_idx: usize,
    pub password: String,
    pub custom_session: String,
    pub custom_user: String,
}

/// Pure data holder — no logic, no methods.
/// All state mutation and query logic lives in `UIAdapter`.
pub struct UIState {
    pub selected_session_idx: usize,
    pub selected_user_idx: usize,
    pub session_input: Input,
    pub user_input: Input,
    pub password_input: Input,
    pub focused_field: Field,
    /// Whether the user is typing a custom value (vs selecting from a list)
    pub custom_session: bool,
    pub custom_user: bool,
    pub auth_error: bool,
    pub pam_messages: Vec<PamMessage>,
    pub themes: Vec<Theme>,
    pub current_theme_idx: usize,
}

impl UIState {
    /// Returns login request formatted for FIDO passwordless authentication (empty password).
    pub fn fido_login_request(&self) -> LoginRequest {
        LoginRequest {
            session_idx: self.selected_session_idx,
            user_idx: self.selected_user_idx,
            password: String::new(),
            custom_session: self.session_input.value().to_string(),
            custom_user: self.user_input.value().to_string(),
        }
    }
}
