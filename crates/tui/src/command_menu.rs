//! Slash-command menu: a filterable command list shown when `/` is typed
//! into an empty prompt (Claude Code interactive mode).
//!
//! Evidence (research doc §4.2): commands appear as `/command` with a
//! description; the list is filterable as the user types; arrow keys move
//! the selection and Enter confirms. The trigger symbol is both input
//! content and a visual mode hint.

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::permission::erase_overlay;
use crate::renderer::theme;

/// One slash command (`/name` with a description).
#[derive(Debug, Clone)]
pub struct SlashCommand {
    /// Command name, without the leading `/`.
    pub name: String,
    /// One-line description shown next to the name.
    pub description: String,
}

impl SlashCommand {
    /// Create a command.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Max visible command rows of the floating menu.
pub const MENU_ROWS: usize = 8;

/// A filterable slash-command list (opened by typing `/` into an empty
/// prompt). The caller owns the input text: it forwards filter characters
/// via [`SlashCommandMenu::set_filter`] and selection keys via
/// [`SlashCommandMenu::key`].
pub struct SlashCommandMenu {
    /// All commands, in menu order.
    pub commands: Vec<SlashCommand>,
    /// Indices into `commands` matching the current filter, in order.
    filtered_indices: Vec<usize>,
    /// Current filter text (what follows `/` in the prompt).
    pub filter: String,
    /// Selected row (index into the filtered list).
    pub selected: usize,
    /// Scroll offset of the filtered list (rows above the window).
    scroll: usize,
    /// Rect of the last draw (click mapping).
    last_rect: Rect,
}

impl SlashCommandMenu {
    /// Open a menu over the given commands (filter empty).
    pub fn new(commands: Vec<SlashCommand>) -> Self {
        let mut this = Self {
            commands,
            filtered_indices: Vec::new(),
            filter: String::new(),
            selected: 0,
            scroll: 0,
            last_rect: Rect::default(),
        };
        this.set_filter("");
        this
    }

    /// Filter the list by the text after `/` (case-insensitive prefix
    /// match). Selection resets to the first match.
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        let needle = filter.to_lowercase();
        self.filtered_indices = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name.to_lowercase().starts_with(&needle))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll = 0;
    }

    /// Commands matching the current filter, in order.
    pub fn filtered(&self) -> Vec<&SlashCommand> {
        self.filtered_indices
            .iter()
            .map(|&i| &self.commands[i])
            .collect()
    }

    /// Handle a selection key. Returns the absolute index (into
    /// [`Self::commands`]) of the confirmed command, if Enter was pressed
    /// with a non-empty match list.
    pub fn key(&mut self, key: KeyEvent) -> Option<usize> {
        let n = self.filtered_indices.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if n > 0 {
                    self.selected = (self.selected + n - 1) % n;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if n > 0 {
                    self.selected = (self.selected + 1) % n;
                }
                None
            }
            KeyCode::Enter => self.filtered_indices.get(self.selected).copied(),
            _ => None,
        }
    }

    /// Handle a mouse event in menu-local coordinates (origin at the
    /// menu's top-left). First click selects a row; a click on the
    /// already-selected row confirms it.
    pub fn click(&mut self, m: &MouseEvent) -> Option<usize> {
        if m.kind != MouseEventKind::Down(MouseButton::Left) || self.last_rect.width == 0 {
            return None;
        }
        let visible = self.filtered_indices.len().min(MENU_ROWS);
        let row = m.row as usize;
        if u16::from(m.column) >= self.last_rect.width || row < 1 || row >= 1 + visible {
            return None;
        }
        let vis_idx = row - 1;
        let abs = self.filtered_indices[self.scroll + vis_idx];
        if self.selected == self.scroll + vis_idx {
            Some(abs)
        } else {
            self.selected = self.scroll + vis_idx;
            None
        }
    }

    /// Paint the menu directly above the bottom of `area` (the prompt):
    /// flush to the left, same width as the prompt line. Returns the rect
    /// used (the caller translates mouse events through it).
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) -> Rect {
        let visible = self.filtered_indices.len().min(MENU_ROWS);
        let height = visible as u16 + 2; // borders
        let rect = Rect {
            x: area.x,
            y: area.y.saturating_sub(height),
            width: area.width,
            height,
        };
        // cover the menu's own area so the transcript does not show through
        erase_overlay(buf, rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .style(theme::overlay())
            .border_style(theme::overlay_border())
            .title(Line::from(Span::styled(
                format!("/{}", self.filter),
                theme::tool_running(),
            )))
            .title_alignment(ratatui::layout::Alignment::Left);
        let inner = block.inner(rect);
        block.render(rect, buf);
        self.last_rect = rect;

        self.scroll = self
            .scroll
            .min(self.filtered_indices.len().saturating_sub(MENU_ROWS));
        for (i, &abs) in self
            .filtered_indices
            .iter()
            .skip(self.scroll)
            .take(MENU_ROWS)
            .enumerate()
        {
            let selected = self.scroll + i == self.selected;
            let cmd = &self.commands[abs];
            let marker = if selected { "❯" } else { " " };
            let name_style = if selected {
                theme::tool_running()
            } else {
                theme::text()
            };
            let desc_style = if selected {
                theme::text()
            } else {
                theme::dim()
            };
            let line = Line::from(vec![
                Span::styled(format!("{} /{}", marker, cmd.name), name_style),
                Span::styled(format!("  {}", cmd.description), desc_style),
            ]);
            buf.set_line(
                inner.x + 1,
                inner.y + i as u16,
                &line,
                inner.width.saturating_sub(1),
            );
        }
        rect
    }
}
