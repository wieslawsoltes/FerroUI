//! Run with `cargo bench -p ferroui-qml-controls --bench visual_collections`.
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_qml::{ir::parse, Instance, Value};
use ferroui_qml_controls::{ControlRegistry, QmlView};
use std::{fmt::Write, hint::black_box, time::Instant};

fn measure(name: &str, iterations: usize, mut action: impl FnMut(usize)) {
    for i in 0..10 {
        action(i);
    }
    let mut micros = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for i in 0..iterations {
            action(i);
        }
        micros.push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
    }
    micros.sort_by(f64::total_cmp);
    println!(
        "{name}: {:.2} us/op (median of 7 x {iterations})",
        micros[3]
    );
}

fn collections() {
    let mut registry = ControlRegistry::standard().unwrap();
    let mut source = String::from(
        "import QtQuick\nListView { id: view; width: 200; height: 200; cacheBuffer: 40; reuseItems: true;
        section.property: \"category\"; section.delegate: Item { required property string section; height: 10 }
        delegate: Item { required property int index; required property int score; height: 10 }
        model: ListModel { id: rows\n",
    );
    for i in 0..8000 {
        writeln!(
            source,
            "ListElement {{ category: \"g{}\"; score: 0 }}",
            i / 60
        )
        .unwrap();
    }
    source.push_str("} }");
    let doc = parse(&source).unwrap();
    let mut model = Instance::new(&doc, registry.types_mut(), Default::default()).unwrap();
    let view = model.root();
    let rows = model.id("rows").unwrap();
    measure(
        "edit unrelated role in sectioned 8k-row ListModel",
        80,
        |i| {
            model
                .call(
                    rows,
                    "setProperty",
                    vec![
                        Value::Number(1.),
                        Value::String("score".into()),
                        Value::Number(i as f64),
                    ],
                )
                .unwrap();
            black_box(model.drain_changes());
        },
    );
    measure("scroll retained 20-row viewport by one pixel", 180, |i| {
        model
            .set(view, "contentY", Value::Number((1000 + i) as f64))
            .unwrap();
        black_box(model.drain_changes());
    });
    measure(
        "recycle visible delegates over large scroll jumps",
        40,
        |i| {
            model
                .set(
                    view,
                    "contentY",
                    Value::Number((1000 + (i % 2) * 50000) as f64),
                )
                .unwrap();
            black_box(model.drain_changes());
        },
    );
}

fn eager_repeater() {
    let mut registry = ControlRegistry::standard().unwrap();
    let mut source = String::from("import QtQuick\nItem { ListModel { id: rows; ");
    for i in 0..300 {
        writeln!(source, "ListElement {{ score: {i} }}").unwrap();
    }
    source.push_str("} Repeater { model: rows; delegate: Item { required property int index; required property int score; width: score } } function append() { rows.append({score: 42}) } function removeLast(){ rows.remove(rows.count - 1) } }");
    let doc = parse(&source).unwrap();
    let mut model = Instance::new(&doc, registry.types_mut(), Default::default()).unwrap();
    let root = model.root();
    measure(
        "append and remove tail of 300-delegate Repeater",
        24,
        |_| {
            model.call(root, "append", vec![]).unwrap();
            model.call(root, "removeLast", vec![]).unwrap();
            black_box(model.drain_changes());
        },
    );
}

fn mounted_layout() {
    let _services = UnitTestApplication::start(TestServices::new());
    let mut source = String::from("import QtQuick\nItem { id: root; width: 100; height: 100; Item { id: marker; anchors.left: parent.left }\n");
    for _ in 0..300 {
        source.push_str("Item { width: 2; height: 2 }\n");
    }
    source.push('}');
    let doc = parse(&source).unwrap();
    let view = QmlView::new(
        &doc,
        &ControlRegistry::standard().unwrap(),
        Default::default(),
    )
    .unwrap();
    let marker = view.id("marker").unwrap();
    measure("change one anchor in mounted 302-item tree", 40, |i| {
        view.set(marker, "anchors.leftMargin", Value::Number((i % 2) as f64))
            .unwrap();
        black_box(view.root());
    });
}

fn main() {
    collections();
    eager_repeater();
    mounted_layout();
}
