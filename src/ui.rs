use crate::config::Config;
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
    Refresh,
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
    ) -> Self {
        Self {
            adapter: UIAdapter::new(
                config,
                sessions,
                users,
                initial_user,
                initial_session,
                console_buffer,
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
        match event::poll(std::time::Duration::from_millis(250))? {
            true => match event::read()? {
                Event::Key(key) => Ok(self.handle_key_event(key)),
                _ => Ok(None),
            },
            false => Ok(None),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<UIResult> {
        if let Some(action) = self.adapter.check_hotkey(key.code) {
            return match action {
                HotkeyAction::Poweroff => Some(UIResult::Poweroff),
                HotkeyAction::Reboot => Some(UIResult::Reboot),
                HotkeyAction::Refresh => Some(UIResult::Refresh),
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
        let (_, chunks, _) = Self::layout(f.area(), self.adapter.show_console());
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

    fn layout(area: Rect, show_console: bool) -> (Rect, [Rect; 4], Option<Rect>) {
        let box_h = 12;
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
        (
            login_area,
            Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(3),
            ])
            .areas(inner),
            console_area,
        )
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

        // Background
        Block::default()
            .style(Style::from(cfg.colors.bg.clone()))
            .render(area, buf);

        let (bx, chunks, console_area) = UI::layout(area, show_console);

        // Box border
        let border = match cfg.behavior.box_type.as_str() {
            "none" => Block::default().borders(Borders::NONE),
            "block" => Block::default()
                .borders(Borders::ALL)
                .border_set(ratatui_core::symbols::border::FULL),
            "rounded" => Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
            _ => Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Plain),
        }
        .style(Style::from(cfg.colors.e_box.clone()));
        border.render(bx, buf);

        // Header: hostname left, clock right
        let [_, hdr] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(chunks[0]);
        let [hl, hr] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(hdr);

        let hostname = nix::unistd::gethostname()
            .unwrap_or_else(|_| "unknown".into())
            .to_string_lossy()
            .into_owned();
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

        // Console panel
        if let Some(console_rect) = console_area {
            let console_lines = self.adapter.console_lines();
            let visible_h = console_rect.height.saturating_sub(2) as usize; // border takes 2 lines
            let skip = console_lines.len().saturating_sub(visible_h);
            let lines: Vec<Line> = console_lines[skip..]
                .iter()
                .map(|l| {
                    Line::from(Span::styled(
                        l.clone(),
                        Style::default().add_modifier(Modifier::DIM),
                    ))
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

        // Footer
        let hotkeys = [
            (&cfg.functions.poweroff, &cfg.strings.f_poweroff),
            (&cfg.functions.reboot, &cfg.strings.f_reboot),
            (&cfg.functions.refresh, &cfg.strings.f_refresh),
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
        Paragraph::new(Line::from(spans))
            .alignment(ratatui::layout::Alignment::Right)
            .render(area, buf);
    }
}
