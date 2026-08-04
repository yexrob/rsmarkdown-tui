//! Smoke test: drive the real event loop paths (draw + route + tick) with
//! various terminal sizes and key/mouse input, as the demo would.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use rsmarkdown_tui::components::agent_view::AgentView;
use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::components::image::ImagePane;
use rsmarkdown_tui::components::list::ListView;
use rsmarkdown_tui::components::markdown::MarkdownViewer;
use rsmarkdown_tui::components::text::TextReader;
use rsmarkdown_tui::{App, PrStatus, Theme};

fn events() -> Vec<Event> {
    let mut v = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    ];
    for y in 0..60u16 {
        v.push(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
        v.push(Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 2,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }
    v
}

fn run_demo(width: u16, height: u16, iterations: usize) {
    let mut app = App::new(vec![
        Box::new(MarkdownViewer::new()),
        Box::new(ImagePane::new()),
        Box::new(AgentChat::new()),
        Box::new(TextReader::new()),
        Box::new(ListView::todo_examples()),
        Box::new(AgentView::new()),
    ]);
    app.set_pr(446, PrStatus::Pending);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
    let evs = events();
    for i in 0..iterations {
        terminal
            .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
            .expect("draw");
        if i < evs.len() {
            let e = evs[i].clone();
            let _ = app.route(e);
        }
        app.tick_components();
    }
    app.set_theme(Theme::light());
    for _ in 0..50 {
        terminal
            .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
            .expect("draw light");
        app.tick_components();
    }
}

#[test]
fn demo_path_never_panics() {
    for height in [1u16, 2, 3, 4, 5, 6, 8, 10, 24, 40] {
        for width in [1u16, 2, 4, 8, 12, 20, 40, 80, 160] {
            run_demo(width, height, 700);
        }
    }
}
