//! Component model: the TUI is a host for pluggable, independently-owned
//! components. Markdown rendering is just one component; text viewers, lists,
//! tables, dashboards etc. implement the same trait.

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::activities::TodoItem;
use crate::agent::Agent;
use crate::app::FooterBadge;
use crate::permission::{DialogAction, PermissionRequest};
use crate::renderer::theme::Theme;

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

    /// Footer badges contributed by this component (Claude Code style:
    /// `← for agents`, subagent status, …).
    fn footer_badges(&self) -> Vec<FooterBadge> {
        Vec::new()
    }

    /// A permission request this component wants to raise right now. The
    /// host polls this each tick and opens a modal dialog when it returns
    /// `Some`; the component learns the outcome through [`Self::on_dialog_closed`].
    fn on_ask(&mut self) -> Option<PermissionRequest> {
        None
    }

    /// The modal permission dialog was closed with this action (only called
    /// if a dialog was actually open).
    fn on_dialog_closed(&mut self, _action: DialogAction) {}

    /// Agent sessions this component wants to broadcast to the host. The
    /// host merges all components' broadcasts each tick and delivers the
    /// combined table to every component via [`Self::absorb_agents`] — so
    /// an overview component (e.g. [`crate::components::agent_view::AgentView`])
    /// collects agents from siblings without knowing them.
    fn agents(&self) -> Vec<Agent> {
        Vec::new()
    }

    /// Receive the merged agent table broadcast by the host (see
    /// [`Self::agents`]). Default: ignore.
    fn absorb_agents(&mut self, _agents: &[Agent]) {}

    /// The checklist this component wants to show in the host task area
    /// (Claude Code: `Ctrl+T` toggles the task list). The host merges all
    /// components' broadcasts and renders up to five tasks.
    fn tasks(&self) -> Vec<TodoItem> {
        Vec::new()
    }

    /// Switch the semantic color theme of this component. The host calls
    /// this on every component when the app theme changes ([`crate::App::set_theme`]).
    fn set_theme(&mut self, _theme: Theme) {}
}
