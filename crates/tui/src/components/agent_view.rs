//! Agent overview component (Claude Code Agent View): a full-view,
//! grouped session table.
//!
//! Evidence (research doc §5.2): sessions are grouped into Pinned / Ready
//! for review / Needs input / Working / Completed with a header line of
//! summary counts; each row is `status icon + session name + current
//! activity + optional PR #N + age`; `Enter` opens a session transcript,
//! `x` dismisses, `Esc` returns; `Space` opens a peek panel with the
//! full status sentence, waiting time and a reply input. Completed rows
//! collapse to `… N more` when there are many.
//!
//! The component is pure display + interaction over the [`crate::agent`]
//! model: it holds no knowledge of chat or any other component. Data
//! arrives through [`Component::absorb_agents`] (the host merges all
//! components' [`Component::agents`] broadcasts) or directly via
//! [`AgentView::set_agents`].

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use crate::agent::{fmt_age, merge_agents, Agent, AgentGroup, AgentStatus};
use crate::app::FooterBadge;
use crate::component::Component;
use crate::renderer::theme;

/// Completed rows above this count collapse to a `… N more` row.
pub const MAX_COMPLETED: usize = 8;
/// Max transcript overlay rows.
pub const TRANSCRIPT_CAP: usize = 20;

/// What the agent view is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    /// Session transcript opened with `Enter`.
    Transcript(usize),
    /// Peek panel opened with `Space` (holds the reply input).
    Peek(usize),
}

/// Agent overview component.
pub struct AgentView {
    /// Session rows, in display order (merge order of the host broadcast).
    agents: Vec<Agent>,
    /// Selected row (flattened display index, including group headers).
    selected: usize,
    /// Scroll offset of the table.
    scroll: u16,
    /// Whether the completed group is expanded (not collapsed).
    completed_expanded: bool,
    /// Open overlay (transcript / peek), if any.
    overlay: Option<Overlay>,
    /// Transcript overlay scroll.
    transcript_scroll: u16,
    /// Peek reply input text.
    reply: String,
    /// Frame counter (working-row animation).
    tick: u64,
    /// Viewport height of the last draw.
    view_height: u16,
    /// Names dismissed with `x` (hidden from merges until taken).
    dismissed: Vec<String>,
}

/// A display row of the table: either a group header or an agent row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TableRow {
    /// Group header (`AgentGroup::title()`).
    Header(AgentGroup),
    /// An agent row (index into [`AgentView::agents`]).
    Agent(usize),
    /// Collapsed tail of the completed group (`… N more`).
    More(usize),
}

