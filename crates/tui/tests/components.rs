//! Headless smoke test for the component framework: mount all built-in
//! components, draw them into buffers, tick them, and assert visible output.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use rsmarkdown_tui::components::list::ListView;
use rsmarkdown_tui::components::markdown::MarkdownViewer;
use rsmarkdown_tui::components::text::TextReader;
use rsmarkdown_tui::Component;

fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        out.push('\n');
    }
    out
}

#[test]
fn markdown_component_draws() {
    let mut c = MarkdownViewer::new();
    // mirror the host loop: tick (streams the demo) then draw
    for _ in 0..30 {
        c.on_tick();
    }
    let area = Rect::new(0, 0, 80, 12);
    let mut buf = Buffer::empty(area);
    c.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(
        text.contains("rsmarkdown"),
        "markdown content visible:\n{text}"
    );
}

#[test]
fn text_component_ticks_and_draws() {
    let mut c = TextReader::new();
    for _ in 0..10 {
        c.on_tick();
    }
    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    c.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("request"), "log lines visible:\n{text}");
}

#[test]
fn list_component_interacts() {
    let mut c = ListView::todo_examples();
    let area = Rect::new(0, 0, 60, 10);
    let mut buf = Buffer::empty(area);
    c.draw(area, &mut buf);
    let before = buffer_text(&buf);
    assert!(before.contains("[ ]"), "open items visible");

    // toggle the first item via space
    let space = Event::Key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(c.event(space), "space should be consumed");
    c.draw(area, &mut buf);
    let after = buffer_text(&buf);
    assert!(after.contains("[x]"), "item toggled:\n{after}");
    assert!(c.status().starts_with("1/12 done"));
}

#[test]
fn host_key_routing() {
    // Tab is host-level: components do not consume it
    let mut c = TextReader::new();
    let tab = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert!(!c.event(tab), "Tab falls through to the host");
}
