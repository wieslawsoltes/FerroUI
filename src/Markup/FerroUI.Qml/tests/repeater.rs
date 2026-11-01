use ferroui_qml::{ir::parse, Error, Instance, ObjectId, TypeDefinition, TypeRegistry, Value};

fn instance(source: &str) -> Result<Instance, Error> {
    instrumented_instance(source, Default::default())
}

fn instrumented_instance(
    source: &str,
    touches: std::rc::Rc<std::cell::Cell<usize>>,
) -> Result<Instance, Error> {
    let mut registry = TypeRegistry::new();
    registry.register(
        "QtQuick",
        "Item",
        TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .property("data", "list<QtObject>", Value::List(vec![]))
            .default_property("data")
            .visual_children("data")
            .context_method(
                "observe",
                vec![ferroui_qml::Parameter {
                    name: "value".into(),
                    ty: Some("string".into()),
                }],
                "string",
                move |_, args| {
                    touches.set(touches.get() + 1);
                    Ok(args[0].clone())
                },
            ),
    )?;
    registry.register("QtQuick", "Repeater", TypeDefinition::quick_repeater())?;
    registry.reexport("QtQuick", "QtQml")?;
    let doc = parse(source).unwrap();
    registry.check(&doc)?;
    Instance::new(&doc, &registry, Default::default())
}

fn item(engine: &mut Instance, repeater: ObjectId, index: i32) -> Option<ObjectId> {
    match engine
        .call(repeater, "itemAt", vec![Value::Number(index as f64)])
        .unwrap()
    {
        Value::Object(id) => Some(id),
        Value::Null => None,
        other => panic!("invalid itemAt result {other:?}"),
    }
}

#[test]
fn implicit_delegate_scope_and_stacking_follow_the_model() {
    let mut engine = instance(
        r#"import QtQuick
        Item { id: root
            property int size: 3
            property int adds: 0
            property int removes: 0
            Item { id: first }
            Repeater { id: rep
                model: root.size
                onItemAdded: (i, child) => { if (rep.itemAt(i) === child && rep.itemAt(i + 1) === null) root.adds++ }
                onItemRemoved: (i, child) => { if (rep.itemAt(i) === child) root.removes++ }
                Item { id: localItem; property int value: modelData + index }
            }
            Item { id: last }
        }"#,
    )
    .unwrap();
    let root = engine.root();
    let rep = engine.id("rep").unwrap();
    let delegates = (0..3)
        .map(|i| item(&mut engine, rep, i).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        engine.get(delegates[2], "value").unwrap(),
        Value::Number(4.)
    );
    assert!(engine.id("localItem").is_none());
    assert!(engine.get(delegates[0], "index").is_err());
    assert_eq!(engine.parent(delegates[0]), Some(root));
    let mut visual = vec![engine.id("first").unwrap(), rep];
    visual.extend_from_slice(&delegates);
    visual.push(engine.id("last").unwrap());
    assert_eq!(
        engine.get(root, "data").unwrap(),
        Value::List(visual.into_iter().map(Value::Object).collect())
    );
    assert_eq!(engine.get(root, "adds").unwrap(), Value::Number(3.));
    assert_eq!(item(&mut engine, rep, -1), None);
    engine.set(root, "size", Value::Number(1.)).unwrap();
    assert_eq!(engine.get(root, "removes").unwrap(), Value::Number(3.));
    assert_eq!(engine.get(root, "adds").unwrap(), Value::Number(4.));
    assert_eq!(engine.get(rep, "count").unwrap(), Value::Number(1.));
    assert!(delegates.iter().all(|id| engine.type_name(*id).is_none()));
    assert_eq!(item(&mut engine, rep, 1), None);
}

