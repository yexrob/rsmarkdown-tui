# rsmarkdown-tui

Streaming markdown, in Rust — a faithful re-implementation of
[jinghaihan/vue-stream-markdown](https://github.com/jinghaihan/vue-stream-markdown)
(itself derived from [vercel/streamdown](https://github.com/vercel/streamdown)),
with the core and display layers separated.

Markdown that arrives *incrementally* (LLM output, typing, pipes) is kept
renderable at every intermediate state: incomplete syntax is self-healed on
the fly, completed blocks are never re-parsed, and only the trailing block is
re-rendered.

## Layout

```
crates/
  core/   rsmarkdown-core — pipeline, zero terminal dependencies
  tui/    rsmarkdown-tui  — ratatui display-layer adapter + interactive demo
```

### `rsmarkdown-core`

```text
raw content
  -> normalize                 CRLF, trailing whitespace, LaTeX pre-processing
  -> parse_markdown_into_blocks   stable block segmentation (footnotes collapse
                                  the document; unclosed HTML / $$ merge blocks)
  -> preprocess LAST block only   syntax self-healing (11 fix steps, ordered)
  -> parse each block to AST      pulldown-cmark bridge, LRU-cached (cap 100)
  -> Document                     handed to any display adapter (Renderer trait)
```

```rust
use rsmarkdown_core::{MarkdownProcessor, Mode, Renderer};

let mut processor = MarkdownProcessor::default();
let mut renderer  = MyRenderer::new();

for chunk in stream {                       // whatever your stream is
    processor.process_streaming(&chunk)     // incremental re-parse
        .into_iter().for_each(|_| {});      // blocks + ASTs
}
```

The `preprocess` steps (in order): `code`, `html`, `footnote`, `strong`,
`emphasis`, `delete`, `taskList`, `link`, `table`, `inlineMath`, `math` —
ported one-to-one from `markmend/core/src/preprocess/*.ts`, including
exclusions (markers inside code blocks, URLs, math, HTML tags) and removal
fallbacks for bare markers.

### `rsmarkdown-tui` — a component TUI framework

The TUI is a small component framework, not a markdown viewer:

```
crates/tui/src/
  component.rs        Component trait (draw / event / on_tick / status / hints)
  app.rs              App host: event loop, focus routing, status bar
  components/         pluggable panes:
    markdown.rs         streaming markdown viewer (uses rsmarkdown-core)
    text.rs             plain-text streaming log (non-markdown component)
    list.rs             selectable task list (interactive component)
  renderer/           StreamMarkdownRenderer: AST -> styled lines (per-block cache)
```

`StreamMarkdownRenderer` implements `rsmarkdown_core::Renderer`: it converts
each block's AST into styled terminal lines with **per-block caching** — only
blocks whose source changed are re-rendered, mirroring the original's memoized
`Block` components. Any component can be mounted into the host.

```
cargo run -p rsmarkdown-tui
```

- `[1] markdown` — streams a demo document chunk-by-chunk (simulated LLM
  output); `t` types markdown yourself and watches unclosed `**bold`, fences,
  tables, task lists get completed live; `p` loads a ~200 KB stress document
- `[2] log` — a streaming plain-text log (proves rendering is not
  markdown-specific)
- `[3] tasks` — a selectable task list (`space` toggles)
- `Tab` / `[` `]` / `1-3` switch focus, `j/k/PgUp/PgDn/g/G`/mouse wheel scroll,
  `s` auto-scroll, `q` quits

Headless checks (no terminal required):

```
cargo run -p rsmarkdown-tui --example headless   # full markdown pipeline
cargo test -p rsmarkdown-tui                     # incl. component smoke tests
```

## Performance

Two tools:

```
cargo bench -p rsmarkdown-core          # criterion benchmarks
cargo run  -p rsmarkdown-tui --example perf   # headless report demo
```

In the TUI, press `p` to instantly load a ~200 KB stress document; the status
bar shows the parse time (µs) and cache hits for every update.

Representative numbers (release build, Apple silicon):

```
doc size         blocks     parse (ms)           MB/s
2 KB                 45           0.62            3.2
64 KB              1349           1.39           45.9
512 KB            10767          10.18           50.3

incremental streaming: 64 KB doc, 1026 chunks of 64 B
  block-cached           710 ms   ~690 µs/chunk   cache hits 100%
  naive full-reparse    ~1000 ms   ~890 µs/chunk
```

Where the time goes on a 512 KB document (per stream update):

```
normalize            4.6 ms  46%   full-text scan (CRLF, LaTeX rewrites)
block split          1.9 ms  19%   full-text scan
preprocess + parse   3.4 ms  34%   only the trailing block (LRU-cached)
```

The block cache turns the AST parse — the single most expensive step — from
O(document) into O(tail) per update, with a 100% hit rate on completed blocks.
The remaining O(document) per-update cost is the normalize + block-split scan,
the same architecture the original project uses. At interactive streaming
rates (~33 ms/frame) a 512 KB document still leaves ~70% of the frame budget
free. `normalize` skips all rewriting when no `$`/`\`/CRLF are present.

## Fidelity

The core ships with the original project's own test suite: 145 assertions
extracted from `test/markmend/preprocess/test-cases.ts` (generated by
`scripts/gen-tests.cjs`, verified against a verbatim JS port of `fixStrong`),
plus hand-written pipeline/parser/block tests:

```
cargo test
```

## Notes on differences

- **Math**: the original's `preprocessLaTeX` mangles single-line `$$x$$` into
  `$$$x$$` (its `$` rewrite also re-processes the `$$` it just inserted),
  breaking single-line formulas. Ours skips `$` characters that are part of an
  existing `$$` pair, so `$$x$$`, `\(x\)` and `\[x\]` all normalize to clean
  `$$x$$`; the single-dollar `$x$` -> `$$x$` rewrite is kept as in the original.

## Terminal math display

Math nodes are rendered as **Unicode text** (`crates/tui/src/renderer/math.rs`),
the approach used by terminal markdown viewers (innomd, latex-terminal):
LaTeX is converted to Unicode glyphs, so

```
$$\int_a^b f(x)\,dx = F(b) - F(a)$$     ->     ∫ₐᵇ f(x) dx = F(b) − F(a)
$$\sum_{i=1}^n i = rac{n(n+1)}{2}$$  ->     Σᵢ₌₁ⁿ i = n(n+1)⁄2
```

Covered: Greek letters, operators (∫ Σ ∏ √ …), Unicode sub/superscripts with
`_(...)`/`^(...)` fallback for missing glyphs, `rac` as `⁄` fractions,
blackboard letters (`\mathbb{R}` -> ℝ), matrices as `(a  b ; c  d)`, accents,
`	ext{}`, and environments. Unknown commands degrade to their plain name.

Alternatives considered: terminal graphics protocols (kitty/sixel/iTerm2
inline images via `ratatui-image`) give true typesetting but need a
math-to-image backend (no mature Rust one; KaTeX requires Node), split
terminal support, and are expensive per-chunk in streaming; block-character
self-rendering (texterm/termula) is a full math layout engine. Unicode text
is zero-dependency, terminal-agnostic, and deterministic — the right fit for
streaming TUI rendering.
- Block tokenization approximates `marked`'s lexer (blank-line runs + fences
  + boundary lines) rather than depending on `marked` itself; block strings
  keep `marked`'s trailing-newline `raw` semantics.
- The parse bridge uses `pulldown-cmark` (the Rust equivalent of
  `mdast-util-from-markdown`), exposed through a small, stable renderer-facing
  AST — display adapters never see pulldown types.
- Rendering is terminal-native: bold/italic/strike/code/link/math styling,
  CJK-aware wrapping and table column fitting (`unicode-width`).
