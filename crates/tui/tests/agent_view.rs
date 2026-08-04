//! Tests for the agent model and the agent overview component: grouping,
//! rows, overlays, dismissal, the host agent broadcast, and the chat demo's
//! subagent export.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Terminal;

use rsmarkdown_tui::agent::{merge_agents, Agent, AgentColor, AgentGroup, AgentStatus};
use rsmarkdown_tui::components::agent_view::AgentView;
use rsmarkdown_tui::{App, Component};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        out.push('\n');
    }
    out
}

fn draw(view: &mut AgentView, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
    terminal
        .draw(|f| view.draw(f.area(), f.buffer_mut()))
        .expect("draw");
    buffer_text(terminal.backend().buffer())
}

fn sample_agents() -> Vec<Agent> {
    vec![
        Agent::new("clawd walk cycle", AgentStatus::Pinned)
            .description("Drawing the walk-cycle sprite frames")
            .activity("drawing")
            .age(Duration::from_secs(180))
            .color(AgentColor::Purple),
        Agent::new("jump physics", AgentStatus::ReadyForReview)
            .description("Opened PR with collision fix")
            .pr(2048)
            .age(Duration::from_secs(7200))
            .color(AgentColor::Blue),
        Agent::new("power-up design", AgentStatus::NeedsInput)
            .description("double jump or wall climb?")
            .waiting(Duration::from_secs(60))
            .age(Duration::from_secs(60))
            .color(AgentColor::Yellow),
        Agent::new("collision detection", AgentStatus::Working)
            .description("Adding swept-AABB checks to CollisionSystem")
            .age(Duration::from_secs(120))
            .color(AgentColor::Cyan),
        Agent::new("title screen", AgentStatus::Completed)
            .description("result: menu, options, and credits done")
            .age(Duration::from_secs(540))
            .color(AgentColor::Green),
        Agent::new("sound effects", AgentStatus::Completed)
            .description("result: 14 SFX exported to assets/audio")
            .age(Duration::from_secs(14400))
            .color(AgentColor::Orange),
        Agent::new("broken build", AgentStatus::Failed)
            .description("compile error in CollisionSystem")
            .age(Duration::from_secs(600))
            .color(AgentColor::Red),
    ]
}

// --- model ---

#[test]
fn status_groups_and_icons() {
    assert_eq!(AgentStatus::Pinned.group(), AgentGroup::Pinned);
    assert_eq!(
        AgentStatus::ReadyForReview.group(),
        AgentGroup::ReadyForReview
    );
    assert_eq!(AgentStatus::NeedsInput.group(), AgentGroup::NeedsInput);
    assert_eq!(AgentStatus::Working.group(), AgentGroup::Working);
    assert_eq!(AgentStatus::Completed.group(), AgentGroup::Completed);
    assert_eq!(
        AgentStatus::Failed.group(),
        AgentGroup::Completed,
        "failed lands in completed"
    );
    assert_eq!(
        AgentStatus::Stopped.group(),
        AgentGroup::Completed,
        "stopped lands in completed"
    );
    assert_eq!(AgentStatus::Working.icon(false), "✽");
    assert_eq!(AgentStatus::Working.icon(true), "✢", "working animates");
    assert_eq!(AgentStatus::Completed.icon(false), "∙");
}

#[test]
fn eight_identity_colors_are_stable() {
    assert_eq!(AgentColor::ALL.len(), 8);
    let a = AgentColor::from_name("explore");
    let b = AgentColor::from_name("explore");
    assert_eq!(a, b, "stable per name");
}

#[test]
fn merge_updates_in_place_and_respects_dismissed() {
    let mut existing = sample_agents();
    let incoming = vec![
        Agent::new("collision detection", AgentStatus::Completed).description("swept-AABB done"),
        Agent::new("new agent", AgentStatus::Working),
    ];
    merge_agents(&mut existing, incoming, &[]);
    assert_eq!(existing.len(), 8, "updated + appended");
    assert_eq!(
        existing[3].status,
        AgentStatus::Completed,
        "updated in place"
    );
    assert!(existing.iter().any(|a| a.name == "new agent"));

    // dismissed names are ignored by later merges
    let mut existing = sample_agents();
    merge_agents(
        &mut existing,
        vec![Agent::new("ghost", AgentStatus::Working)],
        &["ghost".into()],
    );
    assert!(!existing.iter().any(|a| a.name == "ghost"));
}