#[test]
fn list_model_moves_keep_item_identity_and_role_edits_roll_back() {
    let mut engine = instance(
        r#"import QtQuick
        import QtQml.Models
        Item {
            ListModel { id: rows; ListElement { title:"A" } ListElement { title:"B" } }
            Repeater { id: rep; model: rows
                delegate: Item {
                    required property int index
                    required property string title
                    property string label: index + ":" + title
                }
            }
            function change() { rows.move(1,0,1); rows.setProperty(0, "title", "C") }
            function broken() { rows.remove(0); rows.append({title:"X"}); missing() }
        }"#,
    )
    .unwrap();
    let root = engine.root();
    let rep = engine.id("rep").unwrap();
    let a = item(&mut engine, rep, 0).unwrap();
    let b = item(&mut engine, rep, 1).unwrap();
    engine.call(root, "change", vec![]).unwrap();
    assert_eq!(item(&mut engine, rep, 0), Some(b));
    assert_eq!(item(&mut engine, rep, 1), Some(a));
    assert_eq!(engine.get(b, "label").unwrap(), Value::String("0:C".into()));
    assert_eq!(engine.get(a, "label").unwrap(), Value::String("1:A".into()));
    let visual = engine.get(root, "data").unwrap();
    let revision = engine.structure_revision();
    assert!(engine.call(root, "broken", vec![]).is_err());
    assert_eq!(engine.structure_revision(), revision);
    assert_eq!(engine.get(root, "data").unwrap(), visual);
    assert_eq!(item(&mut engine, rep, 0), Some(b));
    assert_eq!(engine.get(b, "label").unwrap(), Value::String("0:C".into()));
}

#[test]
fn detached_repeaters_have_a_count_without_instantiated_delegates() {
    let mut engine = instance(
        r#"import QtQuick
       Repeater { model: ["A","B"]; delegate: Item { required property string modelData } }"#,
    )
    .unwrap();
    let rep = engine.root();
    assert_eq!(engine.get(rep, "count").unwrap(), Value::Number(2.));
    assert_eq!(item(&mut engine, rep, 0), None);
}

#[test]
fn failing_to_supply_a_delegate_input_preserves_the_mounted_instances() {
    let mut engine = instance(
        r#"import QtQuick
      Item { id: root
        Component { id: good; Item {required property string modelData} }
        Component { id: bad; Item {required property int unavailable} }
        Repeater { id: rep; model: ["A","B"]; delegate: good }
      }"#,
    )
    .unwrap();
    let rep = engine.id("rep").unwrap();
    let a = item(&mut engine, rep, 0).unwrap();
    let bad = engine.id("bad").unwrap();
    assert!(engine.set(rep, "delegate", Value::Object(bad)).is_err());
    assert_eq!(item(&mut engine, rep, 0), Some(a));
    assert_eq!(
        engine.get(a, "modelData").unwrap(),
        Value::String("A".into())
    );
}

#[test]
fn destroying_a_dynamic_repeater_retires_its_siblings_and_rejects_direct_delegate_destruction() {
    let mut engine = instance(
        r#"import QtQuick
        Item { id: root
            Component { id: factory; Repeater { model: 2; Item {} } }
        }"#,
    )
    .unwrap();
    let root = engine.root();
    let factory = engine.id("factory").unwrap();
    let rep = engine
        .create_object(factory, Some(root), Default::default())
        .unwrap();
    let a = item(&mut engine, rep, 0).unwrap();
    assert!(engine.destroy_object(a).is_err());
    assert!(engine.call(a, "destroy", vec![]).is_err());
    assert_eq!(item(&mut engine, rep, 0), Some(a));
    engine.destroy_object(rep).unwrap();
    assert_eq!(
        engine.get(root, "data").unwrap(),
        Value::List(vec![Value::Object(factory)])
    );
    assert!(engine.type_name(a).is_none());
    assert_eq!(engine.children(root), &[factory]);
}

