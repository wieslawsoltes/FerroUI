use super::quick_registry;
use ferroui_qml::{ComponentSource, Instance, Value};
use ferroui_qml_build::Compiler;
use std::path::{Path, PathBuf};

mod native {
    include!(concat!(env!("OUT_DIR"), "/delegates.rs"));
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("delegates/Main.qml")
}

fn exercise(mut instance: Instance) {
    let root = instance.root();
    for (action, summary) in [
        (None, "3/2/2/0=one/1=two"),
        (Some("enableEdit"), "30/4/2/0=one/1=two"),
        (Some("changeSeed"), "50/6/2/0=one/1=two"),
        (Some("disableEdit"), "5/2/2/0=one/1=two"),
        (Some("remember"), "5/2/2/0=one/1=two"),
        (Some("moveRow"), "5/2/2/0=two/1=one"),
        (Some("renameRow"), "5/2/2/0=two/1=updated"),
        (Some("eraseRow"), "5/2/1/0=updated/-"),
    ] {
        if let Some(action) = action {
            instance.call(root, action, vec![]).unwrap();
        }
        assert_eq!(
            instance.call(root, "summary", vec![]).unwrap(),
            Value::String(summary.into()),
            "after {action:?}"
        );
        let saved_index = match action {
            Some("moveRow" | "renameRow") => 1.,
            Some("remember" | "eraseRow") => 0.,
            _ => continue,
        };
        assert_eq!(
            instance
                .call(root, "savedAt", vec![Value::Number(saved_index)])
                .unwrap(),
            Value::Bool(true),
            "delegate identity after {action:?}"
        );
    }
}

#[test]
fn source_and_generated_rust_preserve_conditional_bindings_and_delegate_identity() {
    fn run(source: &impl ComponentSource) {
        exercise(Instance::new(source, &quick_registry(), Default::default()).unwrap())
    }
    run(&Compiler::new().load(fixture()).unwrap().document);
    run(&native::compiled_qml());
}

#[cfg(feature = "webscene")]
#[test]
fn webscene_preserves_conditional_bindings_and_delegate_identity() {
    use ferroui_qml_webscene::WebSceneEngine;
    use std::rc::Rc;
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/delegates_js.rs"));
    }
    fn run(source: &impl ComponentSource) {
        exercise(
            Instance::with_engine(
                source,
                &quick_registry(),
                Default::default(),
                Rc::new(WebSceneEngine::from_env().unwrap()),
            )
            .unwrap(),
        )
    }
    run(&Compiler::new()
        .with_javascript()
        .load(fixture())
        .unwrap()
        .document);
    run(&generated::compiled_qml());
}

#[cfg(feature = "webscene")]
#[test]
fn javascript_array_replacement_preserves_element_identity_and_delegate_scope() {
    use ferroui_qml::ir::parse_external;
    use ferroui_qml_webscene::WebSceneEngine;
    use std::rc::Rc;
    let source = r#"import QtQuick
        Item { id: page
            property var rows: [{name:"one"},{name:"two"}]
            Repeater { id: rep; model: page.rows
                Item { property var entry: modelData; property string label: index + ":" + modelData.name }
            }
            function read() { return rep.count + "/" + rep.itemAt(0).label + "/" + (rep.itemAt(0).entry === rows[0]) }
            function replace() { rows = [rows[1]] }
        }"#;
    let doc = parse_external(source).unwrap();
    let mut engine = Instance::with_engine(
        &doc,
        &quick_registry(),
        Default::default(),
        Rc::new(WebSceneEngine::from_env().unwrap()),
    )
    .unwrap();
    let root = engine.root();
    assert_eq!(
        engine.call(root, "read", vec![]).unwrap(),
        Value::String("2/0:one/true".into())
    );
    engine.call(root, "replace", vec![]).unwrap();
    assert_eq!(
        engine.call(root, "read", vec![]).unwrap(),
        Value::String("1/0:two/true".into())
    );
}
