use crate::auth::PamMessage;
use crate::config::{BoxType, Config};
use crate::console::ConsoleBuffer;
use crate::session::{Session, SessionType};
use crate::theme::Theme;
use crate::users::LocalUser;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use ratatui::macros::ratatui_core;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::io;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

const LABEL_W: u16 = 10;
const GAP_W: u16 = 3;
const CONSOLE_H: u16 = 8;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum Field {
    Session,
    #[default]
    User,
    Password,
}

impl Field {
    pub fn next(self) -> Self {
        match self {
            Self::Session => Self::User,
            Self::User => Self::Password,
            Self::Password => Self::Session,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Session => Self::Password,
            Self::User => Self::Session,
            Self::Password => Self::User,
        }
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

impl LoginRequest {
    pub fn new(
        session_idx: usize,
        user_idx: usize,
        password: impl Into<String>,
        custom_session: impl Into<String>,
        custom_user: impl Into<String>,
    ) -> Self {
        Self {
            session_idx,
            user_idx,
            password: password.into(),
            custom_session: custom_session.into(),
            custom_user: custom_user.into(),
        }
    }
}

#[derive(Default)]
pub struct UIContext<'a> {
    pub config: Config,
    pub sessions: Vec<Session>,
    pub users: Vec<LocalUser>,
    pub initial_user: Option<&'a str>,
    pub initial_session: Option<&'a str>,
    pub console_buffer: Option<ConsoleBuffer>,
    pub pam_messages: Vec<PamMessage>,
    pub auth_error: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Poweroff,
    Reboot,
    Fido,
    Theme,
}

pub enum UIResult {
    Login(LoginRequest),
    Poweroff,
    Reboot,
    Exit,
}

pub struct UI {
    pub config: Config,
    pub sessions: Vec<Session>,
    pub users: Vec<LocalUser>,
    pub selected_session_idx: usize,
    pub selected_user_idx: usize,
    pub session_input: Input,
    pub user_input: Input,
    pub password_input: Input,
    pub focused_field: Field,
    pub custom_session: bool,
    pub custom_user: bool,
    pub auth_error: bool,
    pub pam_messages: Vec<PamMessage>,
    pub themes: Vec<Theme>,
    pub current_theme_idx: usize,
    pub console_buffer: Option<ConsoleBuffer>,
}

impl UI {
    pub fn new(ctx: UIContext) -> Self {
        let include_defshell = ctx.config.behavior.include_defshell;

        let (selected_user_idx, custom_user, user_input) = match ctx.initial_user {
            Some(name) => match ctx.users.iter().position(|u| u.username == name) {
                Some(idx) => (idx, false, Input::default()),
                None => (0, true, Input::new(name.to_string())),
            },
            None => (0, false, Input::default()),
        };

        let (selected_session_idx, custom_session, session_input) = match ctx.initial_session {
            Some(name) => {
                if let Some(idx) = ctx.sessions.iter().position(|s| s.name == name) {
                    (idx, false, Input::default())
                } else if include_defshell && ctx.users.get(selected_user_idx).is_some_and(|u| u.shell == name) {
                    (ctx.sessions.len(), false, Input::default())
                } else {
                    (0, true, Input::new(name.to_string()))
                }
            }
            None => (0, false, Input::default()),
        };

        let themes = crate::theme::discover_themes(&ctx.config.colors);

        Self {
            config: ctx.config,
            sessions: ctx.sessions,
            users: ctx.users,
            selected_session_idx,
            selected_user_idx,
            session_input,
            user_input,
            password_input: Input::default(),
            focused_field: Field::User,
            custom_session,
            custom_user,
            auth_error: ctx.auth_error,
            pam_messages: ctx.pam_messages,
            themes,
            current_theme_idx: 0,
            console_buffer: ctx.console_buffer,
        }
    }

    pub fn run(&mut self) -> io::Result<UIResult> {
        ratatui::run(|terminal| loop {
            terminal.draw(|f| self.draw(f))?;
            if let Some(r) = self.handle_events()? {
                return Ok(r);
            }
        })
    }

