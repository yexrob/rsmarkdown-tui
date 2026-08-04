//! Component model: the TUI is a host for pluggable, independently-owned
//! components. Markdown rendering is just one component; text viewers, lists,
//! tables, dashboards etc. implement the same trait.

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// A renderable, interactive pane hosted by [`crate::App`].
///
/// The component owns its own state (content, scroll, focus, streaming
/// position) and only talks to the host through this trait. The host is
/// responsible for layout, event routing and the status bar.
pub trait Component {
    /// Short name shown in the status bar.
    fn title(&self) -> &str;

    /// Paint the component into `area` of `buf`.
    fn draw(&mut self, area: Rect, buf: &mut Buffer);

    /// Handle one input event. Return `true` when consumed, `false` to let
    /// the host handle it (e.g. `q` to quit, `Tab` to switch focus).
    fn event(&mut self, _event: Event) -> bool {
        false
    }

    /// Called on every host tick while this component has focus.
    fn on_tick(&mut self) {}

    /// One-line state summary for the host status bar.
    fn status(&self) -> String {
        String::new()
    }

    /// Key hints for the host status bar.
    fn hints(&self) -> &'static str {
        ""
    }
}
