//! Component-based TUI framework with a markdown display adapter.
//!
//! - [`App`] — host: event loop, focus routing, status bar
//! - [`Component`] — the pluggable pane contract
//! - [`components`] — markdown viewer + plain-text and list examples
//! - [`renderer`] — markdown AST -> styled lines (display adapter)

pub mod app;
pub mod component;
pub mod components;
pub mod renderer;

pub use app::{run_tui, App};
pub use component::Component;
pub use renderer::{plain_text, render_block, render_inlines, truncate, StreamMarkdownRenderer};
