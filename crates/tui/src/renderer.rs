//! TUI display-layer adapter for `rsmarkdown-core`.
//!
//! Renders the per-block AST into styled terminal lines. Per-block caching
//! mirrors the original's memoized `Block` components: only blocks whose source
//! text changed are re-rendered on each stream update.

pub mod math;
pub mod theme;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use rsmarkdown_core::{Alignment, Ast, Block, Document, Inline, ListItem, Renderer};

/// A display adapter: markdown AST -> styled terminal lines.
pub struct StreamMarkdownRenderer {
    width: usize,
    /// per-block cache: (source content, rendered lines)
    cached: Vec<Option<(String, Vec<Line<'static>>)>>,
    lines: Vec<Line<'static>>,
}

impl Default for StreamMarkdownRenderer {
    fn default() -> Self {
        Self::new(80)
    }
}

impl StreamMarkdownRenderer {
    /// Create a renderer for the given wrap width.
    pub fn new(width: usize) -> Self {
        Self {
            width,
            cached: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Change the wrap width (invalidates the per-block cache).
    pub fn set_width(&mut self, width: usize) {
        if self.width != width {
            self.width = width;
            self.cached.clear();
        }
    }

    /// The assembled output lines (after the last `render` call).
    pub fn lines(&self) -> &[Line<'static>] {
        &self.lines
    }

    /// Cached lines of a single block (index into the last rendered document).
    pub fn block_lines(&self, index: usize) -> Option<&[Line<'static>]> {
        self.cached
            .get(index)
            .map(|c| c.as_ref().map(|(_, l)| l.as_slice()))
            .flatten()
    }
}

impl Renderer for StreamMarkdownRenderer {
    fn render(&mut self, doc: &Document) {
        self.cached.resize(doc.blocks.len(), None);
        let mut out: Vec<Line<'static>> = Vec::new();
        for (index, block) in doc.blocks.iter().enumerate() {
            let cached = &mut self.cached[index];
            let rendered = match cached {
                Some((src, lines)) if src == &block.content => lines.clone(),
                _ => {
                    let lines = render_block(&block.ast, self.width);
                    *cached = Some((block.content.clone(), lines.clone()));
                    lines
                }
            };
            out.extend(rendered);
            if !out.last().is_none_or(|l| l.spans.is_empty()) {
                out.push(Line::default());
            }
        }
        self.lines = out;
    }
}

/// Render one block's AST into styled lines.
pub fn render_block(ast: &Option<Ast>, width: usize) -> Vec<Line<'static>> {
    let Some(ast) = ast else { return Vec::new() };
    let mut out = Vec::new();
    for block in &ast.children {
        render_into(block, &mut out, width, 0);
    }
    out
}

fn render_into(block: &Block, out: &mut Vec<Line<'static>>, width: usize, indent: usize) {
    match block {
        Block::Paragraph(inlines) => {
            let lines = render_inlines(inlines, width.saturating_sub(indent));
            push_indented(out, lines, indent);
        }
        Block::Heading { level, children } => {
            let lines = render_inlines(children, width.saturating_sub(indent));
            let lines: Vec<Line<'static>> = lines
                .into_iter()
                .map(|l| l.style(theme::heading(*level)))
                .collect();
            push_indented(out, lines, indent);
        }
        Block::Code { lang, text } => {
            let _ = lang;
            let body = render_code(text, width.saturating_sub(indent));
            push_indented(out, body, indent);
        }
        Block::BlockQuote(children) => {
            let mut inner = Vec::new();
            for child in children {
                render_into(child, &mut inner, width - 2, 0);
            }
            for line in inner {
                let mut styled = Vec::new();
                styled.push(Span::styled("▌ ", theme::quote_bar()));
                for span in line.spans {
                    let mut s = span.style;
                    s = s.patch(theme::quote());
                    styled.push(Span::styled(span.content, s));
                }
                out.push(Line::from(styled));
            }
        }
        Block::List {
            ordered,
            start,
            items,
        } => {
            render_list(items, *ordered, *start, out, width, indent);
        }
        Block::Table {
            headers,
            rows,
            aligns,
        } => {
            render_table(headers, rows, aligns, out, width, indent);
        }
        Block::ThematicBreak => {
            let mut spans = Vec::new();
            let n = width.saturating_sub(indent);
            spans.push(Span::styled("─".repeat(n), theme::hr()));
            out.push(Line::from(spans));
        }
        Block::Html(html) => {
            // render HTML as plain dim text lines
            for line in html.split('\n') {
                let t = line.trim_end();
                if !t.is_empty() {
                    out.push(Line::styled(t.to_string(), theme::dim()));
                }
            }
        }
        Block::FootnoteDefinition { label, children } => {
            let mut inner = Vec::new();
            for child in children {
                render_into(child, &mut inner, width - 2, 0);
            }
            if let Some(first) = inner.first_mut() {
                first
                    .spans
                    .insert(0, Span::styled(format!("[^{}] ", label), theme::footnote()));
            }
            for line in inner {
                let mut styled = Vec::new();
                styled.push(Span::styled("  ", theme::footnote()));
                for span in line.spans {
                    let mut s = span.style;
                    s = s.patch(theme::footnote());
                    styled.push(Span::styled(span.content, s));
                }
                out.push(Line::from(styled));
            }
        }
    }
}

fn push_indented(out: &mut Vec<Line<'static>>, lines: Vec<Line<'static>>, indent: usize) {
    let pad = " ".repeat(indent);
    for line in lines {
        let mut spans = Vec::with_capacity(line.spans.len() + 1);
        if indent > 0 {
            spans.push(Span::raw(pad.clone()));
        }
        spans.extend(line.spans);
        out.push(Line::from(spans));
    }
}

fn render_code(text: &str, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            out.push(Line::default());
            continue;
        }
        let line = truncate(line, width);
        out.push(Line::styled(format!("  {}", line), theme::code_block()));
    }
    out
}

