//! Semantic color tokens for the whole TUI (Claude Code style theming).
//!
//! Every color used by the renderer, the activities, the panels and the
//! host status bar flows through one [`Theme`] — a set of *semantic*
//! tokens (`text`, `inactive`, `success`, `permission`, `claude` accent,
//! …) mapped to concrete colors per preset.
//!
//! Presets (research doc §1): `automatic` follows the terminal (currently
//! resolves to dark), `dark` and `light` are the two concrete palettes.
//! Terminal itself controls the root background; the theme only colors
//! foregrounds, borders and panel backgrounds.
//!
//! Themes are value objects: pick one (`Theme::dark()`, `Theme::light()`),
//! hand it to components ([`crate::Component::set_theme`]) and it drives
//! every paint. The demo and the tests use `Theme::dark()`.

use ratatui::style::{Color, Modifier, Style};

/// Semantic color tokens of one theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Body text color.
    pub text: Color,
    /// Dimmed / inactive text.
    pub inactive: Color,
    /// The main accent (Claude's `claude` token; `⏺`, mode badges, …).
    pub claude: Color,
    /// Suggestion / placeholder text (uncommitted input).
    pub suggestion: Color,
    /// Permission dialog accent (`permission` token).
    pub permission: Color,
    /// Success color (done tools, diff additions, completed rows).
    pub success: Color,
    /// Error color (failed tools, diff removals).
    pub error: Color,
    /// Warning color (needs-input, pending review).
    pub warning: Color,
    /// Memory / remembered-state accent (`remember` token).
    pub remember: Color,
    /// Inline code text.
    pub code_fg: Color,
    /// Inline code background.
    pub code_bg: Color,
    /// Code block text.
    pub code_block_fg: Color,
    /// Code block background.
    pub code_block_bg: Color,
    /// Link color (underlined).
    pub link: Color,
    /// Math formula color.
    pub math: Color,
    /// Heading colors, by level (1-based; beyond the table falls back to
    /// the last entry).
    pub headings: Vec<Color>,
    /// Blockquote text.
    pub quote: Color,
    /// Blockquote left bar.
    pub quote_bar: Color,
    /// Completed task-list marker.
    pub task_done: Color,
    /// Open task-list marker.
    pub task_open: Color,
    /// List bullet / number.
    pub list_marker: Color,
    /// Table grid lines.
    pub table_border: Color,
    /// Table header row.
    pub table_header: Color,
    /// Thematic break.
    pub hr: Color,
    /// Footnote labels.
    pub footnote: Color,
    /// Thinking-block text.
    pub thinking: Color,
    /// Running tool / subagent text.
    pub tool_running: Color,
    /// Tool output preview.
    pub tool_output: Color,
    /// Diff `@@` hunk headers.
    pub diff_hunk: Color,
    /// Diff context lines.
    pub diff_context: Color,
    /// The `✻` file-edit glyph.
    pub diff_edit: Color,
    /// Focused status-bar title (background).
    pub status_bg: Color,
    /// Focused status-bar title (foreground).
    pub status_fg: Color,
    /// Unfocused / single-component status bar.
    pub status_inactive: Color,
    /// Background of floating overlays (panels, menus, dialogs). `Reset`
    /// melts the panel into the terminal's own background (dark theme);
    /// light themes use a near-white tint.
    pub overlay_bg: Color,
    /// Overlay border.
    pub overlay_border: Color,
}

impl Theme {
    /// The dark preset (default; the current look of the TUI).
    pub fn dark() -> Self {
        Self {
            text: Color::Reset,
            inactive: Color::DarkGray,
            claude: Color::LightCyan,
            suggestion: Color::DarkGray,
            permission: Color::LightYellow,
            success: Color::LightGreen,
            error: Color::LightRed,
            warning: Color::LightYellow,
            remember: Color::LightMagenta,
            code_fg: Color::LightYellow,
            code_bg: Color::Rgb(30, 30, 30),
            code_block_fg: Color::Rgb(170, 170, 170),
            code_block_bg: Color::Rgb(24, 24, 24),
            link: Color::LightBlue,
            math: Color::Magenta,
            headings: vec![
                Color::White,
                Color::LightCyan,
                Color::LightBlue,
                Color::DarkGray,
            ],
            quote: Color::Gray,
            quote_bar: Color::LightBlue,
            task_done: Color::LightGreen,
            task_open: Color::DarkGray,
            list_marker: Color::LightCyan,
            table_border: Color::DarkGray,
            table_header: Color::Reset,
            hr: Color::DarkGray,
            footnote: Color::DarkGray,
            thinking: Color::DarkGray,
            tool_running: Color::LightCyan,
            tool_output: Color::DarkGray,
            diff_hunk: Color::LightCyan,
            diff_context: Color::Gray,
            diff_edit: Color::LightYellow,
            status_bg: Color::LightCyan,
            status_fg: Color::Black,
            status_inactive: Color::DarkGray,
            // panels melt into the terminal's own background (Claude Code
            // dialogs are not dark boxes); the overlay still erases content
            // behind the panel, Reset simply shows the terminal background
            overlay_bg: Color::Reset,
            overlay_border: Color::Gray,
        }
    }

