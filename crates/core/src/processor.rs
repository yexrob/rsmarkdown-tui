//! `MarkdownProcessor` — the streaming core. Ported from
//! `markmend/core/src/processor.ts` + `markmend/ast` cache behavior:
//!
//! ```text
//! raw content
//!   -> normalize            (CRLF, trim, LaTeX pre-processing)
//!   -> parse_markdown_into_blocks   (streaming mode; single block in static mode)
//!   -> preprocess LAST block only   (syntax self-healing fixes)
//!   -> parse each block to AST      (LRU-cached, only the tail block is new)
//! ```

use std::collections::HashMap;

use crate::ast::Ast;
use crate::blocks::parse_markdown_into_blocks;
use crate::parse::parse_block;
use crate::preprocess::{normalize, preprocess, PreprocessOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Whole document treated as a single block; no preprocess.
    Static,
    /// Block-split + preprocess on the trailing block (default).
    Streaming,
}

/// One processed block: the (preprocessed) source text and its cached AST.
#[derive(Debug, Clone)]
pub struct BlockResult {
    /// Preprocessed content actually parsed (== `source` except for the last block).
    pub content: String,
    /// Parsed AST (None for empty content).
    pub ast: Option<Ast>,
    /// True when the preprocess step modified the content (streaming tail).
    pub loading: bool,
}

/// Snapshot handed to display adapters.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub blocks: Vec<BlockResult>,
}

impl Document {
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}

/// Tiny LRU cache (max 100 entries, mirrors the original `QuickLRU`).
struct AstCache {
    map: HashMap<String, Ast>,
    order: Vec<String>,
    cap: usize,
}

impl AstCache {
    fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::new(),
            cap,
        }
    }

    fn get(&mut self, key: &str) -> Option<Ast> {
        let hit = self.map.get(key).cloned();
        if hit.is_some() {
            self.order.retain(|k| k != key);
            self.order.push(key.to_string());
        }
        hit
    }

    fn insert(&mut self, key: String, ast: Ast) {
        self.map.insert(key.clone(), ast);
        self.order.push(key);
        if self.order.len() > self.cap {
            let evicted = self.order.remove(0);
            self.map.remove(&evicted);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProcessorOptions {
    pub preprocess: PreprocessOptions,
}

/// Cache effectiveness counters, exposed for benchmarks and status bars.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Block parses satisfied from the LRU cache.
    pub cache_hits: u64,
    /// Block parses that actually ran the markdown parser.
    pub fresh_parses: u64,
}

impl CacheStats {
    pub fn hits(&self) -> u64 {
        self.cache_hits
    }
    pub fn parses(&self) -> u64 {
        self.cache_hits + self.fresh_parses
    }
    pub fn hit_rate(&self) -> f64 {
        let total = self.parses();
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

pub struct MarkdownProcessor {
    options: ProcessorOptions,
    cache: AstCache,
    stats: CacheStats,
}

impl Default for MarkdownProcessor {
    fn default() -> Self {
        Self::new(ProcessorOptions::default())
    }
}

impl MarkdownProcessor {
    pub fn new(options: ProcessorOptions) -> Self {
        Self {
            options,
            cache: AstCache::new(100),
            stats: CacheStats::default(),
        }
    }

    pub fn with_cache_capacity(options: ProcessorOptions, cap: usize) -> Self {
        Self {
            options,
            cache: AstCache::new(cap),
            stats: CacheStats::default(),
        }
    }

    /// Cache effectiveness since the processor was created.
    pub fn cache_stats(&self) -> CacheStats {
        self.stats
    }

    pub fn normalize(&self, content: &str) -> String {
        normalize(content)
    }

    pub fn preprocess(&self, content: &str) -> String {
        preprocess(content, &self.options.preprocess)
    }

    pub fn parse_markdown_into_blocks(&self, content: &str) -> Vec<String> {
        parse_markdown_into_blocks(content)
    }

    /// Parse a single content string into its AST (through the cache).
    pub fn parse(&mut self, content: &str) -> Option<Ast> {
        if content.is_empty() {
            return None;
        }
        if let Some(ast) = self.cache.get(content) {
            self.stats.cache_hits += 1;
            return Some(ast);
        }
        let ast = parse_block(content);
        self.cache.insert(content.to_string(), ast.clone());
        self.stats.fresh_parses += 1;
        Some(ast)
    }

    /// The main entry: full markdown content -> blocks + per-block ASTs.
    pub fn process(&mut self, content: &str, mode: Mode) -> Document {
        let normalized = self.normalize(content);
        if normalized.is_empty() {
            return Document::default();
        }

        let blocks = match mode {
            Mode::Static => vec![normalized],
            Mode::Streaming => self.parse_markdown_into_blocks(&normalized),
        };

        let mut doc = Document {
            blocks: Vec::with_capacity(blocks.len()),
        };
        for (index, block) in blocks.iter().enumerate() {
            let is_last = index == blocks.len() - 1;
            let content = if mode == Mode::Streaming && is_last {
                self.preprocess(block)
            } else {
                block.clone()
            };
            let loading = content != *block;
            let ast = self.parse(&content);
            doc.blocks.push(BlockResult {
                content,
                ast,
                loading,
            });
        }
        doc
    }

    pub fn process_streaming(&mut self, content: &str) -> Document {
        self.process(content, Mode::Streaming)
    }

    pub fn process_static(&mut self, content: &str) -> Document {
        self.process(content, Mode::Static)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let mut p = MarkdownProcessor::default();
        assert!(p.process_streaming("").is_empty());
        assert!(p.process_streaming("\n\n").is_empty());
    }

    #[test]
    fn static_single_block() {
        let mut p = MarkdownProcessor::default();
        let doc = p.process_static("# Hi\n\nText");
        assert_eq!(doc.len(), 1);
        assert!(!doc.blocks[0].loading);
    }

    #[test]
    fn streaming_blocks_and_loading_tail() {
        let mut p = MarkdownProcessor::default();
        let doc = p.process_streaming("# Hi\n\nText **bold");
        assert_eq!(doc.len(), 2);
        assert!(!doc.blocks[0].loading);
        assert!(doc.blocks[1].loading); // preprocessed tail
        assert!(doc.blocks[1].content.ends_with("**"));
    }

    #[test]
    fn completed_blocks_stay_stable() {
        // The classic streaming property: parsing a prefix P then P+chunk yields
        // the same block boundaries for the completed part.
        let mut p = MarkdownProcessor::default();
        let a = p.process_streaming("# Hi\n\nSome text here\n\n```js\nlet x = 1");
        let b = p.process_streaming("# Hi\n\nSome text here\n\n```js\nlet x = 1\n```");
        assert_eq!(a.blocks.len(), 3);
        assert_eq!(b.blocks.len(), 3);
        for i in 0..2 {
            assert_eq!(a.blocks[i].content, b.blocks[i].content);
            assert_eq!(a.blocks[i].ast, b.blocks[i].ast);
        }
    }

    #[test]
    fn cache_hits() {
        let mut p = MarkdownProcessor::default();
        p.process_streaming("a\n\nb\n\nc\n\nd\n\ne\n\nf\n\ng\n\nh\n\ni\n\nj\n\nk");
        // all blocks cached; parse again cheaply
        let doc = p.process_streaming("a\n\nb\n\nc\n\nd\n\ne\n\nf\n\ng\n\nh\n\ni\n\nj\n\nk");
        assert_eq!(doc.len(), 11);
    }
}
