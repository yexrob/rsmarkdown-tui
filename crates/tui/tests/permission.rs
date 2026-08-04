//! End-to-end permission dialog tests: raise a request through the host,
//! draw it as a modal overlay, and drive keys / mouse through `App::route` —
//! the exact path input takes in the running TUI.

use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Terminal;

use rsmarkdown_tui::activities::{diff_lines, Diff};
use rsmarkdown_tui::permission::{DialogAction, PermissionRequest};
use rsmarkdown_tui::{App, Component};

/// Observable state of the probe component, shared with the test.
#[derive(Default)]
struct ProbeState {
    events: Vec<Event>,
    closed: Vec<DialogAction>,
}

/// A probe component that records every event, can raise a permission
/// request, and records dialog outcomes.
struct Probe {
    state: Rc<RefCell<ProbeState>>,
    request: Option<PermissionRequest>,
    ask: bool,
}

impl Component for Probe {
    fn title(&self) -> &str {
        "probe"
    }
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, "probe", Style::default());
    }
    fn event(&mut self, event: Event) -> bool {
        let quit = matches!(&event, Event::Key(k) if k.code == KeyCode::Char('q'));
        self.state.borrow_mut().events.push(event);
        // consume everything except `q`, so the host quit key stays testable
        !quit
    }
    fn on_ask(&mut self) -> Option<PermissionRequest> {
        if self.ask {
            self.request.clone()
        } else {
            None
        }
    }
    fn on_dialog_closed(&mut self, action: DialogAction) {
        self.state.borrow_mut().closed.push(action);
    }
}

fn probe_app() -> (App, Rc<RefCell<ProbeState>>) {
    let state = Rc::new(RefCell::new(ProbeState::default()));
    let probe = Probe {
        state: state.clone(),
        request: None,
        ask: false,
    };
    let app = App::new(vec![Box::new(probe)]);
    (app, state)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

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

/// Edit-file request matching the Claude Code transcript shape
/// (research doc §4.1): title, target, question, numbered options.
fn edit_request() -> PermissionRequest {
    PermissionRequest::new(
        "Edit file",
        "Do you want to proceed?",
        vec![
            "Yes".to_string(),
            "Yes, during this session".to_string(),
            "No, and tell Claude what to do differently (esc)".to_string(),
        ],
    )
    .target("crates/core/src/blocks.rs")
    .source(r#"subagent "explore""#)
    .content(diff_lines(
        &Diff::parse_unified(EDIT_DIFF),
        &rsmarkdown_tui::renderer::theme::Theme::dark(),
    ))
}

const EDIT_DIFF: &str = "--- a/crates/core/src/blocks.rs\n\
+++ b/crates/core/src/blocks.rs\n\
@@ -101,7 +101,7 @@\n\
-    if has_footnote_reference(markdown) || has_footnote_definition(markdown) {\n\
+    if has_footnote_reference(markdown) {\n\
     return vec![markdown.to_string()];\n";

fn draw(app: &mut App, terminal: &mut Terminal<TestBackend>) -> Vec<String> {
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    buffer_text(terminal.backend().buffer())
}

#[test]
fn dialog_draws_title_target_question_and_options() {
    let (mut app, _state) = probe_app();
    app.ask(edit_request());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    let rows = draw(&mut app, &mut terminal);

    assert!(
        rows.iter().any(|r| r.contains("Edit file")),
        "title in border"
    );
    assert!(
        rows.iter().any(|r| r.contains("crates/core/src/blocks.rs")),
        "target line"
    );
    assert!(
        rows.iter().any(|r| r.contains(r#"subagent "explore""#)),
        "requesting agent line"
    );
    assert!(
        rows.iter().any(|r| r.contains("Do you want to proceed?")),
        "question"
    );
    assert!(
        rows.iter().any(|r| r.contains("❯ 1. Yes")),
        "selected option 1 with ❯"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("2. Yes, during this session")),
        "option 2"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("No, and tell Claude what to do differently")),
        "option 3"
    );
    // diff preview reuses the activity diff renderer (red `-` line)
    assert!(
        rows.iter()
            .any(|r| r.contains("-    if has_footnote_reference")),
        "diff preview inside dialog"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("+    if has_footnote_reference")),
        "diff added line"
    );
}

#[test]
fn tiny_window_clips_dialog_instead_of_panicking() {
    // regression: a 6-row terminal window used to overflow the buffer
    // (index outside of buffer at (2, 6))
    let (mut app, _state) = probe_app();
    app.ask(edit_request());
    for height in [1u16, 2, 3, 4, 5, 6] {
        for width in [1u16, 4, 20, 80, 106, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
            terminal
                .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
                .expect("draw");
            // the dialog must fit inside the frame (or be skipped entirely)
            let r = app.dialog_rect();
            assert!(
                r.width <= width && r.height <= height,
                "dialog inside frame"
            );
        }
    }
}

#[test]
fn overlay_background_separates_from_transcript() {
    let (mut app, _state) = probe_app();
    app.ask(edit_request());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    let rows = draw(&mut app, &mut terminal);
    // only the dialog's own rect is covered: the component content above
    // it (probe draws at the top-left) still renders normally
    assert!(
        rows.iter().any(|r| r.contains("probe")),
        "content outside the dialog rect still renders:\n{rows:?}"
    );
    let r = app.dialog_rect();
    // the bottom padding row of the dialog carries the solid overlay bg,
    // hiding whatever the transcript rendered behind it
    let pad_cell = terminal
        .backend()
        .buffer()
        .cell((r.x + 2, r.y + r.height - 2))
        .expect("padding cell");
    assert_eq!(
        pad_cell.bg,
        ratatui::style::Color::Rgb(18, 18, 18),
        "solid overlay background"
    );
    // the border uses the permission accent
    let border_cell = terminal
        .backend()
        .buffer()
        .cell((r.x, r.y))
        .expect("border cell");
    assert_eq!(
        border_cell.fg,
        ratatui::style::Color::LightYellow,
        "permission-colored border"
    );
}

#[test]
fn esc_cancels_and_notifies_component() {
    let (mut app, state) = probe_app();
    app.ask(edit_request());
    assert!(app.permission_open(), "dialog open");
    assert!(app.route(key(KeyCode::Esc)), "esc routed");
    assert!(!app.permission_open(), "dialog closed");
    assert_eq!(app.take_dialog_action(), Some(DialogAction::Cancel));
    assert_eq!(
        state.borrow().closed,
        vec![DialogAction::Cancel],
        "component notified"
    );
}

#[test]
fn digit_confirms_option() {
    let (mut app, state) = probe_app();
    app.ask(edit_request());
    assert!(app.route(key(KeyCode::Char('2'))), "digit routed");
    assert!(!app.permission_open(), "dialog closed");
    assert_eq!(app.take_dialog_action(), Some(DialogAction::Confirm(1)));
    assert_eq!(state.borrow().closed, vec![DialogAction::Confirm(1)]);
}

#[test]
fn arrows_move_selection_then_enter_confirms() {
    let (mut app, _state) = probe_app();
    app.ask(edit_request());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    let rows = draw(&mut app, &mut terminal);
    assert!(rows.iter().any(|r| r.contains("❯ 1. Yes")), "starts on 1");

    assert!(app.route(key(KeyCode::Down)), "down routed");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter()
            .any(|r| r.contains("❯ 2. Yes, during this session")),
        "moved to 2"
    );
    assert!(
        !rows.iter().any(|r| r.contains("❯ 1. Yes")),
        "1 no longer selected"
    );

    assert!(app.route(key(KeyCode::Enter)), "enter routed");
    assert!(!app.permission_open(), "dialog closed");
    assert_eq!(app.take_dialog_action(), Some(DialogAction::Confirm(1)));
}

