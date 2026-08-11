use crate::config::{BoxType, Config};
use crate::console::ConsoleBuffer;
use crate::session::Session;
use crate::ui_adapter::{HotkeyAction, UIAdapter};
use crate::ui_state::Field;
use crate::users::LocalUser;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use ratatui::macros::ratatui_core;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::io;

pub enum UIResult {
    Login(usize, usize, String, String, String),
    Poweroff,
    Reboot,
    Exit,
}

pub struct UI {
    adapter: UIAdapter,
}

const LABEL_W: u16 = 10;
const GAP_W: u16 = 3;
const CONSOLE_H: u16 = 8;

impl UI {
    pub fn new(
        config: Config,
        sessions: Vec<Session>,
        users: Vec<LocalUser>,
        initial_user: Option<&str>,
        initial_session: Option<&str>,
        console_buffer: Option<ConsoleBuffer>,
        pam_messages: Vec<crate::auth::PamMessage>,
        auth_error: bool,
    ) -> Self {
        Self {
            adapter: UIAdapter::new(
                config,
                sessions,
                users,
                initial_user,
                initial_session,
                console_buffer,
                pam_messages,
                auth_error,
            ),
        }
    }

    pub fn run(&mut self) -> io::Result<UIResult> {
        ratatui::run(|terminal| self.main_loop(terminal))
    }

    fn main_loop(&mut self, terminal: &mut DefaultTerminal) -> io::Result<UIResult> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            match self.handle_events()? {
                Some(r) => return Ok(r),
                None => (),
            }
        }
    }

    fn handle_events(&mut self) -> io::Result<Option<UIResult>> {
        let timeout = std::time::Duration::from_millis(self.adapter.config.behavior.refresh_rate);
        match event::poll(timeout)? {
            true => match event::read()? {
                Event::Key(key) => Ok(self.handle_key_event(key)),
                _ => Ok(None),
            },
            false => Ok(None),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<UIResult> {
        self.adapter.clear_auth_error();
        if let Some(action) = self.adapter.check_hotkey(key.code) {
            return match action {
                HotkeyAction::Poweroff => Some(UIResult::Poweroff),
                HotkeyAction::Reboot => Some(UIResult::Reboot),
                HotkeyAction::Fido => {
                    let (s, u, p, cs, cu) = self.adapter.fido_login_data();
                    Some(UIResult::Login(s, u, p, cs, cu))
                }
                HotkeyAction::Theme => {
                    log::debug!("handle_key_event: F3 (theme) hotkey pressed");
                    self.adapter.cycle_theme();
                    None
                }
            };
        }
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Some(UIResult::Exit),
            (KeyCode::Up, _) => self.adapter.move_focus_up(),
            (KeyCode::Down, _) => self.adapter.move_focus_down(),
            (KeyCode::Enter, _) => {
                let (s, u, p, cs, cu) = self.adapter.login_data();
                return Some(UIResult::Login(s, u, p, cs, cu));
            }
            _ => self.adapter.handle_field_key(key),
        }
        None
    }

    // --- Rendering ---

    fn draw(&self, f: &mut Frame) {
        f.render_widget(self, f.area());
        let show_messages = !self.adapter.pam_messages().is_empty();
        let (_, chunks, _) = Self::layout(f.area(), self.adapter.show_console(), show_messages);
        let fi = match self.adapter.focused_field() {
            Field::Session => 1,
            Field::User => 2,
            Field::Password => 3,
        };
        f.set_cursor_position(Position::new(
            chunks[fi].x + LABEL_W + GAP_W + self.adapter.cursor_offset(),
            chunks[fi].y,
        ));
    }

    fn layout(area: Rect, show_console: bool, show_messages: bool) -> (Rect, Vec<Rect>, Option<Rect>) {
        let msg_h = if show_messages { 2 } else { 0 };
        let box_h = 12 + msg_h;
        let total_h = if show_console {
            box_h + 1 + CONSOLE_H // 1 for gap
        } else {
            box_h
        };
        let [vc] = Layout::vertical([Constraint::Length(total_h)])
            .flex(ratatui::layout::Flex::Center)
            .areas(area);
        let [bx] = Layout::horizontal([Constraint::Length(60)])
            .flex(ratatui::layout::Flex::Center)
            .areas(vc);

        let (login_area, console_area) = if show_console {
            let [login, _, console] = Layout::vertical([
                Constraint::Length(box_h),
                Constraint::Length(1),
                Constraint::Length(CONSOLE_H),
            ])
            .areas(bx);
            (login, Some(console))
        } else {
            (bx, None)
        };

        let inner = login_area.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        });

        let field_constraints = if show_messages {
            vec![
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(3),
            ]
        };

        let chunks = Layout::vertical(field_constraints).split(inner).to_vec();

        (login_area, chunks, console_area)
    }

    fn render_field(&self, buf: &mut Buffer, area: Rect, field: Field) {
        let [la, _, va] = Layout::horizontal([
            Constraint::Length(LABEL_W),
            Constraint::Length(GAP_W),
            Constraint::Fill(1),
        ])
        .areas(area);

        let focused = self.adapter.focused_field() == field;
        let mut ls = Style::from(self.adapter.config.colors.e_header.clone());
        if focused {
            ls = ls.add_modifier(Modifier::BOLD);
        }
        Paragraph::new(Span::styled(self.adapter.field_label(field), ls))
            .alignment(ratatui::layout::Alignment::Right)
            .render(la, buf);

        let value = self.adapter.field_display_text(field);
        let mut vs = self.adapter.field_value_style(field);
        if focused {
            vs = vs.add_modifier(Modifier::BOLD);
        }
        let spans = if self.adapter.show_selector(field) {
            vec![
                Span::raw(&*self.adapter.config.strings.opts_pre),
                Span::styled(value, vs),
                Span::raw(&*self.adapter.config.strings.opts_post),
            ]
        } else {
            vec![Span::styled(value, vs)]
        };
        Paragraph::new(Line::from(spans)).render(va, buf);
    }
}

