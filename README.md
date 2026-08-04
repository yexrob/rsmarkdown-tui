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
  activities.rs            agent hints: thinking blocks + tool calls (framework-level)
  image.rs            image backend plumbing (protocol detection, slicing)
  components/         pluggable panes:
    markdown.rs         streaming markdown viewer (uses rsmarkdown-core)
    image.rs            scrollable image viewer
    chat.rs             agent session: thinking + tool hints + markdown replies
    text.rs             plain-text streaming log (non-markdown component)
    list.rs             selectable task list (interactive component)
  renderer/           StreamMarkdownRenderer: AST -> styled lines (per-block cache)
```

## Agent hints (Claude Code style)

The framework ships hint types and rendering in `activities.rs` — any component can
hold `Vec<AgentHint>` and render them with `hint_lines`:

```
⠋ thinking · 1.4s                        (running: braille spinner)
─ thinking · 2.3s · checking imports ─    (done: collapsed, dim)
⠙ bash cargo test -p core                (tool running: cyan)
✓ bash cargo test -p core · 900ms         (done: green, with duration)
✗ curl fetch https://x · 3000ms · exit 7  (error: red)
… 54 passed · finished in 0.02s           (output preview line, dim)
```

The `[3] chat` component runs a scripted agent session: type a message
(`[enter]` to send), watch it think (spinner + collapsed digest), call tools
(`bash`, `read`), then stream a markdown reply through the same display
adapter as the markdown component. Consecutive hints are deduplicated by
identity (tool name / thinking stage) so running -> done updates replace the
right line.

## Terminal image backend

Images render through the best protocol the terminal supports (detected via
`Picker::from_query_stdio`): **kitty graphics** (kitty, ghostty, wezterm…),
sixel, iTerm2, with a unicode half-blocks fallback everywhere else (pure
text — works headless and in CI).

Scroll correctness comes from two layers:

- [`SlicedImage`](ratatui-image) renders only the visible rows of a
  partially-scrolled image (skip/drop), so position is exact at any scroll
  offset — including images *inside* the markdown document flow
  (`![alt](path)` paragraphs, or the generated `demo://gradient`).
- The kitty backend uses **unicode placeholders**, so the terminal keeps the
  picture attached to its cells as the viewport scrolls — no ghost images,
  no manual erase/replace.

The markdown component lays out documents as text lines + image blocks; the
demo doc includes one image, and `[2] image` is a standalone scrollable
viewer component. Headless tests (`tests/image.rs`) verify scroll shifting,
off-screen clipping and image/text interleaving using the half-blocks
protocol.

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
- `[2] image` — scrollable image via the terminal graphics backend
- `[3] chat` — agent session with thinking/tool hints + markdown replies
- `[4] log` — a streaming plain-text log (proves rendering is not
  markdown-specific)
- `[5] tasks` — a selectable task list (`space` toggles)
- `Tab` / `[` `]` / `1-3` switch focus, `j/k/PgUp/PgDn/g/G`/mouse wheel scroll,
  `s` auto-scroll, `q` quits

Headless checks (no terminal required):

```
cargo run -p rsmarkdown-tui --example headless   # full markdown pipeline
cargo test -p rsmarkdown-tui                     # incl. component smoke tests
```

## Using as a library

The TUI is a library, not a markdown viewer — the demo binary only consumes
it. Custom apps assemble [`App`] with their own [`Component`]s:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use rsmarkdown_tui::{App, Component, run_tui};

struct Hello;

impl Component for Hello {
    fn title(&self) -> &str { "hello" }
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, "hello", ratatui::style::Style::default());
    }
}

let mut app = App::new(vec![Box::new(Hello)]);
run_tui(&mut app)?;
```

The public API is fully documented (`cargo doc -p rsmarkdown-tui --open`):
activity model, footer badges, image backend, and the markdown renderer are
all reusable pieces. `crates/tui/examples/custom.rs` shows a self-contained
custom component with `+`/`-` interaction and a footer badge.

## Permission dialog

`App::ask` (or a component's `on_ask`) raises a Claude Code style modal: numbered options with a `❯` selection marker, `Esc` to cancel, Enter / digit / double-click to confirm, and an optional pre-rendered content preview — the caller supplies styled lines, so the demo feeds it with the activity diff renderer (capped at 8 rows).

## Command menu & help

Typing `/` into an empty prompt opens a filterable command menu (arrow keys
move, Enter confirms, Esc closes, mouse click selects / double-click
confirms); `?` toggles a grouped keybinding panel. Both are reusable
library pieces (`SlashCommandMenu`, `HelpPanel`); the demo wires `/clear`
and `/help`.

## Agent overview

`AgentView` renders a Claude Code Agent View style session table (Pinned /
Ready for review / Needs input / Working / Completed, status icons with a
Working animation, PR + age columns, collapsed `… N more` tails, transcript
and peek overlays). The host broadcasts agent state between components:
any component publishes [`Component::agents`], every component receives the
merged table via [`Component::absorb_agents`] — the demo chat feeds its
subagents to the overview with no coupling.

## Expand / collapse policy

Activities auto-expand while active (running thinking / tool / subagent)
and **collapse back when finished** — a click reopens them, and a manual
click survives subsequent updates. Todo checklists deliberately render as
ordinary blocks in the document flow (never pinned or auto-expanded): the
header stays in the transcript and scrolls with the content; a click
reveals the priority-folded window — finished items collapse to a leading
`… N done`, the window shows the in-progress + next unfinished items (at
most 5), and the rest collapse to a trailing `… +N more`.

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
