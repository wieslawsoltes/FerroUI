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

#[test]
fn append_and_insert_batches_preserve_order_and_roll_back_failed_role_conversions() {
    let mut engine=model(r#"
        function init(){ append([{x:1},{x:2}]); insert(0,[{x:3},{x:4}]); return get(0).x + '/' + get(1).x + '/' + count }
        function bad(){ insert(1,[{x:7},{x:"not a number"}]) }
    "#).unwrap();
    let root = engine.root();
    assert_eq!(
        engine.call(root, "init", vec![]).unwrap(),
        Value::String("3/4/4".into())
    );
    assert!(engine.call(root, "bad", vec![]).is_err());
    assert_eq!(engine.get(root, "count").unwrap(), Value::Number(4.));
}

#[test]
fn copied_rows_own_independent_nested_models_and_invalid_role_updates_roll_back() {
    let mut app = model(
        r#"
        ListElement { title:"root"; entries: ListElement { n:1 } }
        function copy(){append(get(0));get(1).entries.setProperty(0,"n",7)}
        function report(){return get(0).entries.get(0).n + "/" + get(1).entries.get(0).n}
        function invalid(){set(0,{entries:[{n:3},4]})}
        function replace(){get(0).entries=[{n:2}]}
    "#,
    )
    .unwrap();
    let root = app.root();
    app.call(root, "copy", vec![]).unwrap();
    assert_eq!(
        app.call(root, "report", vec![]).unwrap(),
        Value::String("1/7".into())
    );
    let before = app.structure_revision();
    assert!(app.call(root, "invalid", vec![]).is_err());
    assert_eq!(app.structure_revision(), before);
    app.call(root, "replace", vec![]).unwrap();
    assert_eq!(
        app.call(root, "report", vec![]).unwrap(),
        Value::String("2/7".into())
    );
}
