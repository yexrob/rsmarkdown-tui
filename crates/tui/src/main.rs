//! Demo: mount three components into the host and run.
//!
//! - `[1] markdown` — streaming markdown viewer (the original demo)
//! - `[2] image`    — scrollable image (kitty/ghostty graphics backend)
//! - `[3] chat`     — agent session: thinking + tool-call hints + markdown
//! - `[4] log`      — plain-text streaming log (non-markdown component)
//! - `[5] tasks`    — selectable task list (interactive component)
//!
//! Tab / `[` `]` / 1-3 switch focus, `q` quits.

use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::components::image::ImagePane;
use rsmarkdown_tui::components::list::ListView;
use rsmarkdown_tui::components::markdown::MarkdownViewer;
use rsmarkdown_tui::components::text::TextReader;
use rsmarkdown_tui::{run_tui, App};

fn main() -> std::io::Result<()> {
    let mut app = App::new(vec![
        Box::new(MarkdownViewer::new()),
        Box::new(ImagePane::new()),
        Box::new(AgentChat::new()),
        Box::new(TextReader::new()),
        Box::new(ListView::todo_examples()),
    ]);
    run_tui(&mut app)
}
