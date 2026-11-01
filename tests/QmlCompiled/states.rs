use ferroui_qml::{Instance, Value};
use std::{path::Path, time::Duration};
mod built {
    include!(concat!(env!("OUT_DIR"), "/states.rs"));
}
fn exercise(mut m: Instance) {
    let root = m.root();
    m.call(root, "open", vec![]).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(20.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(50.));
    m.advance_time(Duration::from_millis(25)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(30.));
    m.advance_time(Duration::from_millis(75)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(100.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(102.));
    m.call(root, "close", vec![]).unwrap();
    m.advance_time(Duration::from_millis(100)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(20.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(50.));
    m.call(root, "play", vec![]).unwrap();
    for (ms, gauge, accent, latched) in [
        (50, 5., "#000000", false),
        (100, 10., "#7f7f7f", false),
        (125, 10., "#ffffff", false),
        (75, 15., "#ffffff", true),
        (50, 20., "#ffffff", true),
    ] {
        m.advance_time(Duration::from_millis(ms)).unwrap();
        assert_eq!(m.get(root, "gauge").unwrap(), Value::Number(gauge));
        assert_eq!(m.get(root, "accent").unwrap(), Value::String(accent.into()));
        assert_eq!(m.get(root, "latched").unwrap(), Value::Bool(latched));
    }
}
fn path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("states/Main.qml")
}
#[test]
fn state_changes_and_transitions_in_source_and_compiled_rust() {
    let types = super::quick_registry();
    let loaded = ferroui_qml_build::Compiler::new().load(path()).unwrap();
    exercise(Instance::new(&loaded.document, &types, Default::default()).unwrap());
    exercise(Instance::new(&built::compiled_qml(), &types, Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn state_changes_and_transitions_in_source_and_compiled_v8() {
    mod guest {
        include!(concat!(env!("OUT_DIR"), "/states_guest.rs"));
    }
    let types = super::quick_registry();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let loaded = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path())
        .unwrap();
    exercise(
        Instance::with_engine(&loaded.document, &types, Default::default(), engine.clone())
            .unwrap(),
    );
    exercise(
        Instance::with_engine(&guest::compiled_qml(), &types, Default::default(), engine).unwrap(),
    );
}

mod property_tracks {
    include!(concat!(env!("OUT_DIR"), "/property_tracks.rs"));
}

fn check_tracks(mut engine: Instance) {
    let root = engine.root();
    engine.call(root, "play", vec![]).unwrap();
    let scalar_names = [
        "straight",
        "clockwiseWrap",
        "counterWrap",
        "shortestWrap",
        "fullTurns",
        "negativeTie",
        "angle",
        "rotation",
        "steps",
        "scalar",
    ];
    let mut previous = 0;
    for line in include_str!("states/property_tracks.tsv").lines() {
        let mut cells = line.split('\t');
        let ms = cells.next().unwrap().parse::<u64>().unwrap();
        engine
            .advance_time(Duration::from_millis(ms - previous))
            .unwrap();
        previous = ms;
        let close = |value: f64, expected: &str, name: &str| {
            let expected = expected.parse::<f64>().unwrap();
            assert!(
                (value - expected).abs() < 1e-8,
                "at {ms} ms {name} = {value}, expected {expected}"
            );
        };
        for name in scalar_names {
            let Value::Number(value) = engine.get(root, name).unwrap() else {
                panic!("missing {name}")
            };
            close(value, cells.next().unwrap(), name);
        }
        assert_eq!(
            engine.get(root, "tint").unwrap(),
            Value::String(cells.next().unwrap().into())
        );
        for name in ["location", "extent", "bounds"] {
            let Value::Geometry(value) = engine.get(root, name).unwrap() else {
                panic!("missing {name}")
            };
            for component in value.components() {
                close(component, cells.next().unwrap(), name);
            }
        }
        assert!(cells.next().is_none());
    }
}

#[test]
fn property_tracks_match_qt_in_source_and_native_code() {
    let registry = super::quick_registry();
    let doc = ferroui_qml_build::Compiler::new()
        .load("states/PropertyTracks.qml")
        .unwrap()
        .document;
    check_tracks(Instance::new(&doc, &registry, Default::default()).unwrap());
    check_tracks(
        Instance::new(
            &property_tracks::compiled_qml(),
            &registry,
            Default::default(),
        )
        .unwrap(),
    );
}

#[cfg(feature = "webscene")]
#[test]
fn property_tracks_match_qt_with_v8_endpoints() {
    mod guest {
        include!(concat!(env!("OUT_DIR"), "/property_tracks_guest.rs"));
    }
    let registry = super::quick_registry();
    let vm = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load("states/PropertyTracks.qml")
        .unwrap()
        .document;
    check_tracks(Instance::with_engine(&doc, &registry, Default::default(), vm.clone()).unwrap());
    check_tracks(
        Instance::with_engine(&guest::compiled_qml(), &registry, Default::default(), vm).unwrap(),
    );
}

#[test]
fn rotation_behaviors_and_transitions_select_their_channels() {
    let doc = ferroui_qml::ir::parse(r#"import QtQuick
        Item {
            id: page
            property real heading: 350
            property real angle: 350
            property real other: 0
            Behavior on heading { RotationAnimation { duration: 100; direction: RotationAnimation.Shortest } }
            states: State { name: "changed"; PropertyChanges { page.angle: 10; page.other: 100 } }
            transitions: Transition { RotationAnimation { duration: 100; direction: RotationAnimation.Clockwise } }
        }"#).unwrap();
    let mut model = Instance::new(&doc, &super::quick_registry(), Default::default()).unwrap();
    let root = model.root();
    model.set(root, "heading", Value::Number(10.)).unwrap();
    model
        .set(root, "state", Value::String("changed".into()))
        .unwrap();
    assert_eq!(model.get(root, "other").unwrap(), Value::Number(100.));
    model.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(model.get(root, "angle").unwrap(), Value::Number(360.));
    assert_eq!(model.get(root, "heading").unwrap(), Value::Number(360.));
}
