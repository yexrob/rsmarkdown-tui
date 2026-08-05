//! Streaming preprocessors ("fix" functions), ported one-to-one from
//! `markmend/core/src/preprocess/*.ts`. Each one makes incomplete markdown
//! syntax parseable *as it streams*, so the AST stays stable.

use crate::pattern::*;
use crate::preprocess::PreprocessOptions;
use crate::scan::*;

/// Find the last backtick run at the end (`/(`+)\s*$/`), returns (run_start, run_len) with
/// any trailing whitespace included in `run_len`.
fn trailing_backtick_run(content: &str) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut end = bytes.len();
    // skip trailing whitespace
    while end > 0 && is_ws(content[..end].chars().next_back().unwrap()) {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && bytes[start - 1] == b'`' {
        start -= 1;
    }
    if end > start {
        Some((start, end - start))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// fixCode
// ---------------------------------------------------------------------------

pub fn fix_code(content: &str) -> String {
    let is_inside = is_inside_unclosed_code_block(content);

    let (cleaned, was_cleaned) = remove_trailing_incomplete_backticks(content);

    if is_inside && was_cleaned {
        fix_code_block(&cleaned)
    } else if !was_cleaned {
        let after_block = fix_code_block(&cleaned);
        fix_inline_code(&after_block)
    } else {
        cleaned
    }
}

fn remove_trailing_incomplete_backticks(content: &str) -> (String, bool) {
    let Some((run_start, run_len)) = trailing_backtick_run(content) else {
        return (content.to_string(), false);
    };
    let seq = &content[run_start..run_start + run_len];
    let seq_len = seq.chars().count();
    let before = &content[..run_start];
    let after = &content[run_start + run_len..];

    if seq_len == 1 {
        // Count backticks in the last paragraph before this one
        let last_para = last_paragraph(before, false);
        let without_code = strip_closed_code_blocks(last_para);
        let count = count_of(&without_code, "`");
        let in_code = is_within_code_block(before, before.len());
        if count % 2 == 1 && !in_code {
            // This ` closes inline code — keep it
            return (content.to_string(), false);
        }
        let trimmed = before.trim_end_matches(is_ws);
        let mut out = trimmed.to_string();
        out.push_str(after);
        (out, true)
    } else if seq_len == 2 {
        let trimmed = before.trim_end_matches(is_ws);
        let mut out = trimmed.to_string();
        out.push_str(after);
        (out, true)
    } else if seq_len == 3 {
        let in_code = is_within_code_block(before, before.len());
        if in_code {
            (content.to_string(), false)
        } else {
            let trimmed = before.trim_end_matches(is_ws);
            let mut out = trimmed.to_string();
            out.push_str(after);
            (out, true)
        }
    } else {
        let trimmed = before.trim_end_matches(is_ws);
        let mut out = trimmed.to_string();
        out.push_str(after);
        (out, true)
    }
}

/// `/```[\s\S]*?```/g` — remove closed code blocks from text.
pub fn strip_closed_code_blocks(text: &str) -> String {
    let ranges = find_closed_code_block_ranges(text);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for &(start, end) in &ranges {
        out.push_str(&text[cursor..start]);
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn fix_code_block(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        if let Some(rel) = memchr::memmem::rfind(content.as_bytes(), b"```") {
            let after_fence = &content[rel + 3..];
            let has_newline = after_fence.contains('\n');
            let first_line = after_fence.split('\n').next().unwrap_or("");
            let has_language = !first_line.trim().is_empty();
            if has_language || has_newline {
                if content.ends_with('\n') {
                    return format!("{}```", content);
                }
                return format!("{}\n```", content);
            }
        }
    }
    content.to_string()
}

fn fix_inline_code(content: &str) -> String {
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];
    let without_code = strip_closed_code_blocks(last_para);
    let count = count_of(&without_code, "`");

    if count % 2 == 1 {
        // Find last standalone backtick (not part of ```)
        let bytes = last_para.as_bytes();
        let mut last_pos: isize = -1;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'`' {
                i += 1;
                continue;
            }
            if crate::scan::is_triple_backtick_at(last_para.as_bytes(), i) {
                if let Some(close_rel) = memchr::memmem::find(&last_para.as_bytes()[i + 3..], b"```") {
                    i += 3 + close_rel + 3 - 1;
                    continue;
                }
            }
            if !is_backtick_part_of_triple(last_para, i) {
                last_pos = i as isize;
            }
            i += 1;
        }
        if last_pos >= 0 {
            let actual = offset + last_pos as usize;
            let after_last = content[actual + 1..].trim();
            if !after_last.is_empty() {
                return format!("{}`", content);
            }
        }
    }
    let _ = start_line;
    content.to_string()
}

// ---------------------------------------------------------------------------
// fixHtml
// ---------------------------------------------------------------------------

fn is_unclosed_html_fragment(fragment: &str) -> bool {
    if !fragment.starts_with('<') || fragment.contains('>') {
        return false;
    }
    if fragment == "<" {
        return true;
    }
    if fragment.chars().count() <= 1 {
        return false;
    }
    let rest = &fragment[1..];
    // `^<!--[\s\S]*$` / `^<\?[\s\S]*$`
    if rest.starts_with("<!--") || rest.starts_with("<?") {
        return true;
    }
    // `^<![A-Z][^>]*$/i`
    if rest.starts_with('!') {
        let after = rest[1..].trim_start_matches(is_ws);
        return after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic());
    }
    let closing = rest.starts_with('/');
    let after = rest.trim_start_matches(|c| c == '/' || is_ws(c));
    let Some(first) = after.chars().next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    // `[\w-]*`
    let tail = &after[first.len_utf8()..];
    let word_end = tail
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        .map(|(i, _)| i)
        .unwrap_or(tail.len());
    let after_word = &tail[word_end..];
    if closing {
        // `^<\/\s*[A-Z][\w-]*\s*$/i`
        after_word.chars().all(is_ws)
    } else {
        // `^<\s*[A-Z][\w-]*(?:\s[^<>]*)?$/i`
        let trimmed = after_word.trim_start_matches(is_ws);
        if trimmed.is_empty() {
            return true;
        }
        let c = trimmed.chars().next().unwrap();
        c != '<' && c != '>'
    }
}

