//! Demo: mount five components into the host and run.
//!
//! - `[1] markdown` — streaming markdown viewer (the original demo)
//! - `[2] image`    — scrollable image (kitty/ghostty graphics backend)
//! - `[3] chat`     — agent session: thinking + tool-call hints + markdown
//! - `[4] log`      — plain-text streaming log (non-markdown component)
//! - `[5] tasks`    — selectable task list (interactive component)
//! - `[6] agents`   — agent overview (absorbs the chat's subagents via the
//!                    host agent broadcast)
//!
//! Tab / `[` `]` / 1-6 switch focus, `q` quits.

use rsmarkdown_tui::components::agent_view::AgentView;
use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::components::image::ImagePane;
use rsmarkdown_tui::components::list::ListView;
use rsmarkdown_tui::components::markdown::MarkdownViewer;
use rsmarkdown_tui::components::text::TextReader;
use rsmarkdown_tui::{run_tui, App, PrStatus};

fn main() -> std::io::Result<()> {
    let mut app = App::new(vec![
        Box::new(MarkdownViewer::new()),
        Box::new(ImagePane::new()),
        Box::new(AgentChat::new()),
        Box::new(TextReader::new()),
        Box::new(ListView::todo_examples()),
        Box::new(AgentView::new()),
    ]);
    // demo pull-request badge (underlined, color-encoded review status)
    app.set_pr(446, PrStatus::Pending);
    run_tui(&mut app)
}
