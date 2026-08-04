//! Permission dialog: Claude Code style numbered options with a `❯` marker.
//!
//! Evidence (research doc §4.1): a tool permission prompt has a title, a
//! target summary, the question *"Do you want to proceed?"*, and numbered
//! options; the current selection is prefixed with `❯`; `Esc` dismisses
//! (the last option is labelled "(esc)"); background subagent prompts name
//! the requesting agent. The dialog is modal — while open it owns the
//! keyboard and is painted as an overlay above the transcript.
//!
//! The host ([`crate::App`]) opens it via [`crate::App::ask`], or a component
//! raises it through [`crate::Component::on_ask`]. Outcomes reach the app
//! owner through [`crate::App::take_dialog_action`].

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::renderer::theme::Theme;

/// Erase `rect` with spaces on the overlay background, so whatever the
/// component behind drew there does not bleed through the panel.
pub fn erase_overlay(buf: &mut Buffer, rect: Rect, theme: &Theme) {
    let fill = " ".repeat(rect.width as usize);
    for y in rect.y..rect.y + rect.height {
        buf.set_string(rect.x, y, &fill, theme.overlay());
    }
}

/// Outcome of the dialog: an option was chosen, or it was dismissed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    /// Option `index` (0-based) was confirmed.
    Confirm(usize),
    /// The dialog was dismissed with `Esc`.
    Cancel,
}

/// A permission request to show: title, target summary, question, numbered
/// options, and optional pre-rendered content preview (e.g. the diff being
/// reviewed).
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Dialog title, e.g. `Read file` / `Edit file`.
    pub title: String,
    /// Target summary shown under the title, e.g. `AP-Cleanup-Documentation.md`.
    pub target: Option<String>,
    /// Requesting agent (e.g. `subagent "explore"`), shown when present.
    pub source: Option<String>,
    /// The question, e.g. `Do you want to proceed?`.
    pub question: String,
    /// Numbered options (numbers are added automatically).
    pub options: Vec<String>,
    /// Pre-rendered content preview, drawn verbatim above the question.
    /// The caller supplies already-styled lines — the demo, for example,
    /// feeds this with [`crate::activities::diff_lines`].
    pub content: Vec<Line<'static>>,
}

impl PermissionRequest {
    /// Create a request with the given title, question and options.
    pub fn new(
        title: impl Into<String>,
        question: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            title: title.into(),
            target: None,
            source: None,
            question: question.into(),
            options,
            content: Vec::new(),
        }
    }

    /// Set the target summary line.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set the requesting agent line.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Attach pre-rendered content lines (drawn above the question).
    pub fn content(mut self, lines: Vec<Line<'static>>) -> Self {
        self.content = lines;
        self
    }
}

/// Max content preview rows inside the dialog; longer content gets a
/// `… +N more` tail.
pub const CONTENT_CAP: usize = 8;

/// The open modal: a request plus the current selection.
pub struct PermissionDialog {
    /// The request being presented.
    pub request: PermissionRequest,
    /// Index of the selected option.
    pub selected: usize,
    /// Rect used by the last draw (option row math).
    last_rect: Rect,
    /// Semantic color theme.
    pub theme: Theme,
}

impl PermissionDialog {
    /// Open a dialog for the given request (first option selected).
    pub fn new(request: PermissionRequest) -> Self {
        Self {
            request,
            selected: 0,
            last_rect: Rect::default(),
            theme: Theme::dark(),
        }
    }

    /// Open a dialog with an explicit theme.
    pub fn with_theme(request: PermissionRequest, theme: Theme) -> Self {
        Self {
            request,
            selected: 0,
            last_rect: Rect::default(),
            theme,
        }
    }

    /// Number of options.
    pub fn option_count(&self) -> usize {
        self.request.options.len()
    }

