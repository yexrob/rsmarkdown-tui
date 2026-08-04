# rsmarkdown-tui

A Rust re-implementation of **streaming markdown**, ported from
[jinghaihan/vue-stream-markdown](https://github.com/jinghaihan/vue-stream-markdown).
Core and display layer are separated: the core crate has zero terminal
dependencies.

## How it works

The pipeline mirrors the original:

1. `normalize` — CRLF cleanup, trailing whitespace, LaTeX pre-processing
2. `parse_markdown_into_blocks` — stable block segmentation
3. `preprocess` — self-heal *incomplete* syntax on the streaming tail:
   unclosed `**bold`, `*italic*`, ~~strikethrough~~, inline `code`, code fences
4. parse each block to an AST, cached with an LRU (only the tail is new)

As content streams in, completed blocks never change — only the last one is
re-parsed. Try typing yourself: press `t` and start writing markdown with
unclosed markers; watch them get completed live.

## A table

| Feature | Status | Note |
| --- | --- | --- |
| Block streaming | done | stable prefixes |
| Code fences | done | completed while streaming |
| Tables | done | separator rows auto-fixed |
| Task lists | done | `- [ ]` / `- [x]` |
| Math | done | `$$...$$` |

## Lists and quotes

- Block parsing is *incremental*
- The renderer only re-paints changed blocks
- Blockquotes keep their shape:

> Streaming markdown is about *stable* intermediate states,
> not just the final document.

A code block:

```rust
fn main() {
    println!("hello streaming");
}
```

$$\int_a^b f(x)\,dx = F(b) - F(a)$$

$$\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$$

$$\lambda = \frac{b}{T} \quad \mathbb{R}^n \to \mathbb{R}$$

任务列表与中文混排测试:

- [x] 已完成事项
- [ ] 未完成事项
