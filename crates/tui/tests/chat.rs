//! Headless tests for the agent chat component and its hint flow.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::{activities, Component};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn click(row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn click_at(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn draw_chat(chat: &mut AgentChat) -> (Buffer, Rect) {
    // tall area: nothing scrolls, so click rows map 1:1 to document rows
    let area = Rect::new(0, 0, 100, 200);
    let mut buf = Buffer::empty(area);
    chat.draw(area, &mut buf);
    (buf, area)
}

fn first_tool_row(chat: &AgentChat) -> u16 {
    // the main bash tool (message-level), not nested subagent tools
    chat.hint_row_ranges()
        .iter()
        .find(|r| r.is_tool && r.path.len() == 1)
        .expect("tool hint row range")
        .start
}

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
fn full_turn_produces_hints_and_reply() {
    let mut chat = AgentChat::new();
    // tick through the whole turn (thinking 1.4s + 2 tools + thinking 0.8s + stream)
    for _ in 0..300 {
        chat.on_tick();
    }
    assert!(chat.phase_done(), "turn should be finished");
    assert_eq!(chat.message_count(), 2); // initial user + assistant
    let text = chat.conversation_text();
    assert!(text.contains("✓ bash"), "done tool hint:\n{text}");
    assert!(text.contains("✓ explore"), "subagent hint:\n{text}");
    assert!(text.contains("✻ Edit"), "diff hint:\n{text}");
    assert!(text.contains("-"), "diff removed lines:\n{text}");
    assert!(text.contains("+"), "diff added lines:\n{text}");
    let tasks = chat.tasks();
    assert_eq!(tasks.len(), 5);
    assert!(
        tasks
            .iter()
            .all(|t| t.status == rsmarkdown_tui::activities::TodoStatus::Done),
        "demo checklist all done (broadcast to the host task area)"
    );
    assert!(text.contains("thinking"), "thinking hint:\n{text}");
    assert!(
        text.contains("54 tests passed"),
        "markdown reply streamed:\n{text}"
    );
}

#[test]
fn typing_submits_user_message() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let before = chat.message_count();
    for c in "hello agent".chars() {
        assert!(chat.event(key(KeyCode::Char(c))), "typing consumed");
    }
    assert!(chat.event(key(KeyCode::Enter)), "enter consumed");
    // submit() appends the user message and starts a turn (assistant message)
    assert_eq!(
        chat.message_count(),
        before + 2,
        "user message + new turn appended"
    );
    assert!(
        chat.conversation_text().contains("hello agent"),
        "typed text visible"
    );
}

#[test]
fn chat_draws_hints_and_reply() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let area = Rect::new(0, 0, 100, 24);
    let mut buf = Buffer::empty(area);
    chat.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("you ›"), "input line");
    // the viewport sits at the bottom of the conversation after the turn
    assert!(text.contains("✻ Edit"), "diff hint visible:\n{text}");
}

#[test]
fn hints_expand_and_collapse() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat); // builds the clickable row map

    let tool = chat.last_tool_hint().expect("tool hint exists");
    assert!(tool.expandable(), "finished tool is expandable");
    assert!(!tool.is_expanded(), "starts collapsed");

    let before = chat.conversation_text();
    assert!(
        !before.contains("$ cargo test -p rsmarkdown-core"),
        "collapsed: no tool body"
    );

    // click the tool hint header
    let row = first_tool_row(&chat);
    assert!(chat.event(click(row)), "click consumed");

    let after = chat.conversation_text();
    assert!(
        after.contains("$ cargo test -p rsmarkdown-core"),
        "tool body visible when expanded:\n{after}"
    );
    assert!(
        after.contains("54 passed; 0 failed"),
        "output body:\n{after}"
    );

    // click again to collapse
    assert!(chat.event(click(row)), "second click consumed");
    let collapsed = chat.conversation_text();
    assert!(
        !collapsed.contains("$ cargo test -p rsmarkdown-core"),
        "collapsed again"
    );
}

#[test]
fn thinking_hint_expands_to_reasoning() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    // the first hint is the thinking block (no todo in the transcript)
    let row = chat.hint_row_ranges().get(0).expect("thinking hint").start;
    assert!(chat.event(click(row)), "click consumed");
    let text = chat.conversation_text();
    assert!(
        text.contains("stable blocks"),
        "reasoning text visible:\n{text}"
    );
}

#[test]
fn clicks_on_plain_rows_do_nothing() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    let before = chat.conversation_text();
    // click the very first row (the user message) — nothing should toggle
    assert!(chat.event(click(0)), "click consumed");
    assert_eq!(chat.conversation_text(), before, "no hint toggled");
}

