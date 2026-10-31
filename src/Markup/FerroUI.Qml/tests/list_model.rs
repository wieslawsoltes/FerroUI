use ferroui_qml::{ir, Instance, TypeRegistry, Value};

fn model(declarations: &str) -> Result<Instance, ferroui_qml::Error> {
    Instance::new(
        &ir::parse(&format!(
            "import QtQml.Models\nListModel {{ {declarations} }}"
        ))
        .unwrap(),
        &TypeRegistry::new(),
        Default::default(),
    )
}

#[test]
fn dynamic_rows_can_change_type_and_failed_edits_preserve_rows() {
    let mut instance = model(
        r#"dynamicRoles: true
        function fill() { append({value: 5}); append({value: "five"}) }
        function clearThenFail(){ clear(); append({value: true}); missing() }"#,
    )
    .unwrap();
    let root = instance.root();
    instance.call(root, "fill", vec![]).unwrap();
    let Value::ModelRow(first) = instance.call(root, "get", vec![Value::Number(0.)]).unwrap()
    else {
        panic!("missing row")
    };
    instance
        .call(
            root,
            "move",
            vec![Value::Number(0.), Value::Number(1.), Value::Number(1.)],
        )
        .unwrap();
    assert_eq!(
        instance.model_role(first, "value").unwrap(),
        Value::Number(5.)
    );
    assert!(instance.call(root, "clearThenFail", vec![]).is_err());
    assert!(instance
        .set(root, "dynamicRoles", Value::Bool(false))
        .is_err());
    assert_eq!(instance.get(root, "count").unwrap(), Value::Number(2.));
    assert_eq!(
        instance.model_role(first, "value").unwrap(),
        Value::Number(5.)
    );
}

#[test]
fn static_rows_reject_dynamic_mode_role_type_changes_and_nonconstant_bindings() {
    assert!(model("dynamicRoles: true; ListElement { cost: 1 }").is_err());
    assert!(model("ListElement { cost: 1 + 1 }").is_err());
    let mut instance = model("ListElement { cost: 5 }").unwrap();
    let root = instance.root();
    assert!(instance
        .call(
            root,
            "setProperty",
            vec![
                Value::Number(0.),
                Value::String("cost".into()),
                Value::Bool(true)
            ]
        )
        .is_err());
    assert!(!instance
        .property_names(root)
        .iter()
        .any(|n| n.starts_with('$')));
    let Value::ModelRow(row) = instance.call(root, "get", vec![Value::Number(0.)]).unwrap() else {
        panic!("missing row")
    };
    assert_eq!(instance.model_role(row, "cost").unwrap(), Value::Number(5.));
}
