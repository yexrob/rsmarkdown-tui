//! AUTO-GENERATED from vue-stream-markdown test fixtures (test-cases.ts).
//! Regenerate with `node scripts/gen-tests.mjs`.
//! @generated

use crate::fix::*;

#[test]
fn generated_fix_code_code_inline() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(fix_code("`"), "", "case 0");
    assert_eq!(fix_code("``"), "", "case 1");
    assert_eq!(fix_code("```"), "", "case 2");
    assert_eq!(fix_code("Text `"), "Text", "case 3");
    assert_eq!(fix_code("Text ``"), "Text", "case 4");
    assert_eq!(fix_code("Text ```"), "Text", "case 5");
    assert_eq!(fix_code("Hello `world"), "Hello `world`", "case 6");
    assert_eq!(fix_code("Hello `world`"), "Hello `world`", "case 7");
    assert_eq!(fix_code("Hello\n\n`"), "Hello", "case 8");
    assert_eq!(fix_code("`a` and `b"), "`a` and `b`", "case 9");
    assert_eq!(
        fix_code("Hello `world\nand more code"),
        "Hello `world\nand more code`",
        "case 10"
    );
    assert_eq!(
        fix_code("```js\ncode\n``` and `inline"),
        "```js\ncode\n``` and `inline`",
        "case 11"
    );
    assert_eq!(
        fix_code("Para1 `one\n\nPara2 `two"),
        "Para1 `one\n\nPara2 `two`",
        "case 12"
    );
    assert_eq!(
        fix_code("Text with ` and more"),
        "Text with ` and more`",
        "case 13"
    );
    assert_eq!(fix_code("Text ````"), "Text", "case 14");
}

#[test]
fn generated_fix_code_code_block() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(
        fix_code("```javascript\nconst x = 1"),
        "```javascript\nconst x = 1\n```",
        "case 0"
    );
    assert_eq!(
        fix_code("```javascript\nconst x = 1`"),
        "```javascript\nconst x = 1\n```",
        "case 1"
    );
    assert_eq!(
        fix_code("```javascript\nconst x = 1``"),
        "```javascript\nconst x = 1\n```",
        "case 2"
    );
    assert_eq!(
        fix_code("```python\nprint(\"hello\")\n"),
        "```python\nprint(\"hello\")\n```",
        "case 3"
    );
    assert_eq!(fix_code("```js\ncode\n```"), "```js\ncode\n```", "case 4");
    assert_eq!(
        fix_code("```javascript\nfunction test() {\n\n  return true;\n}"),
        "```javascript\nfunction test() {\n\n  return true;\n}\n```",
        "case 5"
    );
    assert_eq!(
        fix_code("```js\ncode1\n```\n\nText\n\n```python\ncode2"),
        "```js\ncode1\n```\n\nText\n\n```python\ncode2\n```",
        "case 6"
    );
    assert_eq!(
        fix_code("```\nplain code"),
        "```\nplain code\n```",
        "case 7"
    );
    assert_eq!(fix_code("```javascript"), "```javascript\n```", "case 8");
    assert_eq!(
        fix_code("```js\nconst x = `template\n```"),
        "```js\nconst x = `template\n```",
        "case 9"
    );
    assert_eq!(
        fix_code("```js\nconst x = `template\n```"),
        "```js\nconst x = `template\n```",
        "case 10"
    );
}

#[test]
fn generated_fix_code_code_mixed() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(
        fix_code("```js\ncode\n```\n\nUse `variable"),
        "```js\ncode\n```\n\nUse `variable`",
        "case 0"
    );
    assert_eq!(
        fix_code("Text `inline` code\n\n```js\nconst x = 1"),
        "Text `inline` code\n\n```js\nconst x = 1\n```",
        "case 1"
    );
}

