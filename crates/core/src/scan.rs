//! Byte-level scanning helpers ported from `markmend/core/src/preprocess/utils.ts`
//! and `pattern.ts`. All positions are byte offsets into the original string.

/// JS `\s` (without unicode flag): space, tab, newline, CR, FF, VT.
pub fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000c}' | '\u{000b}')
}

/// JS `String.prototype.trim`-ish (ASCII + unicode whitespace).
pub fn js_trim(s: &str) -> &str {
    let start = s
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let end = s
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(start);
    &s[start..end]
}

/// Trailing whitespace run (`/\s+$/`), returns byte offset of the run start.
pub fn trailing_ws_offset(s: &str) -> usize {
    let mut end = s.len();
    for (i, c) in s.char_indices().rev() {
        if is_ws(c) {
            end = i;
        } else {
            break;
        }
    }
    end
}

/// Count occurrences of an ASCII needle.
pub fn count_of(text: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut rest = text;
    while let Some(pos) = rest.find(needle) {
        count += 1;
        rest = &rest[pos + needle.len()..];
    }
    count
}

/// `/```[\s\S]*?```/g` — all closed fenced code ranges `[start, end)`.
pub fn find_closed_code_block_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut search = 0;
    loop {
        let Some(start) = content[search..].find("```") else {
            break;
        };
        let start = search + start;
        let after = start + 3;
        let Some(rel) = content[after..].find("```") else {
            break;
        };
        let end = after + rel + 3;
        ranges.push((start, end));
        search = end;
    }
    ranges
}

pub fn is_position_in_ranges(position: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|&(start, end)| position >= start && position < end)
}

/// Whether `[start, end)` overlaps any range.
pub fn is_range_overlapping_ranges(start: usize, end: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(rs, re)| {
        (start >= rs && start < re) || (end > rs && end <= re) || (start < rs && end > re)
    })
}

/// Toggle on each ` ``` ` while scanning up to `position`.
/// Byte-safe check: does the triple-backtick sequence start at `i`?
pub(crate) fn is_triple_backtick_at(bytes: &[u8], i: usize) -> bool {
    bytes[i] == b'`' && bytes.get(i + 1) == Some(&b'`') && bytes.get(i + 2) == Some(&b'`')
}

pub fn is_within_code_block(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    let mut in_code = false;
    let mut i = 0;
    while i < position && i < bytes.len() {
        if is_triple_backtick_at(bytes, i) {
            in_code = !in_code;
            i += 3;
        } else {
            i += 1;
        }
    }
    in_code
}

pub fn is_inside_unclosed_code_block(content: &str) -> bool {
    is_within_code_block(content, content.len())
}

/// Last paragraph = content after the last blank line.
/// Returns `(start_line_index, byte_offset_of_start)`.
pub fn last_paragraph_range(content: &str, skip_trailing_empty: bool) -> (usize, usize) {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut start_line = 0;
    for i in (0..lines.len()).rev() {
        if skip_trailing_empty && i == lines.len() - 1 && lines[i].trim().is_empty() {
            continue;
        }
        if lines[i].trim().is_empty() {
            start_line = i + 1;
            break;
        }
    }
    let offset = if start_line == 0 {
        0
    } else {
        (lines[..start_line].join("\n").len() + 1).min(content.len())
    };
    (start_line, offset)
}

pub fn last_paragraph(content: &str, skip_trailing_empty: bool) -> &str {
    let (_start_line, offset) = last_paragraph_range(content, skip_trailing_empty);
    &content[offset..]
}

/// Find last non-empty line index (JS `findLastNonEmptyLineIndex`).
pub fn last_non_empty_line_index(lines: &[&str]) -> isize {
    for (i, line) in lines.iter().enumerate().rev() {
        if !line.trim().is_empty() {
            return i as isize;
        }
    }
    -1
}

pub(crate) fn is_backtick_part_of_triple(text: &str, index: usize) -> bool {
    let mut back = text[..index].chars().rev();
    let before = back.next();
    let before2 = back.next();
    let after = text.get(index + 1..).and_then(|s| s.chars().next());
    let after2 = text.get(index + 2..).and_then(|s| s.chars().next());
    let c = |o: Option<char>| o == Some('`');
    (c(before) && c(before2)) || (c(before) && c(after)) || (c(after) && c(after2))
}

