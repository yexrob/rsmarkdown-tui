//! Performance benchmarks for the streaming markdown core.
//!
//! Scenarios:
//! 1. `full_pass`   — one-shot `process_streaming` over a whole document (the
//!                    worst case: fresh processor, whole doc re-parsed).
//! 2. `incremental` — simulate LLM streaming: a document growing chunk by
//!                    chunk. Ours (block-split + per-block cache) vs the naive
//!                    baseline (reparse the whole doc as one block each time).
//! 3. `blocks`      — block segmentation alone (the O(n) scan that runs on
//!                    every stream update).
//!
//! Run: `cargo bench -p rsmarkdown-core` (add `-- --quick` for CI).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rsmarkdown_core::{MarkdownProcessor, Mode};

/// Deterministic synthetic markdown document of roughly `kb` kilobytes.
/// Mixture of headings, paragraphs, lists, tables, code fences and math.
fn synthetic_doc(kb: usize) -> String {
    let mut out = String::with_capacity(kb * 1024 + 64);
    let blocks = [
        "# Heading\n\nSome **bold** and *italic* text with `inline code` and a [link](https://example.com).\n\n",
        "## Sub heading\n\n- item one\n- item two\n- item three\n\n1. first\n2. second\n\n",
        "| a | b | c |\n|---|---|---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |\n\n",
        "```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n",
        "> a blockquote with **bold** content\n\n$$\\int_a^b f(x)\\,dx = F(b) - F(a)$$\n\n",
        concat!(
            "Plain paragraph with a moderate amount of text to wrap and re-parse, ",
            "repeated enough to be representative of typical markdown bodies.\n\n"
        ),
    ];
    let mut total = 0;
    while total < kb * 1024 {
        for b in blocks {
            out.push_str(b);
            total += b.len();
            if total >= kb * 1024 {
                break;
            }
        }
    }
    out
}

/// Split a document into fixed-size chunks (char-boundary safe) to simulate
/// an LLM token stream.
fn chunks_of(doc: &str, size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    while pos < doc.len() {
        let mut end = (pos + size).min(doc.len());
        while end < doc.len() && !doc.is_char_boundary(end) {
            end += 1;
        }
        chunks.push(doc[pos..end].to_string());
        pos = end;
    }
    chunks
}

fn bench_full_pass(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pass");
    group.sample_size(30);
    for kb in [2usize, 64, 512] {
        let doc = synthetic_doc(kb);
        group.bench_with_input(
            BenchmarkId::new("streaming", format!("{}kb", kb)),
            &doc,
            |b, doc| {
                b.iter(|| {
                    let mut p = MarkdownProcessor::default();
                    black_box(p.process_streaming(black_box(doc)));
                });
            },
        );
    }
    group.finish();
}

fn bench_incremental(c: &mut Criterion) {
    let doc = synthetic_doc(64);
    let chunks = chunks_of(&doc, 64);

    let mut group = c.benchmark_group("incremental");
    group.sample_size(20);

    group.bench_function("block_cached", |b| {
        b.iter(|| {
            let mut p = MarkdownProcessor::default();
            let mut content = String::new();
            for ch in &chunks {
                content.push_str(ch);
                black_box(p.process_streaming(black_box(&content)));
            }
        });
    });

    // naive baseline: whole document treated as one block, re-parsed per chunk
    group.bench_function("naive_full_reparse", |b| {
        b.iter(|| {
            let mut p = MarkdownProcessor::default();
            let mut content = String::new();
            for ch in &chunks {
                content.push_str(ch);
                black_box(p.process(black_box(&content), Mode::Static));
            }
        });
    });

    // steady state: document already large, keep appending small chunks
    let big = synthetic_doc(64);
    let tail = chunks_of(&big[big.len() / 2..], 64);
    group.bench_function("steady_state_64kb", |b| {
        b.iter(|| {
            let mut p = MarkdownProcessor::default();
            let mut content = big.clone();
            for ch in &tail {
                content.push_str(ch);
                black_box(p.process_streaming(black_box(&content)));
            }
        });
    });
    group.finish();
}

fn bench_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("blocks");
    group.sample_size(30);
    for kb in [2usize, 64, 512] {
        let doc = synthetic_doc(kb);
        group.bench_with_input(
            BenchmarkId::new("split", format!("{}kb", kb)),
            &doc,
            |b, doc| {
                let mut p = MarkdownProcessor::default();
                b.iter(|| {
                    black_box(p.parse_markdown_into_blocks(black_box(doc)));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_full_pass, bench_incremental, bench_blocks);
criterion_main!(benches);
