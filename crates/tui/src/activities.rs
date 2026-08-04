//! Claude Code-style agent hints: thinking blocks and tool calls.
//!
//! Both share one abstraction: an [`Activity`] is a foldable pane — a collapsed
//! header line plus optional expandable content. The only difference between
//! thinking and tool calls is their *presentation* ([`ActivityKind`]): thinking
//! reveals reasoning text, tools reveal the command and its output.
//!
//! ```text
//! ✓ bash cargo test -p rsmarkdown-core · 900ms     (collapsed)
//! ─ thinking · 2.3s · checking imports ─           (expanded)
//!     checking the block splitter for fence states
//!     the tail block is the only one that changes
//! ```

use ratatui::text::{Line, Span};

use crate::renderer::theme;

/// Spinner frames (braille), cycled by the host tick.
pub const SPINNERS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Braille spinner frame for the given tick.
pub fn spinner(frame: u64) -> char {
    SPINNERS[(frame as usize) % SPINNERS.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Lifecycle of a tool call.
pub enum ToolStatus {
    /// The tool is executing.
    Running,
    /// The tool finished successfully.
    Done,
    /// The tool failed.
    Error,
}

/// A tool invocation: `✓ bash · cargo test · 12ms`.
#[derive(Debug, Clone)]
/// A tool invocation shown in a transcript.
/// Header: `✓ bash · cargo test · 12ms`; expanded content shows the
/// command and its output.

pub struct ToolCall {
    /// Tool name (e.g. `bash`, `Edit`).
    pub name: &'static str,
    /// Running / done / error.
    pub status: ToolStatus,
    /// Brief command / argument summary.
    /// Brief command / argument summary.
    pub summary: String,
    /// Elapsed time in milliseconds.

    /// Elapsed time in milliseconds.

    /// Elapsed time in milliseconds.
    pub duration_ms: u64,
    /// One-line output preview shown next to the header.
    /// One-line output preview shown in the header.
    pub output: Option<String>,
}

impl ToolCall {
    /// Start a running tool call.

    pub fn running(name: &'static str, summary: impl Into<String>) -> Self {
        Self {
            name,
            status: ToolStatus::Running,
            summary: summary.into(),
            duration_ms: 0,
            output: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether a reasoning block is still running.

pub enum ThinkingState {
    /// Still reasoning.
    Running,
    /// Reasoning finished.
    Done,
}

/// A reasoning block: `⠋ thinking · 1.4s` -> `─ thinking · 1.4s ─`.
#[derive(Debug, Clone)]
/// A reasoning block: `⠋ thinking · 1.4s` -> `─ thinking · 1.4s ─`.

pub struct Thinking {
    /// Running or done.
    pub state: ThinkingState,
    /// Elapsed time in milliseconds.
    pub duration_ms: u64,
    /// Collapsed digest shown once done.
    /// Collapsed digest shown once done.
    pub digest: Option<String>,
    /// Distinguishes consecutive reasoning stages so running/done updates
    /// replace the right hint.
    /// Distinguishes consecutive reasoning stages so running/done updates
    /// replace the right hint.
    pub stage: &'static str,
}

/// Todo lifecycle (mirrors Claude Code: pending -> in_progress -> completed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Todo lifecycle (pending -> in_progress -> completed).

pub enum TodoStatus {
    /// Not started.
    Pending,
    /// Being worked on.
    InProgress,
    /// Completed.
    Done,
}

#[derive(Debug, Clone)]
/// One checklist item.

pub struct TodoItem {
    /// Item text.
    pub text: String,
    /// Current lifecycle state.
    pub status: TodoStatus,
}

/// A task checklist, rendered as `- [ ]` / `- [⠋]` / `- [x]` items.
#[derive(Debug, Clone)]
/// A task checklist, rendered as `- [ ]` / `- [⠋]` / `- [x]` items.

pub struct TodoList {
    /// Checklist title.
    pub title: String,
    /// Items in order.
    pub items: Vec<TodoItem>,
}

impl TodoList {
    /// Number of completed items.

    pub fn done(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Done)
            .count()
    }
    /// Total item count.

    pub fn total(&self) -> usize {
        self.items.len()
    }
    /// Update one item's status.

    pub fn set(&mut self, index: usize, status: TodoStatus) {
        if let Some(item) = self.items.get_mut(index) {
            item.status = status;
        }
    }
}

/// A delegated subagent (Claude Code renders these as a tool-call row:
/// `<agent name> (<task description>)`).
///
/// The subagent's work is **nested**: its expanded view shows its own
/// transcript — the same activity kinds as the main agent (todos, thinking,
/// tool calls, …) plus a final markdown reply.
#[derive(Debug, Clone)]
/// A delegated subagent (`<agent name> (<task description>)`).
///
/// Its expanded view shows a **nested transcript** — the same activity
/// kinds as the main agent (todos, thinking, tool calls) plus a final
/// markdown reply.

pub struct SubAgent {
    /// Agent name (e.g. `explore`).
    pub name: String,
    /// Running / done / error.
    pub status: SubAgentStatus,
    /// Short task description shown next to the name.
    /// Short task description shown next to the name.
    pub task: String,
    /// Elapsed time in milliseconds.
    pub duration_ms: u64,
    /// One-line result preview once done.
    /// One-line result preview once done.
    pub result: Option<String>,
    /// The subagent's own transcript (recursively foldable).
    /// The subagent's own transcript (recursively foldable).
    pub transcript: Vec<Activity>,
    /// The subagent's final markdown reply.
    /// The subagent's final markdown reply.
    pub reply: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Lifecycle of a delegated subagent.

pub enum SubAgentStatus {
    /// The agent is working.
    Running,
    /// The agent finished.
    Done,
    /// The agent failed.
    Error,
}

impl SubAgent {
    /// Start a running subagent.

    pub fn running(name: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: SubAgentStatus::Running,
            task: task.into(),
            duration_ms: 0,
            result: None,
            transcript: Vec::new(),
            reply: String::new(),
        }
    }
}

/// Whether an activity should be shown expanded while it is active:
/// a todo with an in-progress item, or a running subagent.
pub fn auto_expand(h: &Activity) -> bool {
    match &h.kind {
        ActivityKind::Todo(t) => t.items.iter().any(|i| i.status == TodoStatus::InProgress),
        ActivityKind::SubAgent(a) => a.status == SubAgentStatus::Running,
        _ => false,
    }
}

/// One line of a unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
/// One line of a unified diff.

pub enum DiffLine {
    /// Unchanged context line.
    Context(String),
    /// A removed (`-`) line.
    Removed(String),
    /// An added (`+`) line.
    Added(String),
}

/// One hunk of a unified diff.
#[derive(Debug, Clone)]
pub struct Hunk {
    /// `@@ -a,b +c,d @@`
    /// `@@ -a,b +c,d @@`
    pub header: String,
    /// Context / removed / added lines.
    pub lines: Vec<DiffLine>,
}

/// A file edit, rendered as a git-style unified diff.
#[derive(Debug, Clone)]
pub struct Diff {
    /// File path.
    pub path: String,
    /// Hunks in order.
    pub hunks: Vec<Hunk>,
}

impl Diff {
    /// Count added / removed lines across all hunks.
    pub fn stats(&self) -> (usize, usize) {
        let mut added = 0;
        let mut removed = 0;
        for hunk in &self.hunks {
            for line in &hunk.lines {
                match line {
                    DiffLine::Added(_) => added += 1,
                    DiffLine::Removed(_) => removed += 1,
                    DiffLine::Context(_) => {}
                }
            }
        }
        (added, removed)
    }

    /// Parse a unified diff (git format: `---` / `+++` / `@@` / `-` `+` ` `).
    pub fn parse_unified(text: &str) -> Self {
        let mut path = String::new();
        let mut hunks: Vec<Hunk> = Vec::new();
        for line in text.lines() {
            if let Some(p) = line.strip_prefix("+++ b/") {
                path = p.to_string();
                continue;
            }
            if let Some(p) = line.strip_prefix("+++ ") {
                path = p.to_string();
                continue;
            }
            if let Some(header) = line.strip_prefix("@@") {
                let header = format!("@@{}", header);
                hunks.push(Hunk {
                    header,
                    lines: Vec::new(),
                });
                continue;
            }
            if line.starts_with("--- ") {
                continue;
            }
            if let Some(hunk) = hunks.last_mut() {
                if let Some(rest) = line.strip_prefix('-') {
                    if !rest.starts_with('-') {
                        hunk.lines.push(DiffLine::Removed(rest.to_string()));
                        continue;
                    }
                }
                if let Some(rest) = line.strip_prefix('+') {
                    if !rest.starts_with('+') {
                        hunk.lines.push(DiffLine::Added(rest.to_string()));
                        continue;
                    }
                }
                if let Some(rest) = line.strip_prefix(' ') {
                    hunk.lines.push(DiffLine::Context(rest.to_string()));
                } else if !line.is_empty() {
                    hunk.lines.push(DiffLine::Context(line.to_string()));
                }
            }
        }
        Self { path, hunks }
    }
}

/// Render a diff as styled lines: `@@` hunk headers, `-` red, `+` green.
pub fn diff_lines(d: &Diff) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for hunk in &d.hunks {
        out.push(Line::styled(
            hunk.header.clone(),
            crate::renderer::theme::diff_hunk(),
        ));
        for line in &hunk.lines {
            let (prefix, style) = match line {
                DiffLine::Context(_) => (" ", crate::renderer::theme::diff_context()),
                DiffLine::Removed(_) => ("-", crate::renderer::theme::diff_removed()),
                DiffLine::Added(_) => ("+", crate::renderer::theme::diff_added()),
            };
            out.push(Line::from(vec![
                ratatui::text::Span::styled(prefix.to_string(), style),
                ratatui::text::Span::styled(hunk_line_text(line), style),
            ]));
        }
    }
    out
}

fn hunk_line_text(line: &DiffLine) -> String {
    match line {
        DiffLine::Context(t) | DiffLine::Removed(t) | DiffLine::Added(t) => t.clone(),
    }
}

/// What a hint is about — the presentation differs, the foldable behavior
/// does not.
/// What an activity is about — the presentation differs, the foldable
/// behavior does not.
#[derive(Debug, Clone)]
pub enum ActivityKind {
    /// A reasoning block.
    Thinking(Thinking),
    /// A tool invocation.
    Tool(ToolCall),
    /// A delegated subagent with a nested transcript.
    SubAgent(SubAgent),
    /// A task checklist.
    Todo(TodoList),
    /// A file edit, shown as a unified diff.
    Diff(Diff),
}

impl ActivityKind {
    /// Identity used to deduplicate consecutive updates of the same hint.
    pub(crate) fn identity(&self) -> ActivityId {
        match self {
            ActivityKind::Thinking(t) => ActivityId::Thinking(t.stage),
            ActivityKind::Tool(t) => ActivityId::Tool(t.name),
            ActivityKind::SubAgent(a) => ActivityId::SubAgent(a.name.clone()),
            ActivityKind::Todo(t) => ActivityId::Todo(t.title.clone()),
            ActivityKind::Diff(d) => ActivityId::Diff(d.path.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityId {
    Thinking(&'static str),
    Tool(&'static str),
    SubAgent(String),
    Todo(String),
    Diff(String),
}

/// A foldable agent hint. Both thinking blocks and tool calls are this one
/// type: a header line plus expandable content.
#[derive(Debug, Clone)]
/// A foldable agent activity: a collapsed header plus expandable content.
///
/// Every kind (thinking, tool call, subagent, todo, diff) shares the same
/// expand/collapse capability; only the presentation differs.

pub struct Activity {
    /// Which kind of activity.
    pub kind: ActivityKind,
    /// Collapsed (`false`) or expanded (`true`).
    /// Collapsed (`false`) or expanded (`true`).
    pub expanded: bool,
    /// Full content revealed when expanded (reasoning text / tool I/O).
    /// Content revealed when expanded (reasoning text / tool I/O / items).
    pub content: Vec<Line<'static>>,
}

impl Activity {
    /// Create a collapsed activity.

    pub fn new(kind: ActivityKind) -> Self {
        Self {
            kind,
            expanded: false,
            content: Vec::new(),
        }
    }

    /// Expand or collapse.
    /// Expand or collapse.

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Whether expansion reveals anything at all.
    /// Whether expansion reveals anything at all.

    pub fn expandable(&self) -> bool {
        !self.content.is_empty()
    }

    /// Whether the activity is expanded.

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Set the expanded content.

    pub fn set_content(&mut self, content: Vec<Line<'static>>) {
        self.content = content;
    }
}

fn subagent_header(a: &SubAgent, spinner: char) -> Line<'static> {
    let (glyph, style) = match a.status {
        SubAgentStatus::Running => (spinner.to_string(), theme::tool_running()),
        SubAgentStatus::Done => ("✓".to_string(), theme::tool_done()),
        SubAgentStatus::Error => ("✗".to_string(), theme::tool_error()),
    };
    let mut spans = vec![
        Span::styled(format!("{} {} ", glyph, a.name), style),
        Span::styled(format!("({})", a.task), theme::text()),
    ];
    if a.status != SubAgentStatus::Running {
        spans.push(Span::styled(
            format!(" · {:.1}s", a.duration_ms as f64 / 1000.0),
            theme::dim(),
        ));
    }
    if let Some(r) = &a.result {
        if a.status != SubAgentStatus::Running {
            spans.push(Span::styled(format!(" · {}", r), theme::tool_output()));
        }
    }
    Line::from(spans)
}

/// Render a todo list as checklist items (expanded content of a Todo hint).
pub fn todo_lines(t: &TodoList, spinner: char) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for item in &t.items {
        let (marker, style) = match item.status {
            TodoStatus::Pending => (format!("[ ] "), theme::task_open()),
            TodoStatus::InProgress => (format!("[{}] ", spinner), theme::tool_running()),
            TodoStatus::Done => ("[x] ".to_string(), theme::task_done()),
        };
        out.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled(item.text.clone(), style),
        ]));
    }
    out
}

fn todo_header(t: &TodoList, spinner: char) -> Line<'static> {
    let marker = if t.items.iter().any(|i| i.status == TodoStatus::InProgress) {
        format!("{} ", spinner)
    } else {
        String::new()
    };
    Line::from(vec![
        Span::styled(format!("{}todo", marker), theme::tool_running()),
        Span::styled(format!(" · {}/{} tasks", t.done(), t.total()), theme::dim()),
        Span::styled(format!(" · {}", t.title), theme::dim()),
    ])
}

fn diff_header(d: &Diff) -> Line<'static> {
    let (added, removed) = d.stats();
    Line::from(vec![
        Span::styled("✻ Edit · ", theme::diff_edit()),
        Span::styled(d.path.clone(), theme::text()),
        Span::styled(format!(" · +{} −{}", added, removed), theme::diff_hunk()),
    ])
}

fn thinking_header(t: &Thinking, spinner: char) -> Line<'static> {
    let mut spans = Vec::new();
    match t.state {
        ThinkingState::Running => {
            spans.push(Span::styled(
                format!("{} thinking", spinner),
                theme::thinking(),
            ));
        }
        ThinkingState::Done => {
            spans.push(Span::styled("─ thinking", theme::thinking()));
        }
    }
    spans.push(Span::styled(
        format!(" · {:.1}s", t.duration_ms as f64 / 1000.0),
        theme::dim(),
    ));
    if let Some(d) = &t.digest {
        let d = crate::renderer::truncate(d, 60);
        spans.push(Span::styled(format!(" · {}", d), theme::dim()));
    }
    if t.state == ThinkingState::Done {
        spans.push(Span::styled(" ─", theme::thinking()));
    }
    Line::from(spans)
}

fn tool_header(t: &ToolCall, spinner: char) -> Line<'static> {
    let (glyph, style) = match t.status {
        ToolStatus::Running => (spinner.to_string(), theme::tool_running()),
        ToolStatus::Done => ("✓".to_string(), theme::tool_done()),
        ToolStatus::Error => ("✗".to_string(), theme::tool_error()),
    };
    let mut spans = vec![
        Span::styled(format!("{} {} ", glyph, t.name), style),
        Span::styled(t.summary.clone(), theme::text()),
    ];
    if t.status != ToolStatus::Running {
        spans.push(Span::styled(
            format!(" · {}ms", t.duration_ms),
            theme::dim(),
        ));
    }
    if let Some(out) = &t.output {
        if t.status != ToolStatus::Running {
            spans.push(Span::styled(format!(" · {}", out), theme::tool_output()));
        }
    }
    Line::from(spans)
}

