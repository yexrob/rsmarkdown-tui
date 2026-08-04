//! Headless verification of the display adapter: feed the demo doc through the
//! core pipeline and print the plain-text rendering (no terminal required).
use rsmarkdown_core::{MarkdownProcessor, Mode, Renderer};
use rsmarkdown_tui::StreamMarkdownRenderer;

const DEMO: &str = include_str!("../demo.md");

fn main() {
    let mut processor = MarkdownProcessor::default();
    let mut renderer = StreamMarkdownRenderer::new(100);

    // stream the doc in chunks like an LLM would
    let mut content = String::new();
    let mut pos = 0;
    while pos < DEMO.len() {
        let mut end = (pos + 7).min(DEMO.len());
        while end < DEMO.len() && !DEMO.is_char_boundary(end) {
            end += 1;
        }
        content.push_str(&DEMO[pos..end]);
        pos = end;
        let doc = processor.process(&content, Mode::Streaming);
        renderer.render(&doc);
    }
    for line in renderer.lines() {
        let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        println!("{}", plain);
    }
    eprintln!("total lines: {}", renderer.lines().len());
}