    fn handle_events(&mut self) -> io::Result<Option<UIResult>> {
        let timeout = std::time::Duration::from_millis(self.config.behavior.refresh_rate);
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            return Ok(self.handle_key_event(key));
        }
        Ok(None)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<UIResult> {
        self.auth_error = false;
        if let Some(action) = self.check_hotkey(key.code) {
            return match action {
                HotkeyAction::Poweroff => Some(UIResult::Poweroff),
                HotkeyAction::Reboot => Some(UIResult::Reboot),
                HotkeyAction::Fido => Some(UIResult::Login(self.fido_login_request())),
                HotkeyAction::Theme => {
                    self.cycle_theme();
                    None
                }
            };
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(UIResult::Exit),
            (KeyCode::Up, _) => { self.focused_field = self.focused_field.prev(); None }
            (KeyCode::Down, _) => { self.focused_field = self.focused_field.next(); None }
            (KeyCode::Enter, _) => Some(UIResult::Login(self.login_request())),
            _ => { self.handle_field_key(key); None }
        }
    }

    fn handle_field_key(&mut self, key: KeyEvent) {
        self.pam_messages.clear();
        match self.focused_field {
            Field::Session => match key.code {
                KeyCode::Left => self.change_session(-1),
                KeyCode::Right => self.change_session(1),
                _ => {
                    self.custom_session = true;
                    self.session_input.handle_event(&Event::Key(key));
                }
            },
            Field::User => match key.code {
                KeyCode::Left => self.change_user(-1),
                KeyCode::Right => self.change_user(1),
                _ => {
                    self.custom_user = true;
                    self.user_input.handle_event(&Event::Key(key));
                }
            },
            Field::Password => {
                self.password_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn cycle(idx: &mut usize, len: usize, dir: i32) {
        if len > 0 {
            *idx = if dir > 0 { (*idx + 1) % len } else { idx.checked_sub(1).unwrap_or(len - 1) };
        }
    }

    pub fn change_session(&mut self, dir: i32) {
        let count = self.sessions.len() + if self.config.behavior.include_defshell { 1 } else { 0 };
        Self::cycle(&mut self.selected_session_idx, count, dir);
        self.custom_session = false;
        self.session_input = Input::default();
    }

    pub fn change_user(&mut self, dir: i32) {
        Self::cycle(&mut self.selected_user_idx, self.users.len(), dir);
        self.custom_user = false;
        self.user_input = Input::default();
    }

    pub fn check_hotkey(&self, key_code: KeyCode) -> Option<HotkeyAction> {
        let parse = |k: &Option<String>| k.as_deref()?.strip_prefix('F')?.parse::<u8>().ok().map(KeyCode::F);
        if parse(&self.config.functions.poweroff) == Some(key_code) { Some(HotkeyAction::Poweroff) }
        else if parse(&self.config.functions.reboot) == Some(key_code) { Some(HotkeyAction::Reboot) }
        else if parse(&self.config.functions.fido) == Some(key_code) { Some(HotkeyAction::Fido) }
        else if parse(&self.config.functions.theme) == Some(key_code) { Some(HotkeyAction::Theme) }
        else { None }
    }

    pub fn cycle_theme(&mut self) {
        if !self.themes.is_empty() {
            self.current_theme_idx = (self.current_theme_idx + 1) % self.themes.len();
            self.config.colors = self.themes[self.current_theme_idx].colors.clone();
            log::info!(
                "cycle_theme: switched to theme '{}' ({})",
                self.current_theme_name(),
                self.current_theme_path()
            );
        }
    }

    pub fn current_theme_name(&self) -> &str {
        self.themes.get(self.current_theme_idx).map(|t| t.name.as_str()).unwrap_or("")
    }

    pub fn current_theme_path(&self) -> &str {
        self.themes.get(self.current_theme_idx).map(|t| t.path.as_str()).unwrap_or("")
    }

    pub fn current_theme_display(&self) -> String {
        let path = self.current_theme_path();
        if path.is_empty() || path == "default" || path == self.current_theme_name() {
            self.current_theme_name().to_string()
        } else {
            format!("{} ({})", self.current_theme_name(), path)
        }
    }

    pub fn login_request(&self) -> LoginRequest {
        LoginRequest::new(
            self.selected_session_idx,
            self.selected_user_idx,
            self.password_input.value(),
            self.session_input.value(),
            self.user_input.value(),
        )
    }

    pub fn fido_login_request(&self) -> LoginRequest {
        LoginRequest::new(
            self.selected_session_idx,
            self.selected_user_idx,
            "",
            self.session_input.value(),
            self.user_input.value(),
        )
    }

    pub fn session_label(&self) -> &str {
        match self.selected_session_type() {
            Some(SessionType::Xorg) => &self.config.strings.s_xorg,
            Some(SessionType::Wayland) => &self.config.strings.s_wayland,
            _ => &self.config.strings.s_shell,
        }
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
            Field::Session if self.custom_session => self.session_input.value().to_string(),
            Field::Session => self.sessions.get(self.selected_session_idx)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| {
                    self.users.get(self.selected_user_idx)
                        .map(|u| u.shell.clone())
                        .unwrap_or_else(|| "Shell".to_string())
                }),
            Field::User if self.custom_user => self.user_input.value().to_string(),
            Field::User => self.users.get(self.selected_user_idx)
                .map(|u| u.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            Field::Password => "*".repeat(self.password_input.value().len()),
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
                if self.auth_error {
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
                let count = self.sessions.len() + if self.config.behavior.include_defshell { 1 } else { 0 };
                !self.custom_session && count > 0
            }
            Field::User => !self.custom_user && !self.users.is_empty(),
        }
    }

    pub fn cursor_offset(&self) -> u16 {
        let offset = match self.focused_field {
            Field::Session => self.session_input.cursor(),
            Field::User => self.user_input.cursor(),
            Field::Password => self.password_input.cursor(),
        };
        let selector = if self.show_selector(self.focused_field) { self.config.strings.opts_pre.len() } else { 0 };
        (selector + offset) as u16
    }

    fn selected_session_type(&self) -> Option<SessionType> {
        if self.custom_session { None } else { self.sessions.get(self.selected_session_idx).map(|s| s.session_type) }
    }

    fn draw(&self, f: &mut Frame) {
        f.render_widget(self, f.area());
        let show_messages = !self.pam_messages.is_empty();
        let show_console = self.config.behavior.show_console && self.console_buffer.is_some();
        let (_, chunks, _) = Self::layout(f.area(), show_console, show_messages);
        let fi = match self.focused_field {
            Field::Session => 1,
            Field::User => 2,
            Field::Password => 3,
        };
        f.set_cursor_position(Position::new(chunks[fi].x + LABEL_W + GAP_W + self.cursor_offset(), chunks[fi].y));
    }

    pub fn layout(area: Rect, show_console: bool, show_messages: bool) -> (Rect, Vec<Rect>, Option<Rect>) {
        let box_h = 12 + if show_messages { 2 } else { 0 };
        let total_h = if show_console { box_h + 1 + CONSOLE_H } else { box_h };

        let [vc] = Layout::vertical([Constraint::Length(total_h)]).flex(ratatui::layout::Flex::Center).areas(area);
        let [bx] = Layout::horizontal([Constraint::Length(60)]).flex(ratatui::layout::Flex::Center).areas(vc);

        let (login_area, console_area) = if show_console {
            let [login, _, console] = Layout::vertical([Constraint::Length(box_h), Constraint::Length(1), Constraint::Length(CONSOLE_H)]).areas(bx);
            (login, Some(console))
        } else {
            (bx, None)
        };

        let inner = login_area.inner(ratatui::layout::Margin { horizontal: 2, vertical: 1 });
        let field_constraints = if show_messages {
            vec![Constraint::Length(3), Constraint::Length(2), Constraint::Length(2), Constraint::Length(3), Constraint::Length(2)]
        } else {
            vec![Constraint::Length(3), Constraint::Length(2), Constraint::Length(2), Constraint::Length(3)]
        };

        (login_area, Layout::vertical(field_constraints).split(inner).to_vec(), console_area)
    }

    fn render_field(&self, buf: &mut Buffer, area: Rect, field: Field) {
        let [la, _, va] = Layout::horizontal([Constraint::Length(LABEL_W), Constraint::Length(GAP_W), Constraint::Fill(1)]).areas(area);
        let focused = self.focused_field == field;
        let mut ls = Style::from(self.config.colors.e_header.clone());
        let mut vs = self.field_value_style(field);
        if focused {
            ls = ls.add_modifier(Modifier::BOLD);
            vs = vs.add_modifier(Modifier::BOLD);
        }
        Paragraph::new(Span::styled(self.field_label(field), ls)).alignment(ratatui::layout::Alignment::Right).render(la, buf);
        let value = self.field_display_text(field);
        let spans = if self.show_selector(field) {
            vec![Span::raw(&self.config.strings.opts_pre), Span::styled(value, vs), Span::raw(&self.config.strings.opts_post)]
        } else {
            vec![Span::styled(value, vs)]
        };
        Paragraph::new(Line::from(spans)).render(va, buf);
    }
}

impl Widget for &UI {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cfg = &self.config;
        let show_console = cfg.behavior.show_console && self.console_buffer.is_some();
        let show_messages = !self.pam_messages.is_empty();

        Block::default().style(Style::from(cfg.colors.bg.clone())).render(area, buf);
        let (login_area, chunks, console_area) = UI::layout(area, show_console, show_messages);

        let border = match cfg.behavior.box_type {
            BoxType::None => Block::default().borders(Borders::NONE),
            BoxType::Block => Block::default().borders(Borders::ALL).border_set(ratatui_core::symbols::border::FULL),
            BoxType::Rounded => Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded),
            BoxType::Border => Block::default().borders(Borders::ALL),
        }.style(Style::from(cfg.colors.e_box.clone()));
        border.render(login_area, buf);

        // Header: hostname left, clock right
        let [_, hdr] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(chunks[0]);
        let [hl, hr] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(hdr);

        let raw_hostname = nix::unistd::gethostname().map(|h| h.to_string_lossy().into_owned()).unwrap_or_else(|_| "unknown".into());
        let hostname = truncate_str(&raw_hostname, hl.width as usize, &cfg.strings.ellipsis);
        Paragraph::new(Span::styled(hostname, Style::from(cfg.colors.e_hostname.clone()))).render(hl, buf);

        let time = Local::now().format(&cfg.behavior.timefmt).to_string();
        Paragraph::new(Span::styled(time, Style::from(cfg.colors.e_date.clone()))).alignment(ratatui::layout::Alignment::Right).render(hr, buf);

        // Fields
        for (i, &field) in [Field::Session, Field::User, Field::Password].iter().enumerate() {
            self.render_field(buf, chunks[i + 1], field);
        }

        // PAM Messages
        if show_messages && chunks.len() > 4 {
            let msg_spans: Vec<Line> = self.pam_messages.iter().map(|m| {
                let style = match m.msg_type {
                    crate::auth::PamMessageType::Info => Style::default().fg(ratatui::style::Color::Cyan),
                    crate::auth::PamMessageType::Error => Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD),
                };
                Line::from(Span::styled(m.message.clone(), style))
            }).collect();
            Paragraph::new(msg_spans).alignment(ratatui::layout::Alignment::Center).render(chunks[4], buf);
        }

