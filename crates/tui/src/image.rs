//! Image rendering plumbing: protocol detection and scroll-aware layout.
//!
//! Backends (via `ratatui-image`): kitty graphics (kitty/ghostty/wezterm…),
//! sixel, iTerm2, with a unicode half-blocks fallback for every other
//! terminal. Images are rendered with [`SlicedImage`], whose skip/drop logic
//! keeps partially-visible images correct under scrolling; the kitty backend
//! uses unicode placeholders, so the terminal keeps the picture attached to
//! the cells as the viewport moves.

use image::DynamicImage;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui_image::picker::Picker;
use ratatui_image::sliced::{SignedPosition, SlicedImage, SlicedProtocol};
use ratatui_image::Resize;

/// Detect the best graphics protocol the terminal supports; fall back to
/// unicode half-blocks (works everywhere, including headless tests).
pub fn detect_picker() -> Picker {
    Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
}

/// Build a scrollable sliced protocol for an image.
pub fn sliced_for(picker: &Picker, img: DynamicImage, cells: (u16, u16)) -> SlicedProtocol {
    SlicedProtocol::new_with_resize(
        picker,
        img,
        ratatui::layout::Size::new(cells.0, cells.1),
        Resize::Fit(None),
    )
    .expect("image protocol construction")
}

/// Screen rect and widget position for an image inside a scrollable document.
///
/// `doc_top` is the image's first row on screen (may be negative when the
/// image starts above the viewport). Returns `None` when the image is fully
/// outside the area. The caller renders [`SlicedImage`] with the returned
/// position and clipped rect — the skip/drop logic inside the widget renders
/// exactly the visible rows at exactly the right place.
pub fn sliced_image_rect(
    image_rows: u16,
    image_cols: u16,
    doc_top: i16,
    area: Rect,
) -> Option<(Rect, SignedPosition)> {
    let bottom = doc_top + image_rows as i16;
    if bottom <= 0 || doc_top >= area.height as i16 {
        return None;
    }
    let vis_top = doc_top.max(0) as u16;
    let vis_bottom = bottom.min(area.height as i16) as u16;
    let x_off = (area.width.saturating_sub(image_cols) / 2) as i16;
    let clipped = Rect::new(0, vis_top, area.width, vis_bottom - vis_top);
    let position = SignedPosition::from((x_off, doc_top.min(0)));
    Some((clipped, position))
}

/// Render a sliced image into `buf` for the given doc position.
pub fn draw_sliced(sliced: &SlicedProtocol, doc_top: i16, area: Rect, buf: &mut Buffer) {
    let size = sliced.size();
    if let Some((clipped, position)) = sliced_image_rect(size.height, size.width, doc_top, area) {
        SlicedImage::new(sliced, position).render(clipped, buf);
    }
}

/// A demo image generated at runtime: gradient + stripes + circle.
/// Used by the `demo://gradient` pseudo-URL so no binary assets ship in the repo.
pub fn demo_gradient() -> DynamicImage {
    let (w, h) = (640u32, 400u32);
    let mut img = image::RgbaImage::new(w, h);
    let (cx, cy, radius) = (w as f32 * 0.72, h as f32 * 0.42, h as f32 * 0.24);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            // catppuccin-ish gradient: base -> mantle
            let (r1, g1, b1) = (30.0, 30.0, 46.0);
            let (r2, g2, b2) = (49.0, 50.0, 68.0);
            let (mut r, mut g, mut b) =
                (r1 + (r2 - r1) * t, g1 + (g2 - g1) * t, b1 + (b2 - b1) * t);
            // diagonal stripes
            if ((x + y) / 48) % 2 == 0 {
                r = (r + 14.0).min(255.0);
                g = (g + 14.0).min(255.0);
                b = (b + 18.0).min(255.0);
            }
            // accent circle with ring
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if (d - radius).abs() < 4.0 {
                r = 137.0;
                g = 220.0;
                b = 235.0;
            } else if d < radius {
                r = 148.0;
                g = 226.0;
                b = 213.0;
            }
            img.put_pixel(x, y, image::Rgba([r as u8, g as u8, b as u8, 255]));
        }
    }
    DynamicImage::ImageRgba8(img)
}

/// Resolve an image source:
/// - `demo://gradient` — generated demo image
/// - `assets/...`      — file inside `crates/tui/assets` (by manifest dir)
/// - anything else     — treated as a file path
///
/// When an `assets/...` file is missing (e.g. the repo was cloned without it)
/// the demo falls back to the generated gradient so the demo still works.
pub fn resolve_image(url: &str) -> Option<DynamicImage> {
    if url == "demo://gradient" {
        return Some(demo_gradient());
    }
    if let Some(rel) = url.strip_prefix("assets/") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel);
        return image::open(path).ok().or_else(|| Some(demo_gradient()));
    }
    image::open(url).ok()
}
