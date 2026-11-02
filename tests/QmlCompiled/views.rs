use super::quick_registry;
use ferroui_qml::{Instance, Value};
include!(concat!(env!("OUT_DIR"), "/views.rs"));

fn exercise(mut ui: Instance) {
    let root = ui.root();
    ui.call(root, "scrollMiddle", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "report", vec![]).unwrap(),
        Value::String("1000000/20000000/10000000/500000/500000/true".into())
    );
    ui.call(root, "changeCurrent", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "selection", vec![]).unwrap(),
        Value::String("0/updated/updated".into())
    );
    assert!(ui.objects().count() < 24);
}
#[test]
fn virtualized_view_from_generated_rust() {
    exercise(Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn virtualized_view_with_webscene() {
    use std::rc::Rc;
    let path = crate::fixtures::runtime("views/Main.qml");
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path)
        .unwrap();
    exercise(
        Instance::with_engine(
            &source.document,
            &quick_registry(),
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}

mod decorated {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/decorated.rs"));

    fn exercise(mut ui: Instance) {
        let root = ui.root();
        let report = |ui: &mut Instance| ui.call(root, "report", vec![]).unwrap();
        assert_eq!(
            report(&mut ui),
            Value::String("A/161/0/-154/-31/-88/0/-1/AAB/-54".into())
        );
        ui.call(root, "regroup", vec![]).unwrap();
        assert_eq!(
            report(&mut ui),
            Value::String("A/161/0/-154/-31/-88/0/-1/ABB/-65".into())
        );
    }

    #[test]
    fn decorated_sections_from_generated_rust() {
        exercise(Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap());
    }
    #[cfg(feature = "webscene")]
    #[test]
    fn decorated_sections_with_webscene() {
        use std::rc::Rc;
        let path = crate::fixtures::runtime("views/Decorated.qml");
        let unit = ferroui_qml_build::Compiler::new()
            .with_javascript()
            .load(path)
            .unwrap();
        exercise(
            Instance::with_engine(
                &unit.document,
                &quick_registry(),
                Default::default(),
                Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
            )
            .unwrap(),
        );
    }
}

mod reuse {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/reuse.rs"));
    #[test]
    fn pooled_and_reused_handlers_see_row_before_and_after_rebinding() {
        let mut ui = Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap();
        let root = ui.root();
        for i in [30., 80., 140., 10.] {
            ui.call(root, "jump", vec![Value::Number(i)]).unwrap();
        }
        assert_eq!(ui.call(root, "valid", vec![]).unwrap(), Value::Bool(true));
    }
}

mod array_sections {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/array_sections.rs"));

    fn exercise(mut ui: Instance) {
        let id = ui.root();
        let report = |ui: &mut Instance| ui.call(id, "report", vec![]).unwrap();
        assert_eq!(report(&mut ui), Value::String("title//A/AB/38".into()));
        ui.call(id, "regroup", vec![]).unwrap();
        assert_eq!(report(&mut ui), Value::String("title//A/A/34".into()));
    }
    #[test]
    fn grouped_property_on_record_array() {
        exercise(Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap());
    }
    #[cfg(feature = "webscene")]
    #[test]
    fn grouped_property_on_guest_array() {
        use std::{path::Path, rc::Rc};
        let unit = ferroui_qml_build::Compiler::new()
            .with_javascript()
            .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("views/ArraySections.qml"))
            .unwrap();
        exercise(
            Instance::with_engine(
                &unit.document,
                &quick_registry(),
                Default::default(),
                Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
            )
            .unwrap(),
        );
    }
}
