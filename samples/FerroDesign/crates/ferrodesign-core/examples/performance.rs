//! Reproducible CPU benchmark for the document engine, independent of a renderer.
//! Run: cargo run --release -p ferrodesign-core --example performance -- 10000
use ferrodesign_core::*;
use std::{hint::black_box, time::Instant};

fn milliseconds(elapsed: std::time::Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

fn percentile(samples: &mut [f64], fraction: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() - 1) as f64 * fraction).round() as usize]
}

fn main() {
    let count = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000)
        .clamp(1, MAX_DOCUMENT_NODES);
    let columns = (count as f64).sqrt().ceil() as usize;
    let mut document = Document::new("Performance fixture");
    let page = document.pages[0].id;
    let start = Instant::now();
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let x = (index % columns) as f64 * 64.0;
        let y = (index / columns) as f64 * 64.0;
        ids.push(
            document
                .insert_node(
                    page,
                    Node::new(
                        NodeKind::Rectangle,
                        format!("Shape {index}"),
                        x,
                        y,
                        48.0,
                        48.0,
                    )
                    .with_fill(Color::rgb(147, 112, 242)),
                )
                .unwrap(),
        );
    }
    let create_ms = milliseconds(start.elapsed());
    let start = Instant::now();
    let mut editor = Editor::new(document);
    let index_ms = milliseconds(start.elapsed());

    let start = Instant::now();
    let mut hits = 0;
    for index in 0..10_000 {
        let item = (index * 7919) % count;
        let p = Point::new(
            (item % columns) as f64 * 64.0 + 24.0,
            (item / columns) as f64 * 64.0 + 24.0,
        );
        hits += usize::from(black_box(editor.hit_test(black_box(p), 0.5)).is_some());
    }
    let hits_ms = milliseconds(start.elapsed());
    let start = Instant::now();
    let mut visible = 0;
    for index in 0..1000 {
        let offset = (index * 37 % 1000) as f64;
        visible += black_box(editor.visible_nodes(Rect::new(offset, offset, 1200.0, 800.0))).len();
    }
    let cull_ms = milliseconds(start.elapsed());

    editor.selection = vec![ids[count / 2]];
    let start = Instant::now();
    editor.begin_transaction("Benchmark drag");
    let begin_ms = milliseconds(start.elapsed());
    let mut samples = Vec::with_capacity(120);
    for _ in 0..120 {
        let start = Instant::now();
        editor.document.nodes.get_mut(&ids[count / 2]).unwrap().x += 0.25;
        editor.refresh();
        samples.push(milliseconds(start.elapsed()));
    }
    let start = Instant::now();
    assert!(editor.commit_transaction());
    let commit_ms = milliseconds(start.elapsed());
    let start = Instant::now();
    assert!(editor.undo());
    let undo_ms = milliseconds(start.elapsed());
    let start = Instant::now();
    assert!(editor.redo());
    let redo_ms = milliseconds(start.elapsed());
    editor.document.validate().unwrap();

    println!("nodes={count} create_ms={create_ms:.3} index_ms={index_ms:.3}");
    println!(
        "point_queries=10000 hits={hits} total_ms={hits_ms:.3} mean_us={:.3}",
        hits_ms / 10.0
    );
    println!(
        "viewport_queries=1000 total_visible={visible} total_ms={cull_ms:.3} mean_us={cull_ms:.3}"
    );
    println!(
        "drag_frames=120 p50_ms={:.3} p95_ms={:.3}",
        percentile(&mut samples, 0.5),
        percentile(&mut samples, 0.95)
    );
    println!(
        "begin_ms={begin_ms:.3} commit_ms={commit_ms:.3} undo_ms={undo_ms:.3} redo_ms={redo_ms:.3}"
    );
}
