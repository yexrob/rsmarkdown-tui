//! Terminal palette for the markdown renderer.

use ratatui::style::{Color, Modifier, Style};

/// Default body text.
pub fn text() -> Style {
    Style::default()
}
/// Dimmed / secondary text.
pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Bold text.
pub fn strong() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
/// Italic text.
pub fn emphasis() -> Style {
    Style::default().add_modifier(Modifier::ITALIC)
}
/// Struck-through text.
pub fn strikethrough() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::CROSSED_OUT)
}
/// Inline code (light yellow on dark background).
pub fn code() -> Style {
    Style::default()
        .fg(Color::LightYellow)
        .bg(Color::Rgb(30, 30, 30))
}
/// Fenced code block body.
pub fn code_block() -> Style {
    Style::default()
        .fg(Color::Rgb(170, 170, 170))
        .bg(Color::Rgb(24, 24, 24))
}
/// Hyperlinks (underlined blue).
pub fn link() -> Style {
    Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::UNDERLINED)
}
/// Math formulas (magenta italic).
pub fn math() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::ITALIC)
}
/// Heading text for a given level.
pub fn heading(level: u8) -> Style {
    let color = match level {
        1 => Color::White,
        2 => Color::LightCyan,
        3 => Color::LightBlue,
        _ => Color::DarkGray,
    };
    Style::default()
        .fg(color)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}
/// Blockquote text.
pub fn quote() -> Style {
    Style::default().fg(Color::Gray)
}
/// Blockquote left bar.
pub fn quote_bar() -> Style {
    Style::default().fg(Color::LightBlue)
}
/// Completed task-list marker.
pub fn task_done() -> Style {
    Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD)
}
/// Open task-list marker.
pub fn task_open() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// List bullet / number.
pub fn list_marker() -> Style {
    Style::default().fg(Color::LightCyan)
}
/// Table grid lines.
pub fn table_border() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Table header row.
pub fn table_header() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
/// Thematic break line.
pub fn hr() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Footnote labels.
pub fn footnote() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Thinking blocks (dim italic).
pub fn thinking() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC)
}
/// Footer badge for an active mode (`⏵⏵` / `⏸`).
pub fn mode_on() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

/// The `⏺` main-activity dot (Claude's accent color).
pub fn activity_dot() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}
/// Running tool / subagent (cyan bold).
pub fn tool_running() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}
/// Completed tool / subagent (green bold).
pub fn tool_done() -> Style {
    Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD)
}
/// Failed tool / subagent (red bold).
pub fn tool_error() -> Style {
    Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD)
}
/// Tool output preview (dark gray).
pub fn tool_output() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Diff `@@` hunk headers (cyan bold).
pub fn diff_hunk() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}
/// Diff added lines (green).
pub fn diff_added() -> Style {
    Style::default().fg(Color::LightGreen)
}
/// Diff removed lines (red).
pub fn diff_removed() -> Style {
    Style::default().fg(Color::LightRed)
}
/// Diff context lines (gray).
pub fn diff_context() -> Style {
    Style::default().fg(Color::Gray)
}
/// The `✻` file-edit glyph (yellow bold).
pub fn diff_edit() -> Style {
    Style::default()
        .fg(Color::LightYellow)
        .add_modifier(Modifier::BOLD)
}
/// Focused component title in the status bar.
pub fn status() -> Style {
    Style::default().fg(Color::Black).bg(Color::LightCyan)
}
/// Unfocused / single-component status bar.
pub fn status_inactive() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Permission dialog title / accent (Claude's `permission` semantic color).
pub fn permission() -> Style {
    Style::default()
        .fg(Color::LightYellow)
        .add_modifier(Modifier::BOLD)
}
/// Selected option `❯` marker in the permission dialog.
pub fn permission_selected() -> Style {
    Style::default()
        .fg(Color::LightYellow)
        .add_modifier(Modifier::BOLD)
}
/// Solid background for floating overlays (permission dialog, command
/// menu, help panel) so their content stays readable over the transcript.
pub fn overlay() -> Style {
    Style::default().bg(Color::Rgb(18, 18, 18))
}
/// Overlay border, slightly brighter than the default dim border so the
/// panel separates from the content behind it.
pub fn overlay_border() -> Style {
    Style::default().fg(Color::Gray)
}