pub fn fix_html(content: &str) -> String {
    if content.is_empty() || is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let ws_offset = trailing_ws_offset(content);
    if ws_offset == 0 {
        return content.to_string();
    }
    let visible = &content[..ws_offset];
    let Some(fragment_start) = memchr::memrchr(b'<', visible.as_bytes()) else {
        return content.to_string();
    };
    if fragment_start > 0 && content.as_bytes()[fragment_start - 1] == b'\\' {
        return content.to_string();
    }
    let fragment = &visible[fragment_start..];
    if !is_unclosed_html_fragment(fragment) {
        return content.to_string();
    }
    let code_ranges = find_closed_code_block_ranges(content);
    if is_position_in_ranges(fragment_start, &code_ranges) {
        return content.to_string();
    }
    let inline_ranges = find_inline_code_ranges(content, &code_ranges);
    if is_position_in_ranges(fragment_start, &inline_ranges) {
        return content.to_string();
    }
    let before = content[..fragment_start].trim_end_matches([' ', '\t']);
    let trailing = &content[ws_offset..];
    format!("{}{}", before, trailing)
}

// ---------------------------------------------------------------------------
// fixFootnote
// ---------------------------------------------------------------------------

/// `/\[\^[^\]\s]{1,200}\]/` reference (not a definition).
pub fn has_footnote_reference(content: &str) -> bool {
    !find_footnote_refs(content).is_empty()
}

pub fn has_footnote_definition(content: &str) -> bool {
    !fn_def_ranges(content).is_empty()
}

fn fn_def_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        if bytes.get(i + 1) != Some(&b'^') {
            i += 1;
            continue;
        }
        // label: `[^` + chars (no `]`/ws, 1..=200) + `]:`
        let mut j = i + 2;
        let mut label_len = 0;
        while j < bytes.len() && label_len < 200 {
            let c = bytes[j];
            if c == b']' {
                break;
            }
            if is_ws(c as char) {
                label_len = 201;
                break;
            }
            j += 1;
            label_len += 1;
        }
        if (1..=200).contains(&label_len)
            && bytes.get(j) == Some(&b']')
            && bytes.get(j + 1) == Some(&b':')
        {
            ranges.push((i, j + 2));
            i = j + 2;
        } else {
            i += 1;
        }
    }
    ranges
}

/// `/\[\^[^\]\s]{1,200}\](?!:)/` — footnote references not followed by `:`.
pub fn find_footnote_refs(content: &str) -> Vec<(usize, usize, String)> {
    let mut refs = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' || bytes.get(i + 1) != Some(&b'^') {
            i += 1;
            continue;
        }
        let mut j = i + 2;
        let mut label_len = 0;
        let mut label = String::new();
        while j < bytes.len() && label_len < 200 {
            let c = bytes[j];
            if c == b']' {
                break;
            }
            if is_ws(c as char) {
                label_len = 201;
                break;
            }
            label.push(c as char);
            j += 1;
            label_len += 1;
        }
        if (1..=200).contains(&label_len) && bytes.get(j) == Some(&b']') {
            // `(?!:)`
            if bytes.get(j + 1) != Some(&b':') {
                refs.push((i, j + 1, label));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    refs
}

fn get_defined_footnote_labels(content: &str) -> std::collections::HashSet<String> {
    let without_code = strip_closed_code_blocks(content);
    fn_def_ranges(&without_code)
        .iter()
        .map(|&(start, _)| {
            let inner = &without_code[start + 2..];
            let end = memchr::memchr(b']', inner.as_bytes()).unwrap_or(0);
            inner[..end].to_string()
        })
        .collect()
}

fn remove_incomplete_ref_in_last_paragraph(content: &str) -> String {
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];
    // incomplete ref: `\[\^[^\]]*$`
    if !last_para.contains("[^") {
        return content.to_string();
    }
    // find last `[^` with no `]` after it
    let bytes = last_para.as_bytes();
    let mut incomplete_pos = -1isize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' && bytes.get(i + 1) == Some(&b'^') {
            let rest = &last_para[i + 2..];
            if !rest.contains(']') {
                incomplete_pos = i as isize;
            }
        }
        i += 1;
    }
    if incomplete_pos < 0 {
        return content.to_string();
    }
    let incomplete_pos = incomplete_pos as usize;
    let abs = absolute_position(offset, start_line, incomplete_pos, content);
    let code_ranges = find_closed_code_block_ranges(content);
    let inline_ranges = find_inline_code_ranges(content, &code_ranges);
    if is_position_in_ranges(abs, &code_ranges) || is_position_in_ranges(abs, &inline_ranges) {
        return content.to_string();
    }
    let line_end = memchr::memchr(b'\n', &last_para.as_bytes()[incomplete_pos..])
        .map(|p| incomplete_pos + p)
        .unwrap_or(last_para.len());
    let mut ref_start = incomplete_pos;
    if ref_start > 0 && last_para.as_bytes()[ref_start - 1] == b' ' {
        ref_start -= 1;
    }
    let abs_start = absolute_position(offset, start_line, ref_start, content);
    let abs_end = absolute_position(offset, start_line, line_end, content);
    let mut out = content[..abs_start].to_string();
    out.push_str(&content[abs_end..]);
    out
}

fn absolute_position(
    paragraph_offset: usize,
    _start_line: usize,
    relative: usize,
    _content: &str,
) -> usize {
    paragraph_offset + relative
}