fn render_list(
    items: &[ListItem],
    ordered: bool,
    start: u64,
    out: &mut Vec<Line<'static>>,
    width: usize,
    indent: usize,
) {
    let any_task = items.iter().any(|i| i.checked.is_some());
    let marker_w = if ordered {
        let max_num = start + items.len() as u64 - 1;
        max_num.to_string().len() + 2
    } else if any_task {
        4
    } else {
        2
    };
    for (i, item) in items.iter().enumerate() {
        let marker = if let Some(checked) = item.checked {
            if checked {
                Span::styled("[x]", theme::task_done())
            } else {
                Span::styled("[ ]", theme::task_open())
            }
        } else if ordered {
            Span::styled(format!("{}.", start + i as u64), theme::list_marker())
        } else {
            Span::styled("•", theme::list_marker())
        };
        let marker_w_used = marker.width();

        let mut inner = Vec::new();
        for child in &item.children {
            render_into(
                child,
                &mut inner,
                width.saturating_sub(indent + marker_w),
                0,
            );
        }
        if inner.is_empty() {
            out.push(Line::from(vec![Span::raw(" ".repeat(indent)), marker]));
            continue;
        }
        let mut spans = Vec::new();
        spans.push(Span::raw(" ".repeat(indent)));
        spans.push(marker);
        spans.push(Span::raw(
            " ".repeat(marker_w.saturating_sub(marker_w_used)),
        ));
        let first = inner.remove(0);
        spans.extend(first.spans);
        out.push(Line::from(spans));
        for line in inner {
            let mut spans = Vec::new();
            spans.push(Span::raw(" ".repeat(indent + marker_w)));
            spans.extend(line.spans);
            out.push(Line::from(spans));
        }
    }
}

fn render_table(
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    aligns: &[Alignment],
    out: &mut Vec<Line<'static>>,
    width: usize,
    indent: usize,
) {
    let available = width.saturating_sub(indent + 2);
    let mut cols = aligns.len().max(headers.len());
    for row in rows {
        cols = cols.max(row.len());
    }
    if cols == 0 {
        return;
    }
    // plain-text width per cell
    let mut widths = vec![0usize; cols];
    for (c, cell) in headers.iter().enumerate() {
        widths[c] = plain_width(cell);
    }
    for row in rows {
        for (c, cell) in row.iter().enumerate() {
            widths[c] = widths[c].max(plain_width(cell));
        }
    }
    // fit into available width
    let total: usize = widths.iter().sum::<usize>() + (cols.saturating_sub(1) * 3);
    if total > available {
        shrink_columns(&mut widths, available - (cols.saturating_sub(1) * 3));
    }

    let sep = |w: &[usize]| {
        let mut spans = vec![Span::styled("│", theme::table_border())];
        for (i, cw) in w.iter().enumerate() {
            spans.push(Span::styled("─".repeat(*cw + 2), theme::table_border()));
            spans.push(Span::styled(
                if i + 1 == w.len() { "│" } else { "┼" },
                theme::table_border(),
            ));
        }
        spans
    };

    fn push_row(
        out: &mut Vec<Line<'static>>,
        widths: &[usize],
        cells: &[Vec<Inline>],
        style: Style,
    ) {
        let mut spans = vec![Span::styled("│", theme::table_border())];
        for (c, w) in widths.iter().enumerate() {
            let cell = cells.get(c);
            let text = cell.map(|c| plain_text(c)).unwrap_or_default();
            spans.push(Span::raw(" "));
            spans.push(Span::styled(truncate(&text, *w), style));
            let pad = w.saturating_sub(text.width()) + 1;
            spans.push(Span::raw(" ".repeat(pad)));
            spans.push(Span::styled("│", theme::table_border()));
        }
        out.push(Line::from(spans));
    }

    push_row(out, &widths, headers, theme::table_header());
    out.push(Line::from(sep(&widths)));
    for row in rows {
        push_row(out, &widths, row, theme::text());
    }
    let _ = aligns;
}

