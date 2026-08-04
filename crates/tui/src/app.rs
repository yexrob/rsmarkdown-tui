//! Component host: event loop, layout, focus routing and status bar.
//!
//! The host knows nothing about markdown — it owns a list of [`Component`]s,
//! routes input to the focused one and paints its output. This is the layer
//! that makes the TUI a *framework* rather than a markdown viewer.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::Terminal;

use crate::component::Component;
use crate::permission::{DialogAction, PermissionDialog, PermissionRequest};
use crate::renderer::theme;

/// A small status badge shown in the host footer (Claude Code style:
/// `⏸ plan mode on`, `← for agents`, `PR #446`).
#[derive(Debug, Clone)]
pub struct FooterBadge {
    /// Badge text (may contain glyphs).
    pub text: String,
    /// Badge style (color / modifiers).
    pub style: ratatui::style::Style,
}

impl FooterBadge {
    /// Create a badge.
    pub fn new(text: impl Into<String>, style: ratatui::style::Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

/// Session permission mode (displayed in the footer; `m` cycles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
    #[default]
    /// Standard permission mode — no badge.
    Default,
    /// Auto-accept file edits (`⏵⏵ accept edits on`).
    AcceptEdits,
    /// Read-only plan mode (`⏸ plan mode on`).
    Plan,
}

impl SessionMode {
    /// The mode after this one.
    pub fn next(self) -> Self {
        match self {
            SessionMode::Default => SessionMode::AcceptEdits,
            SessionMode::AcceptEdits => SessionMode::Plan,
            SessionMode::Plan => SessionMode::Default,
        }
    }

    /// Claude Code footer badge text for this mode.
    pub fn badge(self) -> Option<FooterBadge> {
        match self {
            SessionMode::Default => None,
            SessionMode::AcceptEdits => Some(FooterBadge::new(
                "⏵⏵ accept edits on (m to cycle)",
                theme::mode_on(),
            )),
            SessionMode::Plan => Some(FooterBadge::new(
                "⏸ plan mode on (m to cycle)",
                theme::mode_on(),
            )),
        }
    }
}

/// Review status of a pull request badge (underline color encodes it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrStatus {
    /// PR approved (green).
    Approved,
    /// PR pending review (yellow).
    Pending,
    /// Changes requested (red).
    ChangesRequested,
    /// Draft PR (gray).
    Draft,
}

impl PrStatus {
    /// Underline color for this status.
    pub fn color(self) -> ratatui::style::Color {
        match self {
            PrStatus::Approved => ratatui::style::Color::LightGreen,
            PrStatus::Pending => ratatui::style::Color::LightYellow,
            PrStatus::ChangesRequested => ratatui::style::Color::LightRed,
            PrStatus::Draft => ratatui::style::Color::DarkGray,
        }
    }
}

/// Component host: event loop, focus routing, mouse translation, footer badges.
pub struct App {
    /// Mounted components, in tab order.
    components: Vec<Box<dyn Component>>,
    /// Index of the focused component.
    focused: usize,
    /// Host tick interval.
    tick: Duration,
    /// When the last tick fired.
    last_tick: Instant,
    /// Content area of the last frame — used to translate mouse coordinates
    /// into component-local space before routing.
    content_area: ratatui::layout::Rect,
    /// Session permission mode.
    mode: SessionMode,
    /// Demo pull-request badge (None hides it).
    pr: Option<(u32, PrStatus)>,
    /// The open permission dialog, if any (modal: owns input while open).
    permission: Option<PermissionDialog>,
    /// Rect the dialog was drawn into last frame — mouse translation.
    dialog_rect: ratatui::layout::Rect,
    /// Completed dialog action awaiting the app owner.
    dialog_action: Option<DialogAction>,
}

impl App {
    /// Create an app with the given components.
    pub fn new(components: Vec<Box<dyn Component>>) -> Self {
        Self {
            components,
            focused: 0,
            tick: Duration::from_millis(33),
            last_tick: Instant::now(),
            content_area: ratatui::layout::Rect::default(),
            mode: SessionMode::default(),
            pr: None,
            permission: None,
            dialog_rect: ratatui::layout::Rect::default(),
            dialog_action: None,
        }
    }

    /// Index of the focused component.
    pub fn focused(&self) -> usize {
        self.focused
    }