/// Render a hint: one header line, plus the content lines when expanded.
///
/// `selected` adds a `▶` cursor marker (used by components with hint
/// navigation). Collapsed-but-expandable hints get a `▸` marker, expanded
/// ones a `▾`.
/// A clickable row range of an activity (document coordinates).
///
/// `path` addresses the activity inside nested subagent transcripts:
/// `[]` is a message-level activity, `[s]` an activity inside subagent `s`,
/// `[s, n]` an activity nested one level deeper.
#[derive(Debug, Clone)]
/// A clickable row range of an activity (document coordinates).

pub struct ActivityRowRange {
    /// First row (inclusive).
    pub start: u16,
    /// Last row (exclusive).
    pub end: u16,
    /// Message index the activity belongs to.
    pub message: usize,
    /// Address inside nested subagent transcripts (`[s, n]` = activity `n`
    /// inside subagent `s`).
    pub path: Vec<usize>,
    /// Whether the activity is a tool call.
    pub is_tool: bool,
}

fn header_for(h: &Activity, spinner: char) -> Line<'static> {
    match &h.kind {
        ActivityKind::Thinking(t) => thinking_header(t, spinner),
        ActivityKind::Tool(t) => tool_header(t, spinner),
        ActivityKind::SubAgent(a) => subagent_header(a, spinner),
        ActivityKind::Todo(t) => todo_header(t, spinner),
        ActivityKind::Diff(d) => diff_header(d),
    }
}

