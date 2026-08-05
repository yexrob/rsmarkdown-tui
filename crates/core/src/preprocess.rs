//! `normalize` + `preprocess` pipeline, ported from
//! `markmend/core/src/preprocess/index.ts` and `vendored/markdown-utils.ts`.

use crate::fix::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct PreprocessOptions {
    /// Treat single `$...$` as inline math (default: false, only `$$` counts).
    pub single_dollar_text_math: bool,
}

/// CRLF -> LF, trim trailing whitespace.
fn proprocess_content(content: &str) -> String {
    if content.contains('\r') {
        let no_cr = content.replace("\r\n", "\n").replace('\r', "\n");
        no_cr.trim_end().to_string()
    } else {
        content.trim_end().to_string()
    }
}

const CODE_BLOCK_PLACEHOLDER: &str = "CODE_BLOCK_PLACEHOLDER";
const DOLLAR_PLACEHOLDER: &str = "_TMP_REPLACE_DOLLAR_";

/// `/\\\[(.*?)\\\]/` -> `$$...$$` (single-line bracket math)
/// `/\\\[([\s\S]*?)\\\]/` -> `$$...$$` (block bracket math)
/// `/\\\((.*?)\\\)/` -> `$$...$$` (paren math)
/// `/(^|[^\\])\$(.+?)\$/` -> `$1$$$2$` (dollar math)
pub fn preprocess_latex(content: &str) -> String {
    // fast path: nothing to rewrite when no LaTeX markers or dollars are present
    if !content.contains(['$', '\\']) {
        return content.to_string();
    }
    let code_blocks: Vec<&str> = {
        let mut blocks = Vec::new();
        let ranges = crate::scan::find_closed_code_block_ranges(content);
        for &(start, end) in &ranges {
            blocks.push(&content[start..end]);
        }
        blocks
    };
    if code_blocks.is_empty() {
        let mut p = content.to_string();
        p = replace_bracket_math(&p, false);
        p = replace_bracket_math(&p, true);
        p = replace_paren_math(&p);
        p = replace_dollar_math(&p);
        return p;
    }
    let mut processed = {
        let ranges = crate::scan::find_closed_code_block_ranges(content);
        let mut out = String::with_capacity(content.len());
        let mut cursor = 0;
        for &(start, end) in &ranges {
            out.push_str(&content[cursor..start]);
            out.push_str(CODE_BLOCK_PLACEHOLDER);
            cursor = end;
        }
        out.push_str(&content[cursor..]);
        out
    };

    processed = replace_bracket_math(&processed, false);
    processed = replace_bracket_math(&processed, true);
    processed = replace_paren_math(&processed);
    processed = replace_dollar_math(&processed);

    // restore code blocks in one pass (escaped `$` kept until the final unescape)
    let mut restored = String::with_capacity(processed.len());
    let mut ph_pos = 0;
    let mut block_idx = 0;
    while let Some(rel) = memchr::memmem::find(&processed.as_bytes()[ph_pos..], CODE_BLOCK_PLACEHOLDER.as_bytes()) {
        restored.push_str(&processed[ph_pos..ph_pos + rel]);
        restored.push_str(&code_blocks[block_idx].replace('$', DOLLAR_PLACEHOLDER));
        ph_pos += rel + CODE_BLOCK_PLACEHOLDER.len();
        block_idx += 1;
    }
    restored.push_str(&processed[ph_pos..]);

    restored.replace(DOLLAR_PLACEHOLDER, "$")
}

