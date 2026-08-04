//! End-to-end theme tests: `App::set_theme` broadcasts to components and
//! every paint path honors the theme.

use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

use rsmarkdown_tui::components::agent_view::AgentView;
use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::{App, Theme};

fn cell_color(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> (Color, Color) {
    let cell = terminal.backend().buffer().cell((x, y)).expect("cell");
    (cell.fg, cell.bg)
}

#[test]
fn app_set_theme_broadcasts_to_components() {
    let mut app = App::new(vec![Box::new(AgentChat::new()), Box::new(AgentView::new())]);
    assert_eq!(*app.theme(), Theme::dark(), "dark by default");

    let light = Theme::light();
    app.set_theme(light.clone());
    assert_eq!(*app.theme(), light, "app theme switched");

    // status bar uses the new theme (focused title bg = light status_bg)
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("backend");
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    let (fg, bg) = cell_color(&terminal, 2, 29); // " [1] chat " title cell
    assert_eq!(bg, Theme::light().status_bg, "status bar themed");
    assert_eq!(fg, Theme::light().status_fg, "status fg themed");
}

#[test]
fn chat_input_line_uses_theme() {
    let mut app = App::new(vec![Box::new(AgentChat::new())]);
    app.set_theme(Theme::light());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("backend");
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    // "you › " prompt cell uses the claude accent of the light theme
    let (fg, _) = cell_color(&terminal, 0, 28);
    assert_eq!(fg, Theme::light().claude, "input prompt themed");
}

#[test]
fn dark_and_light_are_distinct() {
    assert_ne!(Theme::dark(), Theme::light());
    assert_eq!(
        Theme::dark().claude,
        Color::LightCyan,
        "dark keeps the original look"
    );
    assert_eq!(
        Theme::light().overlay_bg,
        Color::Rgb(248, 248, 248),
        "light overlay is near-white"
    );
}
