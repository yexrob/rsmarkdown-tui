//! Markdown string -> AST conversion on top of pulldown-cmark.
//! This is the Rust counterpart of `mdast-util-from-markdown` used by the original.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::ast::{Alignment, Ast, Block, Inline, ListItem};

pub fn parse_block(markdown: &str) -> Ast {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(markdown, options);
    let mut builder = Builder::default();
    for event in parser {
        builder.push(event);
    }
    builder.finish()
}

struct Builder {
    stack: Vec<Frame>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            stack: vec![Frame::Root(Vec::new())],
        }
    }
}

enum Frame {
    Root(Vec<Block>),
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        children: Vec<Inline>,
    },
    BlockQuote(Vec<Block>),
    CodeBlock {
        lang: String,
        text: String,
    },
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Item {
        checked: Option<bool>,
        pending: Option<Vec<Inline>>,
        children: Vec<Block>,
    },
    Table {
        aligns: Vec<Alignment>,
        head: Vec<Vec<Inline>>,
        body: Vec<Vec<Vec<Inline>>>,
        in_head: bool,
    },
    Footnote {
        label: String,
        children: Vec<Block>,
    },
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        text: Vec<Inline>,
        url: String,
    },
    Image {
        text: Vec<Inline>,
        url: String,
    },
    Cell(Vec<Inline>),
    Row(Vec<Vec<Inline>>),
}

fn is_block_frame(f: &Frame) -> bool {
    matches!(
        f,
        Frame::Root(_) | Frame::BlockQuote(_) | Frame::Item { .. } | Frame::Footnote { .. }
    )
}

fn is_inline_frame(f: &Frame) -> bool {
    matches!(
        f,
        Frame::Paragraph(_)
            | Frame::Heading { .. }
            | Frame::Strong(_)
            | Frame::Emphasis(_)
            | Frame::Strikethrough(_)
            | Frame::Link { .. }
            | Frame::Cell(_)
    )
}

