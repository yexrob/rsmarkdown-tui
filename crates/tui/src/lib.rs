//! Component-based TUI framework with terminal graphics and agent-transcript
//! abstractions — a library, not a markdown viewer.
//!
//! The demo binary (`crates/tui/src/main.rs`) only *consumes* the library:
//! every capability below is public API that a custom app can assemble.
//!
//! # Architecture
//!
//! - [`App`] — host: event loop, focus routing, mouse-coordinate translation,
//!   footer badges, status bar. Owns a list of [`Component`]s.
//! - [`Component`] — the pluggable pane contract (`draw` / `event` / `on_tick`
//!   / `status` / `hints` / `footer_badges`).
//! - [`activities`] — Claude Code-style agent transcript model: foldable
//!   [`Activity`]s (thinking / tool calls / subagents / todos / diffs) with
//!   recursive nested transcripts and click-to-toggle layout.
//! - [`image`] — terminal graphics backend (kitty / sixel / iTerm2 with a
//!   half-blocks fallback), scroll-correct sliced rendering.
//! - [`renderer`] — markdown display adapter: AST -> styled lines.
//! - [`components`] — ready-made panes (markdown viewer, image viewer, agent
//!   chat, log, task list).
//!
//! # Minimal custom app
//!
//! ```no_run
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use rsmarkdown_tui::{App, Component, run_tui};
//!
//! struct Hello;
//!
//! impl Component for Hello {
//!     fn title(&self) -> &str { "hello" }
//!     fn draw(&mut self, area: Rect, buf: &mut Buffer) {
//!         buf.set_string(area.x, area.y, "hello from a custom component", ratatui::style::Style::default());
//!     }
//! }
//!
//! fn main() -> std::io::Result<()> {
//!     let mut app = App::new(vec![Box::new(Hello)]);
//!     run_tui(&mut app)
//! }
//! ```

#![warn(missing_docs)]

pub mod activities;
pub mod agent;
pub mod app;
pub mod command_menu;
pub mod component;
pub mod components;
pub mod help;
pub mod image;
pub mod permission;
pub mod renderer;

pub use activities::{
    activities_path_get_mut, activity_lines, auto_expand, diff_lines, layout_activities,
    todo_lines, Activity, ActivityKind, ActivityRowRange, Diff, DiffLine, Hunk, SubAgent,
    SubAgentStatus, Thinking, ThinkingState, TodoItem, TodoList, TodoStatus, ToolCall, ToolStatus,
};
pub use app::{run_tui, App, FooterBadge, PrStatus, SessionMode};
pub use component::Component;
pub use image::{
    demo_gradient, detect_picker, draw_sliced, resolve_image, sliced_for, sliced_image_rect,
};
pub use renderer::{plain_text, render_block, render_inlines, truncate, StreamMarkdownRenderer};
