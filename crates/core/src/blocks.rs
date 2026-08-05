//! Block segmentation, ported from `markmend/core/src/preprocess/vendored/parse-blocks.ts`
//! (itself ported from vercel/streamdown). Splits markdown into stable blocks so only
//! the trailing block needs re-parsing as content streams in.

use crate::fix::{has_footnote_definition, has_footnote_reference};
use crate::scan::{is_ws, js_trim};

/// Does the (trimmed) line start a standalone block boundary that marked's Lexer
/// would split into its own token?
fn starts_block(line: &str) -> bool {
    let t = line.trim_start_matches(is_ws);
    let mut chars = t.chars();
    match chars.next() {
        Some('#') => {
            // ATX heading: `#{1,6} ` or bare heading-like
            let count = t.chars().take_while(|c| *c == '#').count();
            (1..=6).contains(&count)
                && t[count..]
                    .chars()
                    .next()
                    .is_some_and(|c| c == ' ' || is_ws(c))
        }
        Some('>') => true,             // blockquote
        Some('<' | '!' | '?') => true, // html / comments
        Some('`') => true,             // fence handled separately
        _ => false,
    }
}

/// Tokenize markdown into "tokens" the way marked's `Lexer.lex` does at the block
/// level: blank-line-separated runs, with fenced code blocks kept whole.
fn lex_tokens(markdown: &str) -> Vec<&str> {
    let mut tokens: Vec<&str> = Vec::new();
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut token_start: Option<usize> = None; // byte offset of current token
    let mut token_line_start = 0usize; // index of first line of current token
    let mut in_fence = false;

    fn flush<'a>(
        tokens: &mut Vec<&'a str>,
        token_start: &mut Option<usize>,
        markdown: &'a str,
        start_byte: usize,
        end_byte: usize,
    ) {
        if let Some(start) = *token_start {
            tokens.push(&markdown[start..end_byte]);
        }
        *token_start = if start_byte < end_byte {
            Some(start_byte)
        } else {
            None
        };
    }

    let mut byte_offset = 0usize;
    for (line_idx, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for '\n'
        let is_blank = line.trim().is_empty();

        if in_fence {
            // inside a fence: token continues; fence closes on a ``` line
            if js_trim(line).starts_with("```") {
                in_fence = false;
            }
            byte_offset += line_len;
            continue;
        }

        if is_blank {
            flush(
                &mut tokens,
                &mut token_start,
                markdown,
                byte_offset,
                byte_offset,
            );
            token_line_start = line_idx + 1;
            byte_offset += line_len;
            continue;
        }

        let starts_fence = js_trim(line).starts_with("```");
        if starts_fence {
            flush(
                &mut tokens,
                &mut token_start,
                markdown,
                byte_offset,
                byte_offset,
            );
            token_line_start = line_idx;
            token_start = Some(byte_offset);
            in_fence = true;
            byte_offset += line_len;
            continue;
        }

        // block-boundary lines split the current run
        let is_boundary = starts_block(line);
        if is_boundary && token_start.is_some() && line_idx > token_line_start {
            flush(
                &mut tokens,
                &mut token_start,
                markdown,
                byte_offset,
                byte_offset,
            );
            token_line_start = line_idx;
            token_start = Some(byte_offset);
            byte_offset += line_len;
            continue;
        }

        if token_start.is_none() {
            token_start = Some(byte_offset);
            token_line_start = line_idx;
        }
        byte_offset += line_len;
    }

    if let Some(start) = token_start {
        tokens.push(&markdown[start..]);
    }
    tokens
}

fn starts_with_double_dollar(str: &str) -> bool {
    let t = str.trim_start_matches(is_ws);
    t.starts_with("$$")
}

fn ends_with_double_dollar(str: &str) -> bool {
    let t = str.trim_end_matches(is_ws);
    t.ends_with("$$")
}