#[test]
fn generated_fix_strong_strong_asterisk() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(
        fix_strong("Hello **world", &_opts),
        "Hello **world**",
        "case 0"
    );
    assert_eq!(
        fix_strong("Hello **world*", &_opts),
        "Hello **world**",
        "case 1"
    );
    assert_eq!(
        fix_strong("Hello **world**", &_opts),
        "Hello **world**",
        "case 2"
    );
    assert_eq!(fix_strong("**", &_opts), "", "case 3");
    assert_eq!(fix_strong("*", &_opts), "", "case 4");
    assert_eq!(fix_strong("Hello\n\n**", &_opts), "Hello", "case 5");
    assert_eq!(
        fix_strong("Para1 **unclosed\n\nPara2 **text", &_opts),
        "Para1 **unclosed\n\nPara2 **text**",
        "case 6"
    );
    assert_eq!(
        fix_strong("Hello **world\nand more text", &_opts),
        "Hello **world\nand more text**",
        "case 7"
    );
    assert_eq!(
        fix_strong("**asterisk and __underscore", &_opts),
        "**asterisk and __underscore__**",
        "case 8"
    );
    assert_eq!(
        fix_strong("**bold and *mixed", &_opts),
        "**bold and *mixed***",
        "case 9"
    );
    assert_eq!(
        fix_strong("```js\n**bold", &_opts),
        "```js\n**bold",
        "case 10"
    );
    assert_eq!(
        fix_strong("Text ```ignore **inside``` and **open", &_opts),
        "Text ```ignore **inside``` and **open**",
        "case 11"
    );
    assert_eq!(
        fix_strong("**out [x](http://a/**b)", &_opts),
        "**out [x](http://a/**b)",
        "case 12"
    );
    assert_eq!(fix_strong("***", &_opts), "*", "case 13");
    assert_eq!(fix_strong("a\n- **", &_opts), "a\n", "case 14");
    assert_eq!(fix_strong("** *", &_opts), "*", "case 15");
}

#[test]
fn generated_fix_strong_strong_underscore() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(
        fix_strong("Hello __world", &_opts),
        "Hello __world__",
        "case 0"
    );
    assert_eq!(fix_strong("a__b", &_opts), "a__b", "case 1");
    assert_eq!(fix_strong("a\\__b", &_opts), "a\\__b", "case 2");
    assert_eq!(
        fix_strong("Hello __world_", &_opts),
        "Hello __world__",
        "case 3"
    );
    assert_eq!(fix_strong("`a__b`", &_opts), "`a__b`", "case 4");
    assert_eq!(
        fix_strong("`a__b` and __bold", &_opts),
        "`a__b` and __bold__",
        "case 5"
    );
    assert_eq!(
        fix_strong("Hello __world__", &_opts),
        "Hello __world__",
        "case 6"
    );
    assert_eq!(fix_strong("__", &_opts), "", "case 7");
    assert_eq!(fix_strong("_", &_opts), "", "case 8");
    assert_eq!(fix_strong("Hello\n\n__", &_opts), "Hello", "case 9");
    assert_eq!(
        fix_strong("Para1 __unclosed\n\nPara2 __text", &_opts),
        "Para1 __unclosed\n\nPara2 __text__",
        "case 10"
    );
    assert_eq!(
        fix_strong("Hello __world\nand more text", &_opts),
        "Hello __world\nand more text__",
        "case 11"
    );
    assert_eq!(
        fix_strong("__underscore and **asterisk", &_opts),
        "__underscore and **asterisk**__",
        "case 12"
    );
    assert_eq!(
        fix_strong("__bold and _mixed", &_opts),
        "__bold and _mixed___",
        "case 13"
    );
    assert_eq!(
        fix_strong("Text ```ignore __inside``` and __open", &_opts),
        "Text ```ignore __inside``` and __open__",
        "case 14"
    );
    assert_eq!(
        fix_strong("__out [x](http://a/__b)", &_opts),
        "__out [x](http://a/__b)",
        "case 15"
    );
    assert_eq!(fix_strong("___", &_opts), "_", "case 16");
    assert_eq!(fix_strong("a\n- __", &_opts), "a\n", "case 17");
    assert_eq!(fix_strong("__ _", &_opts), "", "case 18");
    assert_eq!(
        fix_strong("```\nconst x = **value\n```", &_opts),
        "```\nconst x = **value\n```",
        "case 19"
    );
    assert_eq!(
        fix_strong("```\nconst x = __value\n```", &_opts),
        "```\nconst x = __value\n```",
        "case 20"
    );
    assert_eq!(
        fix_strong("```\nconst x = **value\n```", &_opts),
        "```\nconst x = **value\n```",
        "case 21"
    );
    assert_eq!(
        fix_strong("```\ncode\n```\n\nText **bold", &_opts),
        "```\ncode\n```\n\nText **bold**",
        "case 22"
    );
    assert_eq!(
        fix_strong("The formula is $$x = 1 + 2**3", &_opts),
        "The formula is $$x = 1 + 2**3",
        "case 23"
    );
    assert_eq!(
        fix_strong("The formula is $$x = 1 + 2__3", &_opts),
        "The formula is $$x = 1 + 2__3",
        "case 24"
    );
    assert_eq!(
        fix_strong(
            "Text **bold** and [link](https://example.com/page_with_underscore) **more",
            &_opts
        ),
        "Text **bold** and [link](https://example.com/page_with_underscore) **more**",
        "case 25"
    );
    assert_eq!(
        fix_strong(
            "Text __bold__ and [link](https://example.com/page_with_underscore) __more",
            &_opts
        ),
        "Text __bold__ and [link](https://example.com/page_with_underscore) __more__",
        "case 26"
    );
}

