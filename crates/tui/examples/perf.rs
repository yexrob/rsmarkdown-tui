//! Performance demo: a headless report showing where streaming time goes.
//!
//! Run: `cargo run -p rsmarkdown-tui --example perf`
//!
//! Covers:
//! 1. one-shot full pass at several document sizes (throughput)
//! 2. per-stage timing breakdown (normalize / block split / preprocess+parse)
//! 3. incremental streaming vs naive full-reparse (the cache win)
//! 4. steady state: per-chunk cost once the document is large

use std::time::Instant;

use rsmarkdown_core::{MarkdownProcessor, Mode};

/// Deterministic synthetic markdown document of roughly `kb` kilobytes.
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

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn us(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1_000_000.0
}

fn main() {
    println!("\n=== rsmarkdown performance demo ===\n");

    // 1. one-shot full pass
    println!(
        "{:<12} {:>10} {:>14} {:>14}",
        "doc size", "blocks", "parse (ms)", "MB/s"
    );
    for kb in [2usize, 64, 512] {
        let doc = synthetic_doc(kb);
        let mut p = MarkdownProcessor::default();
        let t = Instant::now();
        let d = p.process_streaming(&doc);
        let elapsed = t.elapsed();
        println!(
            "{:<12} {:>10} {:>14.2} {:>14.1}",
            format!("{} KB", kb),
            d.blocks.len(),
            ms(elapsed),
            (kb as f64) / ms(elapsed),
        );
    }

    // 2. stage breakdown on a 512 KB doc
    {
        let doc = synthetic_doc(512);
        let mut p = MarkdownProcessor::default();

        let t = Instant::now();
        let normalized = p.normalize(&doc);
        let t_norm = t.elapsed();

        let t = Instant::now();
        let blocks = p.parse_markdown_into_blocks(&normalized);
        let t_blocks = t.elapsed();

        let t = Instant::now();
        let mut doc_result = None;
        for (i, b) in blocks.iter().enumerate() {
            let is_last = i == blocks.len() - 1;
            let content = if is_last { p.preprocess(b) } else { b.clone() };
            let _ = p.parse(&content);
            doc_result = Some(content);
        }
        let t_parse = t.elapsed();
        let _ = doc_result;

        println!("\nstage breakdown (512 KB, {} blocks):", blocks.len());
        println!(
            "  normalize            {:>9.2} ms  {:>5.1}%",
            ms(t_norm),
            100.0 * ms(t_norm) / ms(t_norm + t_blocks + t_parse)
        );
        println!(
            "  block split          {:>9.2} ms  {:>5.1}%",
            ms(t_blocks),
            100.0 * ms(t_blocks) / ms(t_norm + t_blocks + t_parse)
        );
        println!(
            "  preprocess + parse   {:>9.2} ms  {:>5.1}%",
            ms(t_parse),
            100.0 * ms(t_parse) / ms(t_norm + t_blocks + t_parse)
        );
    }

    // 3. incremental streaming vs naive full reparse
    {
        let doc = synthetic_doc(64);
        let chunks = chunks_of(&doc, 64);
        println!(
            "\nincremental streaming: {} KB doc, {} chunks of 64 B",
            doc.len() / 1024,
            chunks.len()
        );

        let mut p = MarkdownProcessor::default();
        let mut content = String::new();
        let t = Instant::now();
        for ch in &chunks {
            content.push_str(ch);
            p.process_streaming(&content);
        }
        let ours = t.elapsed();
        let stats = p.cache_stats();
        println!(
            "  block-cached         {:>9.2} ms   {:>8.1} µs/chunk   cache hits {}/{} ({:.0}%)",
            ms(ours),
            us(ours) / chunks.len() as f64,
            stats.hits(),
            stats.parses(),
            100.0 * stats.hit_rate(),
        );

        let mut p = MarkdownProcessor::default();
        let mut content = String::new();
        let t = Instant::now();
        for ch in &chunks {
            content.push_str(ch);
            p.process(&content, Mode::Static); // whole doc, one block, no cache reuse
        }
        let naive = t.elapsed();
        println!(
            "  naive full-reparse   {:>9.2} ms   {:>8.1} µs/chunk",
            ms(naive),
            us(naive) / chunks.len() as f64,
        );
        println!("  speedup              {:>8.1}x", ms(naive) / ms(ours));
    }

    // 4. steady state: large doc, keep appending
    {
        let big = synthetic_doc(128);
        let tail = chunks_of(&big[big.len() / 2..], 64);
        let mut p = MarkdownProcessor::default();
        let mut content = big.clone();
        p.process_streaming(&content);
        let t = Instant::now();
        for ch in &tail {
            content.push_str(ch);
            p.process_streaming(&content);
        }
        let elapsed = t.elapsed();
        println!(
            "\nsteady state: {} KB doc, {} more chunks",
            content.len() / 1024,
            tail.len()
        );
        println!(
            "  per-chunk parse      {:>8.1} µs   (normalize+split scan the whole doc)",
            us(elapsed) / tail.len() as f64,
        );
        // naive: same content, but whole doc re-parsed as one block each time
        let mut p = MarkdownProcessor::default();
        let mut content = big.clone();
        p.process(&content, Mode::Static);
        let t = Instant::now();
        for ch in &tail {
            content.push_str(ch);
            p.process(&content, Mode::Static);
        }
        let naive = t.elapsed();
        println!(
            "  naive per-chunk      {:>8.1} µs   (full reparse, O(doc) per chunk)",
            us(naive) / tail.len() as f64,
        );
        println!("  speedup              {:>8.1}x", ms(naive) / ms(elapsed));
    }
    println!();
}
