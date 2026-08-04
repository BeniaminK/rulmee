use strum::{EnumCount, VariantArray};
use tui_input::Input;

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
}