// --- component ---

#[test]
fn draws_grouped_table_with_counts() {
    let mut view = AgentView::new();
    view.set_agents(sample_agents());
    let text = draw(&mut view, 100, 40);

    assert!(text.contains("agents · 7 sessions"), "header counts");
    assert!(text.contains("Pinned"), "group header");
    assert!(text.contains("Ready for review"), "group header");
    assert!(text.contains("Needs input"), "group header");
    assert!(text.contains("Working"), "group header");
    assert!(text.contains("Completed"), "group header");
    assert!(text.contains("clawd walk cycle"), "agent row");
    assert!(text.contains("#2048"), "pr column");
    assert!(text.contains("2h"), "age formatted");
    assert!(text.contains("✽ collision detection"), "working icon");
    assert!(text.contains("✻ power-up design"), "needs-input icon");
    assert!(
        text.contains("✕ broken build"),
        "failed icon in completed group"
    );
}

#[test]
fn selection_moves_across_groups() {
    let mut view = AgentView::new();
    view.set_agents(sample_agents());
    assert_eq!(
        view.agents()[view.selected_agent().expect("agent selected")].name,
        "clawd walk cycle",
        "selection starts on the first agent row, not a header"
    );
    view.event(key(KeyCode::Char('j')));
    view.event(key(KeyCode::Char('j')));
    let agent = view.selected_agent().expect("agent selected");
    assert_eq!(
        view.agents()[agent].name,
        "power-up design",
        "jumps over headers"
    );
    view.event(key(KeyCode::Char('k')));
    let agent = view.selected_agent().expect("agent selected");
    assert_eq!(view.agents()[agent].name, "jump physics");
}

#[test]
fn enter_opens_transcript_esc_closes() {
    let mut agent = Agent::new("explore", AgentStatus::Working);
    agent.transcript_lines = vec![
        ratatui::text::Line::from("thinking"),
        ratatui::text::Line::from("running bash"),
    ];
    let mut view = AgentView::new();
    view.set_agents(vec![agent]);
    view.event(key(KeyCode::Enter));
    assert!(view.overlay_open(), "transcript open");
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("transcript · explore"), "overlay title");
    assert!(text.contains("running bash"), "transcript line");
    view.event(key(KeyCode::Esc));
    assert!(!view.overlay_open(), "esc closed");
}

#[test]
fn space_opens_peek_with_reply_input() {
    let mut view = AgentView::new();
    view.set_agents(sample_agents());
    // move to power-up design (waiting 1m)
    view.event(key(KeyCode::Char('j')));
    view.event(key(KeyCode::Char('j')));
    view.event(key(KeyCode::Char(' ')));
    assert!(view.overlay_open(), "peek open");
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("peek · power-up design"), "peek title");
    assert!(text.contains("waiting 1m"), "waiting duration");
    assert!(text.contains("double jump or wall climb?"), "question");
    // type a reply
    view.event(key(KeyCode::Char('y')));
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("reply › y"), "typed into reply");
    view.event(key(KeyCode::Enter));
    assert!(!view.overlay_open(), "enter closes peek");
}

#[test]
fn x_dismisses_agent_and_merge_ignores_it() {
    let mut view = AgentView::new();
    view.set_agents(sample_agents());
    view.event(key(KeyCode::Char('x'))); // dismiss clawd (first agent row)
    assert_eq!(view.agents().len(), 6, "one dismissed");
    assert!(!view.agents().iter().any(|a| a.name == "clawd walk cycle"));
    let dismissed = view.take_dismissed();
    assert_eq!(dismissed, vec!["clawd walk cycle".to_string()]);

    // a later merge brings the same name back (dismissed queue emptied)
    view.merge_agents(vec![Agent::new("clawd walk cycle", AgentStatus::Working)]);
    assert!(view.agents().iter().any(|a| a.name == "clawd walk cycle"));
}