#[test]
fn generated_fix_emphasis_emphasis_asterisk() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(fix_emphasis("*"), "", "case 0");
    assert_eq!(fix_emphasis("Hello *world"), "Hello *world*", "case 1");
    assert_eq!(
        fix_emphasis("Hello *world\nand more text"),
        "Hello *world\nand more text*",
        "case 2"
    );
    assert_eq!(fix_emphasis("Hello *world*"), "Hello *world*", "case 3");
    assert_eq!(fix_emphasis("Hello\n\n*"), "Hello", "case 4");
    assert_eq!(
        fix_emphasis("**bold** and *italic"),
        "**bold** and *italic*",
        "case 5"
    );
    assert_eq!(
        fix_emphasis("Para1 *unclosed\n\nPara2 *text"),
        "Para1 *unclosed\n\nPara2 *text*",
        "case 6"
    );
    assert_eq!(
        fix_emphasis("*asterisk and _underscore"),
        "*asterisk and _underscore_*",
        "case 7"
    );
    assert_eq!(fix_emphasis("```js\n*italic"), "```js\n*italic", "case 8");
    assert_eq!(
        fix_emphasis("*open and **closed**"),
        "*open and **closed***",
        "case 9"
    );
    assert_eq!(fix_emphasis("a\n- *"), "a\n", "case 10");
}

