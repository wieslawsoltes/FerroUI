use ferroui_qml::{ir, Instance, TypeDefinition, TypeRegistry, Value};
use std::collections::BTreeMap;

#[test]
fn declared_enum_keys_are_accessible_by_scope_and_value_aliases_retain_declaration_order() {
    let doc = ir::parse(
        r#"
        import QtQml
        QtObject {
            enum Severity { Unknown = -1, Low, High = 3, Critical = 3 }
            property int scoped: Message.Severity.High
            property int unscoped: Message["Unknown"]
            property int lookup: Qt.enumStringToValue(Message.Severity, "Critical")
            property list<string> labels: Qt.enumValueToStrings(Message.Severity, 3)
            property string low: Qt.enumValueToString(Message.Severity, Message.Low)
        }
    "#,
    )
    .unwrap();
    let view = Instance::new_named("Message", &doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
    let root = view.root();
    assert_eq!(view.get(root, "scoped").unwrap(), Value::Number(3.));
    assert_eq!(view.get(root, "unscoped").unwrap(), Value::Number(-1.));
    assert_eq!(view.get(root, "lookup").unwrap(), Value::Number(3.));
    assert_eq!(
        view.get(root, "labels").unwrap(),
        Value::List(vec![
            Value::String("High".into()),
            Value::String("Critical".into())
        ])
    );
    assert_eq!(view.get(root, "low").unwrap(), Value::String("Low".into()));
}

#[test]
fn enum_scopes_respect_import_aliases_inheritance_and_lexical_masking() {
    let registry = &mut TypeRegistry::new();
    registry
        .register(
            "Demo",
            "State",
            TypeDefinition::default()
                .enumeration("Mode", &[("Ready", 4), ("Done", 9)])
                .unwrap(),
        )
        .unwrap();
    registry
        .register(
            "Demo",
            "Derived",
            TypeDefinition::default().inherits("Demo", "State"),
        )
        .unwrap();
    let doc = ir::parse(
        r#"
        import QtQml
        import Demo as D
        QtObject {
            property int result: D.Derived.Mode.Ready + D.State.Done
            function masked(D) { return D + 10 }
            function bad() { return Qt.enumValueToString(D.State.Mode, 999) }
            property string html: Qt.escapeHtml('<x attr="&">')
        }
    "#,
    )
    .unwrap();
    let mut view = Instance::new(&doc, registry, BTreeMap::new()).unwrap();
    assert_eq!(view.get(view.root(), "result").unwrap(), Value::Number(13.));
    assert_eq!(
        view.get(view.root(), "html").unwrap(),
        Value::String("&lt;x attr=&quot;&amp;&quot;&gt;".into())
    );
    assert_eq!(
        view.call(view.root(), "masked", vec![Value::Number(5.)])
            .unwrap(),
        Value::Number(15.)
    );
    assert!(view.call(view.root(), "bad", vec![]).is_err());
}