#[test]
fn completed_collapses_and_expands() {
    let mut view = AgentView::new();
    view.set_agents(sample_agents());
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("title screen"), "first completed rows shown");
    assert!(
        text.contains("broken build"),
        "all completed shown (3 <= MAX)"
    );

    // many completed agents collapse to a tail
    let mut many = sample_agents();
    for i in 0..10 {
        many.push(Agent::new(format!("extra {i}"), AgentStatus::Completed));
    }
    view.set_agents(many);
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("… 5 more"), "collapsed tail");
    assert!(!text.contains("extra 9"), "hidden");

    view.event(key(KeyCode::Char('e'))); // expand
    let text = draw(&mut view, 100, 40);
    assert!(text.contains("extra 9"), "expanded");
    assert!(!text.contains("… 5 more"), "tail gone");
}

#[test]
fn working_icon_animates_on_tick() {
    let mut view = AgentView::new();
    view.set_agents(vec![Agent::new("worker", AgentStatus::Working)]);
    let text1 = draw(&mut view, 100, 40);
    assert!(text1.contains("✽ worker"));
    view.on_tick();
    let text2 = draw(&mut view, 100, 40);
    assert!(text2.contains("✢ worker"), "animation frame flips");
}

// --- host broadcast ---

/// A component that publishes one agent.
struct Publisher {
    agent: Agent,
}

impl Component for Publisher {
    fn title(&self) -> &str {
        "publisher"
    }
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, "p", Style::default());
    }
    fn agents(&self) -> Vec<Agent> {
        vec![self.agent.clone()]
    }
}

/// A component that records the broadcast it receives.
struct Sink {
    received: Rc<RefCell<Vec<Agent>>>,
}

impl Component for Sink {
    fn title(&self) -> &str {
        "sink"
    }
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, "s", Style::default());
    }
    fn absorb_agents(&mut self, agents: &[Agent]) {
        *self.received.borrow_mut() = agents.to_vec();
    }
}

#[test]
fn host_broadcasts_agents_to_components() {
    let received = Rc::new(RefCell::new(Vec::new()));
    let mut app = App::new(vec![
        Box::new(Publisher {
            agent: Agent::new("broadcast-agent", AgentStatus::Working),
        }),
        Box::new(Sink {
            received: received.clone(),
        }),
    ]);
    app.tick_components();
    let received = received.borrow();
    assert_eq!(received.len(), 1, "received the broadcast");
    assert_eq!(received[0].name, "broadcast-agent");
}

#[test]
fn chat_agents_broadcast_nonempty_during_subagent_phase() {
    let mut chat = rsmarkdown_tui::components::chat::AgentChat::new();
    for _ in 0..60 {
        chat.on_tick();
    }
    let agents = chat.agents();
    assert!(!agents.is_empty(), "chat broadcasts its subagent");
    assert_eq!(agents[0].name, "explore");
    assert_eq!(
        agents[0].status,
        rsmarkdown_tui::agent::AgentStatus::Working
    );
}

#[test]
fn agent_view_receives_chat_subagents_via_host() {
    // end-to-end: the demo chat broadcasts its subagents; an AgentView
    // mounted alongside absorbs them through the host
    let mut app = App::new(vec![
        Box::new(rsmarkdown_tui::components::chat::AgentChat::new()),
        Box::new(AgentView::new()),
    ]);
    // drive the turn into the subagent phase (thinking 1.4s + 1.2s window)
    for _ in 0..60 {
        app.tick_components();
    }
    // only the focused component is drawn; focus the agent view
    app.focus_next();
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).expect("backend");
    terminal
        .draw(|f| app.draw_frame(f.area(), f.buffer_mut()))
        .expect("draw");
    let text = buffer_text(terminal.backend().buffer());
    assert!(
        text.contains("explore"),
        "agent overview absorbed the chat subagent:\n{text}"
    );
    assert!(text.contains("Working"), "subagent grouped as working");
    assert!(
        text.contains("agents · 1 sessions"),
        "header counts the broadcast"
    );
}