        // Console panel
        if let Some(console_rect) = console_area {
            let lines: Vec<String> = self.console_buffer.as_ref().map(|b| b.lock().unwrap().iter().cloned().collect()).unwrap_or_default();
            let visible_h = console_rect.height.saturating_sub(2) as usize;
            let skip = lines.len().saturating_sub(visible_h);
            let styled_lines: Vec<Line> = lines[skip..].iter().map(|l| {
                let style = if l.contains("ERROR") { Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD) }
                    else if l.contains("WARN") { Style::default().fg(ratatui::style::Color::Yellow) }
                    else if l.contains("INFO") { Style::default().fg(ratatui::style::Color::Green) }
                    else if l.contains("DEBUG") { Style::default().fg(ratatui::style::Color::Blue) }
                    else { Style::default().add_modifier(Modifier::DIM) };
                Line::from(Span::styled(l.clone(), style))
            }).collect();

            let console_block = Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).title(" console ").style(Style::from(cfg.colors.e_box.clone()));
            Paragraph::new(styled_lines).block(console_block).render(console_rect, buf);
        }

        // Shortcuts
        let f_fido_label = cfg.strings.f_fido.as_deref().unwrap_or("fido");
        let f_theme_label = cfg.strings.f_theme.as_deref().unwrap_or("theme");
        let hotkeys: [(&Option<String>, &str); 4] = [
            (&cfg.functions.poweroff, &cfg.strings.f_poweroff),
            (&cfg.functions.reboot, &cfg.strings.f_reboot),
            (&cfg.functions.fido, f_fido_label),
            (&cfg.functions.theme, f_theme_label),
        ];
        let ks = Style::from(cfg.colors.e_key.clone());
        let mut spans = vec![];
        for (hk, label) in hotkeys {
            if let Some(h) = hk {
                if !spans.is_empty() { spans.push(Span::raw("  ")); }
                spans.push(Span::raw(format!("{} ", label)));
                spans.push(Span::styled(h, ks));
            }
        }

        let [top_row, _, bottom_row] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Right).render(top_row, buf);

        if cfg.behavior.show_theme {
            let display = self.current_theme_display();
            let truncated = truncate_str(&display, bottom_row.width as usize, &cfg.strings.ellipsis);
            Paragraph::new(Span::styled(truncated, Style::from(cfg.colors.e_date.clone()))).alignment(ratatui::layout::Alignment::Right).render(bottom_row, buf);
        }
    }
}