fn count_double_dollars(str: &str) -> usize {
    let bytes = str.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'$' {
            count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    count
}

/// Split markdown into logical blocks. Footnotes collapse the whole document into
/// a single block; unclosed HTML tags and unclosed `$$` math merge across blocks.
pub fn parse_markdown_into_blocks(markdown: &str) -> Vec<String> {
    if has_footnote_reference(markdown) || has_footnote_definition(markdown) {
        return vec![markdown.to_string()];
    }

    let tokens = lex_tokens(markdown);

    let mut merged: Vec<String> = Vec::new();
    let mut html_stack: Vec<String> = Vec::new();

    for token in tokens {
        let current = token;
        let merged_len = merged.len();

        // inside an unclosed HTML block — merge with previous
        if !html_stack.is_empty() {
            merged[merged_len - 1].push('\n');
            merged[merged_len - 1].push_str(current);
            if let Some(closing) = memchr::memmem::find(current.as_bytes(), b"</") {
                if let Some(rest) = current[closing + 2..].split_whitespace().next() {
                    let tag = rest.trim_end_matches(['>', '/', '\n']).to_string();
                    if let Some(top) = html_stack.last() {
                        if *top == tag {
                            html_stack.pop();
                        }
                    }
                }
            }
            continue;
        }

        // opening HTML block tag without closing in the same token
        // (mirrors `openingTagPattern = /<(\w+)[\s>]/` on html tokens)
        if current.trim_start_matches(is_ws).starts_with('<') {
            let after = current.trim_start_matches(is_ws);
            let after = &after[1..];
            let tag: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let next_after_tag = after[tag.len()..].chars().next();
            let ok_boundary = next_after_tag == Some('>') || next_after_tag.is_some_and(is_ws);
            if !tag.is_empty() && ok_boundary {
                let has_closing = current.contains(&format!("</{}>", tag));
                if !has_closing {
                    html_stack.push(tag);
                }
            }
        }

        let trimmed = current.trim();

        // standalone `$$` closing a previous unclosed math block
        if trimmed == "$$" && merged_len > 0 {
            let previous = &merged[merged_len - 1];
            if starts_with_double_dollar(previous) && count_double_dollars(previous) % 2 == 1 {
                merged[merged_len - 1] = format!("{}{}", previous, current);
                continue;
            }
        }

        // current block ends with `$$` and continues an unclosed math block
        if merged_len > 0 && ends_with_double_dollar(current) {
            let previous = &merged[merged_len - 1];
            let prev_dollar_count = count_double_dollars(previous);
            let curr_dollar_count = count_double_dollars(current);
            if starts_with_double_dollar(previous)
                && prev_dollar_count % 2 == 1
                && !starts_with_double_dollar(current)
                && curr_dollar_count == 1
            {
                merged[merged_len - 1] = format!("{}{}", previous, current);
                continue;
            }
        }

        merged.push(current.to_string());
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_blank_lines() {
        let blocks = parse_markdown_into_blocks("# Title\n\nParagraph one\n\nParagraph two");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], "# Title\n");
        assert_eq!(blocks[1], "Paragraph one\n");
        assert_eq!(blocks[2], "Paragraph two");
    }

    #[test]
    fn code_fence_is_one_block() {
        let blocks = parse_markdown_into_blocks("before\n\n```js\ncode\nmore\n```\n\nafter");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1], "```js\ncode\nmore\n```\n");
    }

    #[test]
    fn unclosed_fence_extends_to_end() {
        let blocks = parse_markdown_into_blocks("before\n\n```js\ncode\nmore");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1], "```js\ncode\nmore");
    }

    #[test]
    fn unclosed_html_merges() {
        let blocks = parse_markdown_into_blocks("<div>\n\ncontent\n\n</div>\n\nafter");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], "<div>\n\ncontent\n\n</div>\n");
    }

    #[test]
    fn math_merge() {
        let blocks = parse_markdown_into_blocks("text\n\n$$\nE = mc^2\n$$\n\nafter");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1], "$$\nE = mc^2\n$$\n");
    }

    #[test]
    fn footnote_collapses_document() {
        let blocks = parse_markdown_into_blocks("Text [^1]\n\n[^1]: note");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn list_items_group() {
        let blocks = parse_markdown_into_blocks("- a\n- b\n- c\n\nafter");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], "- a\n- b\n- c\n");
    }
}