fn collect_complete_references(
    content: &str,
    code_ranges: &[(usize, usize)],
    inline_ranges: &[(usize, usize)],
    def_ranges: &[(usize, usize)],
) -> Vec<(usize, usize, String)> {
    find_footnote_refs(content)
        .into_iter()
        .filter(|&(start, _, _)| {
            !is_position_in_ranges(start, code_ranges)
                && !is_position_in_ranges(start, inline_ranges)
                && !is_position_in_ranges(start, def_ranges)
        })
        .collect()
}

pub fn fix_footnote(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let defined = get_defined_footnote_labels(content);
    let mut result = remove_incomplete_ref_in_last_paragraph(content);

    let code_ranges = find_closed_code_block_ranges(&result);
    let inline_ranges = find_inline_code_ranges(&result, &code_ranges);
    let def_ranges = fn_def_ranges(&result);
    let references =
        collect_complete_references(&result, &code_ranges, &inline_ranges, &def_ranges);
    if references.is_empty() {
        return result;
    }
    for (start, end, label) in references.iter().rev() {
        if defined.contains(label) {
            continue;
        }
        let mut ref_start = *start;
        if ref_start > 0 && result.as_bytes()[ref_start - 1] == b' ' {
            ref_start -= 1;
        }
        let mut out = result[..ref_start].to_string();
        out.push_str(&result[*end..]);
        result = out;
    }
    result
}

// ---------------------------------------------------------------------------
// fixTaskList
// ---------------------------------------------------------------------------

pub fn fix_task_list(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let code_ranges = find_closed_code_block_ranges(content);
    let lines: Vec<&str> = content.split('\n').collect();
    let Some(&last_line) = lines.last() else {
        return content.to_string();
    };
    // position of last line in content
    let last_line_start: usize = lines[..lines.len() - 1].iter().map(|l| l.len() + 1).sum();
    let last_line_end = last_line_start + last_line.len();
    if is_range_overlapping_ranges(last_line_start, last_line_end, &code_ranges) {
        return content.to_string();
    }

    let drop_last = is_quote_incomplete_task_list(last_line)
        || (is_quote_standalone_dash(last_line) && !is_quote_task_list(last_line))
        || is_incomplete_task_list(last_line)
        || (is_standalone_dash(last_line) && !is_task_list(last_line))
        || (is_dash_with_space(last_line) && !is_task_list(last_line));

    if drop_last {
        lines[..lines.len() - 1].join("\n")
    } else {
        content.to_string()
    }
}

// ---------------------------------------------------------------------------
// fixLink
// ---------------------------------------------------------------------------

pub fn fix_link(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let lines: Vec<&str> = content.split('\n').collect();
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];
    let without_code = strip_closed_code_blocks(last_para);

    // 1. trailing standalone `[` / `![` on the last non-empty line
    let last_non_empty = last_non_empty_line_index(&lines);
    if last_non_empty >= 0 {
        let last_line = lines[last_non_empty as usize];
        let t = js_trim(last_line);
        let bracket = if t.ends_with('[') {
            Some("[")
        } else if t.ends_with("![") {
            Some("![")
        } else {
            None
        };
        if let Some(b) = bracket {
            let pos = memchr::memmem::rfind(last_line.as_bytes(), b.as_bytes()).unwrap();
            let before = last_line[..pos].trim_end_matches(is_ws).to_string();
            let mut new_lines: Vec<&str> = lines.clone();
            new_lines[last_non_empty as usize] = &before;
            // drop next line if empty
            if last_non_empty as usize + 1 < new_lines.len()
                && new_lines[last_non_empty as usize + 1].trim().is_empty()
            {
                new_lines.remove(last_non_empty as usize + 1);
            }
            return new_lines.join("\n");
        }
    }

    // 2. `[text` or `![text` — no closing bracket
    if has_incomplete_bracket(&without_code) {
        return format!("{}]()", content);
    }
    // 3. `[text]` / `![alt]` — missing URL
    if ends_with_incomplete_link_text(&without_code) {
        return format!("{}()", content);
    }
    // 4. `[text](url` — unclosed URL
    if ends_with_incomplete_url(&without_code) {
        return format!("{})", content);
    }
    let _ = start_line;
    content.to_string()
}

/// `/!?\[[^\]]*$/` — a `[` or `![` with no `]` afterwards (to end of string).
fn has_incomplete_bracket(text: &str) -> bool {
    memchr::memrchr(b'[', text.as_bytes())
        .is_some_and(|p| !text[p + 1..].contains(']'))
}

/// `/!?\[[^\]]*\]\s*$/` — ends with `[text]` (optional trailing ws).
fn ends_with_incomplete_link_text(text: &str) -> bool {
    let t = js_trim(text);
    if !t.ends_with(']') {
        return false;
    }
    let Some(p) = memchr::memrchr(b'[', t.as_bytes()) else {
        return false;
    };
    !t[p + 1..t.len() - 1].contains(']')
}

/// `/!?\[[^\]]*\]\([^)]*$/` — ends with `[text](...` unclosed.
fn ends_with_incomplete_url(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut found = false;
    while i < bytes.len() {
        if bytes[i] == b'[' || (bytes[i] == b'!' && bytes.get(i + 1) == Some(&b'[')) {
            let label_start = if bytes[i] == b'!' { i + 1 } else { i };
            if let Some(rel) = memchr::memchr(b']', &text.as_bytes()[label_start..]) {
                let after = label_start + rel + 1;
                if text[after..].starts_with('(') {
                    let rest = &text[after + 1..];
                    if !rest.contains(')') {
                        found = true;
                    }
                }
            }
        }
        i += 1;
    }
    found
}

// ---------------------------------------------------------------------------
// fixTable
// ---------------------------------------------------------------------------

