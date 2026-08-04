//! Performance benchmarks for the agent chat component.
//!
//! Scenarios:
//! 1. `turn`        — one full scripted turn (tick until the phase is done).
//! 2. `tick_sub`    — per-tick cost while the subagent rebuilds its nested
//!                    transcript (the busiest phase).
//! 3. `draw`        — one frame after a completed turn (layout + rendering).
//! 4. `draw_11turns`— frame cost with 11 accumulated turns (big transcript).
//!
//! Run: `cargo bench -p rsmarkdown-tui` (add `-- --quick` for CI).

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use rsmarkdown_tui::components::chat::AgentChat;
use rsmarkdown_tui::Component;

fn turn(c: &mut Criterion) {
    c.bench_function("chat/turn", |b| {
        b.iter(|| {
            let mut chat = AgentChat::new();
            for _ in 0..300 {
                chat.on_tick();
            }
            black_box(chat.phase_done());
        })
    });
}

fn tick_subagent(c: &mut Criterion) {
    c.bench_function("chat/tick_mid_subagent", |b| {
        b.iter(|| {
            let mut chat = AgentChat::new();
            for _ in 0..50 {
                chat.on_tick();
            }
            for _ in 0..30 {
                chat.on_tick();
            }
            black_box(chat.phase_done());
        })
    });
}

fn draw(c: &mut Criterion) {
    c.bench_function("chat/draw", |b| {
        let mut chat = AgentChat::new();
        for _ in 0..300 {
            chat.on_tick();
        }
        let area = Rect::new(0, 0, 100, 40);
        let mut buf = Buffer::empty(area);
        b.iter(|| {
            chat.draw(area, &mut buf);
            black_box(&buf);
        })
    });
}

fn draw_big_transcript(c: &mut Criterion) {
    c.bench_function("chat/draw_11_turns", |b| {
        let mut chat = AgentChat::new();
        for round in 0..11 {
            for _ in 0..300 {
                chat.on_tick();
            }
            chat.event(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE,
                ),
            ));
            chat.event(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char('q'),
                    crossterm::event::KeyModifiers::NONE,
                ),
            ));
            black_box(round);
        }
        let area = Rect::new(0, 0, 100, 40);
        let mut buf = Buffer::empty(area);
        b.iter(|| {
            chat.draw(area, &mut buf);
            black_box(&buf);
        })
    });
}

criterion_group!(benches, turn, tick_subagent, draw, draw_big_transcript);
criterion_main!(benches);