    /// The light preset: dark ink on the terminal's light background,
    /// deeper accents for contrast.
    pub fn light() -> Self {
        Self {
            text: Color::Reset,
            inactive: Color::Rgb(110, 110, 110),
            claude: Color::Rgb(0, 120, 130),
            suggestion: Color::Rgb(110, 110, 110),
            permission: Color::Rgb(150, 110, 0),
            success: Color::Rgb(0, 120, 0),
            error: Color::Rgb(170, 0, 0),
            warning: Color::Rgb(150, 110, 0),
            remember: Color::Rgb(130, 0, 130),
            code_fg: Color::Rgb(150, 110, 0),
            code_bg: Color::Rgb(245, 245, 245),
            code_block_fg: Color::Rgb(60, 60, 60),
            code_block_bg: Color::Rgb(242, 242, 242),
            link: Color::Rgb(0, 90, 180),
            math: Color::Rgb(130, 0, 130),
            headings: vec![
                Color::Rgb(30, 30, 30),
                Color::Rgb(0, 120, 130),
                Color::Rgb(0, 90, 180),
                Color::Rgb(120, 120, 120),
            ],
            quote: Color::Rgb(90, 90, 90),
            quote_bar: Color::Rgb(0, 90, 180),
            task_done: Color::Rgb(0, 120, 0),
            task_open: Color::Rgb(120, 120, 120),
            list_marker: Color::Rgb(0, 120, 130),
            table_border: Color::Rgb(120, 120, 120),
            table_header: Color::Rgb(30, 30, 30),
            hr: Color::Rgb(120, 120, 120),
            footnote: Color::Rgb(120, 120, 120),
            thinking: Color::Rgb(120, 120, 120),
            tool_running: Color::Rgb(0, 120, 130),
            tool_output: Color::Rgb(110, 110, 110),
            diff_hunk: Color::Rgb(0, 120, 130),
            diff_context: Color::Rgb(90, 90, 90),
            diff_edit: Color::Rgb(150, 110, 0),
            status_bg: Color::Rgb(0, 120, 130),
            status_fg: Color::White,
            status_inactive: Color::Rgb(120, 120, 120),
            overlay_bg: Color::Rgb(248, 248, 248),
            overlay_border: Color::Rgb(140, 140, 140),
        }
    }

    /// The automatic preset: follows the terminal. Background detection is
    /// environment-specific, so for now this resolves to the dark preset
    /// (documented; the terminal's own background stays visible either way).
    pub fn automatic() -> Self {
        Theme::dark()
    }
}