fn shrink_columns(widths: &mut [usize], budget: usize) {
    let total: usize = widths.iter().sum();
    if total <= budget {
        return;
    }
    let n = widths.len();
    let per = total.saturating_sub(budget).div_ceil(n).max(4);
    for w in widths.iter_mut() {
        if *w > per {
            *w -= per;
        }
    }
    let remaining = budget.min(widths.iter().sum());
    // hard cap
    let mut sum: usize = widths.iter().sum();
    while sum > remaining {
        let mut max_i = 0;
        for i in 0..n {
            if widths[i] > widths[max_i] {
                max_i = i;
            }
        }
        widths[max_i] = widths[max_i].saturating_sub(1);
        sum -= 1;
    }
    for w in widths.iter_mut() {
        *w = (*w).max(1);
    }
}

// ---------------------------------------------------------------------------
// inline rendering + wrapping
// ---------------------------------------------------------------------------

/// Render inline content to wrapped styled lines.
pub fn render_inlines(inlines: &[Inline], width: usize) -> Vec<Line<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    collect_inlines(inlines, theme::text(), &mut spans);
    wrap_spans(spans, width)
}

fn collect_inlines(inlines: &[Inline], style: Style, out: &mut Vec<Span<'static>>) {
    for inline in inlines {
        match inline {
            Inline::Text(t) => out.push(Span::styled(t.clone(), style)),
            Inline::SoftBreak => out.push(Span::raw(" ")),
            Inline::HardBreak => out.push(Span::raw(" ")),
            Inline::Code(c) => out.push(Span::styled(c.clone(), theme::code())),
            Inline::Strong(children) => {
                collect_inlines(children, style.patch(theme::strong()), out)
            }
            Inline::Emphasis(children) => {
                collect_inlines(children, style.patch(theme::emphasis()), out)
            }
            Inline::Strikethrough(children) => {
                collect_inlines(children, style.patch(theme::strikethrough()), out)
            }
            Inline::Link { text, url } => {
                collect_inlines(text, style.patch(theme::link()), out);
                out.push(Span::styled(
                    format!("({})", url),
                    style.patch(theme::dim()),
                ));
            }
            Inline::Image { alt, .. } => {
                out.push(Span::styled(
                    format!("[image: {}]", alt),
                    style.patch(theme::link()),
                ));
            }
            Inline::Math(m, _display) => {
                let content = math::latex_to_unicode(m);
                out.push(Span::styled(content, style.patch(theme::math())));
            }
            Inline::Html(h) => out.push(Span::styled(h.clone(), style.patch(theme::dim()))),
            Inline::FootnoteRef(l) => {
                out.push(Span::styled(
                    format!("[^{}]", l),
                    style.patch(theme::footnote()),
                ));
            }
        }
    }
}

/// Wrap styled spans to `width` columns (CJK-aware), collapsing leading spaces.
fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut col = 0usize;
    let mut leading = true;
    for span in spans {
        let style = span.style;
        let text = span.content;
        for ch in text.chars() {
            let w = ch.width().unwrap_or(0);
            if ch == '\n' {
                push_line(&mut lines, &mut current);
                col = 0;
                leading = true;
                continue;
            }
            if col + w > width {
                push_line(&mut lines, &mut current);
                col = 0;
                leading = true;
            }
            if ch.is_whitespace() && leading {
                continue;
            }
            let mut owned = String::with_capacity(ch.len_utf8());
            owned.push(ch);
            current.push(Span::styled(owned, style));
            col += w;
            if !ch.is_whitespace() {
                leading = false;
            }
        }
    }
    push_line(&mut lines, &mut current);
    lines
}

/// Push `current` onto `lines`, dropping any trailing whitespace spans.
fn push_line(lines: &mut Vec<Line<'static>>, current: &mut Vec<Span<'static>>) {
    while let Some(span) = current.last() {
        if span.content.trim().is_empty() {
            current.pop();
        } else {
            break;
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(std::mem::take(current)));
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Display-width helpers for strings (CJK-aware).
pub trait WidthExt {
    /// Terminal display width in columns.
    fn width(&self) -> usize;
}

impl WidthExt for str {
    fn width(&self) -> usize {
        unicode_width::UnicodeWidthStr::width(self)
    }
}

/// Flatten inline AST to plain text.
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(t) => out.push_str(t),
            Inline::Code(c) => out.push_str(c),
            Inline::Strong(c) | Inline::Emphasis(c) | Inline::Strikethrough(c) => {
                out.push_str(&plain_text(c))
            }
            Inline::Link { text, .. } => out.push_str(&plain_text(text)),
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::Math(m, _) => out.push_str(m),
            Inline::Html(h) => out.push_str(h),
            Inline::FootnoteRef(l) => out.push_str(l),
        }
    }
    out
}

/// Display width of an inline run.
pub fn plain_width(inlines: &[Inline]) -> usize {
    plain_text(inlines).width()
}

/// Truncate a string to `width` display columns (CJK-aware).
pub fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut col = 0usize;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if col + w > width.saturating_sub(1) {
            break;
        }
        out.push(ch);
        col += w;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests;
