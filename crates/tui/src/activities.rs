//! Claude Code-style agent hints: thinking blocks and tool calls.
//!
//! Both share one abstraction: a [`Hint`] is a foldable pane — a collapsed
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

pub fn spinner(frame: u64) -> char {
    SPINNERS[(frame as usize) % SPINNERS.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

/// A tool invocation: `✓ bash · cargo test · 12ms`.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: &'static str,
    pub status: ToolStatus,
    /// Brief command / argument summary.
    pub summary: String,
    pub duration_ms: u64,
    /// One-line output preview shown next to the header.
    pub output: Option<String>,
}

impl ToolCall {
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
pub enum ThinkingState {
    Running,
    Done,
}

/// A reasoning block: `⠋ thinking · 1.4s` -> `─ thinking · 1.4s ─`.
#[derive(Debug, Clone)]
pub struct Thinking {
    pub state: ThinkingState,
    pub duration_ms: u64,
    /// Collapsed digest shown once done.
    pub digest: Option<String>,
    /// Distinguishes consecutive reasoning stages so running/done updates
    /// replace the right hint.
    pub stage: &'static str,
}

/// Todo lifecycle (mirrors Claude Code: pending -> in_progress -> completed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub text: String,
    pub status: TodoStatus,
}

/// A task checklist, rendered as `- [ ]` / `- [⠋]` / `- [x]` items.
#[derive(Debug, Clone)]
pub struct TodoList {
    pub title: String,
    pub items: Vec<TodoItem>,
}

impl TodoList {
    pub fn done(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Done)
            .count()
    }
    pub fn total(&self) -> usize {
        self.items.len()
    }
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
pub struct SubAgent {
    pub name: String,
    pub status: SubAgentStatus,
    /// Short task description shown next to the name.
    pub task: String,
    pub duration_ms: u64,
    /// One-line result preview once done.
    pub result: Option<String>,
    /// The subagent's own transcript (recursively foldable).
    pub transcript: Vec<Activity>,
    /// The subagent's final markdown reply.
    pub reply: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentStatus {
    Running,
    Done,
    Error,
}

impl SubAgent {
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
pub enum DiffLine {
    Context(String),
    Removed(String),
    Added(String),
}

/// One hunk of a unified diff.
#[derive(Debug, Clone)]
pub struct Hunk {
    /// `@@ -a,b +c,d @@`
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A file edit, rendered as a git-style unified diff.
#[derive(Debug, Clone)]
pub struct Diff {
    pub path: String,
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
#[derive(Debug, Clone)]
pub enum ActivityKind {
    Thinking(Thinking),
    Tool(ToolCall),
    SubAgent(SubAgent),
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
pub struct Activity {
    pub kind: ActivityKind,
    /// Collapsed (`false`) or expanded (`true`).
    pub expanded: bool,
    /// Full content revealed when expanded (reasoning text / tool I/O).
    pub content: Vec<Line<'static>>,
}

impl Activity {
    pub fn new(kind: ActivityKind) -> Self {
        Self {
            kind,
            expanded: false,
            content: Vec::new(),
        }
    }

    /// Expand or collapse.
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Whether expansion reveals anything at all.
    pub fn expandable(&self) -> bool {
        !self.content.is_empty()
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

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
pub struct ActivityRowRange {
    pub start: u16,
    pub end: u16,
    pub message: usize,
    pub path: Vec<usize>,
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

fn indent_line(line: &Line<'static>, depth: usize) -> Line<'static> {
    if depth == 0 {
        return line.clone();
    }
    let pad = "  ".repeat(depth);
    let mut spans = Vec::with_capacity(line.spans.len() + 1);
    spans.push(Span::styled(pad, theme::tool_output()));
    spans.extend(line.spans.iter().cloned());
    Line::from(spans)
}

/// Recursive layout of one activity: header + (expanded) content or nested
/// transcript, plus clickable ranges for every visible activity.
fn activity_layout(
    act: &Activity,
    path: &[usize],
    message: usize,
    base_row: u16,
    spinner: char,
    render_reply: &mut dyn FnMut(&str) -> Vec<Line<'static>>,
) -> (Vec<Line<'static>>, Vec<ActivityRowRange>) {
    let mut rows = vec![header_for(act, spinner)];
    let mut cursor = base_row + 1;
    let mut ranges = Vec::new();
    if act.expanded {
        match &act.kind {
            ActivityKind::SubAgent(a) => {
                for (j, nested) in a.transcript.iter().enumerate() {
                    let mut nested_path = path.to_vec();
                    nested_path.push(j);
                    let (lines, nested_ranges) = activity_layout(
                        nested,
                        &nested_path,
                        message,
                        cursor,
                        spinner,
                        render_reply,
                    );
                    for line in &lines {
                        rows.push(indent_line(line, 2));
                    }
                    cursor += lines.len() as u16;
                    ranges.extend(nested_ranges);
                }
                if !a.reply.is_empty() {
                    let reply = render_reply(&a.reply);
                    for line in &reply {
                        rows.push(indent_line(line, 2));
                    }
                    cursor += reply.len() as u16;
                }
            }
            _ => {
                for line in &act.content {
                    rows.push(indent_line(line, 2));
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
        let (lines, mut local) =
            activity_layout(act, &[i], message, doc_row, spinner, render_reply);
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
    let (rows, _) = activity_layout(h, &[0], 0, 0, spinner, &mut render);
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
            "✓ bash cargo test -p core · 12ms · 54 passed"
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
    fn diff_activity_header() {
        let d = Diff::parse_unified("--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-a\n+b\n");
        let header = diff_header(&d);
        assert_eq!(text(&header), "✻ Edit · x.rs · +1 −1");
    }
}
