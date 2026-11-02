//! Standalone consumer of the Rust module emitted by the compiler benchmark.
//! Build twice against the same runtime to compare downstream rustc and startup.
//! Set FERROUI_QML_BENCH_MODULE to the absolute path of the generated file.
use ferroui_qml::{Instance, TypeDefinition, TypeRegistry, Value};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

include!(env!("FERROUI_QML_BENCH_MODULE"));

fn median(mut work: impl FnMut(), count: u32) -> Duration {
    work();
    let mut samples = Vec::new();
    for _ in 0..9 {
        let start = Instant::now();
        for _ in 0..count {
            work();
        }
        samples.push(start.elapsed() / count);
    }
    samples.sort_unstable();
    samples[4]
}

fn main() {
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("width", "real", Value::Number(100.))
                .property("height", "real", Value::Number(80.))
                .property("visible", "bool", Value::Bool(true))
                .property("opacity", "real", Value::Number(1.))
                .property("children", "list<QtObject>", Value::List(Vec::new()))
                .default_property("children")
                .visual_children("children"),
        )
        .unwrap();
    registry
        .register(
            "QtQuick",
            "Rectangle",
            TypeDefinition::default().inherits("QtQuick", "Item"),
        )
        .unwrap();
    registry.reexport("QtQuick", "QtQml").unwrap();
    let component = compiled_qml();
    let instance = Instance::new(&component, &registry, Default::default()).unwrap();
    assert_eq!(instance.objects().count(), 257);
    let materialize = median(
        || {
            black_box(compiled_qml());
        },
        50,
    );
    let instantiate = median(
        || {
            black_box(Instance::new(&component, &registry, Default::default()).unwrap());
        },
        5,
    );
    println!("materialize and drop: {materialize:?}; instantiate and drop: {instantiate:?}");
}
