use crate::config::Config;
use crate::console::ConsoleBuffer;
use crate::session::{Session, SessionType};
use crate::ui_state::{Field, PamMessage, UIState};
use crate::users::LocalUser;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::style::Style;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Poweroff,
    Reboot,
    Refresh,
    Fido,
    Theme,
}

pub struct UIAdapter {
    pub(crate) config: Config,
    sessions: Vec<Session>,
    users: Vec<LocalUser>,
    state: UIState,
    console_buffer: Option<ConsoleBuffer>,
}

impl UIAdapter {
    pub fn new(
        config: Config,
        sessions: Vec<Session>,
        users: Vec<LocalUser>,
        initial_user: Option<&str>,
        initial_session: Option<&str>,
        console_buffer: Option<ConsoleBuffer>,
        pam_messages: Vec<PamMessage>,
        auth_error: bool,
    ) -> Self {
        let include_defshell = config.behavior.include_defshell;

        // Resolve initial user — restore custom value if not in the list
        let (user_idx, custom_user, user_input) = match initial_user {
            Some(name) => match users.iter().position(|u| u.username == name) {
                Some(idx) => (idx, false, Input::default()),
                None => (0, true, Input::new(name.to_string())),
            },
            None => (0, false, Input::default()),
        };

        // Resolve initial session — restore custom value if not in the list
        let (session_idx, custom_session, session_input) = match initial_session {
            Some(name) => {
                if let Some(idx) = sessions.iter().position(|s| s.name == name) {
                    (idx, false, Input::default())
                } else if include_defshell && users.get(user_idx).map_or(false, |u| u.shell == name)
                {
                    (sessions.len(), false, Input::default())
                } else {
                    (0, true, Input::new(name.to_string()))
                }
            }
            None => (0, false, Input::default()),
        };

        let themes = crate::theme::discover_themes(&config.colors);

        Self {
            config,
            sessions,
            users,
            state: UIState {
                selected_session_idx: session_idx,
                selected_user_idx: user_idx,
                session_input,
                user_input,
                password_input: Input::default(),
                focused_field: Field::User,
                custom_session,
                custom_user,
                auth_error,
                pam_messages,
                themes,
                current_theme_idx: 0,
            },
            console_buffer,
        }
    }

    pub fn pam_messages(&self) -> &[PamMessage] {
        &self.state.pam_messages
    }

    pub fn clear_pam_messages(&mut self) {
        self.state.pam_messages.clear();
    }

    #[allow(dead_code)]
    pub fn auth_error(&self) -> bool {
        self.state.auth_error
    }

    #[allow(dead_code)]
    pub fn set_auth_error(&mut self, auth_error: bool) {
        self.state.auth_error = auth_error;
    }

    pub fn clear_auth_error(&mut self) {
        self.state.auth_error = false;
    }

    // --- State mutations ---

    pub fn move_focus_up(&mut self) {
        self.clear_auth_error();
        self.state.focused_field = self.state.focused_field.prev();
    }

    pub fn move_focus_down(&mut self) {
        self.clear_auth_error();
        self.state.focused_field = self.state.focused_field.next();
    }