impl Widget for &UI {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cfg = &self.adapter.config;
        let show_console = self.adapter.show_console();
        let show_messages = !self.adapter.pam_messages().is_empty();

        // Background
        Block::default()
            .style(Style::from(cfg.colors.bg.clone()))
            .render(area, buf);

        let (bx, chunks, console_area) = UI::layout(area, show_console, show_messages);

        // Box border
        let border = match cfg.behavior.box_type {
            BoxType::None => Block::default().borders(Borders::NONE),
            BoxType::Block => Block::default()
                .borders(Borders::ALL)
                .border_set(ratatui_core::symbols::border::FULL),
            BoxType::Rounded => Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
            BoxType::Border => Block::default().borders(Borders::ALL),
        }
        .style(Style::from(cfg.colors.e_box.clone()));
        border.render(bx, buf);

        // Header: hostname left, clock right
        let [_, hdr] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(chunks[0]);
        let [hl, hr] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(hdr);

        let raw_hostname = nix::unistd::gethostname()
            .unwrap_or_else(|_| "unknown".into())
            .to_string_lossy()
            .into_owned();
        let hostname = truncate_str(&raw_hostname, hl.width as usize, &cfg.strings.ellipsis);
        Paragraph::new(Span::styled(
            hostname,
            Style::from(cfg.colors.e_hostname.clone()),
        ))
        .render(hl, buf);

        let time = Local::now().format(&cfg.behavior.timefmt).to_string();
        Paragraph::new(Span::styled(time, Style::from(cfg.colors.e_date.clone())))
            .alignment(ratatui::layout::Alignment::Right)
            .render(hr, buf);

        // Fields
        for (i, &field) in [Field::Session, Field::User, Field::Password]
            .iter()
            .enumerate()
        {
            self.render_field(buf, chunks[i + 1], field);
        }