#[test]
fn removing_a_row_releases_bindings_that_read_through_its_model_handle() {
    let mut engine = instance(
        r#"import QtQuick
        import QtQml.Models
        Item {
            ListModel { id: rows; ListElement { title:"A" } ListElement { title:"B" } }
            Repeater { id: rep; model: rows; Item { property string label: model.title + rows.count } }
        }"#,
    )
    .unwrap();
    let rep = engine.id("rep").unwrap();
    let rows = engine.id("rows").unwrap();
    let b = item(&mut engine, rep, 1).unwrap();
    engine
        .call(rows, "remove", vec![Value::Number(0.)])
        .unwrap();
    assert_eq!(item(&mut engine, rep, 0), Some(b));
    assert_eq!(engine.get(b, "label").unwrap(), Value::String("B1".into()));
}

#[test]
fn single_row_edits_invalidate_only_that_rows_delegates() {
    use std::{cell::Cell, rc::Rc};
    let reads = Rc::new(Cell::new(0));
    let rows = r#"ListElement { title: "row" }"#.repeat(1024);
    let source = format!(
        r#"import QtQuick
        import QtQml.Models
        Item {{
            ListModel {{ id: rows; {rows} }}
            Repeater {{ id: rep; model: rows; Item {{ property string label: observe(model.title) }} }}
            function edit() {{ rows.setProperty(500, "title", "one") }}
            function setRow() {{ rows.set(500, {{title: "two"}}) }}
            function editHandle() {{ rows.get(500).title = "three" }}
            function failedEdit() {{ rows.setProperty(500,"title","four"); missing() }}
        }}"#
    );
    let mut engine = instrumented_instance(&source, reads.clone()).unwrap();
    let root = engine.root();
    let rep = engine.id("rep").unwrap();
    let row = item(&mut engine, rep, 500).unwrap();
    assert_eq!(reads.get(), 1024);
    for (method, text) in [("edit", "one"), ("setRow", "two"), ("editHandle", "three")] {
        reads.set(0);
        engine.call(root, method, vec![]).unwrap();
        assert_eq!(reads.get(), 1, "{method} reevaluated unrelated delegates");
        assert_eq!(item(&mut engine, rep, 500), Some(row));
        assert_eq!(
            engine.get(row, "label").unwrap(),
            Value::String(text.into())
        );
    }
    assert!(engine.call(root, "failedEdit", vec![]).is_err());
    assert_eq!(
        engine.get(row, "label").unwrap(),
        Value::String("three".into())
    );
}

#[test]
fn schema_changes_refresh_all_rows_and_reentrant_role_writes_reach_their_own_delegates() {
    let mut engine = instance(r#"import QtQuick
        import QtQml.Models
        Item { id: root
            property bool editing: false
            ListModel { id: rows; ListElement { title:"A" } ListElement { title:"B" } }
            Repeater { id: rep; model: rows
                Item {
                    required property int index
                    required property var model
                    required property var modelData
                    property string text: model.title
                    onTextChanged: { if (root.editing && index === 0) rows.setProperty(1, "title", text + "!") }
                }
            }
            function edit() { editing = true; rows.setProperty(0, "title","AB") }
            function expand() { rows.setProperty(0, "other", 1) }
        }"#).unwrap();
    let root = engine.root();
    let rep = engine.id("rep").unwrap();
    let first = item(&mut engine, rep, 0).unwrap();
    let second = item(&mut engine, rep, 1).unwrap();
    engine.call(root, "edit", vec![]).unwrap();
    assert_eq!(
        engine.get(second, "text").unwrap(),
        Value::String("AB!".into())
    );
    assert_eq!(
        engine.get(second, "modelData").unwrap(),
        Value::String("AB!".into())
    );
    engine.call(root, "expand", vec![]).unwrap();
    assert_eq!(engine.get(first, "modelData").unwrap(), Value::Undefined);
    assert_eq!(engine.get(second, "modelData").unwrap(), Value::Undefined);
    assert_eq!(item(&mut engine, rep, 1), Some(second));
}
