//! Keybinding help panel, toggled with `?` in an empty prompt (Claude Code
//! interactive mode).
//!
//! Evidence (research doc §4.2): the panel is a temporary overlay on the
//! main view listing the current view's keyboard shortcuts; `?` expands /
//! collapses it, `Esc` closes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::renderer::theme;

/// One keybinding row.
#[derive(Debug, Clone)]
pub struct HelpEntry {
    /// Key name(s), e.g. `enter`, `esc`, `j/k`.
    pub keys: &'static str,
    /// What the key does.
    pub description: &'static str,
}

/// A titled group of keybindings.
#[derive(Debug, Clone)]
pub struct HelpSection {
    /// Section title (e.g. `chat`, `host`).
    pub title: &'static str,
    /// Keybindings in this section.
    pub entries: Vec<HelpEntry>,
}

/// The help overlay: sections of keybindings, scrollable.
pub struct HelpPanel {
    /// Sections, in display order.
    pub sections: Vec<HelpSection>,
    /// Scroll offset of the panel content.
    pub scroll: u16,
}

/// Column width reserved for the key names.
pub const KEYS_COL: u16 = 14;

impl HelpPanel {
    /// Build a panel from sections (scroll starts at the top).
    pub fn new(sections: Vec<HelpSection>) -> Self {
        Self {
            sections,
            scroll: 0,
        }
    }

    /// Handle one key. Returns `true` when the panel was closed (`Esc`).
    pub fn key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll += 1;
                false
            }
            _ => false,
        }
    }

    /// Paint the panel centered in `area` (bordered overlay).
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let width = area.width.min(68);
        let rows = self.total_rows();
        let height = (rows + 2).min(area.height).max(2); // +2 for the borders
        let rect = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(Line::from(Span::styled(
                "Keyboard shortcuts",
                theme::tool_running(),
            )))
            .title_alignment(ratatui::layout::Alignment::Left);
        let inner = block.inner(rect);
        block.render(rect, buf);

        // cap the scroll so the last row stays visible
        let view = inner.height;
        if rows > view {
            self.scroll = self.scroll.min(rows - view);
        } else {
            self.scroll = 0;
        }

        let mut y = inner.y;
        let mut row = 0u16;
        let skip = self.scroll;
        for section in &self.sections {
            if row >= skip + view {
                break;
            }
            // section title
            if row >= skip && y <= inner.y + inner.height - 1 {
                buf.set_line(
                    inner.x + 1,
                    y,
                    &Line::from(Span::styled(section.title, theme::dim())),
                    inner.width.saturating_sub(1),
                );
                y += 1;
                row += 1;
            } else {
                row += 1;
            }
            for entry in &section.entries {
                if row >= skip + view {
                    break;
                }
                if row >= skip && y <= inner.y + inner.height - 1 {
                    let keys = format!("{}", entry.keys);
                    let padding = KEYS_COL.saturating_sub(keys.chars().count() as u16);
                    let line = Line::from(vec![
                        Span::styled(keys, theme::tool_running()),
                        Span::styled(" ".repeat(padding as usize), theme::dim()),
                        Span::styled(entry.description, theme::text()),
                    ]);
                    buf.set_line(inner.x + 1, y, &line, inner.width.saturating_sub(1));
                    y += 1;
                }
                row += 1;
            }
        }
    }

    /// Total content rows (titles + entries).
    pub fn total_rows(&self) -> u16 {
        let mut n = 0u16;
        for section in &self.sections {
            n += 1 + section.entries.len() as u16;
        }
        n
    }
}
