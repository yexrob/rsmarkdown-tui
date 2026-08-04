//! Markdown viewer component: streams markdown through `rsmarkdown-core` and
//! paints it with the display adapter. One component among many — the host
//! treats it like any other.

use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use rsmarkdown_core::{Document, MarkdownProcessor, Mode, Renderer};

use crate::component::Component;
use crate::renderer::StreamMarkdownRenderer;

const DEMO_DOC: &str = include_str!("../../demo.md");

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
}

impl Default for MarkdownViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownViewer {
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
        let lines = self.renderer.lines();
        let total = lines.len() as u16;
        if self.auto_scroll && self.demo_pos < DEMO_DOC.len() {
            self.scroll = total.saturating_sub(area.height);
        }
        let scroll = self.scroll.min(total.saturating_sub(area.height));
        for (y, line) in lines
            .iter()
            .skip(scroll as usize)
            .take(area.height as usize)
            .enumerate()
        {
            buf.set_line(area.x, area.y + y as u16, line, area.width);
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
            "{}B {} blocks {} lines {}/{}  parse {}µs  cached {}",
            self.content.len(),
            self.doc.blocks.len(),
            self.renderer.lines().len(),
            self.scroll,
            self.renderer.lines().len(),
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