    /// Mounted components, in tab order.
    pub fn components(&self) -> usize {
        self.components.len()
    }

    /// Focus the next component.
    pub fn focus_next(&mut self) {
        self.focused = (self.focused + 1) % self.components.len();
        self.last_tick = Instant::now();
    }

    /// Focus the previous component.
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
            terminal.draw(|f| self.draw_frame(f.area(), f.buffer_mut()))?;

            let timeout = self.tick.saturating_sub(self.last_tick.elapsed());
            if event::poll(timeout)? {
                if !self.route(event::read()?) {
                    return Ok(());
                }
            }
            if self.last_tick.elapsed() >= self.tick {
                self.tick_components();
                self.last_tick = Instant::now();
            }
        }
    }

    /// Access the focused component (tests).
    pub fn component_mut(&mut self, index: usize) -> &mut dyn Component {
        &mut *self.components[index]
    }

    /// Draw the host frame (layout + focused component + status bar).
    pub fn draw_frame(&mut self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let [content_area, status_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        self.content_area = content_area;

        let (title, status_text, hints, n) = {
            let focused = &self.components[self.focused];
            (
                focused.title().to_string(),
                focused.status(),
                focused.hints(),
                self.components.len(),
            )
        };
        let badges = self.footer_badges();
        if self.permission.is_none() {
            // modal: while a permission dialog is open the component behind
            // it is not rendered at all (nothing bleeds through the dialog)
            let focused = &mut self.components[self.focused];
            focused.draw(content_area, buf);
        }

        // status bar: focus index, title, badges (mode/agents/PR — most
        // important, first to survive narrow terminals), status, hints, keys
        let title_style = if n > 1 {
            theme::status()
        } else {
            theme::status_inactive()
        };
        let mut status = vec![Span::styled(
            format!(" [{}] {} ", self.focused + 1, title),
            title_style,
        )];
        for badge in &badges {
            status.push(Span::styled(format!("  {}", badge.text), badge.style));
        }
        if let Some(dialog) = &self.permission {
            // modal: the dialog replaces the component status text and hints
            status.push(Span::styled(
                format!(" {} — {}", dialog.request.title, dialog.request.question),
                theme::permission(),
            ));
            status.push(Span::styled(
                format!(
                    " esc cancel · enter confirm · digits 1-{}",
                    dialog.option_count()
                ),
                theme::dim(),
            ));
        } else {
            status.push(Span::styled(format!(" {}", status_text), theme::text()));
            if !hints.is_empty() {
                status.push(Span::styled(format!(" {}", hints), theme::dim()));
            }
            if n > 1 {
                status.push(Span::styled(
                    "  [Tab] next  [m] mode  [q] quit",
                    theme::dim(),
                ));
            } else {
                status.push(Span::styled("  [m] mode  [q] quit", theme::dim()));
            }
        }
        buf.set_line(0, status_area.y, &Line::from(status), status_area.width);

        // modal overlay: painted last so it covers the transcript
        if let Some(dialog) = &mut self.permission {
            self.dialog_rect = dialog.draw(content_area, buf);
        }
    }

    /// Route one input event: focus translation for mouse, then the focused
    /// component, falling back to host keys. Returns `false` to quit.
    pub fn route(&mut self, event: Event) -> bool {
        if self.permission.is_some() {
            return self.route_dialog(event);
        }
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let consumed = {
                    let focused = &mut self.components[self.focused];
                    focused.event(Event::Key(key))
                };
                if consumed {
                    self.last_tick = Instant::now();
                    return true;
                }
                self.handle_host_key(key)
            }
            Event::Mouse(m) => {
                // translate absolute terminal coordinates to the focused
                // component's local space (clicks on the status bar drop)
                let area = self.content_area;
                let inside = m.column >= area.x
                    && m.row >= area.y
                    && m.column < area.x + area.width
                    && m.row < area.y + area.height;
                if inside {
                    let local = MouseEvent {
                        kind: m.kind,
                        column: m.column - area.x,
                        row: m.row - area.y,
                        modifiers: m.modifiers,
                    };
                    let focused = &mut self.components[self.focused];
                    focused.event(Event::Mouse(local));
                }
                true
            }
            _ => true,
        }
    }

    /// Advance the focused component by one tick, broadcast agent sessions
    /// (components publish via [`Component::agents`], all components receive
    /// the merged table via [`Component::absorb_agents`]), then raise any
    /// permission request the focused component asks for.
    pub fn tick_components(&mut self) {
        if self.permission.is_some() {
            return; // modal: the dialog owns the session
        }
        {
            let focused = &mut self.components[self.focused];
            focused.on_tick();
        }
        // agent broadcast: merge every component's sessions by name (last
        // publisher wins) and deliver to all components
        let mut merged: Vec<crate::agent::Agent> = Vec::new();
        for component in &self.components {
            for agent in component.agents() {
                if let Some(slot) = merged.iter_mut().find(|a| a.name == agent.name) {
                    *slot = agent;
                } else {
                    merged.push(agent);
                }
            }
        }
        for component in &mut self.components {
            component.absorb_agents(&merged);
        }
        if self.permission.is_none() {
            let focused = &mut self.components[self.focused];
            if let Some(request) = focused.on_ask() {
                self.ask(request);
            }
        }
    }

    /// Open a modal permission dialog (replaces one already open).
    pub fn ask(&mut self, request: PermissionRequest) {
        self.permission = Some(PermissionDialog::new(request));
        self.dialog_action = None;
        self.last_tick = Instant::now();
    }

    /// Whether a permission dialog is currently open (modal).
    pub fn permission_open(&self) -> bool {
        self.permission.is_some()
    }

    /// Rect the open dialog was drawn into last frame (mouse translation).
    pub fn dialog_rect(&self) -> ratatui::layout::Rect {
        self.dialog_rect
    }

    /// Last completed dialog action (the dialog itself has already closed).
    /// Returns `None` if no dialog has completed since the last call.
    pub fn take_dialog_action(&mut self) -> Option<DialogAction> {
        self.dialog_action.take()
    }

    /// Route one event while the modal dialog owns the session: keys go to
    /// the dialog, mouse clicks inside the dialog are translated to
    /// dialog-local coordinates, clicks outside drop.
    fn route_dialog(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(action) = self.permission.as_mut().expect("dialog").key(key) {
                    self.close_dialog(action);
                }
                true
            }
            Event::Mouse(m) => {
                let r = self.dialog_rect;
                let inside = m.column >= r.x
                    && m.row >= r.y
                    && m.column < r.x + r.width
                    && m.row < r.y + r.height;
                if inside {
                    let local = MouseEvent {
                        kind: m.kind,
                        column: m.column - r.x,
                        row: m.row - r.y,
                        modifiers: m.modifiers,
                    };
                    if let Some(action) = self.permission.as_mut().expect("dialog").click(&local) {
                        self.close_dialog(action);
                    }
                }
                true
            }
            _ => true,
        }
    }

    /// Close the dialog, notify the focused component, and record the action
    /// for the app owner.
    fn close_dialog(&mut self, action: DialogAction) {
        self.dialog_action = Some(action);
        self.permission = None;
        let focused = &mut self.components[self.focused];
        focused.on_dialog_closed(action);
        self.last_tick = Instant::now();
    }

    /// Current session mode (tests).
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Cycle the session permission mode (`m`).
    pub fn cycle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    /// Set the demo pull-request badge (`None` hides it).
    pub fn set_pr(&mut self, number: u32, status: PrStatus) {
        self.pr = Some((number, status));
    }

    /// Footer badges: session mode + focused component's own badges.
    fn footer_badges(&self) -> Vec<FooterBadge> {
        let mut badges = Vec::new();
        if let Some(b) = self.mode.badge() {
            badges.push(b);
        }
        if let Some((n, status)) = self.pr {
            let mut style = ratatui::style::Style::default()
                .fg(status.color())
                .add_modifier(ratatui::style::Modifier::UNDERLINED);
            style = style.add_modifier(ratatui::style::Modifier::BOLD);
            badges.push(FooterBadge::new(format!("PR #{}", n), style));
        }
        let focused = &self.components[self.focused];
        badges.extend(focused.footer_badges());
        badges
    }

    /// Host-level keys (fallback when the focused component did not consume).
    fn handle_host_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => false,
            KeyCode::Char('m') => {
                self.cycle_mode();
                true
            }
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