impl AgentView {
    /// Create an empty agent view.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected: 0,
            scroll: 0,
            completed_expanded: false,
            overlay: None,
            transcript_scroll: 0,
            reply: String::new(),
            tick: 0,
            view_height: 24,
            dismissed: Vec::new(),
        }
    }

    /// Replace the whole agent table (library entry point for direct use).
    pub fn set_agents(&mut self, agents: Vec<Agent>) {
        self.agents = agents;
        self.clamp_selection();
    }

    /// Merge a fresh snapshot (used by the host broadcast).
    pub fn merge_agents(&mut self, incoming: Vec<Agent>) {
        merge_agents(&mut self.agents, incoming, &self.dismissed);
        self.clamp_selection();
    }

    /// Session rows currently shown.
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// Names dismissed with `x`; `take_dismissed` clears the queue.
    pub fn take_dismissed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.dismissed)
    }

    /// Number of completed rows hidden by the collapsed tail, if any.
    fn hidden_completed(&self) -> usize {
        if self.completed_expanded {
            return 0;
        }
        let completed = self
            .agents
            .iter()
            .filter(|a| a.status.group() == AgentGroup::Completed)
            .count();
        completed.saturating_sub(MAX_COMPLETED)
    }

    /// Table rows in display order (group order, agents sorted by their
    /// group, completed collapsed when too many).
    pub(crate) fn rows(&self) -> Vec<TableRow> {
        let mut rows = Vec::new();
        for group in [
            AgentGroup::Pinned,
            AgentGroup::ReadyForReview,
            AgentGroup::NeedsInput,
            AgentGroup::Working,
            AgentGroup::Completed,
        ] {
            let members: Vec<usize> = (0..self.agents.len())
                .filter(|&i| self.agents[i].status.group() == group)
                .collect();
            if members.is_empty() {
                continue;
            }
            rows.push(TableRow::Header(group));
            if group == AgentGroup::Completed {
                let hidden = self.hidden_completed();
                let shown = members.len().saturating_sub(hidden);
                rows.extend(members[..shown].iter().copied().map(TableRow::Agent));
                if hidden > 0 {
                    rows.push(TableRow::More(hidden));
                }
            } else {
                rows.extend(members.into_iter().map(TableRow::Agent));
            }
        }
        rows
    }

    /// The agent index of the selected row, if it is an agent row.
    pub fn selected_agent(&self) -> Option<usize> {
        match self.rows().get(self.selected) {
            Some(TableRow::Agent(i)) => Some(*i),
            _ => None,
        }
    }

    /// Whether an overlay (transcript / peek) is open.
    pub fn overlay_open(&self) -> bool {
        self.overlay.is_some()
    }

    /// Selection moves to a row that is safe to keep; headers and the
    /// collapsed tail are skipped so selection always lands on an agent row.
    fn clamp_selection(&mut self) {
        let rows = self.rows();
        if rows.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.min(rows.len() - 1);
        // if the current row is a header, step down to the first agent row
        while !matches!(rows[self.selected], TableRow::Agent(_)) {
            self.selected = (self.selected + 1) % rows.len();
        }
    }

    /// Move the selection, skipping group headers.
    fn move_selection(&mut self, delta: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let n = rows.len();
        let mut steps = 0;
        while steps < n {
            self.selected =
                (self.selected as isize + delta.signum()).rem_euclid(n as isize) as usize;
            steps += 1;
            if matches!(rows[self.selected], TableRow::Agent(_)) {
                break;
            }
        }
        self.scroll_to_selected();
    }

    fn scroll_to_selected(&mut self) {
        let area_rows = self.view_height as usize;
        if self.selected < self.scroll as usize {
            self.scroll = self.selected as u16;
        } else if self.selected >= self.scroll as usize + area_rows {
            self.scroll = (self.selected + 1 - area_rows) as u16;
        }
    }

    fn dismiss_selected(&mut self) {
        if let Some(i) = self.selected_agent() {
            let name = self.agents[i].name.clone();
            self.dismissed.push(name.clone());
            self.agents.retain(|a| a.name != name);
            self.clamp_selection();
        }
    }

    /// Selected row styled line (icon + name + activity + age).
    fn row_line(&self, i: usize, selected: bool) -> Line<'static> {
        let agent = &self.agents[i];
        let icon = agent
            .status
            .icon(agent.status == AgentStatus::Working && self.tick % 2 == 1);
        let icon_style = Style::default()
            .fg(agent.status.color())
            .add_modifier(Modifier::BOLD);
        let name = format!(" {}", agent.name);
        let name_style = Style::default()
            .fg(agent.color.color())
            .add_modifier(Modifier::BOLD);
        let rest = if agent.description.is_empty() {
            agent.activity.clone()
        } else {
            format!("{}", agent.description)
        };
        let mut spans = vec![
            Span::styled(icon, icon_style),
            Span::styled(name, name_style),
            Span::styled(format!("  {}", rest), theme::dim()),
        ];
        if let Some(pr) = agent.pr {
            spans.push(Span::styled(
                format!("  #{}", pr),
                Style::default()
                    .fg(ratatui::style::Color::LightBlue)
                    .add_modifier(Modifier::UNDERLINED),
            ));
        }
        if let Some(age) = agent.age {
            spans.push(Span::styled(format!("  {}", fmt_age(age)), theme::dim()));
        }
        if selected {
            spans.insert(
                0,
                Span::styled(
                    "❯",
                    Style::default()
                        .fg(agent.status.color())
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }
        Line::from(spans)
    }

    fn draw_table(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = self.rows();
        self.clamp_selection();
        // header: summary counts
        let working = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count();
        let done = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Completed)
            .count();
        let header = Line::from(vec![
            Span::styled("agents", theme::tool_running()),
            Span::styled(format!(" · {} sessions", self.agents.len()), theme::text()),
            Span::styled(format!(" · {} working", working), theme::dim()),
            Span::styled(format!(" · {} done", done), theme::dim()),
        ]);
        buf.set_line(area.x, area.y, &header, area.width);
        let table_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(1),
        };
        self.view_height = table_area.height;

        let mut y = table_area.y;
        let mut skipped = 0u16;
        let mut row_index = 0usize;
        for row in &rows {
            if skipped < self.scroll {
                skipped += 1;
                continue;
            }
            if y >= table_area.y + table_area.height {
                break;
            }
            match row {
                TableRow::Header(group) => {
                    buf.set_line(
                        table_area.x,
                        y,
                        &Line::from(Span::styled(group.title(), theme::dim())),
                        table_area.width,
                    );
                }
                TableRow::More(hidden) => {
                    buf.set_line(
                        table_area.x,
                        y,
                        &Line::from(Span::styled(
                            format!("  … {} more (click to expand)", hidden),
                            theme::dim(),
                        )),
                        table_area.width,
                    );
                }
                TableRow::Agent(i) => {
                    let line = self.row_line(*i, row_index == self.selected);
                    let style = if row_index == self.selected {
                        Style::default().bg(ratatui::style::Color::Rgb(30, 30, 30))
                    } else {
                        Style::default()
                    };
                    buf.set_line(table_area.x, y, &line.style(style), table_area.width);
                }
            }
            y += 1;
            row_index += 1;
            skipped += 1;
        }
    }

    fn toggle_completed(&mut self) {
        self.completed_expanded = !self.completed_expanded;
        self.clamp_selection();
    }

    fn draw_overlay(&mut self, area: Rect, buf: &mut Buffer) {
        match self.overlay {
            Some(Overlay::Transcript(i)) => {
                let title = {
                    let agent = &self.agents[i];
                    format!("transcript · {}", agent.name)
                };
                let lines = self.agents[i].transcript_lines.clone();
                Self::draw_panel(
                    area,
                    buf,
                    &title,
                    &lines,
                    &mut self.transcript_scroll,
                    TRANSCRIPT_CAP,
                );
            }
            Some(Overlay::Peek(i)) => {
                if let Some(agent) = self.agents.get(i) {
                    let mut lines: Vec<Line<'static>> = Vec::new();
                    if !agent.activity.is_empty() {
                        lines.push(Line::styled(agent.activity.clone(), theme::text()));
                    }
                    if let Some(result) = &agent.result {
                        lines.push(Line::styled(format!("result: {}", result), theme::dim()));
                    }
                    if let Some(waiting) = agent.waiting {
                        lines.push(Line::styled(
                            format!("waiting {}", fmt_age(waiting)),
                            theme::text(),
                        ));
                    }
                    if let Some(pr) = agent.pr {
                        lines.push(Line::styled(format!("PR #{}", pr), theme::text()));
                    }
                    let reply_line = Line::from(vec![
                        Span::styled("reply › ", theme::tool_running()),
                        Span::styled(self.reply.clone(), theme::text()),
                        Span::styled("▋", theme::tool_running()),
                    ]);
                    Self::draw_peek(area, buf, &agent.name, &lines, reply_line);
                }
            }
            None => {}
        }
    }

    /// Bottom-docked panel: title + scrollable styled lines.
    fn draw_panel(
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        lines: &[Line<'static>],
        scroll: &mut u16,
        cap: usize,
    ) {
        let shown = lines.len().min(cap);
        let height = (shown as u16 + 2).min(area.height);
        let rect = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(height),
            width: area.width,
            height,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .style(theme::overlay())
            .border_style(theme::overlay_border())
            .title(Line::from(Span::styled(title, theme::tool_running())))
            .title_alignment(ratatui::layout::Alignment::Left);
        let inner = block.inner(rect);
        block.render(rect, buf);
        let view = inner.height as usize;
        if lines.len() > view {
            let max = (lines.len() - view) as u16;
            if *scroll > max {
                *scroll = max;
            }
        } else {
            *scroll = 0;
        }
        for (i, line) in lines.iter().skip(*scroll as usize).take(view).enumerate() {
            buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
        }
    }

    /// Peek panel: status lines + reply input.
    fn draw_peek(
        area: Rect,
        buf: &mut Buffer,
        name: &str,
        lines: &[Line<'static>],
        reply: Line<'static>,
    ) {
        let height = (lines.len() as u16 + 4).min(area.height);
        let rect = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(height),
            width: area.width,
            height,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .style(theme::overlay())
            .border_style(theme::overlay_border())
            .title(Line::from(Span::styled(
                format!("peek · {}", name),
                theme::tool_running(),
            )))
            .title_alignment(ratatui::layout::Alignment::Left);
        let inner = block.inner(rect);
        block.render(rect, buf);
        let mut y = inner.y;
        for line in lines {
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_line(inner.x + 1, y, line, inner.width.saturating_sub(1));
            y += 1;
        }
        if y < inner.y + inner.height {
            buf.set_line(inner.x + 1, y, &reply, inner.width.saturating_sub(1));
        }
    }
}

impl Component for AgentView {
    fn title(&self) -> &str {
        "agents"
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        self.draw_table(area, buf);
        self.draw_overlay(area, buf);
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(overlay) = self.overlay {
                    match overlay {
                        Overlay::Transcript(_) => match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.overlay = None;
                                true
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.transcript_scroll = self.transcript_scroll.saturating_sub(1);
                                true
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.transcript_scroll += 1;
                                true
                            }
                            _ => true,
                        },
                        Overlay::Peek(_) => match key.code {
                            KeyCode::Esc => {
                                self.overlay = None;
                                true
                            }
                            KeyCode::Char(c) if !c.is_control() => {
                                self.reply.push(c);
                                true
                            }
                            KeyCode::Backspace => {
                                self.reply.pop();
                                true
                            }
                            KeyCode::Enter => {
                                self.overlay = None;
                                true
                            }
                            _ => true,
                        },
                    }
                } else {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.move_selection(-1);
                            true
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.move_selection(1);
                            true
                        }
                        KeyCode::Enter => {
                            if let Some(i) = self.selected_agent() {
                                self.transcript_scroll = 0;
                                self.overlay = Some(Overlay::Transcript(i));
                            }
                            true
                        }
                        KeyCode::Char(' ') => {
                            if let Some(i) = self.selected_agent() {
                                self.reply.clear();
                                self.overlay = Some(Overlay::Peek(i));
                            }
                            true
                        }
                        KeyCode::Char('x') => {
                            self.dismiss_selected();
                            true
                        }
                        KeyCode::PageDown => {
                            self.move_selection(10);
                            true
                        }
                        KeyCode::PageUp => {
                            self.move_selection(-10);
                            true
                        }
                        KeyCode::Char('e') => {
                            if self.hidden_completed() > 0 {
                                self.toggle_completed();
                            }
                            true
                        }
                        _ => false,
                    }
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    self.move_selection(-3);
                    true
                }
                MouseEventKind::ScrollDown => {
                    self.move_selection(3);
                    true
                }
                MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    if self.overlay.is_some() {
                        self.overlay = None;
                        return true;
                    }
                    let row = m.row as usize;
                    if row >= self.view_height as usize + 1 {
                        return true;
                    }
                    let rows = self.rows();
                    let index = self.scroll as usize + row.saturating_sub(1);
                    if index < rows.len() {
                        match &rows[index] {
                            TableRow::Agent(i) => {
                                let i = *i;
                                self.selected = index;
                                if self.selected == index {
                                    self.overlay = Some(Overlay::Transcript(i));
                                }
                            }
                            TableRow::More(_) => {
                                self.toggle_completed();
                            }
                            TableRow::Header(_) => {}
                        }
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
    }

    fn absorb_agents(&mut self, agents: &[Agent]) {
        self.merge_agents(agents.to_vec());
    }

    fn status(&self) -> String {
        let working = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count();
        let done = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Completed)
            .count();
        format!(
            "{} sessions · {} working · {} done",
            self.agents.len(),
            working,
            done
        )
    }

    fn hints(&self) -> &'static str {
        if self.overlay.is_some() {
            "[esc] close"
        } else {
            "[j/k] select  [enter] transcript  [space] peek  [x] dismiss  [e] expand"
        }
    }

    fn footer_badges(&self) -> Vec<FooterBadge> {
        let working = self
            .agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count();
        if working > 0 {
            vec![FooterBadge::new(
                format!("← {} agents", working),
                theme::tool_running(),
            )]
        } else {
            Vec::new()
        }
    }
}
