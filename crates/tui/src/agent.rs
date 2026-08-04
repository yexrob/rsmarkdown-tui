//! Agent session model for the agent overview view (Claude Code Agent View).
//!
//! Evidence (research doc §5.2): sessions are grouped into Pinned /
//! Ready for review / Needs input / Working / Completed; each row shows a
//! status icon, the session name, its current activity, an optional PR
//! number and an age on the right; sessions carry one of eight identity
//! colors (red/blue/green/yellow/purple/orange/pink/cyan). Working
//! animates, Needs input is yellow, Completed green, Failed red, Stopped
//! gray. The model is pure data — it does not know about the chat or any
//! other component, so any app can feed it.

use std::time::Duration;

/// Lifecycle of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Pinned to the top of the view.
    Pinned,
    /// Work is done; something (a PR, a diff) is waiting for review.
    ReadyForReview,
    /// The agent is waiting for input.
    NeedsInput,
    /// The agent is actively working.
    Working,
    /// Finished successfully.
    Completed,
    /// Failed.
    Failed,
    /// Stopped / dismissed.
    Stopped,
}

/// The display groups of the overview, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentGroup {
    /// `Pinned`
    Pinned,
    /// `Ready for review`
    ReadyForReview,
    /// `Needs input`
    NeedsInput,
    /// `Working`
    Working,
    /// `Completed`
    Completed,
}

impl AgentGroup {
    /// Group title as shown in the view.
    pub fn title(self) -> &'static str {
        match self {
            AgentGroup::Pinned => "Pinned",
            AgentGroup::ReadyForReview => "Ready for review",
            AgentGroup::NeedsInput => "Needs input",
            AgentGroup::Working => "Working",
            AgentGroup::Completed => "Completed",
        }
    }
}

impl AgentStatus {
    /// Which group a status belongs to (Failed / Stopped land in
    /// `Completed`, colored distinctly).
    pub fn group(self) -> AgentGroup {
        match self {
            AgentStatus::Pinned => AgentGroup::Pinned,
            AgentStatus::ReadyForReview => AgentGroup::ReadyForReview,
            AgentStatus::NeedsInput => AgentGroup::NeedsInput,
            AgentStatus::Working => AgentGroup::Working,
            AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Stopped => {
                AgentGroup::Completed
            }
        }
    }

    /// Status icon; `anim` selects the animation frame for `Working`
    /// (evidence: working rows animate between `✽` and `✢`).
    pub fn icon(self, anim: bool) -> &'static str {
        match self {
            AgentStatus::Pinned => "✽",
            AgentStatus::ReadyForReview => "∙",
            AgentStatus::NeedsInput => "✻",
            AgentStatus::Working => {
                if anim {
                    "✢"
                } else {
                    "✽"
                }
            }
            AgentStatus::Completed => "∙",
            AgentStatus::Failed => "✕",
            AgentStatus::Stopped => "∙",
        }
    }

    /// Semantic status color (evidence: needs-input yellow, completed
    /// green, failed red, stopped gray).
    pub fn color(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            AgentStatus::Pinned => Color::LightCyan,
            AgentStatus::ReadyForReview => Color::LightBlue,
            AgentStatus::NeedsInput => Color::LightYellow,
            AgentStatus::Working => Color::LightCyan,
            AgentStatus::Completed => Color::LightGreen,
            AgentStatus::Failed => Color::LightRed,
            AgentStatus::Stopped => Color::DarkGray,
        }
    }
}

/// One of the eight identity colors assigned to concurrent agents
/// (evidence: red, blue, green, yellow, purple, orange, pink, cyan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentColor {
    /// red
    Red,
    /// blue
    Blue,
    /// green
    Green,
    /// yellow
    Yellow,
    /// purple
    Purple,
    /// orange
    Orange,
    /// pink
    Pink,
    /// cyan
    Cyan,
}