#[test]
fn mouse_click_selects_then_second_click_confirms() {
    let (mut app, _state) = probe_app();
    app.ask(edit_request());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    draw(&mut app, &mut terminal);
    let r = app.dialog_rect();
    assert!(r.width > 0, "dialog rect recorded");

    // click option 2 (absolute terminal coordinates, as a real user would)
    let option2_y = r.y + r.height - 4;
    assert!(app.route(click(r.x + 10, option2_y)), "click routed");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter()
            .any(|r| r.contains("❯ 2. Yes, during this session")),
        "click selected option 2"
    );
    assert!(app.permission_open(), "still open after select");

    // clicking the same option confirms it
    assert!(app.route(click(r.x + 10, option2_y)), "second click routed");
    assert!(!app.permission_open(), "dialog closed");
    assert_eq!(app.take_dialog_action(), Some(DialogAction::Confirm(1)));
}

#[test]
fn modal_blocks_component_and_host_keys() {
    let (mut app, state) = probe_app();
    app.ask(edit_request());

    // `q` must NOT quit while the dialog is open (modal owns the keyboard)
    assert!(app.route(key(KeyCode::Char('q'))), "q swallowed by dialog");
    assert!(app.permission_open(), "still open");
    assert!(app.route(key(KeyCode::Tab)), "tab swallowed");
    assert!(app.permission_open(), "still open");
    assert!(
        state.borrow().events.is_empty(),
        "component never saw input"
    );

    // after dismiss the host keys work again
    assert!(app.route(key(KeyCode::Esc)), "esc closes");
    assert_eq!(
        app.route(key(KeyCode::Char('q'))),
        false,
        "q quits once modal is closed"
    );
}

#[test]
fn component_on_ask_opens_dialog_via_tick() {
    let state = Rc::new(RefCell::new(ProbeState::default()));
    let probe = Probe {
        state: state.clone(),
        request: Some(edit_request()),
        ask: true,
    };
    let mut app = App::new(vec![Box::new(probe)]);
    assert!(!app.permission_open(), "no dialog yet");
    app.tick_components();
    assert!(app.permission_open(), "dialog raised by on_ask");
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter().any(|r| r.contains("Edit file")),
        "dialog visible"
    );
}

#[test]
fn long_content_gets_tail_and_stays_bounded() {
    let mut diff_text = String::from("--- a/x.rs\n+++ b/x.rs\n@@ -1,30 +1,30 @@\n");
    for i in 0..30 {
        diff_text.push_str(&format!(" context line {}\n", i));
    }
    let mut req = edit_request();
    req.content = diff_lines(
        &Diff::parse_unified(&diff_text),
        &rsmarkdown_tui::renderer::theme::Theme::dark(),
    );
    let (mut app, _state) = probe_app();
    app.ask(req);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test backend");
    let rows = draw(&mut app, &mut terminal);
    assert!(
        rows.iter().any(|r| r.contains("… +23 more")),
        "content tail with count"
    );
    let dialog_rows = rows.iter().filter(|r| r.contains("context line")).count();
    assert_eq!(dialog_rows, 7, "8 content rows capped (incl. @@ header)");
}
