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

/// A valid one-pixel PNG with a large uncompressed tEXt chunk makes paint
/// payload-copy costs reproducible without network access or image decoding.
fn embedded_image(payload_bytes: usize) -> String {
    fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(data);
        let mut crc = !0u32;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 { crc = (crc >> 1) ^ (0xEDB88320 & (0u32.wrapping_sub(crc & 1))); }
        }
        output.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
    let mut metadata = b"FerroDesignBench\0".to_vec();
    metadata.resize(metadata.len() + payload_bytes, b'X');
    chunk(&mut png, b"tEXt", &metadata);
    let pixels = [0u8, 123, 97, 255, 255];
    let mut compressed = vec![0x78, 0x01, 0x01, 5, 0, 0xfa, 0xff];
    compressed.extend_from_slice(&pixels);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in pixels { a = (a + u32::from(byte)) % 65521; b = (b + a) % 65521; }
    compressed.extend_from_slice(&((b << 16) | a).to_be_bytes());
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut href = String::with_capacity(22 + png.len().div_ceil(3) * 4);
    href.push_str("data:image/png;base64,");
    for bytes in png.chunks(3) {
        let bits = (u32::from(bytes[0]) << 16)
            | (u32::from(*bytes.get(1).unwrap_or(&0)) << 8) | u32::from(*bytes.get(2).unwrap_or(&0));
        for index in 0..4 {
            href.push(if index > bytes.len() { '=' } else { ALPHABET[((bits >> (18 - index * 6)) & 63) as usize] as char });
        }
    }
    href
}

enum ComponentOperation { Variant(NodeId), Text(String), Visible(bool) }
impl ComponentOperation {
    fn apply_document(&self, document: &mut Document, instance: NodeId) {
        match self {
            Self::Variant(component) => document.swap_instance_component(instance, *component).unwrap(),
            Self::Text(text) => document.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text(text.clone()))).unwrap(),
            Self::Visible(value) => document.set_instance_property(instance, "show", Some(ComponentPropertyValue::Boolean(*value))).unwrap(),
        }
    }
    fn apply_editor(&self, editor: &mut Editor, instance: NodeId) {
        match self {
            Self::Variant(component) => editor.swap_instance_component(instance, *component).unwrap(),
            Self::Text(text) => editor.set_instance_property(instance, "title", Some(ComponentPropertyValue::Text(text.clone()))).unwrap(),
            Self::Visible(value) => editor.set_instance_property(instance, "show", Some(ComponentPropertyValue::Boolean(*value))).unwrap(),
        }
    }
}
#[derive(Default)]
struct ComponentTimings { variant: Vec<f64>, text: Vec<f64>, visible: Vec<f64> }
impl ComponentTimings {
    fn record(&mut self, operation: &ComponentOperation, elapsed: f64) {
        match operation {
            ComponentOperation::Variant(_) => self.variant.push(elapsed),
            ComponentOperation::Text(_) => self.text.push(elapsed),
            ComponentOperation::Visible(_) => self.visible.push(elapsed),
        }
    }
    fn into_json(self) -> Value {
        json!({ "variant": distribution(self.variant), "text": distribution(self.text), "visibility": distribution(self.visible) })
    }
}

