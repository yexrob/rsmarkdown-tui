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
    assert!(text.contains("5/5 tasks"), "todo all done:\n{text}");
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
    // the first hint is the todo list; the second is the thinking block
    let row = chat.hint_row_ranges().get(1).expect("thinking hint").start;
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
#[test]
fn todo_and_subagent_auto_expand() {
    // while a todo/subagent is active they stay expanded; after the turn the
    // expanded state is preserved, so items and nested work stay visible
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let text = chat.conversation_text();
    assert!(
        text.contains("[x] Understand the request"),
        "todo items visible:\n{text}"
    );
    assert!(
        text.contains("Report findings"),
        "nested subagent todo visible:\n{text}"
    );
    assert!(
        text.contains("grep -n parse_markdown_into_blocks"),
        "nested subagent tool visible:\n{text}"
    );
}

#[test]
fn click_during_streaming_stays_expanded() {
    // regression: per-tick identity updates used to reset `expanded`
    let mut chat = AgentChat::new();
    // tick into the subagent phase (~ticks 44..80)
    for _ in 0..60 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    // the subagent activity is auto-expanded while running
    let sub_row = chat
        .hint_row_ranges()
        .iter()
        .find(|r| {
            r.path.len() == 1 && {
                // subagent is the 3rd activity in the message (path [2])
                r.path == [2]
            }
        })
        .map(|r| r.start)
        .expect("subagent row");
    assert!(chat.event(click(sub_row)), "click consumed");

    // keep ticking — the expansion must survive identity updates
    for _ in 0..40 {
        chat.on_tick();
    }
    let text = chat.conversation_text();
    assert!(
        text.contains("Report findings"),
        "subagent nested work still expanded after streaming:\n{text}"
    );
}

#[test]
fn nested_subagent_click() {
    let mut chat = AgentChat::new();
    for _ in 0..300 {
        chat.on_tick();
    }
    let (_, _) = draw_chat(&mut chat);
    // nested activities are expanded while the subagent works; find the
    // sub-grep tool (path [3, 2]) and collapse it with a click
    let grep_row = chat
        .hint_row_ranges()
        .iter()
        .find(|r| r.path.len() == 2 && r.is_tool)
        .map(|r| r.start)
        .expect("nested grep tool row");
    assert!(chat.event(click(grep_row)), "nested click consumed");
    let collapsed = chat.conversation_text();
    assert!(
        !collapsed.contains("blocks.rs:24: pub fn parse_markdown_into_blocks"),
        "nested tool collapsed after click:\n{collapsed}"
    );
    // click again to re-expand
    assert!(chat.event(click(grep_row)), "second nested click consumed");
    let text = chat.conversation_text();
    assert!(
        text.contains("blocks.rs:24: pub fn parse_markdown_into_blocks"),
        "nested tool body after re-expand:\n{text}"
    );
}

#[test]
fn spinner_cycles() {
    let a = activities::spinner(0);
    let b = activities::spinner(1);
    assert_ne!(a, b);
    assert_eq!(activities::spinner(10), a, "cycles after full rotation");
}