/// `/`[^`\n]+`/`-style inline code ranges `[start, end)` outside code blocks.
pub fn find_inline_code_ranges(
    content: &str,
    code_block_ranges: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let mut positions: Vec<usize> = Vec::new();
    let bytes = content.as_bytes();
    for i in 0..bytes.len() {
        if is_position_in_ranges(i, code_block_ranges) {
            continue;
        }
        if bytes[i] != b'`' {
            continue;
        }
        if is_backtick_part_of_triple(content, i) {
            continue;
        }
        positions.push(i);
    }
    let mut ranges = Vec::new();
    for pair in positions.chunks(2) {
        if pair.len() == 2 {
            ranges.push((pair[0], pair[1] + 1));
        }
    }
    ranges
}

/// Mask `* _ ~` inside inline code with spaces (offsets preserved).
pub fn mask_inline_code_markdown_markers(content: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return content.to_string();
    }
    let mut bytes = content.as_bytes().to_vec();
    for &(start, end) in ranges {
        let range = start..end.min(bytes.len());
        for b in &mut bytes[range] {
            if *b == b'*' || *b == b'_' || *b == b'~' {
                *b = b' ';
            }
        }
    }
    String::from_utf8(bytes).expect("masking preserves utf8")
}

/// Odd number of preceding backslashes.
pub fn is_escaped_character(content: &str, index: usize) -> bool {
    let mut backslashes = 0;
    let bytes = content.as_bytes();
    let mut i = index;
    while i > 0 && bytes[i - 1] == b'\\' {
        backslashes += 1;
        i -= 1;
    }
    backslashes % 2 == 1
}

fn is_word_char(c: char) -> bool {
    // ~ `[\p{L}\p{N}]` (combining marks approximated via alphanumeric)
    c.is_alphanumeric()
}

fn is_word_char_at(content: &str, index: usize) -> bool {
    // 调用方可能给非边界（marker 前一字节跨多字节字符）：向后对齐到字符边界。
    let mut i = index;
    while i < content.len() && !content.is_char_boundary(i) {
        i += 1;
    }
    content
        .get(i..)
        .and_then(|s| s.chars().next())
        .is_some_and(is_word_char)
}

pub fn is_underscore_inside_word(content: &str, start: usize, len: usize) -> bool {
    // start 是 marker 的字节偏移（边界）；前一个字符从边界往前取整字符，
    // 避免 start - 1 落在多字节字符中间（曾导致 char boundary panic）。
    let prev = content.get(..start).and_then(|s| s.chars().next_back());
    let next = content.get(start + len..).and_then(|s| s.chars().next());
    prev.is_some_and(is_word_char) && next.is_some_and(is_word_char)
}

pub fn should_ignore_underscore_marker(content: &str, start: usize, len: usize) -> bool {
    is_escaped_character(content, start) || is_underscore_inside_word(content, start, len)
}

/// Mask escaped / intraword underscore runs with spaces.
pub fn mask_invalid_underscore_markers(content: &str) -> String {
    let mut bytes = content.as_bytes().to_vec();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'_' {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < bytes.len() && bytes[index] == b'_' {
            index += 1;
        }
        let run_len = index - run_start;
        if should_ignore_underscore_marker(content, run_start, run_len) {
            for b in &mut bytes[run_start..index] {
                *b = b' ';
            }
        }
    }
    String::from_utf8(bytes).expect("masking preserves utf8")
}