fn component_measurement(count: usize) -> Value {
    let mut document = Document::new("Component performance fixture");
    let page = document.pages[0].id;
    let source_image = embedded_image(64 * 1024);
    let background_image = embedded_image(4 * 1024 * 1024);
    let mut components = Vec::new();
    for variant in 0..2 {
        let master = document.insert_node(page, Node::new(NodeKind::Component,
            format!("Variant {variant}"), variant as f64 * 300.0, 0.0, 240.0, 320.0)).unwrap();
        let mut label = 0;
        for index in 0..32 {
            let kind = if index == 0 { NodeKind::Text } else if index % 4 == 0 { NodeKind::Vector } else { NodeKind::Rectangle };
            let mut node = Node::new(kind, format!("Layer {index}"), (index % 4) as f64 * 56.0,
                (index / 4) as f64 * 38.0, 48.0, 32.0);
            if index == 0 { node.text = format!("Default {variant}"); }
            if index % 8 == 1 {
                node.fills = vec![Fill { paint: Paint::Image { href: source_image.clone(), scale: ImageScale::Fill, crop: None }, opacity: 1.0, visible: true }];
            }
            if kind == NodeKind::Vector {
                node.path.segments.push(PathSegment::MoveTo { point: Point::default() });
                for point in 1..=64 {
                    node.path.segments.push(PathSegment::CubicTo {
                        control1: Point::new(point as f64 / 2.0, 4.0), control2: Point::new(point as f64 / 2.0, 28.0),
                        point: Point::new(point as f64 / 2.0, 16.0),
                    });
                }
            }
            let id = document.insert_node(master, node).unwrap();
            if index == 0 { label = id; }
        }
        for (key, value) in [
            ("title", ComponentPropertyValue::Text(format!("Default {variant}"))),
            ("show", ComponentPropertyValue::Boolean(true)),
        ] {
            document.set_component_property_definition(master, key, ComponentPropertyDefinition {
                name: key.into(), default_value: value, targets: vec![label],
            }).unwrap();
        }
        components.push(master);
    }
    document.create_component_set(&components, "Benchmark variants").unwrap();
    let instance = document.create_component_instance(components[0], page, 0.0, 500.0).unwrap();
    let descendants = document.descendants(instance);
    let unrelated = count.saturating_sub(document.nodes.len()).max(1);
    for index in 0..unrelated {
        let mut node = Node::new(NodeKind::Rectangle, format!("Unrelated {index}"),
            1000.0 + (index % 100) as f64 * 48.0, (index / 100) as f64 * 48.0, 32.0, 32.0);
        if index == 0 {
            node.fills = vec![Fill { paint: Paint::Image { href: background_image.clone(), scale: ImageScale::Fill, crop: None }, opacity: 1.0, visible: true }];
        }
        document.insert_node(page, node).unwrap();
    }
    document.validate().unwrap();
    let original = document.to_json().unwrap();
    let mut reference = document.clone();
    let mut editor = Editor::new(document.clone());
    let mut planned = ComponentTimings::default();
    let mut copied = ComponentTimings::default();
    let mut authored = ComponentTimings::default();
    let iterations = 24;
    for index in 0..iterations {
        let operation = match index % 3 {
            0 => ComponentOperation::Variant(components[1 - (index / 3) % 2]),
            1 => ComponentOperation::Text(format!("Authored title {index} 東京 🦀")),
            _ => ComponentOperation::Visible((index / 3) % 2 == 1),
        };
        let measure_planned = |document: &mut Document, samples: &mut ComponentTimings| {
            let started = Instant::now();
            operation.apply_document(document, instance);
            samples.record(&operation, milliseconds(started));
        };
        let measure_copied = |document: &mut Document, samples: &mut ComponentTimings| {
            let started = Instant::now();
            // Explicit conservative reference: copy the whole document before
            // the same validated operation and replace it only after success.
            let mut candidate = document.clone();
            operation.apply_document(&mut candidate, instance);
            *document = candidate;
            samples.record(&operation, milliseconds(started));
        };
        if index % 2 == 0 {
            measure_planned(&mut document, &mut planned);
            measure_copied(&mut reference, &mut copied);
        } else {
            measure_copied(&mut reference, &mut copied);
            measure_planned(&mut document, &mut planned);
        }
        let started = Instant::now();
        operation.apply_editor(&mut editor, instance);
        authored.record(&operation, milliseconds(started));
        assert!(document == reference, "Paired component state differs at operation {index}");
        assert_eq!(document.descendants(instance), descendants, "Matching IDs must survive repeated variants and exposed-value updates");
    }
    assert_eq!(editor.history_len(), iterations);
    let final_json = document.to_json().unwrap();
    assert_eq!(editor.to_json().unwrap(), final_json);
    let mut undo = Vec::new();
    let mut redo = Vec::new();
    for _ in 0..iterations {
        let started = Instant::now();
        assert!(editor.undo());
        undo.push(milliseconds(started));
    }
    assert_eq!(editor.to_json().unwrap(), original);
    for _ in 0..iterations {
        let started = Instant::now();
        assert!(editor.redo());
        redo.push(milliseconds(started));
    }
    assert_eq!(editor.to_json().unwrap(), final_json);
    editor.document.validate().unwrap();
    json!({
        "kind": "two-matched-variants-with-text-visibility-images-and-vector-paths",
        "nodes": document.nodes.len(), "unrelatedNodes": unrelated,
        "instanceNodes": descendants.len(), "iterations": iterations, "operationsPerKind": iterations / 3,
        "embeddedPngHrefBytesPerSourceImage": source_image.len(),
        "unrelatedEmbeddedPngHrefBytes": background_image.len(), "serializedBytes": final_json.len(),
        "equivalentDocumentsAndStableIds": true, "includesBrowserEvents": false,
        "documentOperationsWithoutHistory": {
            "plannedChanges": planned.into_json(), "fullDocumentCloneReference": copied.into_json(),
            "referenceDefinition": "the same operation plus a whole-document candidate clone and replacement",
        },
        "editorWithHistory": {"operation": authored.into_json(), "undo": distribution(undo), "redo": distribution(redo),
            "exactHistoryRoundTrip": true, "snapshotScope": "whole-document"},
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
    let components = component_measurement(count);

    println!("{}", serde_json::to_string_pretty(&json!({
        "schemaVersion": 5,
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
        "componentAuthoring": components,
        "serialization": {
            "portablePretty": {"bytes": edited.len(), "timing": distribution(pretty)},
            "compactAutosave": {"bytes": compact_bytes, "timing": distribution(compact)},
            "equivalentDocumentRoundTrip": true,
        },
        "parseValidateAndIndex": distribution(reload),
    })).unwrap());
}