/// Recursive layout of one activity: header + (expanded) content or nested
/// transcript, plus clickable ranges for every visible activity.
/// Fold-summary tail for collapsed-but-expandable activities (Claude Code's
/// `… +6 lines (ctrl+o to expand)` affordance, adapted to mouse clicks).
fn fold_tail(act: &Activity) -> Option<String> {
    if act.expanded {
        return None;
    }
    match &act.kind {
        ActivityKind::Tool(_) if !act.content.is_empty() => {
            Some(format!("… +{} lines (click to expand)", act.content.len()))
        }
        ActivityKind::SubAgent(a)
            if a.status == SubAgentStatus::Done && !a.transcript.is_empty() =>
        {
            Some(format!("{} steps (click to expand)", a.transcript.len()))
        }
        ActivityKind::Diff(d) if !d.hunks.is_empty() => {
            // header already carries the +A −R stats
            Some("(click to expand)".to_string())
        }
        _ => None,
    }
}

/// Prepend `prefix` to the first span of a line.
fn prepend_line(line: &Line<'static>, prefix: &str) -> Line<'static> {
    let mut spans = Vec::with_capacity(line.spans.len() + 1);
    spans.push(Span::styled(prefix.to_string(), theme::tool_output()));
    spans.extend(line.spans.iter().cloned());
    Line::from(spans)
}

/// Recursive layout of one activity. `depth == 0` marks message-level
/// activities (they get the `⏺` dot); deeper levels render as a tree with
/// `├─`/`└─` branches. `prefix` is applied to the header row, `cont` to the
/// content rows of this level.
fn activity_layout(
    act: &Activity,
    path: &[usize],
    message: usize,
    base_row: u16,
    spinner: char,
    render_reply: &mut dyn FnMut(&str) -> Vec<Line<'static>>,
    depth: usize,
    prefix: &str,
    cont: &str,
) -> (Vec<Line<'static>>, Vec<ActivityRowRange>) {
    let mut header = header_for(act, spinner);
    if depth == 0 {
        // main-level activity dot (Claude Code: `⏺ Read(.mcp.json)`)
        if matches!(
            act.kind,
            ActivityKind::Tool(_) | ActivityKind::SubAgent(_) | ActivityKind::Diff(_)
        ) {
            header
                .spans
                .insert(0, Span::styled("⏺ ", theme::activity_dot()));
        }
    }
    if let Some(tail) = fold_tail(act) {
        header
            .spans
            .push(Span::styled(format!(" {}", tail), theme::dim()));
    }
    let mut rows = vec![prepend_line(&header, prefix)];
    let mut cursor = base_row + 1;
    let mut ranges = Vec::new();
    if act.expanded {
        match &act.kind {
            ActivityKind::SubAgent(a) => {
                let n = a.transcript.len();
                for (j, nested) in a.transcript.iter().enumerate() {
                    let mut nested_path = path.to_vec();
                    nested_path.push(j);
                    let last = j == n - 1;
                    let branch = if last { "└─ " } else { "├─ " };
                    let child_cont = if last { "   " } else { "│  " };
                    let (lines, nested_ranges) = activity_layout(
                        nested,
                        &nested_path,
                        message,
                        cursor,
                        spinner,
                        render_reply,
                        depth + 1,
                        branch,
                        child_cont,
                    );
                    for line in &lines {
                        rows.push(prepend_line(line, cont));
                    }
                    cursor += lines.len() as u16;
                    ranges.extend(nested_ranges);
                }
                if !a.reply.is_empty() {
                    let reply = render_reply(&a.reply);
                    for line in &reply {
                        rows.push(prepend_line(line, &format!("{}  ", cont)));
                    }
                    cursor += reply.len() as u16;
                }
            }
            _ => {
                for (i, line) in act.content.iter().enumerate() {
                    // the first content line connects with `⎿` (Claude Code's
                    // result connector); the rest stay indented
                    let p = if i == 0 {
                        format!("{}⎿ ", cont)
                    } else {
                        format!("{}  ", cont)
                    };
                    rows.push(prepend_line(line, &p));
                }
                cursor += act.content.len() as u16;
            }
        }
    }
    ranges.push(ActivityRowRange {
        start: base_row,
        end: cursor,
        message,
        path: path.to_vec(),
        is_tool: matches!(act.kind, ActivityKind::Tool(_)),
    });
    (rows, ranges)
}

/// Layout a list of activities into rows + clickable ranges.
///
/// `render_reply` renders a subagent's markdown reply (the display layer owns
/// markdown rendering; this module stays agnostic).
pub fn layout_activities(
    message: usize,
    base_row: u16,
    acts: &[Activity],
    spinner: char,
    render_reply: &mut dyn FnMut(&str) -> Vec<Line<'static>>,
) -> (Vec<Line<'static>>, Vec<ActivityRowRange>) {
    let mut rows = Vec::new();
    let mut ranges = Vec::new();
    let mut doc_row = base_row;
    for (i, act) in acts.iter().enumerate() {
        let (lines, mut local) = activity_layout(
            act,
            &[i],
            message,
            doc_row,
            spinner,
            render_reply,
            0,
            "",
            "",
        );
        doc_row += lines.len() as u16;
        rows.extend(lines);
        ranges.append(&mut local);
    }
    (rows, ranges)
}

/// Walk a path into (possibly nested subagent) activities and return the
/// addressed activity, mutably.
pub fn activities_path_get_mut<'a>(
    acts: &'a mut [Activity],
    path: &[usize],
) -> Option<&'a mut Activity> {
    let (head, rest) = path.split_first()?;
    let act = acts.get_mut(*head)?;
    if rest.is_empty() {
        return Some(act);
    }
    if let ActivityKind::SubAgent(a) = &mut act.kind {
        activities_path_get_mut(&mut a.transcript, rest)
    } else {
        None
    }
}