impl AgentColor {
    /// The eight identity colors, in order.
    pub const ALL: [AgentColor; 8] = [
        AgentColor::Red,
        AgentColor::Blue,
        AgentColor::Green,
        AgentColor::Yellow,
        AgentColor::Purple,
        AgentColor::Orange,
        AgentColor::Pink,
        AgentColor::Cyan,
    ];

    /// Terminal color of this identity.
    pub fn color(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            AgentColor::Red => Color::LightRed,
            AgentColor::Blue => Color::LightBlue,
            AgentColor::Green => Color::LightGreen,
            AgentColor::Yellow => Color::LightYellow,
            AgentColor::Purple => Color::LightMagenta,
            AgentColor::Orange => Color::Rgb(255, 165, 0),
            AgentColor::Pink => Color::Rgb(255, 105, 180),
            AgentColor::Cyan => Color::LightCyan,
        }
    }

    /// Pick a stable identity color from a name (hash-based).
    pub fn from_name(name: &str) -> AgentColor {
        let hash: u32 = name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        Self::ALL[(hash as usize) % Self::ALL.len()]
    }
}

/// One session row of the agent overview.
#[derive(Debug, Clone)]
pub struct Agent {
    /// Session / agent name.
    pub name: String,
    /// One-line description (the `name · description · tokens` status line).
    pub description: String,
    /// Current activity sentence.
    pub activity: String,
    /// Lifecycle status.
    pub status: AgentStatus,
    /// Identity color of the name.
    pub color: AgentColor,
    /// Optional associated PR number (shown as `#N`).
    pub pr: Option<u32>,
    /// Age of the session (shown right-aligned, e.g. `3m` / `2h`).
    pub age: Option<Duration>,
    /// Token count for the status line.
    pub tokens: Option<usize>,
    /// How long the agent has been waiting for input (peek panel).
    pub waiting: Option<Duration>,
    /// Result / status sentence (peek panel).
    pub result: Option<String>,
    /// Pre-rendered transcript lines (`Enter` opens them as an overlay).
    pub transcript_lines: Vec<ratatui::text::Line<'static>>,
}

impl Agent {
    /// Build an agent with the given name and status.
    pub fn new(name: impl Into<String>, status: AgentStatus) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            activity: String::new(),
            status,
            color: AgentColor::Cyan,
            pr: None,
            age: None,
            tokens: None,
            waiting: None,
            result: None,
            transcript_lines: Vec::new(),
        }
    }

    /// Set the one-line description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the current activity sentence.
    pub fn activity(mut self, activity: impl Into<String>) -> Self {
        self.activity = activity.into();
        self
    }

    /// Set the identity color.
    pub fn color(mut self, color: AgentColor) -> Self {
        self.color = color;
        self
    }

    /// Attach an associated PR number.
    pub fn pr(mut self, pr: u32) -> Self {
        self.pr = Some(pr);
        self
    }

    /// Set the session age.
    pub fn age(mut self, age: Duration) -> Self {
        self.age = Some(age);
        self
    }

    /// Set the token count.
    pub fn tokens(mut self, tokens: usize) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the waiting duration (peek panel).
    pub fn waiting(mut self, waiting: Duration) -> Self {
        self.waiting = Some(waiting);
        self
    }

    /// Set the result / status sentence.
    pub fn result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Attach the pre-rendered transcript.
    pub fn transcript(mut self, lines: Vec<ratatui::text::Line<'static>>) -> Self {
        self.transcript_lines = lines;
        self
    }
}

/// Compact age formatting: `42s`, `3m`, `2h`, `1d`.
pub fn fmt_age(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// Merge a fresh snapshot into existing agents: same-name agents are
/// updated in place, new ones appended, order follows the snapshot.
pub fn merge_agents(existing: &mut Vec<Agent>, incoming: Vec<Agent>, dismissed: &[String]) {
    for agent in incoming {
        if dismissed.iter().any(|d| d == &agent.name) {
            continue;
        }
        if let Some(slot) = existing.iter_mut().find(|a| a.name == agent.name) {
            *slot = agent;
        } else {
            existing.push(agent);
        }
    }
}
