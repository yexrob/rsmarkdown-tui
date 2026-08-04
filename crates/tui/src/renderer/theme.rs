//! Terminal palette for the markdown renderer.

use ratatui::style::{Color, Modifier, Style};

pub fn text() -> Style {
    Style::default()
}
pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub fn strong() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
pub fn emphasis() -> Style {
    Style::default().add_modifier(Modifier::ITALIC)
}
pub fn strikethrough() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::CROSSED_OUT)
}
pub fn code() -> Style {
    Style::default()
        .fg(Color::LightYellow)
        .bg(Color::Rgb(30, 30, 30))
}
pub fn code_block() -> Style {
    Style::default()
        .fg(Color::Rgb(170, 170, 170))
        .bg(Color::Rgb(24, 24, 24))
}
pub fn link() -> Style {
    Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::UNDERLINED)
}
pub fn math() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::ITALIC)
}
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
pub fn quote() -> Style {
    Style::default().fg(Color::Gray)
}
pub fn quote_bar() -> Style {
    Style::default().fg(Color::LightBlue)
}
pub fn task_done() -> Style {
    Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD)
}
pub fn task_open() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub fn list_marker() -> Style {
    Style::default().fg(Color::LightCyan)
}
pub fn table_border() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub fn table_header() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
pub fn hr() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub fn footnote() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub fn status() -> Style {
    Style::default().fg(Color::Black).bg(Color::LightCyan)
}
pub fn status_inactive() -> Style {
    Style::default().fg(Color::DarkGray)
}