impl Builder {
    fn push(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag_end) => self.end(tag_end),
            Event::Text(text) => {
                let text = text.into_string();
                if let Some(Frame::CodeBlock { text: buf, .. }) = self.stack.last_mut() {
                    buf.push_str(&text);
                } else {
                    self.inline(Inline::Text(text));
                }
            }
            Event::Code(code) => self.inline(Inline::Code(code.into_string())),
            Event::InlineMath(m) => self.inline(Inline::Math(m.into_string(), false)),
            Event::DisplayMath(m) => self.inline(Inline::Math(m.into_string(), true)),
            Event::Html(html) => self.html(html.into_string()),
            Event::InlineHtml(html) => self.inline(Inline::Html(html.into_string())),
            Event::FootnoteReference(label) => {
                self.inline(Inline::FootnoteRef(label.into_string()))
            }
            Event::SoftBreak => self.inline(Inline::SoftBreak),
            Event::HardBreak => self.inline(Inline::HardBreak),
            Event::Rule => self.block(Block::ThematicBreak),
            Event::TaskListMarker(checked) => {
                // attach to the nearest Item frame
                if let Some(Frame::Item { checked: slot, .. }) = self.stack.last_mut() {
                    *slot = Some(checked);
                }
            }
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.stack.push(Frame::Paragraph(Vec::new())),
            Tag::Heading { level, .. } => self.stack.push(Frame::Heading {
                level: match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                },
                children: Vec::new(),
            }),
            Tag::BlockQuote(_) => self.stack.push(Frame::BlockQuote(Vec::new())),
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.into_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.stack.push(Frame::CodeBlock {
                    lang,
                    text: String::new(),
                });
            }
            Tag::List(start) => self.stack.push(Frame::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
            }),
            Tag::Item => self.stack.push(Frame::Item {
                checked: None,
                pending: None,
                children: Vec::new(),
            }),
            Tag::FootnoteDefinition(label) => {
                self.stack.push(Frame::Footnote {
                    label: label.into_string(),
                    children: Vec::new(),
                });
            }
            Tag::Table(aligns) => self.stack.push(Frame::Table {
                aligns: aligns
                    .iter()
                    .map(|a| match a {
                        pulldown_cmark::Alignment::None => Alignment::None,
                        pulldown_cmark::Alignment::Left => Alignment::Left,
                        pulldown_cmark::Alignment::Center => Alignment::Center,
                        pulldown_cmark::Alignment::Right => Alignment::Right,
                    })
                    .collect(),
                head: Vec::new(),
                body: Vec::new(),
                in_head: false,
            }),
            Tag::TableHead => {
                if let Some(Frame::Table { in_head, .. }) = self.stack.last_mut() {
                    *in_head = true;
                }
            }
            Tag::TableRow => self.stack.push(Frame::Row(Vec::new())),
            Tag::TableCell => self.stack.push(Frame::Cell(Vec::new())),
            Tag::Strong => self.stack.push(Frame::Strong(Vec::new())),
            Tag::Emphasis => self.stack.push(Frame::Emphasis(Vec::new())),
            Tag::Strikethrough => self.stack.push(Frame::Strikethrough(Vec::new())),
            Tag::Link { dest_url, .. } => {
                self.stack.push(Frame::Link {
                    text: Vec::new(),
                    url: dest_url.into_string(),
                });
            }
            Tag::Image { dest_url, .. } => {
                self.stack.push(Frame::Image {
                    text: Vec::new(),
                    url: dest_url.into_string(),
                });
            }
            _ => {}
        }
    }

    fn end(&mut self, tag_end: TagEnd) {
        // TagEnd::TableHead does not correspond to a pushed frame (header cells
        // are collected straight into the Table frame).
        if matches!(tag_end, TagEnd::TableHead) {
            if let Some(Frame::Table { in_head, .. }) = self.stack.last_mut() {
                *in_head = false;
            }
            return;
        }
        let finished = self.stack.pop();
        match (tag_end, finished) {
            (TagEnd::Paragraph, Some(Frame::Paragraph(children))) => {
                self.block(Block::Paragraph(children))
            }
            (TagEnd::Heading(_), Some(Frame::Heading { level, children })) => {
                self.block(Block::Heading { level, children });
            }
            (TagEnd::BlockQuote(_), Some(Frame::BlockQuote(children))) => {
                self.block(Block::BlockQuote(children))
            }
            (TagEnd::CodeBlock, Some(Frame::CodeBlock { lang, text })) => {
                self.block(Block::Code { lang, text })
            }
            (
                TagEnd::List(_),
                Some(Frame::List {
                    ordered,
                    start,
                    items,
                }),
            ) => self.block(Block::List {
                ordered,
                start,
                items,
            }),
            (
                TagEnd::Item,
                Some(Frame::Item {
                    checked,
                    pending,
                    mut children,
                }),
            ) => {
                if let Some(paragraph) = pending {
                    children.push(Block::Paragraph(paragraph));
                }
                if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                    items.push(ListItem { checked, children });
                }
            }
            (TagEnd::FootnoteDefinition, Some(Frame::Footnote { label, children })) => {
                self.block(Block::FootnoteDefinition { label, children });
            }
            (
                TagEnd::Table,
                Some(Frame::Table {
                    aligns, head, body, ..
                }),
            ) => {
                self.block(Block::Table {
                    headers: head,
                    rows: body,
                    aligns,
                });
            }

            (TagEnd::TableRow, Some(Frame::Row(row))) => {
                if let Some(Frame::Table {
                    head,
                    body,
                    in_head,
                    ..
                }) = self.stack.last_mut()
                {
                    if *in_head {
                        for cell in row {
                            head.push(cell);
                        }
                    } else {
                        body.push(row);
                    }
                }
            }
            (TagEnd::TableCell, Some(Frame::Cell(cell))) => match self.stack.last_mut() {
                // header cells arrive without a wrapping TableRow frame
                Some(Frame::Row(row)) => row.push(cell),
                Some(Frame::Table { head, in_head, .. }) if *in_head => head.push(cell),
                _ => {}
            },
            (TagEnd::Strong, Some(Frame::Strong(children))) => {
                self.inline(Inline::Strong(children))
            }
            (TagEnd::Emphasis, Some(Frame::Emphasis(children))) => {
                self.inline(Inline::Emphasis(children))
            }
            (TagEnd::Strikethrough, Some(Frame::Strikethrough(children))) => {
                self.inline(Inline::Strikethrough(children));
            }
            (TagEnd::Link, Some(Frame::Link { text, url })) => {
                self.inline(Inline::Link { text, url })
            }
            (TagEnd::Image, Some(Frame::Image { text, url })) => {
                self.inline(Inline::Image {
                    alt: plain_text(&text),
                    url,
                });
            }
            _ => {}
        }
    }

    fn inline(&mut self, inline: Inline) {
        // find the nearest inline-capable frame
        match self.stack.iter_mut().rev().find(|f| is_inline_frame(f)) {
            Some(Frame::Paragraph(children)) => children.push(inline),
            Some(Frame::Heading { children, .. }) => children.push(inline),
            Some(Frame::Strong(children)) => children.push(inline),
            Some(Frame::Emphasis(children)) => children.push(inline),
            Some(Frame::Strikethrough(children)) => children.push(inline),
            Some(Frame::Link { text, .. }) => text.push(inline),
            Some(Frame::Image { text, .. }) => text.push(inline),
            Some(Frame::Cell(cell)) => cell.push(inline),
            _ => {
                // no inline frame: attach to the enclosing list item (task lists
                // emit bare text without a Paragraph tag), else start an implicit paragraph
                match self.stack.last_mut() {
                    Some(Frame::Item { pending, .. }) => {
                        if let Some(buf) = pending {
                            buf.push(inline);
                        } else {
                            *pending = Some(vec![inline]);
                        }
                    }
                    _ => self.stack.push(Frame::Paragraph(vec![inline])),
                }
            }
        }
    }

    fn html(&mut self, html: String) {
        if let Some(top) = self.stack.last_mut() {
            if is_inline_frame(top) {
                self.inline(Inline::Html(html));
                return;
            }
        }
        self.block(Block::Html(html));
    }

    /// Append a block to the nearest block-capable frame.
    fn block(&mut self, block: Block) {
        match self.stack.iter_mut().rev().find(|f| is_block_frame(f)) {
            Some(Frame::Root(children)) => children.push(block),
            Some(Frame::BlockQuote(children)) => children.push(block),
            Some(Frame::Item { children, .. }) => children.push(block),
            Some(Frame::Footnote { children, .. }) => children.push(block),
            _ => {
                // no block frame (stray block) — ignore
            }
        }
    }

    fn finish(mut self) -> Ast {
        while let Some(frame) = self.stack.pop() {
            if let Frame::Root(children) = frame {
                return Ast { children };
            }
        }
        Ast::default()
    }
}

fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(t) => out.push_str(t),
            Inline::Code(c) => out.push_str(c),
            Inline::Strong(c) | Inline::Emphasis(c) | Inline::Strikethrough(c) => {
                out.push_str(&plain_text(c))
            }
            Inline::Link { text, .. } => out.push_str(&plain_text(text)),
            Inline::SoftBreak | Inline::HardBreak => out.push('\n'),
            Inline::Math(m, _) => out.push_str(m),
            Inline::Html(h) => out.push_str(h),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::FootnoteRef(l) => out.push_str(l),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_and_inline() {
        let ast = parse_block("hello **world** and `code`");
        assert_eq!(ast.children.len(), 1);
        if let Block::Paragraph(inlines) = &ast.children[0] {
            assert_eq!(inlines.len(), 4);
            assert!(matches!(inlines[1], Inline::Strong(_)));
            assert!(matches!(inlines[3], Inline::Code(_)));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn heading_code_table() {
        let ast =
            parse_block("# Title\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |");
        assert_eq!(ast.children.len(), 3);
        assert!(matches!(ast.children[0], Block::Heading { level: 1, .. }));
        assert!(matches!(&ast.children[1], Block::Code { lang, .. } if lang == "rust"));
        assert!(matches!(&ast.children[2], Block::Table { .. }));
    }

    #[test]
    fn task_list() {
        let ast = parse_block("- [x] done\n- [ ] todo");
        if let Block::List { ordered, items, .. } = &ast.children[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].checked, Some(true));
            assert_eq!(items[1].checked, Some(false));
        } else {
            panic!("expected list");
        }
    }
}
