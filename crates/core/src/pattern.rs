//! Shared line patterns (ported from `pattern.ts`) as small predicates/helpers.

use crate::scan::{is_ws, js_trim};

/// `/^\|.*\|.*\|/` — line looks like a table row (>= 3 pipes).
pub fn is_table_row_line(line: &str) -> bool {
    let line = js_trim(line);
    if !line.starts_with('|') {
        return false;
    }
    line.matches('|').count() >= 3
}

fn is_separator_cell(cell: &str) -> bool {
    let cell = js_trim(cell);
    if cell.is_empty() {
        return false;
    }
    let stripped = cell.trim_matches(':');
    stripped.chars().all(|c| c == '-') && stripped.chars().count() >= 3
}

/// `/^\|[\s:]*-{3,}[\s:]*(?:\|[\s:]*-{3,}[\s:]*)+\|?$/`
pub fn is_table_separator_line(line: &str) -> bool {
    let line = js_trim(line);
    if !line.starts_with('|') {
        return false;
    }
    let rest = &line[1..];
    let mut cells: Vec<&str> = rest.split('|').collect();
    if rest.ends_with('|') {
        cells.pop();
    }
    if cells.len() < 2 {
        return false;
    }
    cells.iter().all(|c| is_separator_cell(c))
}

/// `/^\s*-$/`
pub fn is_standalone_dash(line: &str) -> bool {
    js_trim(line) == "-"
}

/// `/^\s*-\s+$/`
pub fn is_dash_with_space(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(is_ws)
}

/// `/^\s*- \[[x ]\]/i`
pub fn is_task_list(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix("- [") else {
        return false;
    };
    let mut chars = rest.chars();
    let first = chars.next().unwrap_or(' ');
    let second = chars.next().unwrap_or(' ');
    (first == 'x' || first == 'X' || first == ' ') && second == ']'
}

/// `/^\s*-\s*\[\s*$/`
pub fn is_incomplete_task_list(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix('-') else {
        return false;
    };
    let Some(rest) = rest.trim_start_matches(is_ws).strip_prefix('[') else {
        return false;
    };
    rest.chars().all(is_ws)
}

/// `/^>\s*-$/`
pub fn is_quote_standalone_dash(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix('>') else {
        return false;
    };
    is_standalone_dash(js_trim(rest))
}

/// `/^>\s*- \[[x ]\]/i`
pub fn is_quote_task_list(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix('>') else {
        return false;
    };
    is_task_list(js_trim(rest))
}

/// `/^>\s*-\s*\[\s*$/`
pub fn is_quote_incomplete_task_list(line: &str) -> bool {
    let t = js_trim(line);
    let Some(rest) = t.strip_prefix('>') else {
        return false;
    };
    is_incomplete_task_list(js_trim(rest))
}

/// Remove a trailing standalone `-` line (pattern `/(\n\n?)-[ \t]*$/`):
/// returns content with that dash line removed, newlines preserved.
pub fn remove_trailing_standalone_dash(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    if end == 0 || bytes[end - 1] != b'-' {
        return content.to_string();
    }
    let dash = end - 1;
    if dash == 0 || bytes[dash - 1] != b'\n' {
        return content.to_string();
    }
    let newline = dash - 1;
    let keep_start = if newline > 0 && bytes[newline - 1] == b'\n' {
        newline - 1
    } else {
        newline
    };
    content[..keep_start + 1].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_lines() {
        assert!(is_table_separator_line("| --- | --- |"));
        assert!(is_table_separator_line("| :---: | ---: |"));
        assert!(is_table_separator_line("| --- | ---"));
        assert!(!is_table_separator_line("| a | b |"));
        assert!(!is_table_separator_line("| --- |"));
        assert!(!is_table_separator_line("| ---"));
    }

    #[test]
    fn task_lines() {
        assert!(is_task_list("- [ ] todo"));
        assert!(is_task_list("- [x] done"));
        assert!(is_task_list("- [X] done"));
        assert!(!is_task_list("- []"));
        assert!(is_incomplete_task_list("- ["));
        assert!(is_incomplete_task_list("-[ "));
    }

    #[test]
    fn dash_removal() {
        assert_eq!(
            remove_trailing_standalone_dash("- [ ] Task 1\n-"),
            "- [ ] Task 1\n"
        );
        assert_eq!(remove_trailing_standalone_dash("a\n\n-"), "a\n");
        assert_eq!(remove_trailing_standalone_dash("no dash"), "no dash");
    }
}
