//! Tests for the slash-command menu and the `?` help panel: filtering,
//! selection, confirmation, scrolling, mouse interaction and drawing.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use rsmarkdown_tui::command_menu::{SlashCommand, SlashCommandMenu, MENU_ROWS};
use rsmarkdown_tui::help::{HelpEntry, HelpPanel, HelpSection};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand::new("clear", "Clear the transcript"),
        SlashCommand::new("help", "Show keyboard shortcuts"),
        SlashCommand::new("context", "Show context usage"),
        SlashCommand::new("model", "Choose a model"),
        SlashCommand::new("status", "Show session status"),
    ]
}

fn names(menu: &SlashCommandMenu) -> Vec<String> {
    menu.filtered().iter().map(|c| c.name.clone()).collect()
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
fn filter_narrows_and_resets_selection() {
    let mut menu = SlashCommandMenu::new(commands());
    assert_eq!(
        names(&menu),
        vec!["clear", "help", "context", "model", "status"]
    );

    menu.set_filter("c");
    assert_eq!(names(&menu), vec!["clear", "context"], "prefix match");
    assert_eq!(menu.selected, 0, "selection resets to first match");

    menu.set_filter("mo");
    assert_eq!(names(&menu), vec!["model"]);
    assert_eq!(menu.selected, 0);

    menu.set_filter("zzz");
    assert!(menu.filtered().is_empty(), "no match");
}

#[test]
fn case_insensitive_filter() {
    let mut menu = SlashCommandMenu::new(commands());
    menu.set_filter("CL");
    assert_eq!(names(&menu), vec!["clear"]);
}

#[test]
fn arrows_move_selection_with_wrap_and_enter_confirms_absolute() {
    let mut menu = SlashCommandMenu::new(commands());
    // filter to [clear(0), context(2)]
    menu.set_filter("c");
    assert_eq!(menu.selected, 0);

    menu.key(key(KeyCode::Down));
    assert_eq!(menu.selected, 1, "moved to context");
    assert_eq!(
        menu.key(key(KeyCode::Enter)),
        Some(2),
        "absolute index into commands"
    );

    // wrap around
    menu.key(key(KeyCode::Down));
    assert_eq!(menu.selected, 0, "wrapped to first");
    menu.key(key(KeyCode::Up));
    assert_eq!(menu.selected, 1, "wrapped back");

    // j/k also move
    menu.key(key(KeyCode::Char('k')));
    assert_eq!(menu.selected, 0);
    menu.key(key(KeyCode::Char('j')));
    assert_eq!(menu.selected, 1);
}

#[test]
fn enter_on_empty_match_returns_none() {
    let mut menu = SlashCommandMenu::new(commands());
    menu.set_filter("zzz");
    assert_eq!(menu.key(key(KeyCode::Enter)), None, "nothing to confirm");
}

#[test]
fn long_lists_scroll_and_draw_caps_rows() {
    let many: Vec<SlashCommand> = (0..20)
        .map(|i| SlashCommand::new(format!("cmd{i:02}"), "demo command"))
        .collect();
    let mut menu = SlashCommandMenu::new(many);
    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    menu.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert_eq!(
        text.matches("/cmd").count(),
        MENU_ROWS,
        "only MENU_ROWS rows drawn"
    );

    // moving down past the window scrolls
    for _ in 0..15 {
        menu.key(key(KeyCode::Down));
    }
    assert_eq!(menu.selected, 15);
    assert!(
        menu.key(key(KeyCode::Enter)).is_some(),
        "selection stays valid"
    );
}

#[test]
fn mouse_click_selects_then_confirms() {
    let mut menu = SlashCommandMenu::new(commands());
    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    let r = menu.draw(area, &mut buf);
    // row of the second command (inside the border)
    let second_y = r.y + 2;
    assert_eq!(
        menu.click(&click(10, second_y)),
        None,
        "first click selects"
    );
    assert_eq!(menu.selected, 1, "second row selected");
    assert_eq!(
        menu.click(&click(10, second_y)),
        Some(1),
        "second click confirms"
    );

    // clicks outside the menu drop
    assert_eq!(menu.click(&click(10, 99)), None);
    assert_eq!(menu.selected, 1, "unchanged");
}

#[test]
fn overlay_background_separates_from_transcript() {
    let mut menu = SlashCommandMenu::new(commands());
    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    let r = menu.draw(area, &mut buf);
    let inner_cell = buf.cell((r.x + 2, r.y + 1)).expect("inner cell");
    assert_eq!(
        inner_cell.bg,
        ratatui::style::Color::Reset,
        "solid overlay background"
    );
    let border_cell = buf.cell((r.x, r.y)).expect("border cell");
    assert_eq!(
        border_cell.fg,
        ratatui::style::Color::Gray,
        "brighter border"
    );
}

#[test]
fn menu_title_shows_live_filter() {
    let mut menu = SlashCommandMenu::new(commands());
    menu.set_filter("mo");
    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    menu.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("/mo"), "filter in the menu title");
}

#[test]
fn help_panel_esc_closes_and_draws_sections() {
    let mut panel = HelpPanel::new(vec![HelpSection {
        title: "chat",
        entries: vec![
            HelpEntry {
                keys: "enter",
                description: "send",
            },
            HelpEntry {
                keys: "esc",
                description: "view",
            },
        ],
    }]);
    assert_eq!(panel.total_rows(), 3, "1 title + 2 entries");

    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    panel.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("Keyboard shortcuts"), "panel title");
    assert!(text.contains("chat"), "section title");
    assert!(text.contains("send"), "entry description");
    assert!(text.contains("enter"), "key name");

    assert!(panel.key(key(KeyCode::Esc)), "esc closes");
    assert!(
        !panel.key(key(KeyCode::Char('x'))),
        "other keys keep it open"
    );
}

#[test]
fn help_panel_scrolls_when_taller_than_area() {
    let mut panel = HelpPanel::new(vec![HelpSection {
        title: "big",
        entries: (0..20)
            .map(|i| HelpEntry {
                keys: "k",
                description: "entry",
            })
            .collect(),
    }]);
    let area = Rect::new(0, 0, 100, 10);
    let mut buf = Buffer::empty(area);
    panel.draw(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("Keyboard shortcuts"), "bordered panel fits");

    panel.key(key(KeyCode::Down));
    let mut buf = Buffer::empty(area);
    panel.draw(area, &mut buf);
    assert_eq!(panel.scroll, 1, "scrolled one row");
    // scroll is capped so the last row stays visible
    for _ in 0..100 {
        panel.key(key(KeyCode::Down));
    }
    panel.draw(area, &mut buf);
    assert_eq!(
        panel.scroll,
        panel.total_rows().saturating_sub(area.height - 2)
    );
}
