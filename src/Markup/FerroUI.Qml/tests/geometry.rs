use ferroui_qml::{ir, Geometry, Instance, TypeRegistry, Value};
use std::collections::BTreeMap;
#[test]
fn geometric_properties_copy_on_assignment_and_write_through_local_references() {
    let doc = ir::parse(
        r#"import QtQml
QtObject { id: root; property int n:2; property point pos: Qt.point(n, n + 2)
    property point snapshot; property rect bounds:"2,4,10x20"; property size mode
    property real rightEdge: bounds.right
    function edit() { let same=pos; same.x+=7; snapshot=pos; pos.y=30; bounds.x=11 }
    function fail() { let v=bounds; v.height=42; root.missing() }
}"#,
    )
    .unwrap();
    let mut vm = Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
    let r = vm.root();
    assert_eq!(
        vm.get(r, "mode").unwrap(),
        Value::Geometry(Geometry::Size {
            width: -1.,
            height: -1.
        })
    );
    vm.call(r, "edit", vec![]).unwrap();
    vm.set(r, "n", Value::Number(19.)).unwrap();
    assert_eq!(
        vm.get(r, "pos").unwrap(),
        Value::Geometry(Geometry::Point { x: 9., y: 30. })
    );
    assert_eq!(
        vm.get(r, "snapshot").unwrap(),
        Value::Geometry(Geometry::Point { x: 9., y: 4. })
    );
    assert_eq!(vm.get(r, "rightEdge").unwrap(), Value::Number(21.));
    assert!(vm.call(r, "fail", vec![]).is_err());
    assert_eq!(vm.get(r, "bounds.height").unwrap(), Value::Number(20.));
    assert!(vm.set(r, "bounds.right", Value::Number(0.)).is_err());
    vm.set(r, "bounds.width", Value::Number(6.)).unwrap();
    assert_eq!(vm.get(r, "rightEdge").unwrap(), Value::Number(17.));
}

#[test]
fn independent_field_bindings_detach_individually_and_a_whole_write_replaces_all() {
    let doc = ir::parse(
        r#"import QtQml
QtObject {id:root;property real n:2; property point cursor
    cursor { x: n; y: n + 1 }
    property real distance: cursor.x + cursor.y
    function replaceX() { cursor.x=17 }
}"#,
    )
    .unwrap();
    let mut m = Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
    let r = m.root();
    assert_eq!(m.get(r, "distance").unwrap(), Value::Number(5.));
    m.call(r, "replaceX", vec![]).unwrap();
    m.set(r, "n", Value::Number(9.)).unwrap();
    assert_eq!(m.get(r, "distance").unwrap(), Value::Number(27.));
    m.set(
        r,
        "cursor",
        Value::Geometry(Geometry::Point { x: 1., y: 3. }),
    )
    .unwrap();
    m.set(r, "n", Value::Number(19.)).unwrap();
    assert_eq!(m.get(r, "distance").unwrap(), Value::Number(4.));
}