        // PAM Messages
        if show_messages && chunks.len() > 4 {
            let msg_spans: Vec<Line> = self
                .adapter
                .pam_messages()
                .iter()
                .map(|m| {
                    let style = match m.msg_type {
                        crate::auth::PamMessageType::Info => {
                            Style::default().fg(ratatui::style::Color::Cyan)
                        }
                        crate::auth::PamMessageType::Error => Style::default()
                            .fg(ratatui::style::Color::Red)
                            .add_modifier(Modifier::BOLD),
                    };
                    Line::from(Span::styled(m.message.clone(), style))
                })
                .collect();

            Paragraph::new(msg_spans)
                .alignment(ratatui::layout::Alignment::Center)
                .render(chunks[4], buf);
        }

        // Console panel
        if let Some(console_rect) = console_area {
            let console_lines = self.adapter.console_lines();
            let visible_h = console_rect.height.saturating_sub(2) as usize; // border takes 2 lines
            let skip = console_lines.len().saturating_sub(visible_h);
            let lines: Vec<Line> = console_lines[skip..]
                .iter()
                .map(|l| {
                    let style = if l.contains("ERROR") {
                        Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD)
                    } else if l.contains("WARN") {
                        Style::default().fg(ratatui::style::Color::Yellow)
                    } else if l.contains("INFO") {
                        Style::default().fg(ratatui::style::Color::Green)
                    } else if l.contains("DEBUG") {
                        Style::default().fg(ratatui::style::Color::Blue)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    };
                    Line::from(Span::styled(l.clone(), style))
                })
                .collect();

            let console_block = Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" console ")
                .style(Style::from(cfg.colors.e_box.clone()));
            Paragraph::new(lines)
                .block(console_block)
                .render(console_rect, buf);
        }

        // Shortcuts / Hotkeys top right
        let f_fido_default = "fido".to_string();
        let f_fido_label = cfg.strings.f_fido.as_ref().unwrap_or(&f_fido_default);
        let f_theme_default = "theme".to_string();
        let f_theme_label = cfg.strings.f_theme.as_ref().unwrap_or(&f_theme_default);
        let hotkeys = [
            (&cfg.functions.poweroff, &cfg.strings.f_poweroff),
            (&cfg.functions.reboot, &cfg.strings.f_reboot),
            (&cfg.functions.fido, f_fido_label),
            (&cfg.functions.theme, f_theme_label),
        ];
        let ks = Style::from(cfg.colors.e_key.clone());
        let mut spans = vec![];
        for (hk, label) in hotkeys {
            if let Some(h) = hk {
                if !spans.is_empty() {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::raw(format!("{} ", label)));
                spans.push(Span::styled(h, ks));
            }
        }

        let [top_row, _, bottom_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        Paragraph::new(Line::from(spans))
            .alignment(ratatui::layout::Alignment::Right)
            .render(top_row, buf);

        if cfg.behavior.show_theme {
            let theme_path = self.adapter.current_theme_path();
            let truncated_theme = truncate_str(theme_path, bottom_row.width as usize, &cfg.strings.ellipsis);
            Paragraph::new(Span::styled(
                truncated_theme,
                Style::from(cfg.colors.e_date.clone()),
            ))
            .alignment(ratatui::layout::Alignment::Right)
            .render(bottom_row, buf);
        }
    }
}