#[test]
fn generated_fix_emphasis_emphasis_underscore() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(fix_emphasis("_"), "", "case 0");
    assert_eq!(fix_emphasis("Hello _world"), "Hello _world_", "case 1");
    assert_eq!(
        fix_emphasis("Hello _world\nand more text"),
        "Hello _world\nand more text_",
        "case 2"
    );
    assert_eq!(fix_emphasis("Hello _world_"), "Hello _world_", "case 3");
    assert_eq!(fix_emphasis("Hello\n\n_"), "Hello", "case 4");
    assert_eq!(
        fix_emphasis("__bold__ and _italic"),
        "__bold__ and _italic_",
        "case 5"
    );
    assert_eq!(fix_emphasis("a_b"), "a_b", "case 6");
    assert_eq!(fix_emphasis("a\\_b"), "a\\_b", "case 7");
    assert_eq!(fix_emphasis("`a_b`"), "`a_b`", "case 8");
    assert_eq!(
        fix_emphasis("`a_b` and _italic"),
        "`a_b` and _italic_",
        "case 9"
    );
    assert_eq!(
        fix_emphasis("Para1 _unclosed\n\nPara2 _text"),
        "Para1 _unclosed\n\nPara2 _text_",
        "case 10"
    );
    assert_eq!(
        fix_emphasis("_underscore and *asterisk"),
        "_underscore and *asterisk*_",
        "case 11"
    );
    assert_eq!(
        fix_emphasis("```\nconst x = *value\n```"),
        "```\nconst x = *value\n```",
        "case 12"
    );
    assert_eq!(
        fix_emphasis("```\nconst x = _value\n```"),
        "```\nconst x = _value\n```",
        "case 13"
    );
    assert_eq!(
        fix_emphasis("```\nconst x = *value\n```"),
        "```\nconst x = *value\n```",
        "case 14"
    );
    assert_eq!(
        fix_emphasis("```\ncode\n```\n\nText *italic"),
        "```\ncode\n```\n\nText *italic*",
        "case 15"
    );
    assert_eq!(
        fix_emphasis("The formula is $$x = 1 + 2*3"),
        "The formula is $$x = 1 + 2*3",
        "case 16"
    );
    assert_eq!(
        fix_emphasis("The formula is $$x = 1 + 2_3"),
        "The formula is $$x = 1 + 2_3",
        "case 17"
    );
    assert_eq!(
        fix_emphasis("Text *italic* and [link](https://example.com/page*value) *more"),
        "Text *italic* and [link](https://example.com/page*value) *more*",
        "case 18"
    );
    assert_eq!(
        fix_emphasis("Text _italic_ and [link](https://example.com/page_with_underscore) _more"),
        "Text _italic_ and [link](https://example.com/page_with_underscore) _more_",
        "case 19"
    );
    assert_eq!(fix_emphasis("<file id=\"test\" name=\"test.txt\" url=\"http://example.com/path_with_underscore?param=value\" size=\"135\" />"), "<file id=\"test\" name=\"test.txt\" url=\"http://example.com/path_with_underscore?param=value\" size=\"135\" />", "case 20");
    assert_eq!(fix_emphasis("<file id=\"test\" name=\"test.txt\" url=\"http://example.com/path*value?param=test\" size=\"135\" />"), "<file id=\"test\" name=\"test.txt\" url=\"http://example.com/path*value?param=test\" size=\"135\" />", "case 21");
    assert_eq!(
        fix_emphasis("_open and __closed__"),
        "_open and __closed___",
        "case 22"
    );
    assert_eq!(fix_emphasis("a\n- _"), "a\n", "case 23");
    assert_eq!(fix_emphasis("```js\n_italic"), "```js\n_italic", "case 24");
}

