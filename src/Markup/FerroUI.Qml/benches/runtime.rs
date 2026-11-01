//! End-to-end owner-thread operations. Parse and register types outside the timed batches.
use ferroui_qml::{ir, IncubationMode, Instance, TypeDefinition, TypeRegistry, Value};
use std::{hint::black_box, time::Instant};

fn measure(name: &str, iterations: usize, mut operation: impl FnMut()) {
    operation();
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            operation();
        }
        samples.push(start.elapsed().as_nanos() / iterations as u128);
    }
    samples.sort();
    println!(
        "{name}: {} ns/op (median of 7 batches of {iterations})",
        samples[3]
    );
}

fn main() {
    let mut registry = TypeRegistry::new();
    for i in 0..128 {
        let mut ty = TypeDefinition::default();
        for j in 0..16 {
            ty = ty.property(&format!("field{j}"), "real", Value::Number(0.));
        }
        registry
            .register("Widgets", &format!("Widget{i}"), ty)
            .unwrap();
    }
    let root = ir::parse("import QtQml\nQtObject { property int value: 3 }").unwrap();
    measure("instantiate root in 128-type registry", 32, || {
        let app = Instance::new(&root, &registry, Default::default()).unwrap();
        assert_eq!(app.get(app.root(), "value").unwrap(), Value::Number(3.));
        black_box(app);
    });

    let children = (0..256)
        .map(|i| format!("property QtObject child{i}: QtObject {{ property int number: {i} }}\n"))
        .collect::<String>();
    let tree = ir::parse(&format!("import QtQml\nQtObject {{ {children} }}")).unwrap();
    let types = TypeRegistry::new();
    measure("instantiate 256 child objects", 20, || {
        let app = Instance::new(&tree, &types, Default::default()).unwrap();
        assert_eq!(app.children(app.root()).len(), 256);
        black_box(app);
    });
    let tree = ir::parse(&format!("import QtQml\nQtObject {{ property Component factory: Component {{ QtObject {{ {children} }} }} }}")).unwrap();
    measure("incubate and publish 256 child objects", 12, || {
        let mut app = Instance::new(&tree, &types, Default::default()).unwrap();
        let root = app.root();
        let Value::Object(factory) = app.get(root, "factory").unwrap() else {
            panic!("missing factory")
        };
        let request = app
            .incubate_object(
                factory,
                Some(root),
                Default::default(),
                IncubationMode::Asynchronous,
            )
            .unwrap()
            .unwrap();
        while app.has_deferred_work() {
            app.process_deferred(1).unwrap();
        }
        let published = request.object(&app).unwrap().unwrap();
        assert_eq!(app.children(published).len(), 256);
        black_box(app);
    });

    let wires = (0..512).map(|i| format!("property Connections wire{i}: Connections {{ target: inactive; function onPing() {{ root.hits++ }} }}\n")).collect::<String>();
    let source = format!("import QtQml\nQtObject {{ id: root; signal ping(); property int hits: 0; property int tick: 0; property QtObject other: QtObject {{ id: inactive; signal ping() }}; property Connections live: Connections {{ target: root; function onPing() {{ root.hits++ }} }}; {wires} }}");
    let doc = ir::parse(&source).unwrap();
    let mut app = Instance::new(&doc, &registry, Default::default()).unwrap();
    let id = app.root();
    let mut tick = 0;
    measure("unrelated write with 513 Connections", 1000, || {
        tick += 1;
        app.set(id, "tick", Value::Number(tick as f64)).unwrap();
    });
    let mut fired = 0;
    measure(
        "emit with one matching target among 513 Connections",
        500,
        || {
            app.emit(id, "ping", vec![]).unwrap();
            fired += 1;
        },
    );
    assert_eq!(app.get(id, "hits").unwrap(), Value::Number(fired as f64));

    let fields = (0..512)
        .map(|i| format!("property int p{i}: tick + {i};\n"))
        .collect::<String>();
    let doc = ir::parse(&format!(
        "import QtQml\nQtObject {{ property int tick: 0; {fields} }}"
    ))
    .unwrap();
    let mut app = Instance::new(&doc, &registry, Default::default()).unwrap();
    let id = app.root();
    measure("update 512 bindings sharing an input", 64, || {
        tick += 1;
        app.set(id, "tick", Value::Number(tick as f64)).unwrap();
    });
    assert_eq!(
        app.get(id, "p511").unwrap(),
        Value::Number((tick + 511) as f64)
    );

    #[cfg(feature = "diagnostics")]
    {
        let profiler = ferroui_qml::diagnostics::Debugger::default();
        app.set_debugger(Some(profiler.clone()));
        measure("update 512 bindings with the profiler attached", 32, || {
            tick += 1;
            app.set(id, "tick", Value::Number(tick as f64)).unwrap();
        });
        assert_eq!(profiler.profile().len(), 512);
    }
}
