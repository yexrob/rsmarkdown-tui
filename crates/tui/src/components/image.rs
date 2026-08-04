//! Image viewer component: displays a scrollable image through the best
//! terminal graphics protocol available (kitty/ghostty, sixel, iTerm2) with a
//! unicode half-blocks fallback. Scroll positions are exact: partially visible
//! rows are skipped/dropped by the sliced renderer.

use ::image::DynamicImage;
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::sliced::SlicedProtocol;

use crate::component::Component;
use crate::image;

pub struct ImagePane {
    picker: Picker,
    sliced: Option<SlicedProtocol>,
    source_name: String,
    protocol_name: &'static str,
    scroll: u16,
}

impl ImagePane {
    /// Create with protocol auto-detection (query the terminal).
    pub fn new() -> Self {
        let picker = image::detect_picker();
        Self::with_picker(picker)
    }

    /// Create with an explicit picker — used by tests (halfblocks).
    pub fn with_picker(picker: Picker) -> Self {
        let protocol_name = match picker.protocol_type() {
            ratatui_image::picker::ProtocolType::Kitty => "kitty",
            ratatui_image::picker::ProtocolType::Sixel => "sixel",
            ratatui_image::picker::ProtocolType::Iterm2 => "iterm2",
            ratatui_image::picker::ProtocolType::Halfblocks => "halfblocks",
        };
        let source = image::demo_gradient();
        let source_name = "demo://gradient".to_string();
        let sliced = image::sliced_for(&picker, source, (60, 18)).into();
        Self {
            picker,
            sliced,
            source_name,
            protocol_name,
            scroll: 0,
        }
    }

    /// Load an image from a file path (or pseudo URL), reusing the picker.
    pub fn load(&mut self, source: &str) -> bool {
        let Some(img) = image::resolve_image(source) else {
            return false;
        };
        self.load_image(img, source.to_string());
        true
    }

    pub fn load_image(&mut self, img: DynamicImage, name: String) {
        let cells = (60u16, 18u16);
        self.sliced = image::sliced_for(&self.picker, img, cells).into();
        self.source_name = name;
        self.scroll = 0;
    }

    pub fn total_rows(&self) -> u16 {
        self.sliced.as_ref().map(|s| s.size().height).unwrap_or(0)
    }

    fn scroll_by(&mut self, delta: i16) {
        self.scroll = self.scroll.saturating_add_signed(delta);
    }
}

impl Default for ImagePane {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ImagePane {
    fn title(&self) -> &str {
        "image"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let Some(sliced) = &self.sliced else {
            return;
        };
        let rows = self.total_rows();
        self.scroll = self.scroll.min(rows.saturating_sub(area.height));
        image::draw_sliced(sliced, -(self.scroll as i16), area, buf);
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
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
                KeyCode::Char('r') => {
                    self.load(&self.source_name.clone());
                    true
                }
                _ => false,
            },
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

    fn status(&self) -> String {
        match &self.sliced {
            Some(s) => format!(
                "{} {}x{} cells  {}/{}",
                self.protocol_name,
                s.size().width,
                s.size().height,
                self.scroll,
                s.size().height,
            ),
            None => format!("{} no image", self.protocol_name),
        }
    }

    fn hints(&self) -> &'static str {
        "[j/k] scroll  [g/G] top/bottom  [r] reload"
    }
}
