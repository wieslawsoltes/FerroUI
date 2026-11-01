use ferroui_qml::{Geometry, Instance, TypeRegistry, Value};
use std::path::{Path, PathBuf};
fn path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("geometry/Main.qml")
}
fn exercise(mut vm: Instance) {
    let r = vm.root();
    assert_eq!(
        vm.get(r, "extent").unwrap(),
        Value::Geometry(Geometry::Size {
            width: -1.,
            height: -1.
        })
    );
    assert_eq!(vm.call(r, "edit", vec![]).unwrap(), Value::Number(36.25));
    assert_eq!(
        vm.get(r, "copy").unwrap(),
        Value::Geometry(Geometry::Point { x: 3.25, y: 2. })
    );
    assert_eq!(
        vm.get(r, "grouped").unwrap(),
        Value::Geometry(Geometry::Size {
            width: 3.25,
            height: 12.
        })
    );
    assert_eq!(
        vm.call(
            r,
            "offset",
            vec![Value::Geometry(Geometry::Point { x: 4., y: 7. })]
        )
        .unwrap(),
        Value::Geometry(Geometry::Point { x: 14., y: 7. })
    );
    assert_eq!(
        vm.call(r, "show", vec![]).unwrap(),
        Value::String("QRectF(2, 4, 8, 8)".into())
    );
    assert!(vm.call(r, "fail", vec![]).is_err());
    assert_eq!(vm.get(r, "frame.height").unwrap(), Value::Number(8.));
    vm.set(
        r,
        "frame",
        Value::Geometry(Geometry::Rect {
            x: 3.,
            y: 5.,
            width: 17.,
            height: 4.,
        }),
    )
    .unwrap();
    assert_eq!(vm.get(r, "edge").unwrap(), Value::Number(29.));
}
#[test]
fn native_geometry() {
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/geometry.rs"));
    }
    let types = TypeRegistry::new();
    let source = ferroui_qml_build::Compiler::new().load(path()).unwrap();
    exercise(Instance::new(&source.document, &types, Default::default()).unwrap());
    exercise(Instance::new(&generated::compiled_qml(), &types, Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn guest_geometry() {
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/geometry_guest.rs"));
    }
    let types = TypeRegistry::new();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path())
        .unwrap();
    exercise(
        Instance::with_engine(&source.document, &types, Default::default(), engine.clone())
            .unwrap(),
    );
    exercise(
        Instance::with_engine(
            &generated::compiled_qml(),
            &types,
            Default::default(),
            engine,
        )
        .unwrap(),
    );
}
