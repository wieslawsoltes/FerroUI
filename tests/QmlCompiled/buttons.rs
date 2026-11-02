use ferroui_qml::{Instance, TypeDefinition, TypeRegistry, Value};
include!(concat!(env!("OUT_DIR"), "/buttons.rs"));

fn registry() -> TypeRegistry {
    let mut types = super::quick_registry();
    for name in ["RadioButton", "Switch", "CheckBox"] {
        let ty = TypeDefinition::default()
            .inherits("QtQuick", "Item")
            .property("text", "string", Value::String(String::new()))
            .property("enabled", "bool", Value::Bool(true))
            .property_group("Accessible")
            .property("Accessible.name", "string", Value::String(String::new()))
            .quick_button(name);
        types.register("QtQuick.Controls", name, ty).unwrap();
    }
    types
        .register(
            "QtQuick.Controls",
            "ButtonGroup",
            TypeDefinition::quick_button_group(),
        )
        .unwrap();
    types
}

fn exercise(mut ui: Instance) {
    let id = ui.root();
    let report = |ui: &mut Instance| ui.call(id, "report", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("2/false/0/false/false/false/0/0/".into())
    );
    assert_eq!(
        ui.call(id, "attached", vec![]).unwrap(),
        Value::String("true/Main power".into())
    );
    for op in ["choose", "choose", "setSecond", "nextCheck"] {
        ui.call(id, op, vec![]).unwrap();
    }
    assert_eq!(
        report(&mut ui),
        Value::String("2/false/1/false/true/false/1/0/TCGCGK".into())
    );
    for op in ["nextCheck", "flip", "groupAll"] {
        ui.call(id, op, vec![]).unwrap();
    }
    assert_eq!(
        report(&mut ui),
        Value::String("2/false/2/true/true/true/2/1/TCGCGKKS".into())
    );
    ui.call(id, "detach", vec![]).unwrap();
    assert_eq!(
        ui.call(id, "attached", vec![]).unwrap(),
        Value::String("false/Backup".into())
    );
}
#[test]
fn buttons_from_compiled_rust() {
    exercise(Instance::new(&compiled_qml(), &registry(), Default::default()).unwrap());
}

#[cfg(feature = "webscene")]
#[test]
fn buttons_with_webscene() {
    use std::rc::Rc;
    let path = crate::fixtures::controls("buttons/Main.qml");
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path)
        .unwrap();
    exercise(
        Instance::with_engine(
            &source.document,
            &registry(),
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}

#[test]
fn moving_button_membership_and_failed_handlers_are_transactional() {
    let doc=ferroui_qml::ir::parse(r#"import QtQuick
    import QtQuick.Controls
    Item { id:page; property bool reject:false
        ButtonGroup {id:left; buttons:[a,b]}
        ButtonGroup {id:right; onClicked:{if(page.reject)missing()}}
        RadioButton {id:a}
        RadioButton {id:b}
        function move(){right.addButton(a)}
        function undo(){right.removeButton(a);left.addButton(a)}
        function press(){a.click()}
        function rejectPress(){reject=true;a.click()}
        function report():string{return left.buttons.length + "/" + right.buttons.length + "/" + a.checked + "/" + reject}
    }"#).unwrap();
    let mut ui = Instance::new(&doc, &registry(), Default::default()).unwrap();
    let root = ui.root();
    let report = |ui: &mut Instance| ui.call(root, "report", vec![]).unwrap();
    ui.call(root, "move", vec![]).unwrap();
    assert_eq!(report(&mut ui), Value::String("1/1/false/false".into()));
    assert!(ui.call(root, "rejectPress", vec![]).is_err());
    assert_eq!(report(&mut ui), Value::String("1/1/false/false".into()));
    ui.call(root, "press", vec![]).unwrap();
    ui.call(root, "undo", vec![]).unwrap();
    assert_eq!(report(&mut ui), Value::String("2/0/true/false".into()));
}

#[test]
fn declarative_toggles_settle_before_completion() {
    let unit=ferroui_qml::ir::parse(r#"import QtQuick
        import QtQuick.Controls
        Item { Switch {id:s; checked:true} CheckBox {id:c; checked:true}
            RadioButton {id:a; checked:true} RadioButton {id:b}
            function snapshot():string{return s.position + "/" + c.checkState + "/" + a.checked + "/" + b.checked}
            function select(){b.checked=true}
        }"#).unwrap();
    let mut ui = Instance::new(&unit, &registry(), Default::default()).unwrap();
    let root = ui.root();
    let get = |ui: &mut Instance| ui.call(root, "snapshot", vec![]).unwrap();
    assert_eq!(get(&mut ui), Value::String("1/2/true/false".into()));
    ui.call(root, "select", vec![]).unwrap();
    assert_eq!(get(&mut ui), Value::String("1/2/false/true".into()));
}
