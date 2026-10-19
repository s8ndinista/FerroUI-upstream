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

#[derive(Default)]
struct GestureTimings {
    begin: Vec<f64>,
    updates: Vec<f64>,
    commit: Vec<f64>,
    total: Vec<f64>,
}
impl GestureTimings {
    fn measure(&mut self, editor: &mut Editor, id: NodeId, scoped: bool) {
        let total = Instant::now();
        let started = Instant::now();
        if scoped {
            editor.begin_node_transaction(&[id], "Drag layer");
        } else {
            editor.begin_transaction("Drag layer");
        }
        self.begin.push(milliseconds(started));
        for _ in 0..24 {
            let started = Instant::now();
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.x += 0.25;
            node.y += 0.125;
            if scoped {
                editor.refresh_preview_nodes(&[id]);
            } else {
                editor.refresh();
            }
            self.updates.push(milliseconds(started));
        }
        let started = Instant::now();
        assert!(editor.commit_transaction());
        self.commit.push(milliseconds(started));
        self.total.push(milliseconds(total));
    }
    fn into_json(self) -> Value {
        json!({
            "begin": distribution(self.begin),
            "update": distribution(self.updates),
            "commit": distribution(self.commit),
            "total": distribution(self.total),
        })
    }
}

fn dense_layout_measurement(count: usize) -> Value {
    let mut document = Document::new("Dense layout fixture");
    let page = document.pages[0].id;
    let children = count.saturating_sub(1);
    let mut row = Node::new(NodeKind::Frame, "Row", 0.0, 0.0,
        children as f64 * 24.0 + 16.0, 80.0);
    row.layout = Some(AutoLayout {
        gap: 2.0,
        padding: Padding::all(8.0),
        height: SizingMode::Fixed,
        ..AutoLayout::default()
    });
    let row = document.insert_node(page, row).unwrap();
    let payload = "Auto layout retains text, paint and style data. Ω 🚀 ".repeat(8);
    for index in 0..children {
        let mut node = Node::new(NodeKind::Text, format!("Text {index}"),
            0.0, 0.0, 12.0, 24.0)
            .with_text(payload.clone(), 16.0, Color::BLACK);
        if index % 4 != 0 {
            node.layout_child.width = SizingMode::Fill;
        }
        document.insert_node(row, node).unwrap();
    }
    let mut timings = Vec::with_capacity(5);
    let mut changed = Vec::with_capacity(5);
    for _ in 0..5 {
        document.nodes.get_mut(&row).unwrap().width += 16.0;
        let started = Instant::now();
        let result = black_box(document.layout_all());
        timings.push(milliseconds(started));
        assert!(result.warnings.is_empty());
        changed.push(result.changed.len());
    }
    document.validate().unwrap();
    json!({
        "kind": "fixed-row-three-quarter-fill-text-children",
        "nodes": document.nodes.len(), "children": children,
        "textBytesPerChild": payload.len(), "parentWidthDelta": 16.0,
        "changedPerIteration": changed, "timing": distribution(timings),
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

    let mut gesture_scoped = GestureTimings::default();
    let mut gesture_unscoped = GestureTimings::default();
    for index in 0..12 {
        if index % 2 == 0 {
            gesture_scoped.measure(&mut editor, selected, true);
            gesture_unscoped.measure(&mut reference, selected, false);
        } else {
            gesture_unscoped.measure(&mut reference, selected, false);
            gesture_scoped.measure(&mut editor, selected, true);
        }
    }
    let after_gestures = editor.to_json().unwrap();
    assert_eq!(reference.to_json().unwrap(), after_gestures);
    assert_eq!(editor.history_len(), iterations + 12);
    for _ in 0..12 {
        assert!(editor.undo());
        assert!(reference.undo());
    }
    assert_eq!(editor.to_json().unwrap(), edited);
    assert_eq!(reference.to_json().unwrap(), edited);
    for _ in 0..12 {
        assert!(editor.redo());
        assert!(reference.redo());
    }
    assert_eq!(editor.to_json().unwrap(), after_gestures);
    assert_eq!(reference.to_json().unwrap(), after_gestures);
    let edited = after_gestures;

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
    let mut no_layout = editor.document.clone();
    let mut no_layout_timings = Vec::with_capacity(5);
    for _ in 0..5 {
        let started = Instant::now();
        let result = black_box(no_layout.layout_all());
        no_layout_timings.push(milliseconds(started));
        assert_eq!(result, LayoutResult::default());
    }
    assert_eq!(no_layout.to_json().unwrap(), edited);
    let dense_layout = dense_layout_measurement(count);

    println!("{}", serde_json::to_string_pretty(&json!({
        "schemaVersion": 3,
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
        "gesture": {"gestures": 12, "updatesPerGesture": 24,
            "deltaPerUpdate": [0.25, 0.125], "includesBrowserEvents": false,
            "unscopedReference": gesture_unscoped.into_json(),
            "scoped": gesture_scoped.into_json()},
        "layout": {"noLayout": distribution(no_layout_timings), "denseMixedFill": dense_layout},
        "serialization": {
            "pretty": {"bytes": edited.len(), "timing": distribution(pretty)},
            "compactCandidate": {"bytes": compact_bytes, "timing": distribution(compact)},
            "changesProductionFormat": false,
        },
        "parseValidateAndIndex": distribution(reload),
    })).unwrap());
}
