//! Repeated engine roots importing the same packaged JavaScript resources.
//! Run with the shipped native worker and library configured as for QmlGallery.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ferroui_qml::{
        ir::{parse, ScriptKind, ScriptResource},
        Engine, TypeRegistry, Value,
    };
    use ferroui_qml_webscene::WebSceneEngine;
    use std::{rc::Rc, time::Instant};
    let mut doc = parse("import QtQml\nQtObject { property int value: 7 }")?;
    for index in 0..32 {
        let url = format!("https://qml.invalid/modules/catalog/helper{index}.js");
        let source = format!("//{}\nvar answer = 42;", "x".repeat(8192));
        doc.scripts.insert(
            url,
            ScriptResource {
                executable: source.clone(),
                import_hook: None,
                preload: false,
                imports: Default::default(),
                source,
                kind: ScriptKind::Script,
                dependencies: Default::default(),
                exports: vec!["answer".into()],
                writable_exports: vec!["answer".into()],
            },
        );
    }
    let mut engine =
        Engine::with_script_engine(&TypeRegistry::new(), Rc::new(WebSceneEngine::from_env()?))?;
    engine.load_root(&doc, Default::default())?;
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..16 {
            let root = engine.load_root(&doc, Default::default())?;
            assert_eq!(engine.instance().get(root, "value")?, Value::Number(7.));
        }
        samples.push(start.elapsed().as_nanos() / 16);
    }
    samples.sort();
    println!(
        "load another root sharing 32 eight-KiB JS files: {} ns/op (median of 7 batches of 16)",
        samples[3]
    );
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
