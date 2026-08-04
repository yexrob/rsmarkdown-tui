//! Selectable task-list component — demonstrates interactive state
//! (selection, toggling) that markdown rendering does not have.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::component::Component;
use crate::renderer::theme;

pub struct TodoItem {
    text: &'static str,
    done: bool,
}

pub struct ListView {
    items: Vec<TodoItem>,
    selected: usize,
    scroll: u16,
}

impl ListView {
    pub fn todo_examples() -> Self {
        let items = [
            "Stream a markdown document",
            "Type markdown with unclosed markers",
            "Benchmark the incremental parser",
            "Port the fix functions one-to-one",
            "Write the README",
            "Ship the criterion benchmarks",
            "Refactor the TUI into components",
            "Add a non-markdown component",
            "Polish the status bar",
            "Write CJK-aware wrapping tests",
            "Push the repo to GitHub",
            "Celebrate with a terminal break",
        ]
        .into_iter()
        .map(|text| TodoItem { text, done: false })
        .collect();
        Self {
            items,
            selected: 0,
            scroll: 0,
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let n = self.items.len() as i32;
        self.selected = ((self.selected as i32 + delta).rem_euclid(n)) as usize;
    }

    fn toggle(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            item.done = !item.done;
        }
    }

    fn done_count(&self) -> usize {
        self.items.iter().filter(|i| i.done).count()
    }
}

impl Component for ListView {
    fn title(&self) -> &str {
        "tasks"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        // keep selection visible
        if self.selected < self.scroll as usize {
            self.scroll = self.selected as u16;
        } else if self.selected >= self.scroll as usize + area.height as usize {
            self.scroll = (self.selected - area.height as usize + 1) as u16;
        }

        for (y, idx) in (self.scroll as usize..)
            .take(area.height as usize)
            .enumerate()
        {
            let Some(item) = self.items.get(idx) else {
                break;
            };
            let selected = idx == self.selected;
            let (marker, marker_style) = if item.done {
                ("[x]", theme::task_done())
            } else {
                ("[ ]", theme::task_open())
            };
            let mut style = if item.done {
                Style::default().add_modifier(Modifier::DIM)
            } else {
                Style::default()
            };
            if selected {
                style = style.add_modifier(Modifier::REVERSED);
            }
            let line = Line::from(vec![
                Span::styled(marker.to_string(), marker_style),
                Span::styled(" ".to_string(), style),
                Span::styled(item.text.to_string(), style),
            ]);
            buf.set_line(area.x, area.y + y as u16, &line, area.width);
        }
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.move_selection(1);
                    true
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.move_selection(-1);
                    true
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.toggle();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn status(&self) -> String {
        format!(
            "{}/{} done  selected {}",
            self.done_count(),
            self.items.len(),
            self.selected + 1
        )
    }

    fn hints(&self) -> &'static str {
        "[j/k] move  [space] toggle"
    }
}
