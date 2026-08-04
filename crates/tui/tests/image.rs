//! Headless image-backend tests: scroll positions and layout must stay exact
//! using the unicode half-blocks protocol (no terminal required).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;

use rsmarkdown_tui::components::image::ImagePane;
use rsmarkdown_tui::components::markdown::MarkdownViewer;
use rsmarkdown_tui::{image, Component};

fn row_text(buf: &Buffer, y: u16, width: u16) -> String {
    let mut out = String::new();
    for x in 0..width {
        out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
    }
    out
}

/// Is this row part of a rendered (half-block) image?
fn is_image_row(buf: &Buffer, y: u16, width: u16) -> bool {
    row_text(buf, y, width).contains(['▀', '▄', '█'])
}

#[test]
fn image_pane_scroll_moves_content() {
    let picker = Picker::halfblocks();
    let mut pane = ImagePane::with_picker(picker);
    let area = Rect::new(0, 0, 60, 8);
    let mut buf = Buffer::empty(area);
    pane.draw(area, &mut buf);
    let top_at_zero = row_text(&buf, 0, 60);

    // scroll down one row: content must shift by exactly one row
    let down = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(pane.event(down));
    let mut buf2 = Buffer::empty(area);
    pane.draw(area, &mut buf2);
    let top_at_one = row_text(&buf2, 0, 60);
    let second_at_zero = row_text(&buf, 1, 60);

    assert_ne!(top_at_zero, top_at_one, "scroll must change the view");
    assert_eq!(
        top_at_one, second_at_zero,
        "row N+1 of the previous frame must become row N after scrolling down"
    );
}

#[test]
fn image_pane_clips_off_screen_rows() {
    let picker = Picker::halfblocks();
    let mut pane = ImagePane::with_picker(picker);
    let area = Rect::new(0, 0, 60, 5);
    let total = pane.total_rows();
    let mut buf = Buffer::empty(area);

    // scroll to the very bottom: only the last `area.height` rows are visible,
    // and no image content leaks above the viewport
    for _ in 0..total + 10 {
        let down = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        ));
        pane.event(down);
    }
    pane.draw(area, &mut buf);
    for y in 0..area.height {
        assert!(
            is_image_row(&buf, y, 60),
            "row {y} should show image content at the bottom"
        );
    }
}

#[test]
fn markdown_image_flows_with_text() {
    // doc: text, image, text — image must sit between the two text blocks and
    // shift correctly when scrolled
    let mut viewer = MarkdownViewer::new();
    viewer.set_content("intro line\n\n![terminal demo](demo://gradient)\n\noutro line");
    for _ in 0..3 {
        viewer.on_tick();
    }
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    viewer.draw(area, &mut buf);

    let rows: Vec<String> = (0..area.height).map(|y| row_text(&buf, y, 80)).collect();
    let text_rows: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.contains("intro line") || r.contains("outro line"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(text_rows.len(), 2, "both text lines visible");
    assert!(text_rows[0] < text_rows[1], "text order preserved");

    // the image must sit between the two text rows
    let image_rows: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.contains(['▀', '▄', '█']))
        .map(|(i, _)| i)
        .collect();
    assert!(!image_rows.is_empty(), "image visible:\n{rows:?}");
    assert!(
        image_rows[0] > text_rows[0] && *image_rows.last().unwrap() < text_rows[1],
        "image between text blocks"
    );
}

#[test]
fn markdown_image_scrolls_partially_off() {
    let mut viewer = MarkdownViewer::new();
    viewer.set_content("intro line\n\n![terminal demo](demo://gradient)\n\noutro line");
    for _ in 0..3 {
        viewer.on_tick();
    }
    let area = Rect::new(0, 0, 80, 6);
    let mut buf = Buffer::empty(area);

    // scroll down until the image top is exactly at the viewport top
    for _ in 0..30 {
        let down = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        ));
        viewer.event(down);
    }
    viewer.draw(area, &mut buf);
    // image occupies the top rows now; no text above it
    assert!(
        is_image_row(&buf, 0, 80),
        "image starts at viewport top after scroll"
    );
}
