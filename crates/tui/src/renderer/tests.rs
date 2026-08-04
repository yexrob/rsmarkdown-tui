//! Renderer unit tests: verify the adapter produces sane plain-text output.

use rsmarkdown_core::parse::parse_block;
use rsmarkdown_core::{Block, Inline};

use crate::renderer::theme::Theme;
use crate::renderer::{plain_text, render_block, render_inlines};

fn plain(lines: &[ratatui::text::Line]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn paragraph_wraps() {
    let ast = parse_block("hello world this is a long paragraph that must wrap");
    let ast = Some(ast);
    let lines = render_block(&ast, 20, &Theme::dark());
    let text = plain(&lines);
    assert!(text.len() >= 2);
    assert_eq!(text[0], "hello world this is");
    assert_eq!(text[1], "a long paragraph tha");
    assert_eq!(text[2], "t must wrap");
}

#[test]
fn heading_and_inline_styles() {
    let ast = parse_block("# Title **bold** `code`");
    let ast = Some(ast);
    let lines = render_block(&ast, 80, &Theme::dark());
    assert_eq!(plain(&lines), vec!["Title bold code"]);
    // first span (heading glyph styling) — verify bold modifier is applied
    let spans = &lines[0].spans;
    let bold = spans
        .iter()
        .filter(|s| s.content.as_ref() == "bold")
        .all(|s| {
            s.style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        });
    assert!(bold, "strong text should be bold");
}

#[test]
fn table_layout() {
    let ast = parse_block("| a | b |\n|---|---|\n| 1 | 22 |");
    let ast = Some(ast);
    let lines = render_block(&ast, 40, &Theme::dark());
    let text = plain(&lines);
    assert_eq!(text[0], "│ a │ b  │");
    assert_eq!(text[1], "│───┼────│");
    assert_eq!(text[2], "│ 1 │ 22 │");
}

#[test]
fn task_list_and_quote() {
    let ast = parse_block("- [x] done\n- [ ] todo\n\n> quoted");
    let ast = Some(ast);
    let lines = render_block(&ast, 40, &Theme::dark());
    let text = plain(&lines);
    assert!(text.iter().any(|l| l.contains("[x] done")));
    assert!(text.iter().any(|l| l.contains("[ ] todo")));
    assert!(text.iter().any(|l| l.contains("▌ quoted")));
}

#[test]
fn cjk_width_respected() {
    let inlines = vec![Inline::Text("中文测试文本".to_string())];
    let lines = render_inlines(&inlines, 8, &Theme::dark()); // 6 CJK chars * 2 = 12 cols > 8
    let text = plain(&lines);
    assert!(text.len() >= 2, "should wrap CJK: {:?}", text);
}

#[test]
fn plain_text_collector() {
    let ast = parse_block("a **b** and `c` and [link](https://x)");
    if let Block::Paragraph(inlines) = &ast.children[0] {
        assert_eq!(plain_text(inlines), "a b and c and link");
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn math_renders_unicode() {
    // single-line `$$...$$` survives normalize and renders as Unicode math
    use rsmarkdown_core::MarkdownProcessor;
    let mut p = MarkdownProcessor::default();
    let doc = p.process_streaming("$$\\int_a^b f(x)\\,dx = F(b) - F(a)$$");
    let mut r = crate::StreamMarkdownRenderer::new(100);
    rsmarkdown_core::Renderer::render(&mut r, &doc);
    let text: String = r
        .lines()
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert_eq!(text, "∫ₐᵇ f(x) dx = F(b) − F(a)");
}

#[test]
fn math_inline() {
    use rsmarkdown_core::MarkdownProcessor;
    let mut p = MarkdownProcessor::default();
    let doc = p.process_streaming("inline $x^2$ math");
    let mut r = crate::StreamMarkdownRenderer::new(100);
    rsmarkdown_core::Renderer::render(&mut r, &doc);
    let text: String = r
        .lines()
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    // single-dollar math: the original's `$$x$` rewrite + fixInlineMath
    // completion leaves stray literal `$` text nodes — kept faithful
    assert_eq!(text, "inline $x² math$$");
}
