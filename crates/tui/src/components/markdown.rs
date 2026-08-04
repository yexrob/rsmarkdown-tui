//! Markdown viewer component: streams markdown through `rsmarkdown-core` and
//! paints it with the display adapter. One component among many — the host
//! treats it like any other.

use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use ratatui_image::picker::Picker;
use ratatui_image::sliced::SlicedProtocol;

use rsmarkdown_core::{Block, Document, Inline, MarkdownProcessor, Mode, Renderer};

use crate::component::Component;
use crate::image;
use crate::renderer::StreamMarkdownRenderer;

const DEMO_DOC: &str = include_str!("../../demo.md");

/// One row of the document layout: either text lines or a rendered image.
enum LayoutItem {
    Text(Vec<ratatui::text::Line<'static>>),
    /// Index into `self.images`.
    Image(usize),
}

/// A loaded image paragraph (url -> sliced protocol).
struct LoadedImage {
    url: String,
    sliced: SlicedProtocol,
}

/// Streaming markdown viewer component (demo doc + stress mode).
pub struct MarkdownViewer {
    processor: MarkdownProcessor,
    renderer: StreamMarkdownRenderer,
    content: String,
    demo_pos: usize,
    typing: bool,
    auto_scroll: bool,
    scroll: u16,
    doc: Document,
    last_parse_us: u64,
    cache_hits: u64,
    last_content: String,
    width: usize,
    layout: Vec<LayoutItem>,
    images: Vec<LoadedImage>,
    picker: Picker,
}

impl Default for MarkdownViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownViewer {
    /// Create the viewer; the demo document starts streaming immediately.
    pub fn new() -> Self {
        let mut this = Self {
            processor: MarkdownProcessor::default(),
            renderer: StreamMarkdownRenderer::new(80),
            content: String::new(),
            demo_pos: 0,
            typing: false,
            auto_scroll: true,
            scroll: 0,
            doc: Document::default(),
            last_parse_us: 0,
            cache_hits: 0,
            last_content: String::new(),
            width: 80,
            layout: Vec::new(),
            images: Vec::new(),
            picker: image::detect_picker(),
        };
        this.advance_demo();
        this.advance_demo();
        this.refresh();
        this
    }

    fn refresh(&mut self) {
        // skip the whole pipeline when the content did not change — the host
        // ticks us 30x/s even while idle
        if self.last_content == self.content {
            return;
        }
        self.last_content.clone_from(&self.content);
        let t = Instant::now();
        self.doc = self.processor.process(&self.content, Mode::Streaming);
        self.last_parse_us = t.elapsed().as_micros() as u64;
        self.renderer.render(&self.doc);
        self.cache_hits = self.processor.cache_stats().hits();
        self.build_layout();
    }

    /// True when a block is a paragraph whose trailing element is an image.
    /// (pulldown-cmark emits the alt text as a separate leading Text event.)
    fn image_paragraph(block: &Block) -> Option<&str> {
        if let Block::Paragraph(inlines) = block {
            if let Some(Inline::Image { url, .. }) = inlines.last() {
                return Some(url);
            }
        }
        None
    }

    /// Get or load the sliced protocol for an image url.
    fn image_for(&mut self, url: &str) -> Option<usize> {
        if let Some(i) = self.images.iter().position(|im| im.url == url) {
            return Some(i);
        }
        let img = image::resolve_image(url)?;
        let sliced = image::sliced_for(&self.picker, img, (46, 14));
        self.images.push(LoadedImage {
            url: url.to_string(),
            sliced,
        });
        Some(self.images.len() - 1)
    }

    /// Rebuild the document layout (text lines + images) from the parsed blocks.
    fn build_layout(&mut self) {
        self.layout.clear();
        let mut block_lines = Vec::with_capacity(self.doc.blocks.len());
        for index in 0..self.doc.blocks.len() {
            let block = &self.doc.blocks[index];
            let is_image = block
                .ast
                .as_ref()
                .and_then(|ast| ast.children.iter().find_map(Self::image_paragraph))
                .map(|url| url.to_string());
            let lines = self.renderer.block_lines(index).map(|l| l.to_vec());
            block_lines.push((is_image, lines));
        }
        for (is_image, lines) in block_lines {
            if let Some(url) = is_image {
                if let Some(img_idx) = self.image_for(&url) {
                    self.layout.push(LayoutItem::Image(img_idx));
                    continue;
                }
            }
            if let Some(lines) = lines {
                self.layout.push(LayoutItem::Text(lines));
            }
        }
    }

    /// Raw content (diagnostic helper).
    pub fn content_debug(&self) -> &str {
        &self.content
    }

    /// Parsed blocks (diagnostic helper).
    pub fn blocks_debug(&self) -> Vec<&rsmarkdown_core::Block> {
        let mut out = Vec::new();
        for b in &self.doc.blocks {
            if let Some(ast) = &b.ast {
                out.extend(ast.children.iter());
            }
        }
        out
    }

    /// Number of loaded images (test/diagnostic helper).
    pub fn images_count(&self) -> usize {
        self.images.len()
    }

    /// Number of layout items (test/diagnostic helper).
    pub fn layout_items(&self) -> usize {
        self.layout.len()
    }