/// `/|/` count minus 1 = number of columns.
fn column_count(row: &str) -> usize {
    row.matches('|').count().saturating_sub(1)
}

fn generate_separator(columns: usize) -> String {
    let mut s = String::from("|");
    for _ in 0..columns {
        s.push_str(" --- |");
    }
    s
}

pub fn fix_table(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let code_ranges = find_closed_code_block_ranges(content);

    let last_para = last_paragraph(content, true);
    let paragraph_lines: Vec<&str> = last_para
        .split('\n')
        .filter(|l| !l.trim().is_empty())
        .collect();
    if paragraph_lines.is_empty() {
        return content.to_string();
    }

    // find first potential header row
    let mut header_row_index = -1isize;
    let mut header_row = "";
    for (i, line) in paragraph_lines.iter().enumerate() {
        let t = js_trim(line);
        if is_table_row_line(t) || (t.starts_with('|') && t.chars().count() > 1) {
            header_row_index = i as isize;
            header_row = t;
            break;
        }
    }
    if header_row_index < 0 {
        return content.to_string();
    }
    let header_row_index = header_row_index as usize;

    // header row must not sit inside a closed code block
    let header_row_pos = content.rfind(header_row).unwrap_or(0);
    let header_row_end = header_row_pos + header_row.len();
    if is_range_overlapping_ranges(header_row_pos, header_row_end, &code_ranges) {
        return content.to_string();
    }

    let is_header_complete = header_row.ends_with('|');
    let completed_header = if is_header_complete {
        header_row.to_string()
    } else {
        format!("{} |", js_trim(header_row))
    };
    let header_columns = column_count(&completed_header);

    let before = &content[..header_row_pos];
    let after = &content[header_row_pos + header_row.len()..];

    // Case 1: header is last line of paragraph
    if header_row_index == paragraph_lines.len() - 1 {
        let new_content = if is_header_complete {
            content.to_string()
        } else {
            format!("{}{}{}", before, completed_header, after)
        };
        let sep = generate_separator(header_columns);
        if new_content.ends_with('\n') {
            return format!("{}{}", new_content, sep);
        }
        return format!("{}\n{}", new_content, sep);
    }

    // Case 2: next line is already a matching separator
    let next_line = js_trim(paragraph_lines[header_row_index + 1]);
    if is_table_separator_line(next_line) && column_count(next_line) == header_columns {
        if !is_header_complete {
            return format!("{}{}{}", before, completed_header, after);
        }
        return content.to_string();
    }

    // Case 3: incomplete separator or data row below — insert/replace separator
    let after_lines: Vec<&str> = after.split('\n').collect();
    let next_line_in_content = after_lines.get(1).copied().unwrap_or("");
    let new_header = if is_header_complete {
        header_row
    } else {
        &completed_header
    };
    let sep = generate_separator(header_columns);

    if next_line_in_content.starts_with('|') && next_line_in_content.contains('-') {
        let remaining: Vec<&str> = after_lines[2..].to_vec();
        let mut out = format!("{}{}\n{}", before, new_header, sep);
        if !remaining.is_empty() {
            out.push('\n');
            out.push_str(&remaining.join("\n"));
        }
        return out;
    }

    let remaining: Vec<&str> = after_lines[1..].to_vec();
    let mut out = format!("{}{}\n{}", before, new_header, sep);
    if !remaining.is_empty() {
        out.push('\n');
        out.push_str(&remaining.join("\n"));
    }
    out
}

// ---------------------------------------------------------------------------
// fixMath / fixInlineMath
// ---------------------------------------------------------------------------

pub fn fix_math(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let lines: Vec<&str> = content.split('\n').collect();
    let mut in_code = false;
    let mut delimiters: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if js_trim(line).starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if js_trim(line) == "$$" {
            delimiters.push(i);
        }
    }
    if delimiters.len() % 2 == 1 {
        let last = *delimiters.last().unwrap();
        let has_content = lines[last + 1..].iter().any(|l| {
            let t = js_trim(l);
            !t.is_empty() && t != "$$"
        });
        if has_content {
            if content.ends_with('\n') {
                return format!("{}$$", content);
            }
            return format!("{}\n$$", content);
        }
        return lines[..last].join("\n");
    }
    content.to_string()
}

/// Find the last `$$` that is not inside a code block / inline code.
fn find_last_dollar_pair(text: &str) -> isize {
    let bytes = text.as_bytes();
    let mut in_code = false;
    let mut in_inline = false;
    let mut last = -1isize;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if crate::scan::is_triple_backtick_at(text.as_bytes(), i) {
            in_code = !in_code;
            in_inline = false;
            i += 3;
            continue;
        }
        if !in_code && bytes[i] == b'`' {
            if !is_backtick_part_of_triple(text, i) {
                in_inline = !in_inline;
            }
            i += 1;
            continue;
        }
        if !in_code && !in_inline && bytes[i] == b'$' && bytes[i + 1] == b'$' {
            last = i as isize;
            i += 2;
            continue;
        }
        i += 1;
    }
    last
}

pub fn fix_inline_math(content: &str) -> String {
    if content == "$" {
        return String::new();
    }
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];

    let without_code = strip_closed_code_blocks(last_para);
    let without_inline = strip_inline_code(&without_code);
    let count = count_of(&without_inline, "$$");

    if count % 2 == 1 {
        let last_dollar = find_last_dollar_pair(last_para);
        if last_dollar < 0 {
            return content.to_string();
        }
        let last_dollar = last_dollar as usize;
        let after_last = &last_para[last_dollar + 2..];
        if after_last.starts_with('\n') || after_last.contains('\n') {
            return content.to_string();
        }
        if js_trim(after_last) == "$" {
            return content.to_string();
        }
        let mut after_last = after_last.to_string();
        let mut should_remove_trailing = false;
        if after_last.ends_with('$') && !after_last.ends_with("$$") {
            should_remove_trailing = true;
            after_last.pop();
        }
        if !after_last.trim().is_empty() {
            if should_remove_trailing {
                let actual = offset + last_dollar;
                let before_math = &content[..actual + 2];
                let after_math = &last_para[last_dollar + 2..last_para.len() - 1];
                return format!("{}{}$$", before_math, after_math);
            }
            return format!("{}$$", content);
        }
        let actual = offset + last_dollar;
        let out = content[..actual].trim_end_matches(is_ws).to_string();
        return out;
    }
    let _ = start_line;
    content.to_string()
}

