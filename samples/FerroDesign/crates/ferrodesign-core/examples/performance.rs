//! Reproducible CPU work, history, and serialization measurements.
//! Run: cargo run --release -p ferrodesign-core --example performance -- 10000
//! Unscoped and scoped workloads use identical documents and edits in one process.
//! Native engine timings exclude browser input/render/bridge work.
use ferrodesign_core::*;
use serde_json::{json, Value};
use std::{hint::black_box, time::Instant};

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| samples[((samples.len() - 1) as f64 * fraction).round() as usize];
    json!({
        "samples": samples.len(),
        "p50Ms": percentile(0.5),
        "p95Ms": percentile(0.95),
        "maxMs": samples.last(),
        "totalMs": samples.iter().sum::<f64>(),
    })
}

fn main() {
    let count = std::env::args().nth(1)
        .and_then(|s| s.parse::<usize>().ok()).unwrap_or(10_000)
        .clamp(1, MAX_DOCUMENT_NODES);
    let columns = (count as f64).sqrt().ceil() as usize;
    let mut document = Document::new("Performance fixture");
    let page = document.pages[0].id;
    let started = Instant::now();
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        ids.push(document.insert_node(page, Node::new(
            NodeKind::Rectangle, format!("Shape {index}"),
            (index % columns) as f64 * 64.0,
            (index / columns) as f64 * 64.0,
            48.0, 48.0,
        ).with_fill(Color::rgb(147, 112, 242))).unwrap());
    }
    let create_ms = milliseconds(started);
    document.validate().unwrap();
    let started = Instant::now();
    let mut editor = Editor::new(document.clone());
    let index_ms = milliseconds(started);

    let started = Instant::now();
    let mut hits = 0;
    for index in 0..10_000 {
        let item = (index * 7919) % count;
        let p = Point::new(
            (item % columns) as f64 * 64.0 + 24.0,
            (item / columns) as f64 * 64.0 + 24.0,
        );
        hits += usize::from(black_box(editor.hit_test(black_box(p), 0.5)).is_some());
    }
    let hit_ms = milliseconds(started);
    assert_eq!(hits, 10_000);
    let started = Instant::now();
    let mut visible = 0;
    for index in 0..1000 {
        let offset = (index * 37 % 1000) as f64;
        visible += black_box(editor.visible_nodes(Rect::new(offset, offset, 1200.0, 800.0))).len();
    }
    let cull_ms = milliseconds(started);
    editor.select_all();
    let started = Instant::now();
    let selection_roots = black_box(editor.root_selection());
    let root_selection_ms = milliseconds(started);
    assert_eq!(selection_roots.len(), count);

    let selected = ids[count / 2];
    let mut reference = Editor::new(document);
    editor.selection = vec![selected];
    reference.selection = vec![selected];
    let original = editor.to_json().unwrap();
    let iterations = 48;
    let mut scoped = Vec::with_capacity(iterations);
    let mut unscoped = Vec::with_capacity(iterations);
    for index in 0..iterations {
        // Alternate order so one implementation does not always receive the
        // warmer CPU caches. Setup and JSON export are outside both timings.
        let measure_scoped = |editor: &mut Editor, samples: &mut Vec<f64>| {
            let started = Instant::now();
            editor.move_selection(1.0, -0.25);
            samples.push(milliseconds(started));
        };
        let measure_unscoped = |editor: &mut Editor, samples: &mut Vec<f64>| {
            let started = Instant::now();
            editor.transact("Move layers", |document| {
                let node = document.nodes.get_mut(&selected).unwrap();
                node.x += 1.0;
                node.y -= 0.25;
            });
            samples.push(milliseconds(started));
        };
        if index % 2 == 0 {
            measure_scoped(&mut editor, &mut scoped);
            measure_unscoped(&mut reference, &mut unscoped);
        } else {
            measure_unscoped(&mut reference, &mut unscoped);
            measure_scoped(&mut editor, &mut scoped);
        }
    }
    assert_eq!(editor.history_len(), iterations);
    assert_eq!(editor.to_json().unwrap(), reference.to_json().unwrap());
    let edited = editor.to_json().unwrap();
    let mut undo_scoped = Vec::with_capacity(iterations);
    let mut undo_unscoped = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        assert!(editor.undo());
        undo_scoped.push(milliseconds(started));
        let started = Instant::now();
        assert!(reference.undo());
        undo_unscoped.push(milliseconds(started));
    }
    assert_eq!(editor.to_json().unwrap(), original);
    assert_eq!(reference.to_json().unwrap(), original);
    let mut redo_scoped = Vec::with_capacity(iterations);
    let mut redo_unscoped = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        assert!(editor.redo());
        redo_scoped.push(milliseconds(started));
        let started = Instant::now();
        assert!(reference.redo());
        redo_unscoped.push(milliseconds(started));
    }
    assert_eq!(editor.to_json().unwrap(), edited);
    assert_eq!(reference.to_json().unwrap(), edited);
    editor.document.validate().unwrap();

    let mut pretty = Vec::with_capacity(5);
    let mut compact = Vec::with_capacity(5);
    let mut compact_bytes = 0;
    for _ in 0..5 {
        let started = Instant::now();
        let text = black_box(editor.to_json().unwrap());
        pretty.push(milliseconds(started));
        assert_eq!(text.len(), edited.len());
        let started = Instant::now();
        let text = black_box(serde_json::to_string(&editor.document).unwrap());
        compact.push(milliseconds(started));
        compact_bytes = text.len();
        assert_eq!(Document::from_json(&text).unwrap().to_json().unwrap(), edited);
    }
    let mut reload = Vec::with_capacity(3);
    for _ in 0..3 {
        let started = Instant::now();
        let loaded = Editor::new(Document::from_json(black_box(&edited)).unwrap());
        reload.push(milliseconds(started));
        assert_eq!(loaded.to_json().unwrap(), edited);
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "schemaVersion": 2,
        "measurement": "native-engine-same-process",
        "releaseBuild": !cfg!(debug_assertions),
        "nodes": count,
        "workload": {"kind": "flat-filled-rectangle-grid", "columns": columns,
            "selectedNodes": 1, "nudgeIterations": iterations, "delta": [1.0, -0.25]},
        "equivalentDocumentsAndHistory": true,
        "createMs": create_ms,
        "indexMs": index_ms,
        "pointQueries": {"queries": 10000, "hits": hits, "totalMs": hit_ms},
        "viewportQueries": {"queries": 1000, "totalVisible": visible, "totalMs": cull_ms},
        "allSelectionRoots": {"selected": count, "roots": selection_roots.len(), "totalMs": root_selection_ms},
        "nudge": {"unscopedReference": distribution(unscoped), "scoped": distribution(scoped)},
        "undo": {"unscopedReference": distribution(undo_unscoped), "scoped": distribution(undo_scoped)},
        "redo": {"unscopedReference": distribution(redo_unscoped), "scoped": distribution(redo_scoped)},
        "serialization": {
            "pretty": {"bytes": edited.len(), "timing": distribution(pretty)},
            "compactCandidate": {"bytes": compact_bytes, "timing": distribution(compact)},
            "changesProductionFormat": false,
        },
        "parseValidateAndIndex": distribution(reload),
    })).unwrap());
}
