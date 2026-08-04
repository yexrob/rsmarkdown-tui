//! Library usage example: a custom component mounted into the host — no
//! markdown, no demo — showing `rsmarkdown-tui` as a reusable TUI library.
//!
//! Run: `cargo run -p rsmarkdown-tui --example custom`

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use rsmarkdown_tui::{run_tui, App, Component, FooterBadge};

/// A counter component: two plain-text rows and a key that increments.
struct Counter {
    value: u64,
}

impl Component for Counter {
    fn title(&self) -> &str {
        "counter"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![Span::styled(
            "Counter",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )]);
        let value = Line::from(vec![
            Span::styled("value: ", Style::default()),
            Span::styled(
                self.value.to_string(),
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   (press + to increment, - to decrement)",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        buf.set_line(area.x, area.y, &title, area.width);
        buf.set_line(area.x, area.y + 1, &value, area.width);
    }

    fn event(&mut self, event: Event) -> bool {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.value += 1;
                        return true;
                    }
                    KeyCode::Char('-') => {
                        self.value = self.value.saturating_sub(1);
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    fn status(&self) -> String {
        format!("value {}", self.value)
    }

    fn footer_badges(&self) -> Vec<FooterBadge> {
        vec![FooterBadge::new(
            format!("value {}", self.value),
            Style::default().fg(Color::LightGreen),
        )]
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App::new(vec![Box::new(Counter { value: 0 })]);
    run_tui(&mut app)
}