/// /`[^`\n]+`/g — remove inline code spans.
pub fn strip_inline_code(text: &str) -> String {
    let ranges = find_inline_code_ranges(text, &find_closed_code_block_ranges(text));
    if ranges.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for &(start, end) in &ranges {
        out.push_str(&text[cursor..start]);
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_inline_completion() {
        assert_eq!(fix_code("`"), "");
        assert_eq!(fix_code("``"), "");
        assert_eq!(fix_code("```"), "");
        assert_eq!(fix_code("Text `"), "Text");
        assert_eq!(fix_code("Text ``"), "Text");
        assert_eq!(fix_code("Text ```"), "Text");
        assert_eq!(fix_code("Hello `world"), "Hello `world`");
        assert_eq!(fix_code("Hello `world`"), "Hello `world`");
        assert_eq!(fix_code("Hello\n\n`"), "Hello");
        assert_eq!(fix_code("`a` and `b"), "`a` and `b`");
        assert_eq!(
            fix_code("Hello `world\nand more code"),
            "Hello `world\nand more code`"
        );
        assert_eq!(fix_code("Text ````"), "Text");
    }

    #[test]
    fn code_block_completion() {
        assert_eq!(
            fix_code("```javascript\nconst x = 1"),
            "```javascript\nconst x = 1\n```"
        );
        assert_eq!(
            fix_code("```javascript\nconst x = 1`"),
            "```javascript\nconst x = 1\n```"
        );
        assert_eq!(
            fix_code("```javascript\nconst x = 1``"),
            "```javascript\nconst x = 1\n```"
        );
        assert_eq!(
            fix_code("```python\nprint(\"hello\")\n"),
            "```python\nprint(\"hello\")\n```"
        );
        assert_eq!(fix_code("```js\ncode\n```"), "```js\ncode\n```");
        assert_eq!(
            fix_code("```javascript\nfunction test() {\n\n  return true;\n}"),
            "```javascript\nfunction test() {\n\n  return true;\n}\n```"
        );
        assert_eq!(fix_code("```javascript"), "```javascript\n```");
    }

    #[test]
    fn html_fragments() {
        assert_eq!(fix_html("Hello <div"), "Hello");
        assert_eq!(fix_html("Hello <div>\ncontent"), "Hello <div>\ncontent");
        assert_eq!(fix_html("Hello <br"), "Hello");
        assert_eq!(fix_html("Hello <"), "Hello");
        assert_eq!(fix_html("Hello </di"), "Hello");
    }

    #[test]
    fn footnote_removal() {
        assert_eq!(fix_footnote("Text [^1] and [^2]"), "Text and");
        assert_eq!(fix_footnote("Text [^1]"), "Text");
        assert_eq!(
            fix_footnote("Text [^1]\n\n[^1]: def"),
            "Text [^1]\n\n[^1]: def"
        );
        assert_eq!(
            fix_footnote("```\n[^1]\n```\n\nText [^1]"),
            "```\n[^1]\n```\n\nText"
        );
        assert_eq!(fix_footnote("Text [^1"), "Text");
        assert_eq!(
            fix_footnote("Text [^1]\nand more text"),
            "Text\nand more text"
        );
        assert_eq!(fix_footnote("Text `[^1]` and [^1]"), "Text `[^1]` and");
    }

    #[test]
    fn task_list_cleanup() {
        assert_eq!(fix_task_list("- [ ] Task 1\n-"), "- [ ] Task 1");
        assert_eq!(
            fix_task_list("- [ ] Task 1\n- [x] Task 2\n-"),
            "- [ ] Task 1\n- [x] Task 2"
        );
        assert_eq!(fix_task_list("- [ ] Task 1\n  - ["), "- [ ] Task 1");
        assert_eq!(
            fix_task_list("> **Note**: quote\n\n> -"),
            "> **Note**: quote\n"
        );
        assert_eq!(fix_task_list("- item\n-"), "- item");
        assert_eq!(fix_task_list("-"), "");
        assert_eq!(fix_task_list("- ["), "");
        assert_eq!(fix_task_list("- [ ] Task 1\n- "), "- [ ] Task 1");
        assert_eq!(fix_task_list("> - ["), "");
        assert_eq!(fix_task_list("> -"), "");
    }

    #[test]
    fn link_completion() {
        assert_eq!(fix_link("[Google"), "[Google]()");
        assert_eq!(fix_link("[Google]"), "[Google]()");
        assert_eq!(fix_link("Text [ content"), "Text [ content]()");
        assert_eq!(fix_link("[Google]("), "[Google]()");
        assert_eq!(
            fix_link("[Google](https://www.goo"),
            "[Google](https://www.goo)"
        );
        assert_eq!(
            fix_link("[Google](https://www.google.com)"),
            "[Google](https://www.google.com)"
        );
        assert_eq!(fix_link("Text ["), "Text");
        assert_eq!(fix_link("Text [ "), "Text");
        assert_eq!(fix_link("Text [\n"), "Text");
        assert_eq!(fix_link("![alt"), "![alt]()");
        assert_eq!(fix_link("![alt]"), "![alt]()");
        assert_eq!(fix_link("![]("), "![]()");
        assert_eq!(
            fix_link("[text](https://example.com/page*value"),
            "[text](https://example.com/page*value)"
        );
    }

    #[test]
    fn table_fixes() {
        assert_eq!(fix_table("| a | b |\n"), "| a | b |\n| --- | --- |");
        assert_eq!(fix_table("| a | b |\n| ---"), "| a | b |\n| --- | --- |");
        assert_eq!(
            fix_table("| a | b |\n| --- | --- |"),
            "| a | b |\n| --- | --- |"
        );
        assert_eq!(
            fix_table("| a | b |\n| --- | --- |\n| 1 | 2 |"),
            "| a | b |\n| --- | --- |\n| 1 | 2 |"
        );
        assert_eq!(fix_table("| a | b"), "| a | b |\n| --- | --- |");
    }

    #[test]
    fn math_fixes() {
        assert_eq!(fix_math("$$\nE = mc^2"), "$$\nE = mc^2\n$$");
        assert_eq!(fix_math("$$\nE = mc^2\n$$"), "$$\nE = mc^2\n$$");
        assert_eq!(fix_math("$$\n"), "");
        assert_eq!(
            fix_inline_math("The formula is $$x = 1"),
            "The formula is $$x = 1$$"
        );
        assert_eq!(fix_inline_math("$"), "");
        assert_eq!(fix_inline_math("Text $$"), "Text");
        assert_eq!(fix_inline_math("$$"), "");
    }
}

// ---------------------------------------------------------------------------
// fixStrong
// ---------------------------------------------------------------------------

/// Scan the original last paragraph for the last `**` / `__` marker position,
/// skipping fenced code blocks and inline code ranges.
fn last_double_marker_pos(
    last_para: &str,
    inline_ranges: &[(usize, usize)],
    marker: &str,
) -> isize {
    let bytes = last_para.as_bytes();
    let mut in_code = false;
    let mut last: isize = -1;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if crate::scan::is_triple_backtick_at(last_para.as_bytes(), i) {
            in_code = !in_code;
            i += 3;
            continue;
        }
        if is_position_in_ranges(i, inline_ranges) {
            i += 1;
            continue;
        }
        if in_code {
            i += 1;
            continue;
        }
        let is_marker = (marker == "**" && bytes[i] == b'*' && bytes[i + 1] == b'*')
            || (marker == "__" && bytes[i] == b'_' && bytes[i + 1] == b'_');
        if is_marker {
            last = i as isize;
            i += 2;
            continue;
        }
        i += 1;
    }
    last
}