impl Theme {
    /// Default body text.
    pub fn text(&self) -> Style {
        Style::default().fg(self.text)
    }
    /// Dimmed / secondary text.
    pub fn dim(&self) -> Style {
        Style::default().fg(self.inactive)
    }
    /// Bold text.
    pub fn strong(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }
    /// Italic text.
    pub fn emphasis(&self) -> Style {
        Style::default().add_modifier(Modifier::ITALIC)
    }
    /// Struck-through text.
    pub fn strikethrough(&self) -> Style {
        Style::default()
            .fg(self.inactive)
            .add_modifier(Modifier::CROSSED_OUT)
    }
    /// Inline code.
    pub fn code(&self) -> Style {
        Style::default().fg(self.code_fg).bg(self.code_bg)
    }
    /// Fenced code block body.
    pub fn code_block(&self) -> Style {
        Style::default()
            .fg(self.code_block_fg)
            .bg(self.code_block_bg)
    }
    /// Hyperlinks (underlined).
    pub fn link(&self) -> Style {
        Style::default()
            .fg(self.link)
            .add_modifier(Modifier::UNDERLINED)
    }
    /// Math formulas (italic).
    pub fn math(&self) -> Style {
        Style::default()
            .fg(self.math)
            .add_modifier(Modifier::ITALIC)
    }
    /// Heading text for a given level (1-based).
    pub fn heading(&self, level: u8) -> Style {
        let color = if level == 0 {
            self.text
        } else {
            self.headings
                .get((level - 1) as usize)
                .copied()
                .unwrap_or(*self.headings.last().unwrap_or(&self.text))
        };
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    }
    /// Blockquote text.
    pub fn quote(&self) -> Style {
        Style::default().fg(self.quote)
    }
    /// Blockquote left bar.
    pub fn quote_bar(&self) -> Style {
        Style::default().fg(self.quote_bar)
    }
    /// Completed task-list marker.
    pub fn task_done(&self) -> Style {
        Style::default()
            .fg(self.task_done)
            .add_modifier(Modifier::BOLD)
    }
    /// Open task-list marker.
    pub fn task_open(&self) -> Style {
        Style::default().fg(self.task_open)
    }
    /// List bullet / number.
    pub fn list_marker(&self) -> Style {
        Style::default().fg(self.list_marker)
    }
    /// Table grid lines.
    pub fn table_border(&self) -> Style {
        Style::default().fg(self.table_border)
    }
    /// Table header row.
    pub fn table_header(&self) -> Style {
        Style::default()
            .fg(self.table_header)
            .add_modifier(Modifier::BOLD)
    }
    /// Thematic break line.
    pub fn hr(&self) -> Style {
        Style::default().fg(self.hr)
    }
    /// Footnote labels.
    pub fn footnote(&self) -> Style {
        Style::default().fg(self.footnote)
    }
    /// Thinking blocks (dim italic).
    pub fn thinking(&self) -> Style {
        Style::default()
            .fg(self.thinking)
            .add_modifier(Modifier::ITALIC)
    }
    /// Footer badge for an active mode (`⏵⏵` / `⏸`).
    pub fn mode_on(&self) -> Style {
        Style::default()
            .fg(self.claude)
            .add_modifier(Modifier::BOLD)
    }
    /// The `⏺` main-activity dot (the `claude` accent).
    pub fn activity_dot(&self) -> Style {
        Style::default()
            .fg(self.claude)
            .add_modifier(Modifier::BOLD)
    }
    /// Running tool / subagent.
    pub fn tool_running(&self) -> Style {
        Style::default()
            .fg(self.tool_running)
            .add_modifier(Modifier::BOLD)
    }
    /// Completed tool / subagent.
    pub fn tool_done(&self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }
    /// Failed tool / subagent.
    pub fn tool_error(&self) -> Style {
        Style::default().fg(self.error).add_modifier(Modifier::BOLD)
    }
    /// Tool output preview.
    pub fn tool_output(&self) -> Style {
        Style::default().fg(self.tool_output)
    }
    /// Diff `@@` hunk headers.
    pub fn diff_hunk(&self) -> Style {
        Style::default()
            .fg(self.diff_hunk)
            .add_modifier(Modifier::BOLD)
    }
    /// Diff added lines.
    pub fn diff_added(&self) -> Style {
        Style::default().fg(self.success)
    }
    /// Diff removed lines.
    pub fn diff_removed(&self) -> Style {
        Style::default().fg(self.error)
    }
    /// Diff context lines.
    pub fn diff_context(&self) -> Style {
        Style::default().fg(self.diff_context)
    }
    /// The `✻` file-edit glyph.
    pub fn diff_edit(&self) -> Style {
        Style::default()
            .fg(self.diff_edit)
            .add_modifier(Modifier::BOLD)
    }
    /// Focused component title in the status bar.
    pub fn status(&self) -> Style {
        Style::default().fg(self.status_fg).bg(self.status_bg)
    }
    /// Unfocused / single-component status bar.
    pub fn status_inactive(&self) -> Style {
        Style::default().fg(self.status_inactive)
    }
    /// Permission dialog title / accent.
    pub fn permission(&self) -> Style {
        Style::default()
            .fg(self.permission)
            .add_modifier(Modifier::BOLD)
    }
    /// Selected option `❯` marker in the permission dialog.
    pub fn permission_selected(&self) -> Style {
        Style::default()
            .fg(self.permission)
            .add_modifier(Modifier::BOLD)
    }
    /// Solid background for floating overlays.
    pub fn overlay(&self) -> Style {
        Style::default().bg(self.overlay_bg)
    }
    /// Overlay border.
    pub fn overlay_border(&self) -> Style {
        Style::default().fg(self.overlay_border)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_is_default_look() {
        let t = Theme::dark();
        assert_eq!(t.text(), Style::default().fg(Color::Reset));
        assert_eq!(t.claude, Color::LightCyan);
        assert_eq!(t.success, Color::LightGreen);
        assert_eq!(
            t.overlay_bg,
            Color::Reset,
            "dark panels melt into the terminal"
        );
    }

    #[test]
    fn light_uses_dark_ink() {
        let t = Theme::light();
        assert_ne!(t.claude, Theme::dark().claude, "accents differ");
        assert_eq!(t.status_fg, Color::White, "status inverts");
        assert_eq!(t.success, Color::Rgb(0, 120, 0));
    }

    #[test]
    fn automatic_resolves() {
        let t = Theme::automatic();
        assert_eq!(t, Theme::dark(), "automatic = dark for now");
    }

    #[test]
    fn heading_falls_back_for_deep_levels() {
        let t = Theme::dark();
        assert_eq!(t.heading(1).fg, Some(Color::White));
        assert_eq!(t.heading(9).fg, Some(Color::DarkGray), "beyond table");
        assert_eq!(t.heading(0).fg, Some(Color::Reset), "level 0 = plain");
    }
}