    /// Left/Right always cycles — resets custom mode so typed values can be changed.
    pub fn handle_field_key(&mut self, key: KeyEvent) {
        self.clear_auth_error();
        self.clear_pam_messages();
        match self.state.focused_field {
            Field::Session => match key.code {
                KeyCode::Left => self.change_session(-1),
                KeyCode::Right => self.change_session(1),
                _ => {
                    self.state.custom_session = true;
                    self.state.session_input.handle_event(&Event::Key(key));
                }
            },
            Field::User => match key.code {
                KeyCode::Left => self.change_user(-1),
                KeyCode::Right => self.change_user(1),
                _ => {
                    self.state.custom_user = true;
                    self.state.user_input.handle_event(&Event::Key(key));
                }
            },
            Field::Password => {
                self.state.password_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn change_session(&mut self, dir: i32) {
        let count = self.sessions.len()
            + if self.config.behavior.include_defshell {
                1
            } else {
                0
            };
        if count == 0 {
            return;
        }
        let cur = self.state.selected_session_idx;
        self.state.selected_session_idx = if dir > 0 {
            (cur + 1) % count
        } else if cur > 0 {
            cur - 1
        } else {
            count - 1
        };
        log::debug!(
            "change_session: dir={}, old_idx={}, new_idx={}, sessions.len={}, count={}",
            dir,
            cur,
            self.state.selected_session_idx,
            self.sessions.len(),
            count
        );
        self.state.custom_session = false;
        self.state.session_input = Input::default();
    }

    fn change_user(&mut self, dir: i32) {
        let count = self.users.len();
        if count == 0 {
            return;
        }
        let cur = self.state.selected_user_idx;
        self.state.selected_user_idx = if dir > 0 {
            (cur + 1) % count
        } else if cur > 0 {
            cur - 1
        } else {
            count - 1
        };
        self.state.custom_user = false;
        self.state.user_input = Input::default();
    }

    // --- Hotkey matching ---

    pub fn check_hotkey(&self, key_code: KeyCode) -> Option<HotkeyAction> {
        let is = |hk: &Option<String>| hk.as_deref().and_then(Self::parse_key) == Some(key_code);
        if is(&self.config.functions.poweroff) {
            return Some(HotkeyAction::Poweroff);
        }
        if is(&self.config.functions.reboot) {
            return Some(HotkeyAction::Reboot);
        }
        if is(&self.config.functions.refresh) {
            return Some(HotkeyAction::Refresh);
        }
        if is(&self.config.functions.fido) {
            return Some(HotkeyAction::Fido);
        }
        if is(&self.config.functions.theme) {
            return Some(HotkeyAction::Theme);
        }
        None
    }

    fn parse_key(k: &str) -> Option<KeyCode> {
        k.strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .map(KeyCode::F)
    }

    pub fn cycle_theme(&mut self) {
        if self.state.themes.is_empty() {
            return;
        }
        self.state.current_theme_idx =
            (self.state.current_theme_idx + 1) % self.state.themes.len();
        self.config.colors = self.state.themes[self.state.current_theme_idx]
            .colors
            .clone();
    }

    // --- Result extraction ---

    pub fn login_data(&self) -> (usize, usize, String, String, String) {
        (
            self.state.selected_session_idx,
            self.state.selected_user_idx,
            self.state.password_input.value().to_string(),
            self.state.session_input.value().to_string(),
            self.state.user_input.value().to_string(),
        )
    }

    pub fn fido_login_data(&self) -> (usize, usize, String, String, String) {
        self.state.fido_login_tuple()
    }

    // --- View queries (called by renderer) ---

    pub fn focused_field(&self) -> Field {
        self.state.focused_field
    }

    pub fn field_label(&self, field: Field) -> &str {
        match field {
            Field::Session => self.session_label(),
            Field::User => &self.config.strings.e_user,
            Field::Password => &self.config.strings.e_passwd,
        }
    }

    pub fn field_display_text(&self, field: Field) -> String {
        match field {
            Field::Session if self.state.custom_session => {
                self.state.session_input.value().to_string()
            }
            Field::Session => self
                .sessions
                .get(self.state.selected_session_idx)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| {
                    // At the defshell slot — show the selected user's shell
                    self.users
                        .get(self.state.selected_user_idx)
                        .map(|u| u.shell.clone())
                        .unwrap_or_else(|| "Shell".to_string())
                }),
            Field::User if self.state.custom_user => self.state.user_input.value().to_string(),
            Field::User => self
                .users
                .get(self.state.selected_user_idx)
                .map(|u| u.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            Field::Password => "*".repeat(self.state.password_input.value().len()),
        }
    }

    pub fn field_value_style(&self, field: Field) -> Style {
        match field {
            Field::Session => {
                let color = match self.selected_session_type() {
                    Some(SessionType::Xorg) => self.config.colors.s_xorg.clone(),
                    Some(SessionType::Wayland) => self.config.colors.s_wayland.clone(),
                    _ => self.config.colors.s_shell.clone(),
                };
                Style::from(color)
            }
            Field::User => Style::from(self.config.colors.e_user.clone()),
            Field::Password => {
                if self.state.auth_error {
                    Style::from(self.config.colors.e_badpasswd.clone())
                } else {
                    Style::from(self.config.colors.e_passwd.clone())
                }
            }
        }
    }

    pub fn show_selector(&self, field: Field) -> bool {
        match field {
            Field::Password => false,
            Field::Session => {
                let count = self.sessions.len()
                    + if self.config.behavior.include_defshell {
                        1
                    } else {
                        0
                    };
                !self.state.custom_session && count > 0
            }
            Field::User => !self.state.custom_user && !self.users.is_empty(),
        }
    }

    pub fn cursor_offset(&self) -> u16 {
        let offset = match self.state.focused_field {
            Field::Session => self.state.session_input.cursor(),
            Field::User => self.state.user_input.cursor(),
            Field::Password => self.state.password_input.cursor(),
        };
        let selector = if self.show_selector(self.state.focused_field) {
            self.config.strings.opts_pre.len()
        } else {
            0
        };
        (selector + offset) as u16
    }

    pub fn show_console(&self) -> bool {
        self.config.behavior.show_console && self.console_buffer.is_some()
    }

    pub fn console_lines(&self) -> Vec<String> {
        match &self.console_buffer {
            Some(buf) => buf.lock().unwrap().iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    pub fn session_label(&self) -> &str {
        match self.selected_session_type() {
            Some(SessionType::Xorg) => &self.config.strings.s_xorg,
            Some(SessionType::Wayland) => &self.config.strings.s_wayland,
            Some(SessionType::Shell) => &self.config.strings.s_shell,
            None => &self.config.strings.s_shell,
        }
    }

    fn selected_session_type(&self) -> Option<SessionType> {
        if self.state.custom_session {
            None
        } else {
            self.sessions
                .get(self.state.selected_session_idx)
                .map(|s| s.session_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::session::{ExecType, Session, SessionType};

    #[test]
    fn test_configured_session_type_strings() {
        let mut config = Config::default();
        config.strings.s_wayland = "Wayland Display".to_string();
        config.strings.s_xorg = "X11 Server".to_string();
        config.strings.s_shell = "Terminal Shell".to_string();

        let sessions = vec![
            Session {
                name: "Sway".to_string(),
                exec: ExecType::Desktop(vec!["sway".to_string()]),
                session_type: SessionType::Wayland,
                desktop_names: None,
            },
            Session {
                name: "i3".to_string(),
                exec: ExecType::Desktop(vec!["i3".to_string()]),
                session_type: SessionType::Xorg,
                desktop_names: None,
            },
            Session {
                name: "bash".to_string(),
                exec: ExecType::Shell("bash".to_string()),
                session_type: SessionType::Shell,
                desktop_names: None,
            },
        ];

        let mut adapter = UIAdapter::new(config, sessions, vec![], None, None, None, vec![], false);

        // Initially index 0: Sway (Wayland)
        assert_eq!(adapter.session_label(), "Wayland Display");
        assert_eq!(adapter.field_label(Field::Session), "Wayland Display");

        // Cycle to index 1: i3 (Xorg)
        adapter.change_session(1);
        assert_eq!(adapter.session_label(), "X11 Server");
        assert_eq!(adapter.field_label(Field::Session), "X11 Server");

        // Cycle to index 2: bash (Shell)
        adapter.change_session(1);
        assert_eq!(adapter.session_label(), "Terminal Shell");
        assert_eq!(adapter.field_label(Field::Session), "Terminal Shell");
    }

    #[test]
    fn test_auth_error_styling_and_clearing() {
        let config = Config::default();
        let mut adapter =
            UIAdapter::new(config.clone(), vec![], vec![], None, None, None, vec![], true);

        assert!(adapter.auth_error());
        assert_eq!(
            adapter.field_value_style(Field::Password),
            Style::from(config.colors.e_badpasswd.clone())
        );

        adapter.clear_auth_error();

        assert!(!adapter.auth_error());
        assert_eq!(
            adapter.field_value_style(Field::Password),
            Style::from(config.colors.e_passwd.clone())
        );
    }

    #[test]
    fn test_fido_hotkey_detection_and_empty_password_flow() {
        let mut config = Config::default();
        config.functions.fido = Some("F3".to_string());
        config.strings.f_fido = Some("fido_key".to_string());

        let adapter = UIAdapter::new(config, vec![], vec![], None, None, None, vec![], false);

        assert_eq!(
            adapter.check_hotkey(crossterm::event::KeyCode::F(3)),
            Some(HotkeyAction::Fido)
        );

        let (_, _, password, _, _) = adapter.fido_login_data();
        assert_eq!(password, "");
    }

    #[test]
    fn test_theme_hotkey_detection_and_cycling() {
        let config = Config::default();
        let mut adapter = UIAdapter::new(
            config,
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            Vec::new(),
            false,
        );

        // F3 is the default theme hotkey
        assert_eq!(
            adapter.check_hotkey(KeyCode::F(3)),
            Some(HotkeyAction::Theme)
        );

        // Themes should contain at least the default theme
        assert!(!adapter.state.themes.is_empty());
        assert_eq!(adapter.state.current_theme_idx, 0);

        // Cycling with a single theme should wrap back to 0
        let initial_len = adapter.state.themes.len();
        adapter.cycle_theme();
        assert_eq!(adapter.state.current_theme_idx, 1 % initial_len);
    }
}