/// Sanitized last paragraph for marker counting (strong flavor).
fn strong_counting_text(content: &str, options: &PreprocessOptions) -> (String, String) {
    let last_para = last_paragraph(content, true).to_string();
    let code_ranges = find_closed_code_block_ranges(&last_para);
    let inline_ranges = find_inline_code_ranges(&last_para, &code_ranges);
    let masked = mask_inline_code_markdown_markers(&last_para, &inline_ranges);
    let no_code = strip_closed_code_blocks(&masked);
    let no_urls = remove_urls_from_text(&no_code);
    let no_math = remove_math_blocks_from_text(&no_urls, options.single_dollar_text_math);
    let marker_counted = mask_invalid_underscore_markers(&no_math);
    (no_math, marker_counted)
}

pub fn fix_strong(content: &str, options: &PreprocessOptions) -> String {
    if content == "*" || content == "_" {
        return String::new();
    }
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let (para_line, offset) = last_paragraph_range(content, true);
    let last_para = &content[offset..];
    let code_ranges = find_closed_code_block_ranges(last_para);
    let inline_ranges = find_inline_code_ranges(last_para, &code_ranges);

    let masked = mask_inline_code_markdown_markers(last_para, &inline_ranges);
    let no_code = strip_closed_code_blocks(&masked);
    let no_urls = remove_urls_from_text(&no_code);
    let no_math = remove_math_blocks_from_text(&no_urls, options.single_dollar_text_math);
    let marker_counted = mask_invalid_underscore_markers(&no_math);

    let ends_with_single_asterisk = content.ends_with('*') && !content.ends_with("**");
    let ends_with_single_underscore = content.ends_with('_') && !content.ends_with("__");

    let asterisk_count = count_of(&no_math, "**");
    let underscore_count = count_of(&marker_counted, "__");

    let mut needs_asterisk_completion = false;
    let mut needs_underscore_completion = false;
    let mut needs_asterisk_removal = false;
    let mut needs_underscore_removal = false;

    if asterisk_count % 2 == 1 {
        let last_star = last_double_marker_pos(last_para, &inline_ranges, "**");
        let absolute = offset + last_star as usize;
        if is_within_math_block(content, absolute, options.single_dollar_text_math)
            || is_within_link_or_image_url(content, absolute)
            || is_within_html_tag(content, absolute)
        {
            return content.to_string();
        }
        let pos = memchr::memmem::rfind(no_math.as_bytes(), b"**").unwrap_or(0);
        if !no_math[pos + 2..].trim().is_empty() {
            needs_asterisk_completion = true;
        } else {
            needs_asterisk_removal = true;
        }
    }

    if underscore_count % 2 == 1 {
        let last_us = last_double_marker_pos_underscore(last_para, &inline_ranges);
        if last_us >= 0 {
            let absolute = offset + last_us as usize;
            if is_within_math_block(content, absolute, options.single_dollar_text_math)
                || is_within_link_or_image_url(content, absolute)
                || is_within_html_tag(content, absolute)
            {
                return content.to_string();
            }
            let pos = memchr::memmem::rfind(marker_counted.as_bytes(), b"__").unwrap_or(0);
            if !marker_counted[pos + 2..].trim().is_empty() {
                needs_underscore_completion = true;
            } else {
                needs_underscore_removal = true;
            }
        }
    }

    let mut removed_trailing_single = false;
    let mut content = content.to_string();

    if ends_with_single_asterisk && (needs_asterisk_completion || needs_asterisk_removal) {
        content.truncate(content.len() - 1);
        removed_trailing_single = true;
        let (no_math2, _) = strong_counting_text(&content, options);
        if count_of(&no_math2, "**") % 2 == 1 {
            let pos = memchr::memmem::rfind(no_math2.as_bytes(), b"**").unwrap_or(0);
            if !no_math2[pos + 2..].trim().is_empty() {
                needs_asterisk_completion = true;
                needs_asterisk_removal = false;
            } else {
                needs_asterisk_removal = true;
                needs_asterisk_completion = false;
            }
        }
    }

    if ends_with_single_underscore && (needs_underscore_completion || needs_underscore_removal) {
        content.truncate(content.len() - 1);
        removed_trailing_single = true;
        let (_, marker_counted2) = strong_counting_text(&content, options);
        if count_of(&marker_counted2, "__") % 2 == 1 {
            let pos = memchr::memmem::rfind(marker_counted2.as_bytes(), b"__").unwrap_or(0);
            if !marker_counted2[pos + 2..].trim().is_empty() {
                needs_underscore_completion = true;
                needs_underscore_removal = false;
            } else {
                needs_underscore_removal = true;
                needs_underscore_completion = false;
            }
        }
    }

    if needs_asterisk_removal {
        let mut result = content[..content.len().saturating_sub(2)]
            .trim_end()
            .to_string();
        result = remove_trailing_standalone_dash(&result);
        return result;
    }

    if needs_underscore_removal {
        let (_, offset2) = last_paragraph_range(&content, false);
        let new_para = &content[offset2..];
        let pos = memchr::memmem::rfind(new_para.as_bytes(), b"__").unwrap_or(0);
        let absolute = offset2 + pos;
        let mut result = content[..absolute].trim_end().to_string();
        result = remove_trailing_standalone_dash(&result);
        return result;
    }

    if needs_asterisk_completion && needs_underscore_completion {
        let first_star = memchr::memmem::find(no_math.as_bytes(), b"**").unwrap_or(usize::MAX);
        let first_us = memchr::memmem::find(marker_counted.as_bytes(), b"__").unwrap_or(usize::MAX);
        if first_star < first_us {
            return append_before_trailing_whitespace(&content, "__**");
        }
        return append_before_trailing_whitespace(&content, "**__");
    }

    if needs_asterisk_completion {
        if !removed_trailing_single {
            let (no_math2, _) = strong_counting_text(&content, options);
            let without_double = no_math2.replace("**", "");
            if count_of(&without_double, "*") % 2 == 1 {
                return append_before_trailing_whitespace(&content, "***");
            }
        }
        return append_before_trailing_whitespace(&content, "**");
    }

    if needs_underscore_completion {
        if !removed_trailing_single {
            let (_, marker_counted2) = strong_counting_text(&content, options);
            let without_double = marker_counted2.replace("__", "");
            if count_of(&without_double, "_") % 2 == 1 {
                return append_before_trailing_whitespace(&content, "___");
            }
        }
        return append_before_trailing_whitespace(&content, "__");
    }

    let _ = (lines, para_line);
    content
}

