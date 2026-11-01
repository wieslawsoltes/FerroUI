//! The same sequence of observable calls is made by the optional Qt harness.
use ferroui_qml::{ir, ComponentSource, Instance, TypeRegistry, Value};
use std::rc::Rc;
include!(concat!(env!("OUT_DIR"), "/reference.rs"));

/// The fixture only imports native QtQml and defines its reusable type inline.
struct InlineOnly;
impl ir::ComponentResolver for InlineOnly {
    fn resolve(
        &mut self,
        _from: &str,
        _imports: &[ir::Import],
        _ty: &str,
    ) -> Result<Option<ir::ComponentSource>, String> {
        Ok(None)
    }
}

pub fn run(mut observe: impl FnMut(&str, &str, &str, &str)) {
    let source = ir::parse(include_str!("Probe.qml")).unwrap();
    let source = ir::link(
        ir::ComponentSource {
            key: "Probe.qml".into(),
            document: Rc::new(source),
        },
        &mut InlineOnly,
    )
    .unwrap();
    backend("source", &source, &mut observe);
    backend("compiled", &compiled_qml(), &mut observe);
}

fn backend(
    mode: &str,
    source: &impl ComponentSource,
    observe: &mut impl FnMut(&str, &str, &str, &str),
) {
    let mut engine = Instance::new(source, &TypeRegistry::new(), Default::default()).unwrap();
    for row in include_str!("qt-6.12.0.tsv").lines() {
        engine.process_deferred(usize::MAX).unwrap();
        let (case, expected) = row.split_once('\t').unwrap();
        let result = engine.call(engine.root(), case, Vec::new()).unwrap();
        let Value::String(actual) = result else {
            panic!("{mode}/{case} did not return a string");
        };
        observe(mode, case, &actual, expected);
    }
}