/// Render an activity without a reply renderer (nested replies render as
/// plain content) — used by tests and simple displays.
pub fn activity_lines(h: &Activity, spinner: char) -> Vec<Line<'static>> {
    let mut render = |_: &str| Vec::new();
    let (rows, _) = activity_layout(h, &[0], 0, 0, spinner, &mut render, 0, "", "");
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn thinking(stage: &'static str, state: ThinkingState) -> Activity {
        let mut h = Activity::new(ActivityKind::Thinking(Thinking {
            state,
            duration_ms: 2300,
            digest: if state == ThinkingState::Done {
                Some("checking imports".into())
            } else {
                None
            },
            stage,
        }));
        if state == ThinkingState::Done {
            h.set_content(vec![Line::styled("reasoning line", Style::default())]);
        }
        h
    }

    #[test]
    fn thinking_collapsed_and_expanded() {
        let mut h = thinking("understand", ThinkingState::Done);
        assert!(h.expandable());
        let lines = activity_lines(&h, '⠋');
        assert_eq!(text(&lines[0]), "─ thinking · 2.3s · checking imports ─");
        assert_eq!(lines.len(), 1, "collapsed: header only");

        h.toggle();
        assert!(h.is_expanded());
        let lines = activity_lines(&h, '⠋');
        assert_eq!(text(&lines[0]), "─ thinking · 2.3s · checking imports ─");
        assert_eq!(lines.len(), 2, "expanded: header + content");
        assert!(text(&lines[1]).contains("reasoning line"));

        h.toggle();
        assert!(!h.is_expanded());
    }

    #[test]
    fn running_hint_is_not_expandable() {
        let h = thinking("understand", ThinkingState::Running);
        assert!(!h.expandable());
        let lines = activity_lines(&h, '⠋');
        assert_eq!(text(&lines[0]), "⠋ thinking · 2.3s");
    }

    #[test]
    fn tool_collapsed_and_expanded() {
        let mut h = Activity::new(ActivityKind::Tool(ToolCall {
            name: "bash",
            status: ToolStatus::Done,
            summary: "cargo test -p core".into(),
            duration_ms: 12,
            output: Some("54 passed".into()),
        }));
        h.set_content(vec![
            Line::styled("$ cargo test -p core", Style::default()),
            Line::styled("54 passed; 0 failed", Style::default()),
        ]);
        let lines = activity_lines(&h, '⠙');
        assert_eq!(
            text(&lines[0]),
            "⏺ ✓ bash cargo test -p core · 12ms · 54 passed … +2 lines (click to expand)"
        );

        h.toggle();
        let lines = activity_lines(&h, '⠙');
        assert!(text(&lines[1]).contains("$ cargo test -p core"));
        assert!(text(&lines[2]).contains("54 passed; 0 failed"));
    }

    #[test]
    fn spinner_cycles() {
        let a = spinner(0);
        let b = spinner(1);
        assert_ne!(a, b);
        assert_eq!(spinner(10), a, "cycles after full rotation");
    }

    #[test]
    fn todo_lifecycle_and_rendering() {
        let mut todo = TodoList {
            title: "Fix".to_string(),
            items: vec![
                TodoItem {
                    text: "a".into(),
                    status: TodoStatus::Done,
                },
                TodoItem {
                    text: "b".into(),
                    status: TodoStatus::InProgress,
                },
                TodoItem {
                    text: "c".into(),
                    status: TodoStatus::Pending,
                },
            ],
        };
        assert_eq!(todo.done(), 1);
        assert_eq!(todo.total(), 3);
        let lines = todo_lines(&todo, '⠋');
        assert!(text(&lines[0]).starts_with("[x] "));
        assert!(text(&lines[1]).starts_with("[⠋] "));
        assert!(text(&lines[2]).starts_with("[ ] "));

        todo.set(2, TodoStatus::Done);
        assert_eq!(todo.done(), 2);

        let header = todo_header(&todo, '⠋');
        assert!(text(&header).contains("2/3 tasks"), "{}", text(&header));
    }

    #[test]
    fn subagent_running_and_done() {
        let mut a = SubAgent::running("explore", "Find the parser");
        a.duration_ms = 1200;
        let l = subagent_header(&a, '⠋');
        assert!(
            text(&l).starts_with("⠋ explore (Find the parser)"),
            "{}",
            text(&l)
        );

        let done = SubAgent {
            name: "explore".into(),
            status: SubAgentStatus::Done,
            task: "Find the parser".into(),
            duration_ms: 1200,
            result: Some("found blocks.rs:24".into()),
            transcript: Vec::new(),
            reply: String::new(),
        };
        let l = subagent_header(&done, '⠋');
        assert!(
            text(&l).starts_with("✓ explore (Find the parser)"),
            "{}",
            text(&l)
        );
        assert!(text(&l).contains("blocks.rs:24"));
    }

    #[test]
    fn diff_parse_and_render() {
        let d = Diff::parse_unified(
            "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,3 @@\n ctx\n-old\n+new\n",
        );
        assert_eq!(d.path, "src/lib.rs");
        assert_eq!(d.hunks.len(), 1);
        assert_eq!(d.hunks[0].header, "@@ -1,2 +1,3 @@");
        assert_eq!(d.stats(), (1, 1));
        let lines = diff_lines(&d);
        assert_eq!(text(&lines[0]), "@@ -1,2 +1,3 @@");
        assert!(text(&lines[1]).starts_with(" ctx"), "{}", text(&lines[1]));
        assert!(text(&lines[2]).starts_with("-old"), "{}", text(&lines[2]));
        assert!(text(&lines[3]).starts_with("+new"), "{}", text(&lines[3]));
    }

    #[test]
    fn hierarchy_symbols_and_tree() {
        // a done subagent with a nested transcript renders as a tree
        let mut sub = Activity::new(ActivityKind::SubAgent(SubAgent {
            name: "explore".into(),
            status: SubAgentStatus::Done,
            task: "Find the parser".into(),
            duration_ms: 1200,
            result: Some("found it".into()),
            transcript: vec![
                {
                    let mut g = Activity::new(ActivityKind::Tool(ToolCall {
                        name: "grep",
                        status: ToolStatus::Done,
                        summary: "-n parse".into(),
                        duration_ms: 10,
                        output: Some("blocks.rs:24".into()),
                    }));
                    g.set_content(vec![Line::styled("blocks.rs:24: found", Style::default())]);
                    g.expanded = true;
                    g
                },
                Activity::new(ActivityKind::Thinking(Thinking {
                    state: ThinkingState::Done,
                    duration_ms: 300,
                    digest: Some("scanning".into()),
                    stage: "scan",
                })),
            ],
            reply: String::new(),
        }));
        sub.expanded = true;
        let lines = activity_lines(&sub, '⠋');
        assert!(
            text(&lines[0]).starts_with("⏺ ✓ explore (Find the parser)"),
            "{}",
            text(&lines[0])
        );
        // tree branches
        assert!(
            text(&lines[1]).starts_with("├─ ✓ grep"),
            "{}",
            text(&lines[1])
        );
        assert!(text(&lines[2]).starts_with("│  ⎿ "), "{}", text(&lines[2]));
        assert!(
            text(&lines[3]).starts_with("└─ ─ thinking"),
            "{}",
            text(&lines[3])
        );

        // collapsed: fold tail counts the steps
        sub.expanded = false;
        let lines = activity_lines(&sub, '⠋');
        assert!(
            text(&lines[0]).contains("2 steps (click to expand)"),
            "{}",
            text(&lines[0])
        );
    }

    #[test]
    fn diff_activity_header() {
        let d = Diff::parse_unified("--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-a\n+b\n");
        let header = diff_header(&d);
        assert_eq!(text(&header), "✻ Edit · x.rs · +1 −1");
    }
}
