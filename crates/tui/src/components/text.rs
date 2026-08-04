//! Plain-text streaming log viewer — a non-markdown component proving the
//! host works for arbitrary content.

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::component::Component;

const MAX_LINES: usize = 10_000;

/// Streaming plain-text log component.
pub struct TextReader {
    lines: Vec<Line<'static>>,
    scroll: u16,
    auto_scroll: bool,
    seq: u64,
}

impl Default for TextReader {
    fn default() -> Self {
        Self::new()
    }
}

impl TextReader {
    /// Create an empty log; it starts generating entries on tick.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            auto_scroll: true,
            seq: 0,
        }
    }

    fn emit(&mut self) {
        let level = self.seq % 10;
        let (tag, style) = match level {
            0..=6 => ("INFO ", Style::default().fg(Color::LightGreen)),
            7..=8 => ("WARN ", Style::default().fg(Color::LightYellow)),
            _ => ("ERROR", Style::default().fg(Color::LightRed)),
        };
        let ts = format!(
            "{:02}:{:02}:{:02}.{:03}",
            (self.seq * 3) % 24,
            (self.seq * 7) % 60,
            (self.seq * 13) % 60,
            (self.seq * 37) % 1000,
        );
        let msg = if level >= 9 {
            format!("request #{} failed after retries (timeout)", self.seq)
        } else {
            format!(
                "request #{} ok in {}.{}ms",
                self.seq,
                1 + level,
                (self.seq * 7) % 10
            )
        };
        let line = Line::from(vec![
            Span::styled(format!("{}  ", ts), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", tag), style),
            Span::styled(msg, Style::default()),
        ]);
        self.lines.push(line);
        if self.lines.len() > MAX_LINES {
            self.lines.drain(0..self.lines.len() - MAX_LINES);
        }
        self.seq += 1;
    }

    fn scroll_by(&mut self, delta: i16) {
        self.scroll = self.scroll.saturating_add_signed(delta);
        self.auto_scroll = false;
    }
}

impl Component for TextReader {
    fn title(&self) -> &str {
        "log"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let total = self.lines.len() as u16;
        if self.auto_scroll {
            self.scroll = total.saturating_sub(area.height);
        }
        let scroll = self.scroll.min(total.saturating_sub(area.height));
        for (y, line) in self
            .lines
            .iter()
            .skip(scroll as usize)
            .take(area.height as usize)
            .enumerate()
        {
            buf.set_line(area.x, area.y + y as u16, line, area.width);
        }
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_by(1);
                    true
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_by(-1);
                    true
                }
                KeyCode::PageDown => {
                    self.scroll_by(10);
                    true
                }
                KeyCode::PageUp => {
                    self.scroll_by(-10);
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.scroll = 0;
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.scroll = u16::MAX;
                    true
                }
                KeyCode::Char('s') => {
                    self.auto_scroll = !self.auto_scroll;
                    true
                }
                _ => false,
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_by(-3);
                    true
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_by(3);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn on_tick(&mut self) {
        // one line per host tick (~30 lines/s at the default tick rate)
        self.emit();
    }

    fn status(&self) -> String {
        format!(
            "{} lines seq {} {}/{}",
            self.lines.len(),
            self.seq,
            self.scroll,
            self.lines.len()
        )
    }

    fn hints(&self) -> &'static str {
        "[j/k] scroll  [s] autoscroll"
    }
}
