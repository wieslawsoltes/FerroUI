use ferroui_qml::{ir, Instance, TypeRegistry, Value};
use std::collections::BTreeMap;

fn load(source: &str) -> Instance {
    let doc = ir::parse(&format!("import QtQml\nQtObject {{ {source} }}")).unwrap();
    Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap()
}

#[test]
fn mutations_preserve_identity_and_errors_restore_deleted_and_new_properties() {
    let mut vm = load(
        r#"
        property var map: ({child:{n:5}, "2":7, child2: null})
        property var aliasMap: map
        property var items: [map, 8, 9]
        property list<int> sequence: [3, 4]
        function invalid() {
            let rec=aliasMap
            rec.child.n += 7
            rec["newField"]=items
            items.length=1
            items.push("other")
            sequence="invalid"
        }
        function state(): string {
            return map.child.n + "/" + map[2] + "/" + (map === items[0]) + "/" + items.length + "/" + items[1] + "/" + map.newField
        }
        function literal(value) { return ({value, value2: value+1, value2: 9}) }
    "#,
    );
    let root = vm.root();
    assert!(vm.call(root, "invalid", vec![]).is_err());
    assert_eq!(
        vm.call(root, "state", vec![]).unwrap(),
        Value::String("5/7/true/3/8/undefined".into())
    );
    assert_eq!(
        vm.get(root, "sequence").unwrap(),
        Value::List(vec![Value::Number(3.), Value::Number(4.)])
    );
    let Value::Record(r) = vm.call(root, "literal", vec![Value::Number(6.)]).unwrap() else {
        panic!("record")
    };
    assert_eq!(vm.record_entries(&r).unwrap()["value"], Value::Number(6.));
    assert_eq!(vm.record_entries(&r).unwrap()["value2"], Value::Number(9.));
    let mut other = load("property var rec");
    assert!(other
        .set(other.root(), "rec", Value::Record(r.clone()))
        .is_err());
    assert!(other
        .new_record(BTreeMap::from([("foreign".into(), Value::Record(r))]))
        .is_err());
}

#[test]
fn sparse_arrays_preserve_holes_and_length_rolls_back_on_invalid_updates() {
    let mut vm = load(
        r#"
        property var a:[]
        function sparse() {
            a[4294967293]="z"
            let copy = a.slice(-2)
            let summary = a.length + "/" + copy.join("-") + "/" + Array.isArray(copy)
            a.length=2
            a.push(a)
            return summary + "/" + a.join("") + "/" + (a.at(-1)===a)
        }
        function fail() { a.length=-1 }
        function empty() { return []===[] }
    "#,
    );
    let root = vm.root();
    assert_eq!(
        vm.call(root, "sparse", vec![]).unwrap(),
        Value::String("4294967294/-z/true//true".into())
    );
    assert!(vm.call(root, "fail", vec![]).is_err());
    assert_eq!(vm.call(root, "empty", vec![]).unwrap(), Value::Bool(false));
}