/// `/!?\[[^\]]*\]\([^)]*\)/g` — replace each complete link/image with `[text]()`.
fn replace_complete_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let ch = text[i..].chars().next().expect("char at boundary");
        let rest = &text[i..];
        let is_image = rest.starts_with("![");
        if ch == '[' || (is_image && ch == '!') {
            let label_start = i + usize::from(is_image);
            if let Some(close_rel) = text[label_start..].find(']') {
                let after_label = label_start + close_rel + 1;
                if text[after_label..].starts_with('(') {
                    let paren_rest = &text[after_label + 1..];
                    if let Some(cp) = paren_rest.find(')') {
                        out.push_str(&text[i..after_label]);
                        out.push_str("()");
                        i = after_label + 1 + cp + 1;
                        continue;
                    }
                }
            }
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `/!?\[[^\]]*\]\([^)]*$/g` — replace an unclosed link at the end with `[text](`.
fn replace_incomplete_link_suffix(text: &str) -> String {
    let bytes = text.as_bytes();
    // find the LAST `](` such that everything after it is non-`)`
    let mut best: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' || (bytes[i] == b'!' && bytes.get(i + 1) == Some(&b'[')) {
            let label_start = if bytes[i] == b'!' { i + 1 } else { i };
            if let Some(rel) = text[label_start..].find(']') {
                let after = label_start + rel + 1;
                if text[after..].starts_with('(') {
                    let rest = &text[after + 1..];
                    if !rest.contains(')') && !rest.is_empty() {
                        best = Some(after - 1);
                        i = text.len();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    if let Some(pos) = best {
        let mut out = text[..pos].to_string();
        out.push_str("](");
        out
    } else {
        text.to_string()
    }
}

/// Remove code blocks, HTML tags, and link/image URL bodies from text,
/// so markdown markers inside URLs don't get counted.
pub fn remove_urls_from_text(text: &str) -> String {
    // 1. remove code blocks
    let mut result = String::with_capacity(text.len());
    let ranges = find_closed_code_block_ranges(text);
    let mut cursor = 0;
    for &(start, end) in &ranges {
        result.push_str(&text[cursor..start]);
        cursor = end;
    }
    result.push_str(&text[cursor..]);

    // 2. remove HTML tags `<[^>]*>`
    let mut no_html = String::with_capacity(result.len());
    let mut i = 0;
    while i < result.len() {
        let ch = result[i..].chars().next().expect("char at boundary");
        if ch == '<' {
            if let Some(rel) = result[i..].find('>') {
                no_html.push(' ');
                i += rel + 1;
                continue;
            }
        }
        no_html.push(ch);
        i += ch.len_utf8();
    }

    // 3. complete links -> `[text]()`
    let after_complete = replace_complete_links(&no_html);
    // 4. incomplete link suffix -> `[text](`
    replace_incomplete_link_suffix(&after_complete)
}

/// Remove `$$...$$` block math (and optionally `$...$` inline math) spans.
pub fn remove_math_blocks_from_text(text: &str, single_dollar_enabled: bool) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'$') {
            i += 2;
            continue;
        }
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'$') {
            // find closing `$$`
            let mut j = i + 2;
            let mut closing = None;
            while j + 1 < bytes.len() {
                if bytes[j] == b'$' && bytes[j + 1] == b'$' {
                    closing = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(pos) = closing {
                bytes.drain(i..pos + 2);
                continue; // same position re-checked
            } else {
                bytes.truncate(i);
                break;
            }
        }
        if single_dollar_enabled && bytes[i] == b'$' {
            let mut j = i + 1;
            let mut closing = None;
            while j < bytes.len() {
                if bytes[j] == b'$' && bytes.get(j - 1) != Some(&b'\\') {
                    closing = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(pos) = closing {
                bytes.drain(i..pos + 1);
                continue;
            } else {
                bytes.truncate(i);
                break;
            }
        }
        i += 1;
    }
    String::from_utf8(bytes).expect("math removal preserves utf8")
}

/// Whether `position` sits inside a `$$` math span (or `$` inline math when enabled).
pub fn is_within_math_block(text: &str, position: usize, single_dollar_enabled: bool) -> bool {
    let mut in_block = false;
    let mut in_inline = false;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < position && i < text.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'$') {
            i += 2;
            continue;
        }
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'$') {
            in_block = !in_block;
            i += 2;
            continue;
        }
        if single_dollar_enabled && !in_block && bytes[i] == b'$' {
            in_inline = !in_inline;
        }
        i += 1;
    }
    in_block || in_inline
}

/// Whether `position` is inside `](...)` of a link/image.
pub fn is_within_link_or_image_url(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    let mut i = position;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b')' => return false,
            b'(' if i > 0 && bytes[i - 1] == b']' => return true,
            b'(' => return false,
            b'\n' => return false,
            _ => {}
        }
    }
    false
}

/// Whether `position` is inside an unclosed HTML tag (`<...>`).
pub fn is_within_html_tag(text: &str, position: usize) -> bool {
    let mut in_tag = false;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < position && i < text.len() {
        if bytes[i] == b'<' && (i == 0 || bytes[i - 1] != b'\\') {
            in_tag = true;
        } else if bytes[i] == b'>' && in_tag && (i == 0 || bytes[i - 1] != b'\\') {
            in_tag = false;
        }
        i += 1;
    }
    in_tag
}