/// Underscore variant of the last-marker scan (respects intraword/escaped `__`).
fn last_double_marker_pos_underscore(last_para: &str, inline_ranges: &[(usize, usize)]) -> isize {
    let bytes = last_para.as_bytes();
    let mut in_code = false;
    let mut last: isize = -1;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if crate::scan::is_triple_backtick_at(last_para.as_bytes(), i) {
            in_code = !in_code;
            i += 3;
            continue;
        }
        if is_position_in_ranges(i, inline_ranges) {
            i += 1;
            continue;
        }
        if in_code {
            i += 1;
            continue;
        }
        if bytes[i] == b'_' && bytes[i + 1] == b'_' {
            if should_ignore_underscore_marker(last_para, i, 2) {
                i += 2;
                continue;
            }
            last = i as isize;
            i += 2;
            continue;
        }
        i += 1;
    }
    last
}

// ---------------------------------------------------------------------------
// fixEmphasis
// ---------------------------------------------------------------------------

pub fn fix_emphasis(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];
    let code_ranges = find_closed_code_block_ranges(last_para);
    let inline_ranges = find_inline_code_ranges(last_para, &code_ranges);

    let masked = mask_inline_code_markdown_markers(last_para, &inline_ranges);
    let no_code = strip_closed_code_blocks(&masked);
    let no_urls = remove_urls_from_text(&no_code);
    let marker_counted = mask_invalid_underscore_markers(&no_urls);

    let without_double_star = marker_counted.replace("**", "");
    let asterisk_count = count_of(&without_double_star, "*");
    let without_double_us = marker_counted.replace("__", "");
    let underscore_count = count_of(&without_double_us, "_");

    let mut needs_asterisk_completion = false;
    let mut needs_underscore_completion = false;
    let mut needs_asterisk_removal = false;
    let mut needs_underscore_removal = false;

    if asterisk_count % 2 == 1 {
        // find last single `*` not part of `**`, not in code/url/math/html
        let mut last_star: isize = -1;
        let bytes = last_para.as_bytes();
        let mut i = last_para.len();
        while i > 0 {
            i -= 1;
            if is_position_in_ranges(i, &code_ranges) || is_position_in_ranges(i, &inline_ranges) {
                continue;
            }
            if bytes[i] == b'*' {
                if i > 0 && bytes[i - 1] == b'*' {
                    continue;
                }
                let absolute = offset + i;
                if !is_within_math_block(content, absolute, false)
                    && !is_within_link_or_image_url(content, absolute)
                    && !is_within_html_tag(content, absolute)
                {
                    last_star = i as isize;
                    break;
                }
            }
        }
        if last_star < 0 {
            return content.to_string();
        }
        let has_content_after = last_para[last_star as usize + 1..]
            .chars()
            .any(|c| !c.is_whitespace());
        if has_content_after {
            needs_asterisk_completion = true;
        } else {
            needs_asterisk_removal = true;
        }
    }

    if underscore_count % 2 == 1 {
        let mut last_us: isize = -1;
        let bytes = last_para.as_bytes();
        let mut i = last_para.len();
        while i > 0 {
            i -= 1;
            if is_position_in_ranges(i, &code_ranges) || is_position_in_ranges(i, &inline_ranges) {
                continue;
            }
            if bytes[i] == b'_' {
                if i > 0 && bytes[i - 1] == b'_' {
                    continue;
                }
                if should_ignore_underscore_marker(last_para, i, 1) {
                    continue;
                }
                let absolute = offset + i;
                if !is_within_math_block(content, absolute, false)
                    && !is_within_link_or_image_url(content, absolute)
                    && !is_within_html_tag(content, absolute)
                {
                    last_us = i as isize;
                    break;
                }
            }
        }
        if last_us < 0 {
            return content.to_string();
        }
        let has_content_after = last_para[last_us as usize + 1..]
            .chars()
            .any(|c| !c.is_whitespace());
        if has_content_after {
            needs_underscore_completion = true;
        } else {
            needs_underscore_removal = true;
        }
    }

    if needs_asterisk_removal {
        let mut result = content[..content.len() - 1].trim_end().to_string();
        result = remove_trailing_standalone_dash(&result);
        return result;
    }

    if needs_underscore_removal {
        // find last single `_` position in original text
        let mut last_us_abs: isize = -1;
        let bytes = last_para.as_bytes();
        let mut i = last_para.len();
        while i > 0 {
            i -= 1;
            if is_position_in_ranges(i, &code_ranges) || is_position_in_ranges(i, &inline_ranges) {
                continue;
            }
            if bytes[i] == b'_' && (i == 0 || bytes[i - 1] != b'_') {
                if should_ignore_underscore_marker(last_para, i, 1) {
                    continue;
                }
                let absolute = (offset + i) as isize;
                if !is_within_math_block(content, absolute as usize, false)
                    && !is_within_link_or_image_url(content, absolute as usize)
                    && !is_within_html_tag(content, absolute as usize)
                {
                    last_us_abs = absolute;
                    break;
                }
            }
        }
        let mut result = content[..last_us_abs.max(0) as usize]
            .trim_end()
            .to_string();
        result = remove_trailing_standalone_dash(&result);
        return result;
    }

    if needs_asterisk_completion && needs_underscore_completion {
        let first_star = memchr::memchr(b'*', without_double_star.as_bytes()).unwrap_or(usize::MAX);
        let first_us = memchr::memchr(b'_', without_double_us.as_bytes()).unwrap_or(usize::MAX);
        if first_star < first_us {
            return format!("{}_*", content);
        }
        return format!("{}*_", content);
    }

    if needs_asterisk_completion {
        return format!("{}*", content);
    }
    if needs_underscore_completion {
        return format!("{}_", content);
    }
    let _ = start_line;
    content.to_string()
}