fn replace_bracket_math(content: &str, multiline: bool) -> String {
    // find `\[` ... `\]` spans, convert content to `$$content$$`
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    while i < content.len() {
        let ch = content[i..].chars().next().expect("char at boundary");
        if ch == '\\' && content[i + 1..].starts_with('[') {
            let rest = &content[i + 2..];
            if let Some(rel) = memchr::memmem::find(rest.as_bytes(), b"\\]") {
                let equation = &rest[..rel];
                let span_len = equation.len();
                if (multiline || !equation.contains('\n')) && (span_len > 0 || equation.is_empty())
                {
                    out.push_str("$$");
                    out.push_str(equation);
                    out.push_str("$$");
                    i += 2 + span_len + 2;
                    continue;
                }
            }
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn replace_paren_math(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    while let Some(rel) = memchr::memmem::find(&content.as_bytes()[i..], b"\\(") {
        let open = i + rel;
        let rest = &content[open + 2..];
        let Some(close_rel) = memchr::memmem::find(rest.as_bytes(), b"\\)") else {
            break;
        };
        let equation = &rest[..close_rel];
        if equation.contains('\n') {
            break;
        }
        out.push_str(&content[i..open]);
        out.push_str("$$");
        out.push_str(equation);
        out.push_str("$$");
        i = open + 2 + equation.len() + 2;
    }
    out.push_str(&content[i..]);
    out
}

/// `/(^|[^\\])\$(.+?)\$/g` -> `$1$$$2$`
///
/// Improvement over the original: a `$` that is part of an existing `$$` pair is
/// left untouched, so single-line `$$x$$` survives normalize intact (the original
/// mangles it into `$$$x$$` and breaks single-line math).
fn replace_dollar_math(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    let mut span_start = 0;
    while i < bytes.len() {
        let prefix_ok = i == 0 || bytes[i - 1] != b'\\';
        let part_of_double = bytes.get(i + 1) == Some(&b'$') || (i > 0 && bytes[i - 1] == b'$');
        if bytes[i] == b'$' && prefix_ok && !part_of_double {
            if let Some(rel) = memchr::memchr(b'$', &content.as_bytes()[i + 1..]) {
                let equation = &content[i + 1..i + 1 + rel];
                if !equation.is_empty() && !equation.contains('\n') {
                    out.push_str(&content[span_start..i]);
                    out.push_str("$$");
                    out.push_str(equation);
                    out.push('$');
                    i += 1 + rel + 1;
                    span_start = i;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&content[span_start..]);
    out
}

/// Ordered preprocess steps — order matters (ported from `DEFAULT_PREPROCESS_STEP_NAMES`).
pub fn preprocess(content: &str, options: &PreprocessOptions) -> String {
    let mut result = content.to_string();
    result = fix_code(&result);
    result = fix_html(&result);
    result = fix_footnote(&result);
    result = fix_strong(&result, options);
    result = fix_emphasis(&result);
    result = fix_delete(&result);
    result = fix_task_list(&result);
    result = fix_link(&result);
    result = fix_table(&result);
    result = fix_inline_math(&result);
    result = fix_math(&result);
    result
}

pub fn normalize(content: &str) -> String {
    let cleaned = proprocess_content(content);
    preprocess_latex(&cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_crlf() {
        assert_eq!(normalize("a\r\nb\r\n\r\n"), "a\nb");
        assert_eq!(normalize("  text  \n\n"), "  text");
    }

    #[test]
    fn latex_preprocessing() {
        // `\(x\)` / `\[x\]` convert to `$$...$$` and — unlike the original —
        // are NOT re-mangled by the dollar rewrite
        assert_eq!(preprocess_latex(r"\(x^2\)"), "$$x^2$$");
        assert_eq!(preprocess_latex(r"\[x^2\]"), "$$x^2$$");
        assert_eq!(preprocess_latex(r"$x$ and $y$"), "$$x$ and $$y$");
        assert_eq!(preprocess_latex("```\n$100\n```"), "```\n$100\n```");
        // single-line `$$...$$` is preserved (original mangles it to `$$$...$$`)
        assert_eq!(preprocess_latex("$$x$$"), "$$x$$");
        assert_eq!(
            preprocess_latex("The formula is $$x = 1$$"),
            "The formula is $$x = 1$$"
        );
        assert_eq!(preprocess_latex("$$\nE = mc^2\n$$"), "$$\nE = mc^2\n$$");
    }

    #[test]
    fn pipeline_order() {
        // fixTaskList must run before fixLink: `-[` is a task list start, not a link
        let opts = PreprocessOptions::default();
        let r = preprocess("- [", &opts);
        assert_eq!(r, "");
    }
}