/// Append `suffix` before trailing whitespace.
pub fn append_before_trailing_whitespace(content: &str, suffix: &str) -> String {
    let ws = trailing_ws_offset(content);
    format!("{}{}{}", &content[..ws], suffix, &content[ws..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_code_ranges() {
        let s = "```js\ncode\n``` and `inline";
        let ranges = find_closed_code_block_ranges(s);
        assert_eq!(ranges, vec![(0, 14)]);
        assert!(is_position_in_ranges(5, &ranges));
        assert!(!is_position_in_ranges(20, &ranges));
    }

    #[test]
    fn underscore_after_multibyte_char_is_safe() {
        // 回归：`` `x`，_y_ `` 曾因 start-1 落在 '，'（3 字节）中间 panic。
        let content = "`x`，_y_";
        let code = find_closed_code_block_ranges(content);
        let inline = find_inline_code_ranges(content, &code);
        let masked = mask_inline_code_markdown_markers(content, &inline);
        let star_pos = masked.find('_').unwrap();
        assert!(!should_ignore_underscore_marker(content, star_pos, 1));
    }

    #[test]
    fn underscore_inside_cjk_word_boundaries() {
        // 中文相邻 `_x_` 处理不 panic（多字节边界安全）。
        let content = "中文_x_结尾";
        let star_pos = content.find('_').unwrap();
        let _ = should_ignore_underscore_marker(content, star_pos, 1);
        assert!(content.is_char_boundary(star_pos + 1));
    }

    #[test]
    fn inline_code_ranges_skip_triples() {
        let s = "```a``` text `b` and `c";
        let ranges = find_inline_code_ranges(s, &find_closed_code_block_ranges(s));
        // `b` at ... and `c` unmatched -> single
        assert_eq!(ranges.len(), 1);
        assert_eq!(&s[ranges[0].0..ranges[0].1], "`b`");
    }

    #[test]
    fn last_paragraph_split() {
        let (line, off) = last_paragraph_range("Para1\n\nPara2 text", false);
        assert_eq!(line, 2);
        assert_eq!(&"Para1\n\nPara2 text"[off..], "Para2 text");
        // skipTrailingEmpty=true skips only ONE trailing empty line; a double
        // trailing newline leaves an empty last paragraph (matches the JS)
        let (line2, off2) = last_paragraph_range("Para1 **bold**\n\n", true);
        assert_eq!(line2, 2);
        assert_eq!(off2, 16);
        let (line3, _) = last_paragraph_range("Para1\n\nPara2\n", true);
        assert_eq!(line3, 2);
        let (line4, _) = last_paragraph_range("**Contribution\n", true);
        assert_eq!(line4, 0);
    }

    #[test]
    fn url_removal_keeps_label() {
        let s = "[text](https://example.com/page_with_underscore) more";
        let r = remove_urls_from_text(s);
        assert_eq!(r, "[text]() more");
    }

    #[test]
    fn incomplete_url_suffix() {
        let s = "Visit [Google](https://www.goo";
        let r = remove_urls_from_text(s);
        assert_eq!(r, "Visit [Google](");
    }

    #[test]
    fn math_removal() {
        let s = "a $$x = 1$$ b $$y = 2$$ c";
        assert_eq!(remove_math_blocks_from_text(s, false), "a  b  c");
        let s2 = "unclosed $$x";
        assert_eq!(remove_math_blocks_from_text(s2, false), "unclosed ");
    }

    #[test]
    fn underscore_inside_word_ignored() {
        let s = "snake_case text";
        assert!(should_ignore_underscore_marker(s, 5, 1));
        let s2 = "snake case _";
        assert!(!should_ignore_underscore_marker(s2, 10, 1));
    }

    #[test]
    fn append_before_trailing() {
        assert_eq!(
            append_before_trailing_whitespace("text  ", "**"),
            "text**  "
        );
        assert_eq!(append_before_trailing_whitespace("text", "**"), "text**");
    }

    #[test]
    fn backtick_after_cjk_does_not_panic() {
        // index 落在多字节字符（中文标点）之后：`text[..index-1]` 会跨 char 边界。
        let s = "中文：`code` 后";
        let tick = s.find('`').unwrap();
        let _ = is_backtick_part_of_triple(s, tick);
        let s2 = "注解``code``";
        let tick = s2.find('`').unwrap();
        let _ = is_backtick_part_of_triple(s2, tick);
    }

    #[test]
    fn within_link_url() {
        assert!(is_within_link_or_image_url("[t](https://a", 9));
        // position before the closing paren is still inside the URL
        assert!(is_within_link_or_image_url("[t](https://a)", 9));
        assert!(is_within_link_or_image_url("[t](https://a) mid", 9));
        assert!(!is_within_link_or_image_url("[t](https://a)", 14));
        assert!(!is_within_link_or_image_url("plain text", 4));
    }
}
