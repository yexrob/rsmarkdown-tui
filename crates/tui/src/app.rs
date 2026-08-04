//! Component host: event loop, layout, focus routing and status bar.
//!
//! The host knows nothing about markdown — it owns a list of [`Component`]s,
//! routes input to the focused one and paints its output. This is the layer
//! that makes the TUI a *framework* rather than a markdown viewer.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Frame, Terminal};

use crate::component::Component;
use crate::renderer::theme;

pub struct App {
    components: Vec<Box<dyn Component>>,
    focused: usize,
    tick: Duration,
    last_tick: Instant,
}

impl App {
    pub fn new(components: Vec<Box<dyn Component>>) -> Self {
        Self {
            components,
            focused: 0,
            tick: Duration::from_millis(33),
            last_tick: Instant::now(),
        }
    }

    pub fn focused(&self) -> usize {
        self.focused
    }

    pub fn components(&self) -> usize {
        self.components.len()
    }

    pub fn focus_next(&mut self) {
        self.focused = (self.focused + 1) % self.components.len();
        self.last_tick = Instant::now();
    }

    pub fn focus_prev(&mut self) {
        self.focused = (self.focused + self.components.len() - 1) % self.components.len();
        self.last_tick = Instant::now();
    }

    /// Run the event loop until a component asks to quit (`q`).
    pub fn run(
        &mut self,
        terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| draw(f, self))?;

            let timeout = self.tick.saturating_sub(self.last_tick.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        let consumed = {
                            let focused = &mut self.components[self.focused];
                            focused.event(Event::Key(key))
                        };
                        if !consumed && !self.handle_host_key(key) {
                            return Ok(());
                        }
                        self.last_tick = Instant::now();
                    }
                    other => {
                        let consumed = {
                            let focused = &mut self.components[self.focused];
                            focused.event(other)
                        };
                        let _ = consumed;
                    }
                }
            }
            if self.last_tick.elapsed() >= self.tick {
                let focused = &mut self.components[self.focused];
                focused.on_tick();
                self.last_tick = Instant::now();
            }
        }
    }

    /// Host-level keys (fallback when the focused component did not consume).
    fn handle_host_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => false,
            KeyCode::Tab => {
                self.focus_next();
                true
            }
            KeyCode::BackTab => {
                self.focus_prev();
                true
            }
            KeyCode::Char(']') => {
                self.focus_next();
                true
            }
            KeyCode::Char('[') => {
                self.focus_prev();
                true
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u8 - b'1') as usize;
                if idx < self.components.len() {
                    self.focused = idx;
                }
                true
            }
            _ => true,
        }
    }
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let [content_area, status_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let (title, status_text, hints, n) = {
        let focused = &app.components[app.focused];
        (
            focused.title().to_string(),
            focused.status(),
            focused.hints(),
            app.components.len(),
        )
    };
    let focused = &mut app.components[app.focused];
    focused.draw(content_area, f.buffer_mut());

    // status bar: focus index, component title, its status line and hints
    let title_style = if n > 1 {
        theme::status()
    } else {
        theme::status_inactive()
    };
    let mut status = vec![
        Span::styled(format!(" [{}] {} ", app.focused + 1, title), title_style),
        Span::styled(format!(" {} ", status_text), theme::text()),
    ];
    if !hints.is_empty() {
        status.push(Span::styled(format!(" {}", hints), theme::dim()));
    }
    if n > 1 {
        status.push(Span::styled("  [Tab] next  [q] quit", theme::dim()));
    } else {
        status.push(Span::styled("  [q] quit", theme::dim()));
    }
    f.render_widget(Paragraph::new(Line::from(status)), status_area);
}

/// Convenience: enter raw mode + alternate screen, run the app, restore.
pub fn run_tui(app: &mut App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    crossterm::execute!(stdout, crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    let result = app.run(&mut terminal);
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}
