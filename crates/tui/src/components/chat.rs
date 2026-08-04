//! Agent chat component: a Claude Code-style session with foldable thinking
//! and tool-call hints, plus streaming markdown replies.
//!
//! Thinking and tool calls share the same [`Activity`] abstraction — both can be
//! expanded/collapsed with `[enter]`/`[space]`; only their presentation
//! differs (reasoning text vs tool I/O). The agent is scripted (no real LLM):
//! every user message triggers a turn of thinking -> tool calls -> markdown.

use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use rsmarkdown_core::{MarkdownProcessor, Renderer};

use crate::activities::{
    self, activities_path_get_mut, Activity, ActivityKind, ActivityRowRange, Diff, SubAgent,
    SubAgentStatus, Thinking, ThinkingState, TodoItem, TodoList, TodoStatus, ToolCall, ToolStatus,
};
use crate::component::Component;
use crate::renderer::{theme, StreamMarkdownRenderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Message role in the chat transcript.
pub enum Role {
    /// A user message.
    User,
    /// An assistant message.
    Assistant,
}

/// One message: role, text, and (assistant-only) activities.
pub struct ChatMessage {
    /// User or assistant.
    pub role: Role,
    /// User text or assistant markdown source.
    pub text: String,
    /// Assistant-only: the foldable hint events preceding the reply.
    pub hints: Vec<Activity>,
    /// Assistant-only: how far the reply has streamed.
    pub stream_pos: usize,
}

impl ChatMessage {
    fn user(text: String) -> Self {
        Self {
            role: Role::User,
            text,
            hints: Vec::new(),
            stream_pos: 0,
        }
    }
    fn assistant() -> Self {
        Self {
            role: Role::Assistant,
            text: String::new(),
            hints: Vec::new(),
            stream_pos: 0,
        }
    }
}

/// One planned tool call in a turn's script.
struct PlannedTool {
    name: &'static str,
    summary: &'static str,
    duration_ms: u64,
    /// Output preview (header line).
    preview: &'static str,
    /// Full output (expanded content).
    output: &'static str,
}

/// Current agent phase of a turn.
enum TurnPhase {
    Todo,
    Thinking {
        elapsed_ms: u64,
        target_ms: u64,
        reasoning: String,
    },
    SubAgent {
        call: SubAgent,
        elapsed_ms: u64,
        target_ms: u64,
    },
    Tool {
        call: ToolCall,
        elapsed_ms: u64,
        tools: Vec<PlannedTool>,
        index: usize,
    },
    Edit {
        elapsed_ms: u64,
        target_ms: u64,
    },
    Thinking2 {
        elapsed_ms: u64,
        target_ms: u64,
    },
    Stream,
}

const TODO_ITEMS: [&str; 5] = [
    "Understand the request",
    "Explore the codebase",
    "Run the tests",
    "Edit the file",
    "Verify the result",
];

const DIFF_SRC: &str = r#"--- a/crates/core/src/blocks.rs
+++ b/crates/core/src/blocks.rs
@@ -101,7 +101,7 @@
-    if has_footnote_reference(markdown) || has_footnote_definition(markdown) {
+    if has_footnote_reference(markdown) {
         return vec![markdown.to_string()];
     }
 
     let tokens = lex_tokens(markdown);
@@ -113,7 +113,7 @@
-        if !html_stack.is_empty() {
+        if let Some(top) = html_stack.last() {
             merged[merged_len - 1].push('\n');
             merged[merged_len - 1].push_str(current);
-            if let Some(closing) = current.find("</") {
+            if current.contains("</") {
                 if let Some(rest) = current[closing + 2..].split_whitespace().next() {
                     let tag = rest.trim_end_matches(['>', '/', '\n']).to_string();
                     if let Some(top) = html_stack.last() {
                         if *top == tag {
                             html_stack.pop();
                         }
                     }
                 }
             }
         }
     }

     let mut merged: Vec<String> = Vec::new();
-    let mut html_stack: Vec<String> = Vec::new();
+    let html_stack: Vec<String> = Vec::new();
 
     for token in tokens {
         let current = token;
         let merged_len = merged.len();

         // inside an unclosed HTML block — merge with previous
-        if !html_stack.is_empty() {
+        if let Some(top) = html_stack.last() {
             merged[merged_len - 1].push('\n');
             merged[merged_len - 1].push_str(current);
-            if let Some(closing) = current.find("</") {
+            if current.contains("</") {
                 if let Some(rest) = current[closing + 2..].split_whitespace().next() {
                     let tag = rest.trim_end_matches(['>', '/', '\n']).to_string();
                     if let Some(top) = html_stack.last() {
                         if *top == tag {
                             html_stack.pop();
                         }
                     }
                 }
             }
         }
     }
}"#;

const ANSWER: &str = "Done — here is the summary:

- The **core** pipeline re-parses only the trailing block
- Completed blocks hit the 100% LRU cache
- Tool calls and thinking are foldable hints, like a real agent session

```bash
cargo test -p rsmarkdown-core
```

**54 tests passed.** The reply itself is markdown, streamed through the same
display adapter as the markdown component.";

const REASONING_UNDERSTAND: [&str; 4] = [
    "Let me check how the pipeline is structured.",
    "The document is split into stable blocks by blank lines and fences.",
    "Only the trailing block needs re-parsing as new chunks arrive.",
    "Preprocessing self-heals incomplete syntax on that block.",
];

const REASONING_COMPOSE: [&str; 2] = [
    "The checks passed — composing the final answer as markdown.",
    "Summarizing the architecture and the verification results.",
];

/// Agent chat component with a scripted demo turn (thinking -> subagent -> tools -> reply).
pub struct AgentChat {
    processor: MarkdownProcessor,
    renderer: StreamMarkdownRenderer,
    messages: Vec<ChatMessage>,
    rendered: HashMap<usize, (String, Vec<Line<'static>>)>,
    reply_cache: HashMap<String, Vec<Line<'static>>>,
    phase: Option<TurnPhase>,
    tick: u64,
    scroll: u16,
    input: String,
    typing: bool,
    width: usize,
    /// Row ranges of the foldable hints from the last draw (doc coordinates),
    /// used to map mouse clicks to hints.
    activity_ranges: Vec<ActivityRowRange>,
    /// Chat viewport height (last draw), excludes the input line.
    view_height: u16,
}

impl AgentChat {
    /// Create a chat component (demo: one scripted turn runs immediately).
    pub fn new() -> Self {
        let mut this = Self {
            processor: MarkdownProcessor::default(),
            renderer: StreamMarkdownRenderer::new(80),
            messages: Vec::new(),
            rendered: HashMap::new(),
            reply_cache: HashMap::new(),
            phase: None,
            tick: 0,
            scroll: 0,
            input: String::new(),
            typing: true,
            width: 80,
            activity_ranges: Vec::new(),
            view_height: 24,
        };
        this.submit("What does the pipeline look like?");
        this
    }

    /// Submit a user message and start a new agent turn.
    fn submit(&mut self, text: &str) {
        self.messages.push(ChatMessage::user(text.to_string()));
        self.start_turn();
        self.scroll = u16::MAX; // follow the new turn
    }

    fn start_turn(&mut self) {
        self.messages.push(ChatMessage::assistant());
        self.phase = Some(TurnPhase::Todo);
    }

    fn current_assistant(&mut self) -> Option<&mut ChatMessage> {
        self.messages
            .last_mut()
            .filter(|m| m.role == Role::Assistant)
    }

    /// Push a hint, replacing the trailing hint with the same identity
    /// (tool name / thinking stage) — keeps running -> done updates in place.
    fn push_hint(&mut self, hint: Activity) {
        let Some(msg) = self.current_assistant() else {
            return;
        };
        let id = hint.kind.identity();
        let mut hint = hint;
        // while active (running todo / subagent) keep it expanded; otherwise
        // preserve the user's expand/collapse choice across identity updates
        hint.expanded = if crate::activities::auto_expand(&hint) {
            true
        } else {
            msg.hints
                .iter()
                .rev()
                .find(|l| l.kind.identity() == id)
                .map_or(false, |l| l.expanded)
        };
        // replace the previous occurrence IN PLACE (single evolving todo,
        // running -> done updates) instead of appending duplicates
        if let Some(pos) = msg.hints.iter().position(|l| l.kind.identity() == id) {
            msg.hints[pos] = hint;
        } else {
            msg.hints.push(hint);
        }
    }

    /// Build the turn's todo hint (one item in progress).
    fn todo_hint(&self, in_progress: usize) -> Activity {
        let todo = TodoList {
            title: "Implement the fix".to_string(),
            items: TODO_ITEMS
                .iter()
                .enumerate()
                .map(|(i, text)| TodoItem {
                    text: text.to_string(),
                    status: if i < in_progress {
                        TodoStatus::Done
                    } else if i == in_progress {
                        TodoStatus::InProgress
                    } else {
                        TodoStatus::Pending
                    },
                })
                .collect(),
        };
        let content = activities::todo_lines(&todo, activities::spinner(self.tick));
        let mut hint = Activity::new(ActivityKind::Todo(todo));
        hint.set_content(content);
        hint
    }

    /// Reasoning lines up to the current progress fraction.
    fn reasoning_lines(reasoning: &[&'static str], progress: f32) -> Vec<Line<'static>> {
        let n = ((reasoning.len() as f32 * progress).ceil() as usize).clamp(1, reasoning.len());
        reasoning[..n]
            .iter()
            .map(|l| Line::styled(l.to_string(), Style::default()))
            .collect()
    }

    /// Advance the agent by `dt` simulated milliseconds.
    fn advance(&mut self, dt_ms: u64) {
        let Some(mut phase) = self.phase.take() else {
            return;
        };
        let mut finished = false;
        match &mut phase {
            TurnPhase::Todo => {
                self.push_hint(self.todo_hint(0));
                phase = TurnPhase::Thinking {
                    elapsed_ms: 0,
                    target_ms: 1400,
                    reasoning: String::new(),
                };
            }
            TurnPhase::Thinking {
                elapsed_ms,
                target_ms,
                reasoning,
            } => {
                *elapsed_ms += dt_ms;
                let progress = (*elapsed_ms as f32 / *target_ms as f32).min(1.0);
                *reasoning = REASONING_UNDERSTAND
                    .iter()
                    .take(
                        ((REASONING_UNDERSTAND.len() as f32 * progress).ceil() as usize)
                            .clamp(1, REASONING_UNDERSTAND.len()),
                    )
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                let mut hint = Activity::new(ActivityKind::Thinking(Thinking {
                    state: ThinkingState::Running,
                    duration_ms: *elapsed_ms,
                    digest: None,
                    stage: "understand",
                }));
                hint.set_content(Self::reasoning_lines(&REASONING_UNDERSTAND, progress));
                self.push_hint(hint);
                if *elapsed_ms >= *target_ms {
                    let mut hint = Activity::new(ActivityKind::Thinking(Thinking {
                        state: ThinkingState::Done,
                        duration_ms: *target_ms,
                        digest: Some("understanding the request".to_string()),
                        stage: "understand",
                    }));
                    hint.set_content(
                        REASONING_UNDERSTAND
                            .iter()
                            .map(|l| Line::styled(l.to_string(), Style::default()))
                            .collect(),
                    );
                    self.push_hint(hint);
                    // todo: step 1 done, step 2 in progress
                    self.push_hint(self.todo_hint(1));
                    phase = TurnPhase::SubAgent {
                        call: SubAgent::running(
                            "explore",
                            "Find the block splitter and its fence handling",
                        ),
                        elapsed_ms: 0,
                        target_ms: 1200,
                    };
                }
            }
            TurnPhase::SubAgent {
                call,
                elapsed_ms,
                target_ms,
            } => {
                *elapsed_ms += dt_ms;
                let p = (*elapsed_ms as f32 / *target_ms as f32).min(1.0);
                call.duration_ms = *elapsed_ms;

                // build the subagent's nested transcript by progress
                let mut transcript: Vec<Activity> = Vec::new();
                // sub-todo
                let mut sub_todo = Activity::new(ActivityKind::Todo(TodoList {
                    title: "Explore the codebase".to_string(),
                    items: vec![
                        TodoItem {
                            text: "Read blocks.rs".to_string(),
                            status: if p >= 0.35 {
                                TodoStatus::Done
                            } else {
                                TodoStatus::InProgress
                            },
                        },
                        TodoItem {
                            text: "Report findings".to_string(),
                            status: if p >= 0.8 {
                                TodoStatus::Done
                            } else if p >= 0.35 {
                                TodoStatus::InProgress
                            } else {
                                TodoStatus::Pending
                            },
                        },
                    ],
                }));
                let items = match &sub_todo.kind {
                    ActivityKind::Todo(t) => t,
                    _ => unreachable!(),
                };
                let sub_todo_lines = activities::todo_lines(items, activities::spinner(self.tick));
                sub_todo.set_content(sub_todo_lines);
                transcript.push(sub_todo);
                // sub-thinking
                if p >= 0.2 {
                    let mut thinking = Activity::new(ActivityKind::Thinking(Thinking {
                        state: if p >= 0.5 {
                            ThinkingState::Done
                        } else {
                            ThinkingState::Running
                        },
                        duration_ms: (*elapsed_ms as u64),
                        digest: Some("scanning blocks.rs".to_string()),
                        stage: "scan",
                    }));
                    thinking.set_content(vec![
                        Line::styled("Looking for parse_markdown_into_blocks…", Style::default()),
                        Line::styled("Fence tracking is line-based.", Style::default()),
                    ]);
                    transcript.push(thinking);
                }
                // sub-grep tool
                if p >= 0.5 {
                    let mut grep = Activity::new(ActivityKind::Tool(ToolCall {
                        name: "grep",
                        status: if p >= 0.8 {
                            ToolStatus::Done
                        } else {
                            ToolStatus::Running
                        },
                        summary: "-n parse_markdown_into_blocks crates/core/src/blocks.rs"
                            .to_string(),
                        duration_ms: ((p * 600.0) as u64),
                        output: if p >= 0.8 {
                            Some("blocks.rs:24".to_string())
                        } else {
                            None
                        },
                    }));
                    if p >= 0.8 {
                        grep.set_content(vec![
                            Line::styled(
                                "$ grep -n parse_markdown_into_blocks crates/core/src/blocks.rs",
                                Style::default(),
                            ),
                            Line::styled(
                                "blocks.rs:24: pub fn parse_markdown_into_blocks",
                                Style::default(),
                            ),
                        ]);
                    }
                    transcript.push(grep);
                }
                let reply = "Found it — `parse_markdown_into_blocks` lives in `blocks.rs`:

- line-based tokenization with fence tracking
- html and math merging in the same module";

                let mut hint = Activity::new(ActivityKind::SubAgent(SubAgent {
                    name: call.name.clone(),
                    status: if p >= 1.0 {
                        SubAgentStatus::Done
                    } else {
                        SubAgentStatus::Running
                    },
                    task: call.task.clone(),
                    duration_ms: *elapsed_ms,
                    result: if p >= 1.0 {
                        Some("found parse_markdown_into_blocks · blocks.rs:24".to_string())
                    } else {
                        None
                    },
                    transcript: transcript.clone(),
                    reply: if p >= 1.0 {
                        reply.to_string()
                    } else {
                        String::new()
                    },
                }));
                // nested activities stay expanded while the agent works
                for act in &mut subagent_of(&mut hint).transcript {
                    act.expanded = true;
                }
                self.push_hint(hint);

                if *elapsed_ms >= *target_ms {
                    // todo: step 2 done, step 3 in progress
                    self.push_hint(self.todo_hint(2));
                    phase = TurnPhase::Tool {
                        call: ToolCall::running("bash", "cargo test -p rsmarkdown-core"),
                        elapsed_ms: 0,
                        tools: vec![PlannedTool {
                            name: "bash",
                            summary: "cargo test -p rsmarkdown-core",
                            duration_ms: 900,
                            preview: "54 passed · finished in 0.02s",
                            output: concat!(
                                "$ cargo test -p rsmarkdown-core\n",
                                "running 54 tests\n",
                                "test result: ok. 54 passed; 0 failed; 0 ignored; ",
                                "0 measured; 0 filtered out\n",
                                "finished in 0.02s"
                            ),
                        }],
                        index: 0,
                    };
                }
            }
            TurnPhase::Tool {
                call,
                elapsed_ms,
                tools,
                index,
            } => {
                *elapsed_ms += dt_ms;
                call.duration_ms = *elapsed_ms;
                self.push_hint(Activity::new(ActivityKind::Tool(call.clone())));
                if *elapsed_ms >= tools[*index].duration_ms {
                    let mut hint = Activity::new(ActivityKind::Tool(ToolCall {
                        name: tools[*index].name,
                        status: ToolStatus::Done,
                        summary: tools[*index].summary.to_string(),
                        duration_ms: tools[*index].duration_ms,
                        output: Some(tools[*index].preview.to_string()),
                    }));
                    hint.set_content(
                        tools[*index]
                            .output
                            .lines()
                            .map(|l| Line::styled(l.to_string(), Style::default()))
                            .collect(),
                    );
                    self.push_hint(hint);
                    *index += 1;
                    if *index < tools.len() {
                        *call = ToolCall::running(tools[*index].name, tools[*index].summary);
                        *elapsed_ms = 0;
                    } else {
                        // todo: step 3 done, step 4 in progress
                        self.push_hint(self.todo_hint(3));
                        phase = TurnPhase::Edit {
                            elapsed_ms: 0,
                            target_ms: 700,
                        };
                    }
                }
            }
            TurnPhase::Edit {
                elapsed_ms,
                target_ms,
            } => {
                *elapsed_ms += dt_ms;
                let mut call = ToolCall::running("Edit", "crates/core/src/blocks.rs");
                call.duration_ms = *elapsed_ms;
                self.push_hint(Activity::new(ActivityKind::Tool(call)));
                if *elapsed_ms >= *target_ms {
                    // the running Edit row becomes the diff activity
                    if let Some(msg) = self.current_assistant() {
                        if matches!(
                            msg.hints.last().map(|h| &h.kind),
                            Some(ActivityKind::Tool(t)) if t.name == "Edit"
                        ) {
                            msg.hints.pop();
                        }
                    }
                    let diff = Diff::parse_unified(DIFF_SRC);
                    let content = activities::diff_lines(&diff);
                    let mut hint = Activity::new(ActivityKind::Diff(diff));
                    hint.set_content(content);
                    self.push_hint(hint);
                    // todo: step 4 done, step 5 in progress
                    self.push_hint(self.todo_hint(4));
                    phase = TurnPhase::Thinking2 {
                        elapsed_ms: 0,
                        target_ms: 800,
                    };
                }
            }
            TurnPhase::Thinking2 {
                elapsed_ms,
                target_ms,
            } => {
                *elapsed_ms += dt_ms;
                let progress = (*elapsed_ms as f32 / *target_ms as f32).min(1.0);
                let mut hint = Activity::new(ActivityKind::Thinking(Thinking {
                    state: ThinkingState::Running,
                    duration_ms: *elapsed_ms,
                    digest: None,
                    stage: "compose",
                }));
                hint.set_content(Self::reasoning_lines(&REASONING_COMPOSE, progress));
                self.push_hint(hint);
                if *elapsed_ms >= *target_ms {
                    let mut hint = Activity::new(ActivityKind::Thinking(Thinking {
                        state: ThinkingState::Done,
                        duration_ms: *target_ms,
                        digest: Some("composing the reply".to_string()),
                        stage: "compose",
                    }));
                    hint.set_content(
                        REASONING_COMPOSE
                            .iter()
                            .map(|l| Line::styled(l.to_string(), Style::default()))
                            .collect(),
                    );
                    self.push_hint(hint);
                    // todo: all done
                    self.push_hint(self.todo_hint(TODO_ITEMS.len()));
                    phase = TurnPhase::Stream;
                }
            }
            TurnPhase::Stream => {
                if let Some(msg) = self.current_assistant() {
                    let mut end = (msg.stream_pos + 6).min(ANSWER.len());
                    while end < ANSWER.len() && !ANSWER.is_char_boundary(end) {
                        end += 1;
                    }
                    msg.text = ANSWER[..end].to_string();
                    msg.stream_pos = end;
                    if end >= ANSWER.len() {
                        finished = true;
                    }
                }
            }
        }
        if !finished {
            self.phase = Some(phase);
        }
    }

    /// Rendered lines for one assistant message (markdown, cached per content).
    fn message_lines(&mut self, idx: usize) -> Vec<Line<'static>> {
        let msg = &self.messages[idx];
        if msg.text.is_empty() {
            return Vec::new();
        }
        if let Some((c, lines)) = self.rendered.get(&idx) {
            if *c == msg.text {
                return lines.clone();
            }
        }
        self.renderer.set_width(self.width);
        let doc = self.processor.process_streaming(&msg.text);
        self.renderer.render(&doc);
        let lines = self.renderer.lines().to_vec();
        self.rendered.insert(idx, (msg.text.clone(), lines.clone()));
        lines
    }

    /// Total chat height in rows (user lines + hint header/content + markdown).
    fn chat_height(&self) -> u16 {
        let mut h = 0u16;
        for (i, msg) in self.messages.iter().enumerate() {
            match msg.role {
                Role::User => h = h.saturating_add(1),
                Role::Assistant => {
                    for hint in &msg.hints {
                        h = h.saturating_add(1);
                        if hint.expanded {
                            h = h.saturating_add(hint.content.len() as u16);
                        }
                    }
                    let md_rows = self
                        .rendered
                        .get(&i)
                        .map(|(c, l)| if *c == msg.text { l.len() as u16 } else { 0 })
                        .unwrap_or(0);
                    h = h.saturating_add(md_rows);
                }
            }
        }
        h
    }

    fn scroll_by(&mut self, delta: i16) {
        self.scroll = self.scroll.saturating_add_signed(delta);
    }

    /// Toggle the activity whose header row (document coordinates) was
    /// clicked — nested subagent activities are addressed by their path.
    fn toggle_at(&mut self, doc_row: u16) {
        let Some(range) = self
            .activity_ranges
            .iter()
            .find(|r| doc_row >= r.start && doc_row < r.end)
        else {
            return;
        };
        let path = range.path.clone();
        if let Some(msg) = self.messages.get_mut(range.message) {
            if let Some(act) = activities_path_get_mut(&mut msg.hints, &path) {
                act.toggle();
            }
        }
    }

    // --- test/diagnostic helpers ---

    /// Whether the scripted turn has finished (diagnostic).
    pub fn phase_done(&self) -> bool {
        self.phase.is_none()
    }

    /// Number of messages (diagnostic).
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Full conversation as plain text (hints + replies + user lines).
    pub fn conversation_text(&self) -> String {
        let mut out = String::new();
        for msg in &self.messages {
            match msg.role {
                Role::User => out.push_str(&format!("you: {}\n", msg.text)),
                Role::Assistant => {
                    for hint in &msg.hints {
                        for line in activities::activity_lines(hint, '⠿') {
                            out.push_str(
                                &line
                                    .spans
                                    .iter()
                                    .map(|s| s.content.as_ref())
                                    .collect::<String>(),
                            );
                            out.push('\n');
                        }
                    }
                    out.push_str(&msg.text);
                    out.push('\n');
                }
            }
        }
        out
    }

    /// Row ranges of foldable activities from the last draw (doc
    /// coordinates), used by tests to click on activities.
    pub fn hint_row_ranges(&self) -> &[ActivityRowRange] {
        &self.activity_ranges
    }

    /// First tool hint of the last assistant message, if any.
    pub fn last_tool_hint(&self) -> Option<&Activity> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .and_then(|m| {
                m.hints
                    .iter()
                    .find(|h| matches!(h.kind, ActivityKind::Tool(_)))
            })
    }
}

impl Component for AgentChat {
    fn title(&self) -> &str {
        "chat"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let [chat_area, input_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        self.view_height = chat_area.height;
        if self.width != chat_area.width as usize {
            self.width = chat_area.width as usize;
            self.rendered.clear();
        }

        // paint input line
        let caret = if self.typing { '▋' } else { ' ' };
        let input_line = Line::from(vec![
            Span::styled("you › ", theme::tool_running()),
            Span::styled(self.input.clone(), theme::text()),
            Span::styled(caret.to_string(), theme::tool_running()),
        ]);
        buf.set_line(0, input_area.y, &input_line, input_area.width);

        let spinner = activities::spinner(self.tick);

        // pre-render every message into a flat row list, recording the
        // document row range of each activity (incl. nested subagent
        // transcripts) for click-to-toggle
        let mut rows: Vec<Line<'static>> = Vec::new();
        self.activity_ranges.clear();
        let mut doc_row = 0u16;
        for i in 0..self.messages.len() {
            match self.messages[i].role {
                Role::User => {
                    rows.push(Line::from(vec![
                        Span::styled("you ", theme::tool_running()),
                        Span::styled(self.messages[i].text.clone(), theme::text()),
                    ]));
                    doc_row += 1;
                }
                Role::Assistant => {
                    // render nested subagent markdown replies through the
                    // display adapter (borrows disjoint fields)
                    let width = self.width;
                    let mut render = |reply: &str| {
                        render_markdown_impl(
                            &mut self.processor,
                            &mut self.renderer,
                            &mut self.reply_cache,
                            width,
                            reply,
                        )
                    };
                    let (lines, mut ranges) = activities::layout_activities(
                        i,
                        doc_row,
                        &self.messages[i].hints,
                        spinner,
                        &mut render,
                    );
                    doc_row += lines.len() as u16;
                    rows.extend(lines);
                    self.activity_ranges.append(&mut ranges);
                    let md = self.message_lines(i);
                    doc_row += md.len() as u16;
                    rows.extend(md);
                }
            }
        }

        let total = rows.len() as u16;
        let scroll = self.scroll.min(total.saturating_sub(chat_area.height));
        self.scroll = scroll;
        for (y, line) in rows
            .iter()
            .skip(scroll as usize)
            .take(chat_area.height as usize)
            .enumerate()
        {
            buf.set_line(0, chat_area.y + y as u16, line, chat_area.width);
        }
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if self.typing {
                    match key.code {
                        KeyCode::Char(c) => {
                            self.input.push(c);
                            return true;
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                            return true;
                        }
                        KeyCode::Enter => {
                            let text = std::mem::take(&mut self.input);
                            if !text.is_empty() {
                                self.submit(&text);
                            }
                            return true;
                        }
                        KeyCode::Esc => {
                            self.typing = false;
                            return true;
                        }
                        _ => {}
                    }
                }
                match key.code {
                    KeyCode::Esc => {
                        self.typing = true;
                        true
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.scroll_by(1);
                        true
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.scroll_by(-1);
                        true
                    }
                    KeyCode::PageDown => {
                        self.scroll_by(10);
                        true
                    }
                    KeyCode::PageUp => {
                        self.scroll_by(-10);
                        true
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.scroll = 0;
                        true
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.scroll = u16::MAX;
                        true
                    }
                    _ => false,
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_by(-3);
                    true
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_by(3);
                    true
                }
                // click a hint header to expand / collapse it
                MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    let chat_height = self.view_height;
                    if m.row < chat_height {
                        self.toggle_at(self.scroll + m.row);
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn on_tick(&mut self) {
        self.tick += 1;
        self.advance(33);
    }

    fn status(&self) -> String {
        let done = self
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant && !m.text.is_empty())
            .count();
        let running = self.phase.is_some();
        let hints: usize = self.messages.iter().map(|m| m.hints.len()).sum();
        format!(
            "{} messages · {} replies · {} hints{} · {} rows",
            self.messages.len(),
            done,
            hints,
            if running { " · working…" } else { "" },
            self.chat_height(),
        )
    }

    fn hints(&self) -> &'static str {
        if self.typing {
            "[enter] send  [esc] view  click a hint to expand"
        } else {
            "[esc] type  [j/k] scroll  click a hint to expand/collapse"
        }
    }

    fn footer_badges(&self) -> Vec<crate::app::FooterBadge> {
        // Claude Code: `← for agents` / `← 2 agents`, flashing `← 2 done`
        let mut running = 0;
        let mut done = 0;
        for msg in &self.messages {
            for hint in &msg.hints {
                if let ActivityKind::SubAgent(a) = &hint.kind {
                    match a.status {
                        SubAgentStatus::Running => running += 1,
                        SubAgentStatus::Done => done += 1,
                        SubAgentStatus::Error => {}
                    }
                }
            }
        }
        if running > 0 {
            let text = if running == 1 {
                "← for agents".to_string()
            } else {
                format!("← {} agents", running)
            };
            vec![crate::app::FooterBadge::new(text, theme::tool_running())]
        } else if done > 0 {
            let text = if done == 1 {
                "← 1 done".to_string()
            } else {
                format!("← {} done", done)
            };
            vec![crate::app::FooterBadge::new(text, theme::tool_done())]
        } else {
            Vec::new()
        }
    }
}

fn subagent_of(hint: &mut Activity) -> &mut SubAgent {
    match &mut hint.kind {
        ActivityKind::SubAgent(a) => a,
        _ => unreachable!("subagent expected"),
    }
}

fn render_markdown_impl(
    processor: &mut MarkdownProcessor,
    renderer: &mut StreamMarkdownRenderer,
    cache: &mut HashMap<String, Vec<Line<'static>>>,
    width: usize,
    src: &str,
) -> Vec<Line<'static>> {
    if src.is_empty() {
        return Vec::new();
    }
    if let Some(lines) = cache.get(src) {
        return lines.clone();
    }
    renderer.set_width(width);
    let doc = processor.process_streaming(src);
    renderer.render(&doc);
    let lines = renderer.lines().to_vec();
    cache.insert(src.to_string(), lines.clone());
    lines
}