pub fn truncate_str(s: &str, max_width: usize, ellipsis: &str) -> String {
    if Span::raw(s).width() <= max_width { return s.to_string(); }
    if max_width == 0 { return String::new(); }

    let ellipsis_width = Span::raw(ellipsis).width();
    if ellipsis_width >= max_width {
        return ellipsis.chars().scan(0, |w, c| {
            *w += Span::raw(c.to_string()).width();
            if *w <= max_width { Some(c) } else { None }
        }).collect();
    }

    let target_w = max_width - ellipsis_width;
    let mut buf: String = s.chars().scan(0, |w, c| {
        *w += Span::raw(c.to_string()).width();
        if *w <= target_w { Some(c) } else { None }
    }).collect();
    buf.push_str(ellipsis);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ExecType;

    #[test]
    fn test_truncate_str_cases() {
        assert_eq!(truncate_str("short", 10, "…"), "short");
        assert_eq!(truncate_str("verylonghostname", 8, "…"), "verylon…");
        assert_eq!(truncate_str("verylonghostname", 8, "..."), "veryl...");
        assert_eq!(truncate_str("hostname", 0, "…"), "");
        assert_eq!(truncate_str("hostname", 2, "..."), "..");
        assert_eq!(truncate_str("hostname", 1, "..."), ".");
    }

    #[test]
    fn test_custom_border_rendering() {
        let area = Rect::new(0, 0, 60, 15);
        for (box_type, expected_char) in [(BoxType::Border, "┌"), (BoxType::Rounded, "╭"), (BoxType::Block, "█")] {
            let mut config = Config::default();
            config.behavior.box_type = box_type;
            let ui = UI::new(UIContext { config, ..Default::default() });
            let mut buf = Buffer::empty(area);
            Widget::render(&ui, area, &mut buf);
            let (bx, _, _) = UI::layout(area, false, false);
            assert_eq!(buf.cell((bx.x, bx.y)).unwrap().symbol(), expected_char);
        }
    }

    #[test]
    fn test_show_theme_rendering() {
        let mut config = Config::default();
        config.behavior.show_theme = true;
        let mut ui = UI::new(UIContext { config: config.clone(), ..Default::default() });
        ui.themes = vec![crate::theme::Theme::new("test_theme", "/etc/lidm/themes/test_theme.toml", config.colors)];
        ui.cycle_theme();
        let expected = ui.current_theme_display();

        let area = Rect::new(0, 0, 100, 20);
        let mut buf = Buffer::empty(area);
        Widget::render(&ui, area, &mut buf);

        let bottom_text: String = (0..area.width).filter_map(|x| buf.cell((x, area.height - 1)).map(|c| c.symbol())).collect();
        assert!(bottom_text.contains(&expected));
    }

    #[test]
    fn test_configured_session_type_strings() {
        let mut config = Config::default();
        config.strings.s_wayland = "Wayland Display".to_string();
        config.strings.s_xorg = "X11 Server".to_string();
        config.strings.s_shell = "Terminal Shell".to_string();

        let sessions = vec![
            Session { name: "Sway".into(), exec: ExecType::Desktop(vec!["sway".into()]), session_type: SessionType::Wayland, desktop_names: None },
            Session { name: "i3".into(), exec: ExecType::Desktop(vec!["i3".into()]), session_type: SessionType::Xorg, desktop_names: None },
            Session { name: "bash".into(), exec: ExecType::Shell("bash".into()), session_type: SessionType::Shell, desktop_names: None },
        ];

        let mut ui = UI::new(UIContext { config, sessions, ..Default::default() });
        assert_eq!(ui.session_label(), "Wayland Display");
        ui.change_session(1);
        assert_eq!(ui.session_label(), "X11 Server");
        ui.change_session(1);
        assert_eq!(ui.session_label(), "Terminal Shell");
    }

    #[test]
    fn test_auth_error_styling_and_clearing() {
        let config = Config::default();
        let mut ui = UI::new(UIContext { config: config.clone(), auth_error: true, ..Default::default() });
        assert!(ui.auth_error);
        assert_eq!(ui.field_value_style(Field::Password), Style::from(config.colors.e_badpasswd.clone()));
        ui.auth_error = false;
        assert_eq!(ui.field_value_style(Field::Password), Style::from(config.colors.e_passwd.clone()));
    }

    #[test]
    fn test_fido_hotkey_detection() {
        let mut config = Config::default();
        config.functions.fido = Some("F3".into());
        let ui = UI::new(UIContext { config, ..Default::default() });
        assert_eq!(ui.check_hotkey(KeyCode::F(3)), Some(HotkeyAction::Fido));
        assert_eq!(ui.fido_login_request().password, "");
    }

    #[test]
    fn test_login_request_constructors() {
        let req = LoginRequest::new(1, 2, "secret", "wayland", "alice");
        assert_eq!(req.session_idx, 1);
        assert_eq!(req.user_idx, 2);
        assert_eq!(req.password, "secret");
        assert_eq!(req.custom_session, "wayland");
        assert_eq!(req.custom_user, "alice");
    }

    #[test]
    fn test_show_console_rendering() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let mut config = Config::default();
        config.behavior.show_console = true;
        let buffer: ConsoleBuffer = Arc::new(Mutex::new(VecDeque::from(vec![
            "INFO: system initialized".to_string(),
            "WARN: low memory".to_string(),
            "ERROR: auth failure".to_string(),
        ])));

        let ui = UI::new(UIContext {
            config,
            console_buffer: Some(buffer),
            ..Default::default()
        });

        let area = Rect::new(0, 0, 80, 25);
        let mut buf = Buffer::empty(area);
        Widget::render(&ui, area, &mut buf);

        let (_, _, console_rect) = UI::layout(area, true, false);
        assert!(console_rect.is_some());
        let cr = console_rect.unwrap();
        assert_eq!(cr.height, CONSOLE_H);

        // Find "console" title in the buffer
        let mut text = String::new();
        for y in cr.y..cr.y + cr.height {
            for x in cr.x..cr.x + cr.width {
                if let Some(cell) = buf.cell((x, y)) {
                    text.push_str(cell.symbol());
                }
            }
        }
        assert!(text.contains("console"));
        assert!(text.contains("ERROR: auth failure"));
    }
}