#[test]
fn finished_activities_collapse_by_default() {
    // active activities (running todo / subagent) stay expanded; once
    // finished they collapse back — a click is required to reopen them
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let text = chat.conversation_text();
    assert!(
        !text.contains("todo · "),
        "todo does not render in the transcript (host task area instead):\n{text}"
    );
    assert!(
        !text.contains("Report findings"),
        "finished subagent transcript collapsed:\n{text}"
    );
    assert!(
        !text.contains("grep -n parse_markdown_into_blocks"),
        "finished subagent nested tool collapsed:\n{text}"
    );
    // the checklist itself is broadcast, all done
    let tasks = chat.tasks();
    assert_eq!(tasks.len(), 5);
    assert!(
        tasks
            .iter()
            .all(|t| t.status == rsmarkdown_tui::activities::TodoStatus::Done),
        "checklist completed in the task area"
    );
}

#[test]
fn checklist_broadcasts_through_the_turn() {
    // the checklist lives in the task area (host), not the transcript:
    // statuses evolve from pending to done as the scripted turn runs
    let mut chat = AgentChat::new();
    chat.on_tick(); // todo phase: first item in progress
    let text = chat.conversation_text();
    assert!(
        !text.contains("todo · "),
        "no todo block in the transcript:\n{text}"
    );
    let tasks = chat.tasks();
    assert_eq!(
        tasks[0].status,
        rsmarkdown_tui::activities::TodoStatus::InProgress
    );
    assert_eq!(
        tasks[4].status,
        rsmarkdown_tui::activities::TodoStatus::Pending
    );
    for _ in 0..300 {
        chat.on_tick();
    }
    let tasks = chat.tasks();
    assert!(
        tasks
            .iter()
            .all(|t| t.status == rsmarkdown_tui::activities::TodoStatus::Done),
        "all done at the end of the turn"
    );
}

#[test]
fn click_during_streaming_stays_collapsed() {
    // regression: per-tick identity updates used to reset `expanded`
    let mut chat = AgentChat::new();
    // tick into the subagent phase (~ticks 44..80)
    for _ in 0..60 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    // the subagent activity is auto-expanded while running; a click folds it
    let sub_row = chat
        .hint_row_ranges()
        .iter()
        .find(|r| {
            r.path.len() == 1 && {
                // subagent is the 2nd activity in the message (path [1])
                r.path == [1]
            }
        })
        .map(|r| r.start)
        .expect("subagent row");
    assert!(chat.event(click(sub_row)), "click consumed");

    // keep ticking — the user's collapse must survive identity updates
    // (per-tick subagent updates used to reset `expanded`)
    for _ in 0..40 {
        chat.on_tick();
    }
    let text = chat.conversation_text();
    assert!(
        !text.contains("Report findings"),
        "user collapse survives streaming:\n{text}"
    );
}

#[test]
fn nested_subagent_click() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    // finished subagent is collapsed; click its header to open the nested
    // transcript, then click a nested tool to fold just that one
    let sub_row = chat
        .hint_row_ranges()
        .iter()
        .find(|r| r.path.len() == 1 && r.path == [1])
        .map(|r| r.start)
        .expect("subagent row");
    assert!(chat.event(click(sub_row)), "subagent header click");
    let expanded = chat.conversation_text();
    assert!(
        expanded.contains("grep -n parse_markdown_into_blocks"),
        "nested work visible after subagent click:\n{expanded}"
    );
    // the nested grep tool is collapsed (finished); click it to expand
    let (_, _) = draw_chat(&mut chat);
    let grep_row = chat
        .hint_row_ranges()
        .iter()
        .find(|r| r.path.len() == 2 && r.is_tool)
        .map(|r| r.start)
        .expect("nested grep tool row");
    assert!(chat.event(click(grep_row)), "nested click consumed");
    let text = chat.conversation_text();
    assert!(
        text.contains("blocks.rs:24: pub fn parse_markdown_into_blocks"),
        "nested tool body after click:\n{text}"
    );
}

#[test]
fn spinner_cycles() {
    let a = activities::spinner(0);
    let b = activities::spinner(1);
    assert_ne!(a, b);
    assert_eq!(activities::spinner(10), a, "cycles after full rotation");
}

// --- slash command menu + help panel integration ---

fn draw_chat_text(chat: &mut AgentChat) -> String {
    let (buf, _) = draw_chat(chat);
    buffer_text(&buf)
}

