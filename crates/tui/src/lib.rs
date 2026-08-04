//! TUI display-layer adapter for `rsmarkdown-core`.

pub mod renderer;

pub use renderer::{plain_text, render_block, render_inlines, truncate, StreamMarkdownRenderer};
