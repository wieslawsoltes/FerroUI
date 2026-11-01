use ferroui_qml::{ir::parse, Instance, TypeDefinition, TypeRegistry, Value};

fn view(source: &str) -> Instance {
    let mut types = TypeRegistry::new();
    let item = TypeDefinition::default()
        .inherits("QtQml", "QtObject")
        .property("width", "real", Value::Undefined)
        .property("height", "real", Value::Undefined)
        .property("x", "real", Value::Number(0.))
        .property("y", "real", Value::Number(0.))
        .property("data", "list<QtObject>", Value::List(vec![]))
        .visual_children("data")
        .default_property("data");
    types.register("QtQuick", "Item", item).unwrap();
    types
        .register("QtQuick", "ListView", TypeDefinition::quick_list_view())
        .unwrap();
    types
        .register("QtQuick", "Repeater", TypeDefinition::quick_repeater())
        .unwrap();
    types.reexport("QtQuick", "QtQml").unwrap();
    Instance::new(&parse(source).unwrap(), &types, Default::default()).unwrap()
}

#[test]
fn million_row_view_realizes_only_viewport_and_current_delegate() {
    let mut ui = view(include_str!("../../../../tests/QmlCompiled/views/Main.qml"));
    let root = ui.root();
    ui.call(root, "scrollMiddle", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "report", vec![]).unwrap(),
        Value::String("1000000/20000000/10000000/500000/500000/true".into())
    );
    assert!(ui.objects().count() < 24);
    ui.call(root, "changeCurrent", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "selection", vec![]).unwrap(),
        Value::String("0/updated/updated".into())
    );
}
#[test]
fn retained_geometry_can_change_and_failed_scroll_rolls_back_all_delegates() {
    let mut ui = view(
        r#"import QtQuick
        ListView { id: list; height:80; width:20; model:100
            delegate: Item { required property int index; height:index === 2 ? 40 : 20; width:20 }
            function fail(){positionViewAtIndex(50,ListView.Center); missing()}
        }"#,
    );
    let list = ui.root();
    let at = |ui: &mut Instance, i| {
        ui.call(list, "itemAtIndex", vec![Value::Number(i)])
            .unwrap()
    };
    let two = at(&mut ui, 2.);
    let three = at(&mut ui, 3.);
    assert_eq!(three, Value::Null);
    assert!(ui.call(list, "fail", vec![]).is_err());
    assert_eq!(at(&mut ui, 2.), two);
    assert_eq!(ui.get(list, "contentY").unwrap(), Value::Number(0.));
    let Value::Object(two) = two else {
        panic!("missing delegate")
    };
    ui.set(two, "height", Value::Number(15.)).unwrap();
    let Value::Object(three) = at(&mut ui, 3.) else {
        panic!("missed newly visible delegate")
    };
    assert_eq!(ui.get(three, "y").unwrap(), Value::Number(55.));
}
#[test]
fn delegate_access_modes_control_required_and_implicit_writes() {
    let source = r#"import QtQuick
    import QtQml.Models
    Item {
        ListModel {id:rows; ListElement {title:"A"}}
        Repeater {id:r; model:rows; delegateModelAccess:DelegateModel.ACCESS
            delegate: Item { DECL
                function direct(){title="B"}
                function projected(){model.title="C"}
            }
        }
        function direct(){r.itemAt(0).direct()}
        function projected(){r.itemAt(0).projected()}
        function value(){return rows.get(0).title}
    }"#;
    for required in [true, false] {
        for mode in ["ReadOnly", "ReadWrite", "Qt5ReadWrite"] {
            let mut ui = view(&source.replace("ACCESS", mode).replace(
                "DECL",
                if required {
                    "required property string title; required property var model"
                } else {
                    ""
                },
            ));
            let root = ui.root();
            let call = |ui: &mut Instance, name| ui.call(root, name, vec![]);
            let direct = call(&mut ui, "direct");
            assert_eq!(
                direct.is_ok(),
                required || mode != "ReadOnly",
                "{mode} {required}"
            );
            let expected = if mode == "ReadWrite" || !required && mode == "Qt5ReadWrite" {
                "B"
            } else {
                "A"
            };
            assert_eq!(
                call(&mut ui, "value").unwrap(),
                Value::String(expected.into())
            );
            let projected = call(&mut ui, "projected");
            assert_eq!(projected.is_ok(), mode != "ReadOnly");
            if projected.is_ok() {
                assert_eq!(call(&mut ui, "value").unwrap(), Value::String("C".into()));
            }
        }
    }
}

#[test]
fn reversed_sections_measure_decoration_and_react_to_model_edits() {
    let mut ui = view(include_str!(
        "../../../../tests/QmlCompiled/views/Decorated.qml"
    ));
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
fn pooled_items_survive_rebinding_and_failed_scroll_restores_row_source() {
    let mut ui = view(
        r#"import QtQuick
    ListView { id:v; width:20; height:40; cacheBuffer:0; reuseItems:true
        property int pooled:0; property int reused:0
        model: ListModel {id:rows; ListElement {title:"A"} ListElement {title:"B"} ListElement {title:"C"} ListElement {title:"D"} ListElement {title:"E"} ListElement {title:"F"} }
        delegate: Item {required property int index; required property string title; required property var model
            width:20; height:20; property string observed:""
            ListView.onPooled: v.pooled += 1
            ListView.onReused: {v.reused += 1; observed = index + title + model.title}
            function edit() {model.title = "edited"}
        }
        function fail(){positionViewAtEnd();missing()}
        function at(i:int):string {return rows.get(i).title}
    }"#,
    );
    let id = ui.root();
    let Value::Object(old) = ui.call(id, "itemAtIndex", vec![Value::Number(1.)]).unwrap() else {
        panic!("visible delegate")
    };
    assert!(ui.call(id, "fail", vec![]).is_err());
    ui.call(old, "edit", vec![]).unwrap();
    assert_eq!(
        ui.call(id, "at", vec![Value::Number(1.)]).unwrap(),
        Value::String("edited".into())
    );
    ui.call(id, "positionViewAtEnd", vec![]).unwrap();
    assert_eq!(ui.get(id, "pooled").unwrap(), Value::Number(1.));
    assert_eq!(ui.get(id, "reused").unwrap(), Value::Number(1.));
    assert_eq!(
        ui.get(old, "observed").unwrap(),
        Value::String("4EE".into())
    );
    ui.call(old, "edit", vec![]).unwrap();
    assert_eq!(
        ui.call(id, "at", vec![Value::Number(4.)]).unwrap(),
        Value::String("edited".into())
    );
}

#[test]
fn overlay_header_and_footer_reserve_room_for_the_positioned_row() {
    let mut ui = view(
        r#"import QtQuick
        ListView {width:45; height:45; spacing:3; model:5
            headerPositioning:ListView.OverlayHeader; footerPositioning:ListView.OverlayFooter
            header:Item {width:7; height:7} footer:Item {width:9; height:9}
            delegate:Item {width:20; height:20}
            function jump(){positionViewAtIndex(3,ListView.Beginning)}
            function report():string {return contentY + "/" + headerItem.y + "/" + footerItem.y}
        }"#,
    );
    let id = ui.root();
    assert_eq!(
        ui.call(id, "report", vec![]).unwrap(),
        Value::String("-7/-7/29".into())
    );
    ui.call(id, "jump", vec![]).unwrap();
    assert_eq!(
        ui.call(id, "report", vec![]).unwrap(),
        Value::String("62/62/98".into())
    );
}