#[test]
fn generated_fix_delete_delete() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_link_link() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_link_image() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_inline_math_inline_math() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(
        fix_inline_math("The formula is $$x = 1"),
        "The formula is $$x = 1$$",
        "case 0"
    );
    assert_eq!(
        fix_inline_math("The formula is $$x = 1$"),
        "The formula is $$x = 1$$",
        "case 1"
    );
    assert_eq!(
        fix_inline_math("The formula is $$x = 1$$"),
        "The formula is $$x = 1$$",
        "case 2"
    );
    assert_eq!(fix_inline_math("$$"), "", "case 3");
    assert_eq!(fix_inline_math("$"), "", "case 4");
    assert_eq!(fix_inline_math("Text $$"), "Text", "case 5");
    assert_eq!(fix_inline_math("Hello\n\n$$"), "Hello", "case 6");
    assert_eq!(
        fix_inline_math("Para1 $$x$$\n\nPara2 $$y"),
        "Para1 $$x$$\n\nPara2 $$y$$",
        "case 7"
    );
    assert_eq!(
        fix_inline_math("Hello $$world\nand more text"),
        "Hello $$world\nand more text",
        "case 8"
    );
    assert_eq!(
        fix_inline_math("$$\\int u \\, dv = uv - \\int v \\, du$"),
        "$$\\int u \\, dv = uv - \\int v \\, du$$",
        "case 9"
    );
    assert_eq!(
        fix_inline_math("Para1 $$x$$\n\nPara2 $$y = 1$"),
        "Para1 $$x$$\n\nPara2 $$y = 1$$",
        "case 10"
    );
    assert_eq!(
        fix_inline_math("Text `$$` and $$x = 1"),
        "Text `$$` and $$x = 1$$",
        "case 11"
    );
    assert_eq!(
        fix_inline_math("```\n$$x = 1\n```"),
        "```\n$$x = 1\n```",
        "case 12"
    );
    assert_eq!(
        fix_inline_math("Wrap inline mathematical expressions with `$$`:"),
        "Wrap inline mathematical expressions with `$$`:",
        "case 13"
    );
    assert_eq!(
        fix_inline_math(
            "The sum of the first $$n$$ natural numbers: $$\\sum_{i=1}^{n} i = \\frac{n("
        ),
        "The sum of the first $$n$$ natural numbers: $$\\sum_{i=1}^{n} i = \\frac{n($$",
        "case 14"
    );
    assert_eq!(
        fix_inline_math("Formula: $$x_{i}^{2} + y_{j}"),
        "Formula: $$x_{i}^{2} + y_{j}$$",
        "case 15"
    );
    assert_eq!(
        fix_inline_math("Equation: $$\\int_{a}^{b} f(x) \\, dx = F(b) - F(a"),
        "Equation: $$\\int_{a}^{b} f(x) \\, dx = F(b) - F(a$$",
        "case 16"
    );
    assert_eq!(
        fix_inline_math("The premium plan costs $7,000 and includes **priority support"),
        "The premium plan costs $7,000 and includes **priority support",
        "case 17"
    );
    assert_eq!(
        fix_inline_math("```js\n$$x = 1"),
        "```js\n$$x = 1",
        "case 18"
    );
    assert_eq!(fix_inline_math("$$$"), "$$$", "case 19");
    assert_eq!(fix_inline_math("`$$x"), "`$$x", "case 20");
    assert_eq!(
        fix_inline_math("```ignore $$``` and $$x = 1"),
        "```ignore $$``` and $$x = 1$$",
        "case 21"
    );
}

#[test]
fn generated_fix_math_math() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_table_table() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_task_list_task_list() {
    let _opts = crate::preprocess::PreprocessOptions::default();
    assert_eq!(fix_task_list("- [ ] Task 1\n-"), "- [ ] Task 1", "case 0");
    assert_eq!(fix_task_list("- [X] Task 1\n-"), "- [X] Task 1", "case 1");
    assert_eq!(
        fix_task_list("- [ ] Task 1\n- [x] Task 2\n-"),
        "- [ ] Task 1\n- [x] Task 2",
        "case 2"
    );
    assert_eq!(
        fix_task_list("- [ ] Task 1\n- [x] Task 2"),
        "- [ ] Task 1\n- [x] Task 2",
        "case 3"
    );
    assert_eq!(fix_task_list("-"), "", "case 4");
    assert_eq!(fix_task_list("- ["), "", "case 5");
    assert_eq!(fix_task_list("- [ ] Task 1\n- "), "- [ ] Task 1", "case 6");
    assert_eq!(fix_task_list("- [ ] Task 1\n- ["), "- [ ] Task 1", "case 7");
    assert_eq!(fix_task_list("> - ["), "", "case 8");
    assert_eq!(fix_task_list("> -"), "", "case 9");
    assert_eq!(
        fix_task_list("```\n- task item\n```"),
        "```\n- task item\n```",
        "case 10"
    );
    assert_eq!(
        fix_task_list("```\n- task item\n```"),
        "```\n- task item\n```",
        "case 11"
    );
    assert_eq!(
        fix_task_list("```\ncode\n```\n\n- [ ] Task"),
        "```\ncode\n```\n\n- [ ] Task",
        "case 12"
    );
    assert_eq!(
        fix_task_list("```\ncode\n```\n\n-"),
        "```\ncode\n```\n",
        "case 13"
    );
    assert_eq!(fix_task_list("```js\n- ["), "```js\n- [", "case 14");
    assert_eq!(fix_task_list("- [ ] Task 1\n"), "- [ ] Task 1\n", "case 15");
}

#[test]
fn generated_fix_footnote_footnote() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}

#[test]
fn generated_fix_html_html() {
    let _opts = crate::preprocess::PreprocessOptions::default();
}
