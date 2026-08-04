//! `rsmarkdown-core` — streaming markdown core.
//!
//! Pipeline (mirrors `jinghaihan/vue-stream-markdown`'s markmend package):
//!
//! ```text
//! raw content
//!   -> normalize            (CRLF, trailing whitespace, LaTeX pre-processing)
//!   -> parse_markdown_into_blocks   (streaming mode; single block in static mode)
//!   -> preprocess LAST block only   (syntax self-healing: code/strong/emphasis/delete/
//!                                    task-list/link/table/inline-math/math/html/footnote)
//!   -> parse each block to AST      (LRU-cached, only the tail block is new)
//!   -> Renderer trait               (display adapters implement this)
//! ```
//!
//! The core crate has zero terminal/display dependencies.

pub mod ast;
pub mod blocks;
pub mod fix;
#[cfg(test)]
mod generated_tests;
pub mod parse;
pub mod pattern;
pub mod preprocess;
pub mod processor;
pub mod renderer;
pub mod scan;

pub use ast::{Alignment, Ast, Block, Inline, ListItem};
pub use preprocess::PreprocessOptions;
pub use processor::{BlockResult, Document, MarkdownProcessor, Mode, ProcessorOptions};
pub use renderer::{NullRenderer, Renderer};