    /// Handle one key. Returns the resulting action, or `None` if the dialog
    /// stays open (selection moved).
    pub fn key(&mut self, key: KeyEvent) -> Option<DialogAction> {
        let n = self.option_count();
        if n == 0 {
            return Some(DialogAction::Cancel);
        }
        match key.code {
            KeyCode::Esc => Some(DialogAction::Cancel),
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                self.selected = (self.selected + n - 1) % n;
                None
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % n;
                None
            }
            KeyCode::Enter => Some(DialogAction::Confirm(self.selected)),
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u8 - b'1') as usize;
                if idx < n {
                    Some(DialogAction::Confirm(idx))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handle a mouse event in dialog-local coordinates (origin at the
    /// dialog's top-left). First click selects an option; a click on the
    /// already-selected option confirms it.
    pub fn click(&mut self, m: &MouseEvent) -> Option<DialogAction> {
        if m.kind != MouseEventKind::Down(MouseButton::Left) {
            return None;
        }
        if let Some(i) = self.option_at(m.row) {
            if i == self.selected {
                Some(DialogAction::Confirm(i))
            } else {
                self.selected = i;
                None
            }
        } else {
            None
        }
    }

    /// Option index whose row (dialog-local) contains `y`, if any.
    fn option_at(&self, y: u16) -> Option<usize> {
        let n = self.option_count();
        if n == 0 || self.last_rect.height < 2 + n as u16 {
            return None;
        }
        // options are the last inner rows: bottom border at `height - 1`,
        // one padding row above it, then the options bottom-up
        let last = self.last_rect.height - 2;
        let first = last - n as u16;
        let row = y;
        if row >= first && row < last {
            Some((row - first) as usize)
        } else {
            None
        }
    }

    /// Paint the dialog as a bottom block of `area`: full width, flush to
    /// the bottom (the host's status bar sits directly below). Returns the
    /// rect actually used (the host translates mouse events through it).
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) -> Rect {
        let content_count = self.request.content.len();
        let content_rows = content_count.min(CONTENT_CAP);
        let extra = if content_count > CONTENT_CAP { 1 } else { 0 };
        let rows = 2
            + usize::from(self.request.target.is_some())
            + usize::from(self.request.source.is_some())
            + content_rows
            + extra
            + 1 // question
            + self.option_count()
            + 1; // bottom padding
        let height = (rows as u16).min(area.height);
        let rect = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(height),
            width: area.width,
            height,
        };
        // a bordered box needs at least 2 rows; tiny areas just skip the
        // dialog instead of underflowing the inner rect
        if rect.height < 2 {
            self.last_rect = rect;
            return rect;
        }

        // cover the panel's own area so content behind it does not show
        // through; everything outside the rect keeps rendering normally
        erase_overlay(buf, rect, &self.theme);
        let block = Block::default()
            .borders(Borders::ALL)
            .style(self.theme.overlay())
            .border_style(self.theme.permission())
            .title(Line::from(Span::styled(
                self.request.title.clone(),
                self.theme.permission(),
            )))
            .title_alignment(ratatui::layout::Alignment::Left);
        let inner = block.inner(rect);
        block.render(rect, buf);
        self.last_rect = rect;

        let mut y = inner.y;
        if let Some(target) = &self.request.target {
            buf.set_line(
                inner.x + 2,
                y,
                &Line::from(Span::styled(target, self.theme.dim())),
                inner.width.saturating_sub(2),
            );
            y += 1;
        }
        if let Some(source) = &self.request.source {
            buf.set_line(
                inner.x + 2,
                y,
                &Line::from(vec![
                    Span::styled("⏺ ", self.theme.activity_dot()),
                    Span::styled(source, self.theme.dim()),
                ]),
                inner.width.saturating_sub(2),
            );
            y += 1;
        }
        for line in self.request.content.iter().take(content_rows) {
            buf.set_line(inner.x + 1, y, line, inner.width.saturating_sub(1));
            y += 1;
        }
        if content_count > CONTENT_CAP {
            buf.set_line(
                inner.x + 2,
                y,
                &Line::from(Span::styled(
                    format!("… +{} more", content_count - content_rows),
                    self.theme.dim(),
                )),
                inner.width.saturating_sub(2),
            );
            y += 1;
        }
        // question
        buf.set_line(
            inner.x + 2,
            y,
            &Line::from(Span::styled(
                self.request.question.clone(),
                self.theme.text(),
            )),
            inner.width.saturating_sub(2),
        );
        y += 1;
        // numbered options; `❯` marks the selection
        for (i, option) in self.request.options.iter().enumerate() {
            let selected = i == self.selected;
            let marker = if selected { "❯ " } else { "  " };
            let spans = vec![
                Span::styled(
                    marker,
                    if selected {
                        self.theme.permission_selected()
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(format!("{}. {}", i + 1, option), self.theme.text()),
            ];
            buf.set_line(
                inner.x + 2,
                y,
                &Line::from(spans),
                inner.width.saturating_sub(2),
            );
            y += 1;
        }
        rect
    }
}
