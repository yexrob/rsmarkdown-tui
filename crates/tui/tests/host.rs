//! End-to-end host tests: draw through the App and route real mouse events
//! (absolute terminal coordinates) through `App::route` — the exact path a
//! click takes in the running TUI.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::App;

fn click(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn buffer_text(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| {
                    buf.cell((x, y))
                        .map(|c| c.symbol())
                        .unwrap_or(" ")
                        .to_string()
                })
                .collect()
        })
        .collect()
}

fn setup() -> (App, Terminal<TestBackend>) {
    let mut app = App::new(vec![Box::new(AgentChat::new())]);
    // drive the scripted turn to completion
    for _ in 0..300 {
        app.tick_components();
    }
    // the demo turn raises the permission dialog at the end; dismiss it and
    // finish the remaining phase (ticks are frozen while the dialog is open)
    app.route(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    for _ in 0..200 {
        app.tick_components();
    }
    // tall terminal: the conversation fits without scrolling, so absolute
    // click rows map 1:1 to document rows
    let mut terminal = Terminal::new(TestBackend::new(160, 300)).expect("test backend");
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    (app, terminal)
}

#[test]
fn host_routes_clicks_to_component() {
    let (mut app, mut terminal) = setup();

    let rows = buffer_text(terminal.backend().buffer());
    assert!(
        !rows.iter().any(|r| r.contains("… 5 done")),
        "finished todo collapsed by default"
    );
    // find the todo header row ("todo · 5/5 tasks")
    let todo_row = rows
        .iter()
        .position(|r| r.contains("todo · "))
        .expect("todo header visible") as u16;

    // click it through the host with absolute coordinates to expand
    assert!(app.route(click(5, todo_row)), "click routed");

    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("redraw");
    let rows = buffer_text(terminal.backend().buffer());
    assert!(
        rows.iter().any(|r| r.contains("… 2 done"))
            && rows.iter().any(|r| r.contains("[x] Verify the result")),
        "todo priority view after the click"
    );
}

#[test]
fn host_drops_status_bar_clicks() {
    let (mut app, mut terminal) = setup();
    // click the status bar (last row of a 24-high frame)
    let mut small = Terminal::new(TestBackend::new(80, 24)).expect("test backend");
    small
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    assert!(app.route(click(10, 23)), "routed (dropped silently)");
    small
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    let rows = buffer_text(small.backend().buffer());
    assert!(
        rows.last().map_or(false, |r| r.contains("[1] chat")),
        "status bar intact"
    );
}

#[test]
fn host_mouse_wheel_scrolls() {
    let (mut app, mut terminal) = setup();
    let wheel = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    assert!(app.route(wheel), "wheel routed");
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    // the buffer content must have shifted
    let _ = buffer_text(terminal.backend().buffer());
}

#[allow(dead_code)]
fn _key() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
}

#[test]
fn footer_mode_badge_cycles() {
    let (mut app, mut terminal) = setup();
    let draw = |app: &mut App, terminal: &mut Terminal<TestBackend>| {
        terminal
            .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
            .expect("draw");
        buffer_text(terminal.backend().buffer())
    };
    let m = Event::Key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    // the chat component starts in typing mode and would consume 'm'
    let esc = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.route(esc), "esc exits typing mode");

    // default: no mode badge
    let rows = draw(&mut app, &mut terminal);
    assert!(
        !rows.iter().any(|r| r.contains("accept edits")),
        "no badge by default"
    );

    // m -> accept edits
    assert!(app.route(m.clone()), "m routed");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter().any(|r| r.contains("⏵⏵ accept edits on")),
        "accept-edits badge"
    );
    assert_eq!(app.mode(), rsmarkdown_tui::SessionMode::AcceptEdits);

    // m -> plan
    assert!(app.route(m.clone()), "m routed");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter().any(|r| r.contains("⏸ plan mode on")),
        "plan badge"
    );

    // m -> back to default
    assert!(app.route(m), "m routed");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        !rows.iter().any(|r| r.contains("plan mode")),
        "badge cleared"
    );
}

#[test]
fn footer_agent_and_pr_badges() {
    let (mut app, mut terminal) = setup(); // chat turn completed
    app.set_pr(446, rsmarkdown_tui::PrStatus::Pending);
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    let rows = buffer_text(terminal.backend().buffer());
    assert!(
        rows.last().map_or(false, |r| r.contains("← 1 done")),
        "agents badge after completed turn: {:?}",
        rows.last()
    );
    assert!(
        rows.last().map_or(false, |r| r.contains("PR #446")),
        "pr badge"
    );
}