// ---------------------------------------------------------------------------
// fixDelete
// ---------------------------------------------------------------------------

pub fn fix_delete(content: &str) -> String {
    if is_inside_unclosed_code_block(content) {
        return content.to_string();
    }
    let (start_line, offset) = last_paragraph_range(content, false);
    let last_para = &content[offset..];

    let no_code = strip_closed_code_blocks(last_para);
    let no_urls = remove_urls_from_text(&no_code);
    let count = count_of(&no_urls, "~~");

    let ends_with_single_tilde = content.ends_with('~') && !content.ends_with("~~");

    if ends_with_single_tilde {
        let without_last = &content[..content.len() - 1];
        let no_code2 = strip_closed_code_blocks(&without_last[offset..]);
        let no_urls2 = remove_urls_from_text(&no_code2);
        let count2 = count_of(&no_urls2, "~~");
        if count2 % 2 == 1 {
            let last_pos = memchr::memmem::rfind(no_urls2.as_bytes(), b"~~").unwrap_or(0);
            if (last_pos > 0 || no_urls2.starts_with("~~")) && !no_urls2[last_pos + 2..].is_empty()
            {
                return format!("{}~", content);
            }
        } else {
            return without_last.to_string();
        }
    }

    if count % 2 == 1 {
        let mut actual_last: isize = -1;
        let bytes = last_para.as_bytes();
        let mut in_code = false;
        let mut i = 0;
        while i + 1 < bytes.len() {
            if crate::scan::is_triple_backtick_at(last_para.as_bytes(), i) {
                in_code = !in_code;
                i += 3;
                continue;
            }
            if in_code {
                i += 1;
                continue;
            }
            if bytes[i] == b'~' && bytes[i + 1] == b'~' {
                actual_last = i as isize;
                i += 2;
                continue;
            }
            i += 1;
        }
        if actual_last < 0 {
            return content.to_string();
        }
        let absolute = offset + actual_last as usize;
        if is_within_math_block(content, absolute, false)
            || is_within_link_or_image_url(content, absolute)
            || is_within_html_tag(content, absolute)
        {
            return content.to_string();
        }
        let after_last = no_urls[memchr::memmem::rfind(no_urls.as_bytes(), b"~~").unwrap_or(0) + 2..].to_string();
        if !after_last.trim().is_empty() {
            return format!("{}~~", content);
        }
        let before_tilde = &content[..content.len() - after_last.len() - 2];
        return before_tilde.trim_end().to_string();
    }
    let _ = start_line;
    content.to_string()
}