    /// Layout items as (kind, rows) — diagnostic helper.
    pub fn layout_debug(&self) -> Vec<(String, u16)> {
        let mut out = Vec::new();
        for item in &self.layout {
            match item {
                LayoutItem::Text(l) => out.push(("text".into(), l.len() as u16)),
                LayoutItem::Image(i) => out.push((
                    "image".into(),
                    self.images
                        .get(*i)
                        .map(|im| im.sliced.size().height)
                        .unwrap_or(0),
                )),
            }
        }
        out
    }

    /// Document height in rows (text lines + image heights).
    fn layout_height(&self) -> u16 {
        let mut h = 0u16;
        for item in &self.layout {
            match item {
                LayoutItem::Text(lines) => h = h.saturating_add(lines.len() as u16),
                LayoutItem::Image(idx) => {
                    h = h.saturating_add(
                        self.images
                            .get(*idx)
                            .map(|im| im.sliced.size().height)
                            .unwrap_or(0),
                    );
                }
            }
        }
        h
    }

    fn advance_demo(&mut self) {
        if self.demo_pos >= DEMO_DOC.len() {
            return;
        }
        let mut chunk_end = (self.demo_pos + 2).min(DEMO_DOC.len());
        while chunk_end < DEMO_DOC.len() && !DEMO_DOC.is_char_boundary(chunk_end) {
            chunk_end += 1;
        }
        self.content.push_str(&DEMO_DOC[self.demo_pos..chunk_end]);
        self.demo_pos = chunk_end;
    }

    fn restart_demo(&mut self) {
        self.content.clear();
        self.demo_pos = 0;
        self.typing = false;
        self.scroll = 0;
        self.auto_scroll = true;
    }

    /// Replace the content directly (used by tests and external feeds).
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
        self.demo_pos = DEMO_DOC.len();
        self.typing = false;
        self.refresh();
    }

    /// Generate a ~200 KB stress document and load it instantly.
    fn load_stress_doc(&mut self) {
        let unit = DEMO_DOC;
        let mut big = String::with_capacity(200 * 1024);
        while big.len() < 200 * 1024 {
            big.push_str(unit);
        }
        self.content = big;
        self.demo_pos = DEMO_DOC.len();
        self.typing = false;
        self.auto_scroll = false;
        self.scroll = 0;
        self.refresh();
    }

    fn scroll_by(&mut self, delta: i16) {
        self.scroll = self.scroll.saturating_add_signed(delta);
        self.auto_scroll = false;
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.typing {
            match key.code {
                KeyCode::Char(c) => {
                    if c == '`' && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return true;
                    }
                    self.content.push(c);
                    return true;
                }
                KeyCode::Backspace => {
                    self.content.pop();
                    return true;
                }
                KeyCode::Esc => {
                    self.typing = false;
                    return true;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('t') => {
                self.typing = !self.typing;
                self.auto_scroll = false;
                true
            }
            KeyCode::Char('d') => {
                self.restart_demo();
                true
            }
            KeyCode::Char('s') => {
                self.auto_scroll = !self.auto_scroll;
                true
            }
            KeyCode::Char('r') => {
                self.restart_demo();
                self.advance_demo();
                true
            }
            KeyCode::Char('p') => {
                self.load_stress_doc();
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_by(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_by(-1);
                true
            }
            KeyCode::PageDown => {
                self.scroll_by(10);
                true
            }
            KeyCode::PageUp => {
                self.scroll_by(-10);
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll = u16::MAX;
                true
            }
            KeyCode::Esc => {
                self.typing = false;
                true
            }
            _ => false,
        }
    }
}

impl Component for MarkdownViewer {
    fn title(&self) -> &str {
        "markdown"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width as usize != self.width {
            self.width = area.width as usize;
            self.refresh();
        }
        // total document height (text lines + image rows)
        let total = self.layout_height();
        if self.auto_scroll && self.demo_pos < DEMO_DOC.len() {
            self.scroll = total.saturating_sub(area.height);
        }
        let scroll = self.scroll.min(total.saturating_sub(area.height));

        let mut doc_row = 0u16;
        for item in &self.layout {
            match item {
                LayoutItem::Text(lines) => {
                    for (i, line) in lines.iter().enumerate() {
                        let y = doc_row + i as u16;
                        if y >= scroll && y < scroll + area.height {
                            buf.set_line(area.x, area.y + y - scroll, line, area.width);
                        }
                    }
                    doc_row += lines.len() as u16;
                }
                LayoutItem::Image(idx) => {
                    if let Some(img) = self.images.get(*idx) {
                        image::draw_sliced(&img.sliced, doc_row as i16 - scroll as i16, area, buf);
                    }
                    doc_row += self
                        .images
                        .get(*idx)
                        .map(|im| im.sliced.size().height)
                        .unwrap_or(0);
                }
            }
        }
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key(key),
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_by(-3);
                    true
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_by(3);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn on_tick(&mut self) {
        if !self.typing {
            self.advance_demo();
        }
        self.refresh();
    }

    fn status(&self) -> String {
        format!(
            "{}B {} blocks {} rows {}/{} {} images  parse {}µs  cached {}",
            self.content.len(),
            self.doc.blocks.len(),
            self.layout_height(),
            self.scroll,
            self.layout_height(),
            self.images.len(),
            self.last_parse_us,
            self.cache_hits,
        )
    }

    fn hints(&self) -> &'static str {
        if self.typing {
            "[esc] stop typing  [d] demo"
        } else {
            "[t] type  [d] demo  [p] stress 200KB  [j/k] scroll  [s] autoscroll"
        }
    }
}
