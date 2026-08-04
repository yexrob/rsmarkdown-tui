//! The core/display seam. Display adapters (TUI, web, etc.) implement
//! [`Renderer`]; the core never depends on any display library.

use crate::processor::Document;

/// Display-layer contract. The adapter decides how to paint the AST.
///
/// Implementations are free to diff against their previous input to re-render
/// only the changed blocks (the TUI adapter does exactly that).
pub trait Renderer {
    /// Render (or update) the given document.
    fn render(&mut self, doc: &Document);
}

/// A renderer that produces no output — useful as a benchmark baseline.
pub struct NullRenderer;

impl Renderer for NullRenderer {
    fn render(&mut self, _doc: &Document) {}
}
