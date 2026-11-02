//! Steady-state round trips to the shipped WebScene V8 worker.
//! Configure the native library and worker exactly as for the QML gallery.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ferroui_qml::{ir::parse_external, Instance, TypeRegistry, Value};
    use ferroui_qml_webscene::WebSceneEngine;
    use std::fmt::Write;
    use std::rc::Rc;
    use std::time::Instant;

    for properties in [8, 1024] {
        let mut source = String::from("import QtQml\nQtObject {\n");
        for n in 0..properties {
            writeln!(source, "property int p{n}: {n}")?;
        }
        source.push_str(
            "function ping(x) { return x + 1 }\nfunction readOne(x) { return x + p7 }\n}",
        );
        let document = parse_external(&source)?;
        let mut model = Instance::with_engine(
            &document,
            &TypeRegistry::new(),
            Default::default(),
            Rc::new(WebSceneEngine::from_env()?),
        )?;
        for (method, increment) in [("ping", 1.), ("readOne", 7.)] {
            let mut samples = Vec::new();
            for _ in 0..9 {
                let start = Instant::now();
                for n in 0..128 {
                    let value = model.call(model.root(), method, vec![Value::Number(n as f64)])?;
                    assert_eq!(value, Value::Number(n as f64 + increment));
                }
                samples.push(start.elapsed().as_secs_f64() * 1e6 / 128.);
            }
            samples.remove(0);
            samples.sort_by(f64::total_cmp);
            println!(
                "{properties} properties / {method}: {:.2} us/call",
                (samples[3] + samples[4]) / 2.
            );
        }
    }
    Ok(())
}
#[cfg(target_arch = "wasm32")]
fn main() {}
