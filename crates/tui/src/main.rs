//! Interactive streaming-markdown TUI demo.
//!
//! Renders live-streaming markdown (simulated LLM output, or your own typing)
//! through `rsmarkdown-core` + the `StreamMarkdownRenderer` display adapter.
//!
//! Keys: j/k/↑/↓ scroll, PgUp/PgDn, g/G top/bottom, t typing mode, d demo mode,
//! s auto-scroll, r restart, q quit.

mod renderer;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use renderer::theme;
use renderer::StreamMarkdownRenderer;
use rsmarkdown_core::{Document, MarkdownProcessor, Mode, Renderer};

const DEMO_DOC: &str = include_str!("../demo.md");

struct App {
    processor: MarkdownProcessor,
    renderer: StreamMarkdownRenderer,
    content: String,
    demo_pos: usize,
    typing: bool,
    auto_scroll: bool,
    scroll: u16,
    doc: Document,
    last_tick: Instant,
    blocks_rendered: usize,
    width_dirty: bool,
    last_parse_us: u64,
    cache_hits: u64,
    last_content: String,
}

impl App {
    fn new() -> Self {
        Self {
            processor: MarkdownProcessor::default(),
            renderer: StreamMarkdownRenderer::new(80),
            content: String::new(),
            demo_pos: 0,
            typing: false,
            auto_scroll: true,
            scroll: 0,
            doc: Document::default(),
            last_tick: Instant::now(),
            blocks_rendered: 0,
            width_dirty: true,
            last_parse_us: 0,
            cache_hits: 0,
            last_content: String::new(),
        }
    }

    fn restart_demo(&mut self) {
        self.content.clear();
        self.demo_pos = 0;
        self.typing = false;
        self.scroll = 0;
        self.auto_scroll = true;
    }

    fn refresh(&mut self) {
        // skip the whole pipeline when the content did not change — the tick
        // loop calls this 30x/s even while idle
        if self.last_content == self.content {
            return;
        }
        self.last_content.clone_from(&self.content);
        let t = std::time::Instant::now();
        self.doc = self.processor.process(&self.content, Mode::Streaming);
        self.last_parse_us = t.elapsed().as_micros() as u64;
        self.renderer.render(&self.doc);
        self.blocks_rendered = self.doc.blocks.len();
        self.cache_hits = self.processor.cache_stats().hits();
    }

    /// Generate a ~200 KB stress document and load it instantly.
    fn load_stress_doc(&mut self) {
        let unit = include_str!("../demo.md");
        let mut big = String::with_capacity(200 * 1024);
        while big.len() < 200 * 1024 {
            big.push_str(unit);
        }
        self.content = big;
        self.demo_pos = DEMO_DOC.len(); // demo finished
        self.typing = false;
        self.auto_scroll = false;
        self.scroll = 0;
        self.refresh();
    }

    fn advance_demo(&mut self) {
        if self.demo_pos >= DEMO_DOC.len() {
            return;
        }
        let mut chunk_end = (self.demo_pos + 2).min(DEMO_DOC.len());
        while chunk_end < DEMO_DOC.len() && !DEMO_DOC.is_char_boundary(chunk_end) {
            chunk_end += 1;
        }
        self.content.push_str(&DEMO_DOC[self.demo_pos..chunk_end]);
        self.demo_pos = chunk_end;
    }

    fn total_lines(&self) -> usize {
        self.renderer.lines().len()
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    let mut app = App::new();
    app.refresh();
    app.advance_demo();

    let tick = Duration::from_millis(33);
    let result = run(&mut terminal, &mut app, tick);
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tick: Duration,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;
        let timeout = tick.saturating_sub(app.last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if !handle_key(app, key) {
                        return Ok(());
                    }
                    app.last_tick = Instant::now();
                }
                Event::Mouse(m) => match m.kind {
                    event::MouseEventKind::ScrollUp => app.scroll = app.scroll.saturating_sub(3),
                    event::MouseEventKind::ScrollDown => app.scroll = app.scroll.saturating_add(3),
                    _ => {}
                },
                _ => {}
            }
        }
        if app.last_tick.elapsed() >= tick {
            if !app.typing {
                app.advance_demo();
            }
            app.refresh();
            app.last_tick = Instant::now();
        }
        if app.auto_scroll && app.demo_pos < DEMO_DOC.len() {
            let h = terminal.size()?.height;
            let lines = app.total_lines() as u16;
            if lines > h {
                app.scroll = lines.saturating_sub(h);
            }
        }
    }
}

fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if app.typing {
        match key.code {
            KeyCode::Char(c) => {
                if c == '`' && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return true;
                }
                app.content.push(c);
                return true;
            }
            KeyCode::Backspace => {
                app.content.pop();
                return true;
            }
            KeyCode::Esc => {
                app.typing = false;
                return true;
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Char('q') => false,
        KeyCode::Char('t') => {
            app.typing = !app.typing;
            app.auto_scroll = false;
            true
        }
        KeyCode::Char('d') => {
            app.restart_demo();
            true
        }
        KeyCode::Char('s') => {
            app.auto_scroll = !app.auto_scroll;
            true
        }
        KeyCode::Char('p') => {
            app.load_stress_doc();
            true
        }
        KeyCode::Char('r') => {
            app.restart_demo();
            app.advance_demo();
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll = app.scroll.saturating_add(1);
            app.auto_scroll = false;
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll = app.scroll.saturating_sub(1);
            true
        }
        KeyCode::PageDown => {
            app.scroll = app.scroll.saturating_add(10);
            app.auto_scroll = false;
            true
        }
        KeyCode::PageUp => {
            app.scroll = app.scroll.saturating_sub(10);
            true
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.scroll = 0;
            true
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.scroll = u16::MAX;
            true
        }
        KeyCode::Esc => {
            app.typing = false;
            true
        }
        _ => true,
    }
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let [content_area, status_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    app.renderer.set_width(content_area.width as usize);
    if app.width_dirty {
        app.refresh();
        app.width_dirty = false;
    }
    let lines = app.renderer.lines();

    let total = lines.len() as u16;
    let max_scroll = total.saturating_sub(content_area.height);
    let scroll = app.scroll.min(max_scroll);

    let visible: Vec<Line> = lines
        .iter()
        .skip(scroll as usize)
        .take(content_area.height as usize)
        .cloned()
        .collect();

    f.render_widget(
        Paragraph::new(visible).wrap(Wrap { trim: false }),
        content_area,
    );

    let mode = if app.typing {
        Span::styled(" TYPE ", theme::status())
    } else if app.demo_pos < DEMO_DOC.len() {
        Span::styled(" STREAM ", theme::status())
    } else {
        Span::styled(" DONE ", theme::status_inactive())
    };
    let status = Line::from(vec![
        mode,
        Span::raw(format!(
            " {}B {} blocks {} lines {}/{}  parse {}µs  cached {}  ",
            app.content.len(),
            app.blocks_rendered,
            app.total_lines(),
            scroll,
            total,
            app.last_parse_us,
            app.cache_hits,
        )),
        Span::styled(
            if app.typing {
                "[esc] stop  [d] demo  [q] quit"
            } else {
                "[t] type  [d] demo  [p] stress 200KB  [j/k] scroll  [s] autoscroll  [q] quit"
            },
            Style::default(),
        ),
    ]);
    f.render_widget(Paragraph::new(status), status_area);
}