pub fn truncate_str(s: &str, max_width: usize, ellipsis: &str) -> String {
    let s_width = Span::raw(s).width();
    if s_width <= max_width {
        return s.to_string();
    }

    if max_width == 0 {
        return String::new();
    }

    let ellipsis_width = Span::raw(ellipsis).width();
    if ellipsis_width >= max_width {
        let mut buf = String::new();
        let mut current_w = 0;
        for c in ellipsis.chars() {
            let cw = Span::raw(c.to_string()).width();
            if current_w + cw > max_width {
                break;
            }
            buf.push(c);
            current_w += cw;
        }
        return buf;
    }

    let target_w = max_width - ellipsis_width;
    let mut buf = String::new();
    let mut current_w = 0;
    for c in s.chars() {
        let cw = Span::raw(c.to_string()).width();
        if current_w + cw > target_w {
            break;
        }
        buf.push(c);
        current_w += cw;
    }
    buf.push_str(ellipsis);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_no_truncation_needed() {
        assert_eq!(truncate_str("short", 10, "…"), "short");
        assert_eq!(truncate_str("exact10chrs", 11, "…"), "exact10chrs");
    }

    #[test]
    fn test_truncate_str_with_single_char_ellipsis() {
        assert_eq!(truncate_str("verylonghostname", 8, "…"), "verylon…");
    }

    #[test]
    fn test_truncate_str_with_multi_char_ellipsis() {
        assert_eq!(truncate_str("verylonghostname", 8, "..."), "veryl...");
    }

    #[test]
    fn test_truncate_str_zero_max_width() {
        assert_eq!(truncate_str("hostname", 0, "…"), "");
    }

    #[test]
    fn test_truncate_str_small_max_width() {
        assert_eq!(truncate_str("hostname", 2, "..."), "..");
        assert_eq!(truncate_str("hostname", 1, "..."), ".");
    }

    #[test]
    fn test_custom_border_rendering() {
        let area = Rect::new(0, 0, 60, 15);

        for (box_type, expected_char) in [
            (BoxType::Border, "┌"),
            (BoxType::Rounded, "╭"),
            (BoxType::Block, "█"),
        ] {
            let mut config = Config::default();
            config.behavior.box_type = box_type;
            let ui = UI::new(config, vec![], vec![], None, None, None, vec![], false);
            let mut buf = Buffer::empty(area);
            Widget::render(&ui, area, &mut buf);

            let (bx, _, _) = UI::layout(area, false, false);
            let top_left = buf.cell((bx.x, bx.y)).unwrap();
            assert_eq!(
                top_left.symbol(),
                expected_char,
                "Expected top-left character for {:?}",
                box_type
            );
        }

        // Test BoxType::None
        let mut config = Config::default();
        config.behavior.box_type = BoxType::None;
        let ui = UI::new(config, vec![], vec![], None, None, None, vec![], false);
        let mut buf = Buffer::empty(area);
        Widget::render(&ui, area, &mut buf);
        let (bx, _, _) = UI::layout(area, false, false);
        let top_left = buf.cell((bx.x, bx.y)).unwrap();
        assert_ne!(top_left.symbol(), "┌");
        assert_ne!(top_left.symbol(), "╭");
        assert_ne!(top_left.symbol(), "█");
    }

    #[test]
    fn test_show_theme_rendering() {
        let mut config = Config::default();
        config.behavior.show_theme = true;

        let mut ui = UI::new(config.clone(), vec![], vec![], None, None, None, vec![], false);
        ui.adapter.state.themes = vec![crate::theme::Theme {
            name: "test_theme".to_string(),
            path: "/etc/lidm/themes/test_theme.toml".to_string(),
            config,
        }];
        ui.adapter.cycle_theme();
        let expected_path = ui.adapter.current_theme_path().to_string();

        let area = Rect::new(0, 0, 100, 20);
        let mut buf = Buffer::empty(area);

        Widget::render(&ui, area, &mut buf);

        // Top row (line 0) should render hotkeys
        let mut top_text = String::new();
        for x in 0..area.width {
            if let Some(cell) = buf.cell((x, 0)) {
                top_text.push_str(cell.symbol());
            }
        }
        assert!(
            top_text.contains("poweroff") && top_text.contains("F1"),
            "Top row should display hotkey shortcuts"
        );

        // Bottom row (line height-1) should render theme path
        let bottom_y = area.height - 1;
        let mut bottom_text = String::new();
        for x in 0..area.width {
            if let Some(cell) = buf.cell((x, bottom_y)) {
                bottom_text.push_str(cell.symbol());
            }
        }
        assert!(
            bottom_text.contains(&expected_path),
            "Bottom row should display active theme path when show_theme is true"
        );
    }
}