#[test]
fn slash_in_empty_prompt_opens_command_menu() {
    let mut chat = AgentChat::new();
    // the scripted turn keeps running; stop the agent so the transcript is quiet
    for _ in 0..400 {
        chat.on_tick();
    }
    // esc to view, esc back to typing (typing starts on)
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    assert!(!chat.menu_open(), "no menu yet");
    assert!(chat.event(key(KeyCode::Char('/'))), "slash consumed");
    assert!(chat.menu_open(), "menu opened by / in empty prompt");
    let text = draw_chat_text(&mut chat);
    assert!(text.contains("/clear"), "command row drawn");
    assert!(text.contains("Clear the transcript"), "description drawn");
    // position: directly above the prompt line, same width (no floating box)
    let r = chat.menu_rect();
    assert_eq!(r.x, 0, "left-aligned with the prompt");
    assert_eq!(r.width, 100, "same width as the prompt line");
    assert_eq!(r.y + r.height, 199, "sits directly on the input line");
    // the transcript still renders outside the menu rect; only the menu's
    // own area is covered (solid overlay background)
    assert!(
        text.contains("thinking"),
        "transcript above the menu still renders:\n{text}"
    );
    let (buf, _) = draw_chat(&mut chat);
    let r = chat.menu_rect();
    let cell = buf.cell((r.x + 5, r.y + 1)).expect("menu inner cell");
    assert_eq!(
        cell.bg,
        ratatui::style::Color::Rgb(18, 18, 18),
        "menu area covered with the overlay background"
    );
}

#[test]
fn slash_menu_filters_as_you_type_and_backspace_closes() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Char('/')));
    chat.event(key(KeyCode::Char('m')));
    chat.event(key(KeyCode::Char('o')));
    let text = draw_chat_text(&mut chat);
    assert!(text.contains("/mo"), "filter shown in menu title");
    assert!(!text.contains("/clear"), "filtered out");

    chat.event(key(KeyCode::Backspace));
    chat.event(key(KeyCode::Backspace));
    assert!(chat.menu_open(), "menu still open with just the /");
    chat.event(key(KeyCode::Backspace));
    assert!(!chat.menu_open(), "backspacing past the / closes the menu");
}

#[test]
fn enter_confirms_clear_command() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    assert!(chat.message_count() > 0, "transcript has messages");
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Char('/')));
    chat.event(key(KeyCode::Char('c')));
    chat.event(key(KeyCode::Enter));
    assert!(!chat.menu_open(), "menu closed after confirm");
    assert_eq!(chat.message_count(), 0, "/clear wiped the transcript");
}

#[test]
fn esc_closes_menu_keeping_slash_in_input() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Char('/')));
    chat.event(key(KeyCode::Esc));
    assert!(!chat.menu_open(), "esc closed the menu");
    let text = draw_chat_text(&mut chat);
    assert!(text.contains("you › /"), "the / stays in the input");
}

#[test]
fn question_mark_toggles_help_panel() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    assert!(!chat.help_open(), "no panel yet");
    assert!(chat.event(key(KeyCode::Char('?'))), "? consumed");
    assert!(chat.help_open(), "panel opened");
    let text = draw_chat_text(&mut chat);
    assert!(text.contains("Keyboard shortcuts"), "panel drawn");
    assert!(text.contains("send message"), "entry drawn");
    assert!(
        text.contains("thinking"),
        "transcript still renders outside the help panel:\n{text}"
    );

    // ? again closes it (toggle)
    assert!(chat.event(key(KeyCode::Char('?'))));
    assert!(!chat.help_open(), "panel toggled off");
    // and esc closes it too
    chat.event(key(KeyCode::Char('?')));
    assert!(chat.help_open());
    assert!(chat.event(key(KeyCode::Esc)));
    assert!(!chat.help_open(), "esc closed the panel");
}

#[test]
fn help_command_via_menu_opens_panel() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Char('/')));
    chat.event(key(KeyCode::Char('h')));
    chat.event(key(KeyCode::Enter));
    assert!(!chat.menu_open(), "menu closed");
    assert!(chat.help_open(), "/help opened the panel");
    assert_eq!(chat.message_count(), 2, "transcript untouched by /help");
}

#[test]
fn menu_not_opened_from_view_mode() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc)); // typing -> view mode
    assert!(!chat.menu_open());
    // in view mode / is not a command trigger; it must not open the menu
    chat.event(key(KeyCode::Char('/')));
    assert!(!chat.menu_open(), "/ only triggers from the prompt");
}

#[test]
fn mouse_click_selects_and_confirms_menu_command() {
    let mut chat = AgentChat::new();
    for _ in 0..400 {
        chat.on_tick();
    }
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Esc));
    chat.event(key(KeyCode::Char('/')));
    // draw so the menu rect is recorded
    let (buf, area) = draw_chat(&mut chat);
    let text = buffer_text(&buf);
    assert!(text.contains("/clear"), "menu visible");
    let r = chat.menu_rect();
    assert!(r.width > 0, "menu rect recorded");

    // first click on the second row selects it (the menu is centered, so
    // click inside its horizontal span)
    let second_y = r.y + 2; // first inner row is the first command
    chat.event(click_at(r.x + 10, second_y));
    let (buf, _) = draw_chat(&mut chat);
    let text = buffer_text(&buf);
    assert!(text.contains("❯ /help"), "second row selected");
    assert!(chat.menu_open(), "select only");

    // second click confirms
    chat.event(click_at(r.x + 10, second_y));
    assert!(!chat.menu_open(), "second click confirmed");
    assert!(chat.help_open(), "/help ran");
    let _ = area;
}
