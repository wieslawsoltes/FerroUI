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
    let mut ui = view(include_str!("fixtures/views/Main.qml"));
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
    let mut ui = view(include_str!("fixtures/views/Decorated.qml"));
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

#[test]
fn sticky_next_label_follows_the_last_visible_group() {
    let mut ui = view(
        r#"import QtQuick
        ListView {width:100;height:61;cacheBuffer:0;model:["A","A","B","B","C","D","E","F"]
            section.property:"modelData"
            section.labelPositioning: ViewSection.InlineLabels | ViewSection.CurrentLabelAtStart | ViewSection.NextLabelAtEnd
            section.delegate: Item {property string label:section; width:10;height:5}
            delegate:Item {width:10;height:10}
        }"#,
    );
    let id = ui.root();
    for (offset, want) in [
        (0., vec![("A", 0.), ("B", 25.), ("C", 50.), ("D", 56.)]),
        (8., vec![("A", 8.), ("B", 25.), ("C", 50.), ("D", 64.)]),
        (
            15.,
            vec![("A", 15.), ("B", 25.), ("C", 50.), ("D", 65.), ("E", 71.)],
        ),
    ] {
        ui.set(id, "contentY", Value::Number(offset)).unwrap();
        let Value::List(items) = ui.get(id, "data").unwrap() else {
            panic!("view children")
        };
        let labels = items
            .into_iter()
            .filter_map(|o| {
                let Value::Object(o) = o else { return None };
                let Value::String(label) = ui.get(o, "label").ok()? else {
                    return None;
                };
                let Value::Number(y) = ui.get(o, "y").ok()? else {
                    return None;
                };
                Some((label, y))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            want.into_iter()
                .map(|(s, y)| (s.to_owned(), y))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn unrelated_role_edits_retain_section_delegates_and_failed_regrouping_rolls_back() {
    let mut ui = view(
        r#"import QtQuick
        ListView { id: v; height:100; width:100; property int createdLabels: 0
            model: ListModel { id: rows; ListElement { category: "A"; score: 0 }; ListElement { category: "B"; score: 0 } }
            section.property: "category"
            section.delegate: Item { required property string section; height: 10; Component.onCompleted: v.createdLabels++ }
            delegate: Item { required property int score; property string label: ListView.section; height: 10 }
            function unrelated(){ rows.set(0,{score: 17, category:"A"}); return itemAtIndex(0).score }
            function fail(){ rows.setProperty(0,"category","B"); missing() }
            function regroup(){ rows.setProperty(0,"category","B"); return itemAtIndex(0).label }
        }"#,
    );
    let root = ui.root();
    let labels = ui.get(root, "createdLabels").unwrap();
    assert_eq!(labels, Value::Number(2.));
    assert_eq!(
        ui.call(root, "unrelated", vec![]).unwrap(),
        Value::Number(17.)
    );
    assert_eq!(ui.get(root, "createdLabels").unwrap(), labels);
    assert!(ui.call(root, "fail", vec![]).is_err());
    assert_eq!(ui.get(root, "createdLabels").unwrap(), labels);
    assert_eq!(
        ui.call(root, "regroup", vec![]).unwrap(),
        Value::String("B".into())
    );
}

#[test]
fn local_section_edits_match_a_full_rebuild_at_every_boundary() {
    let mut ui = view(
        r#"import QtQuick
        ListView { id: v; height:200; width:100
            model: ListModel { id: rows; ListElement { category:"" }; ListElement { category:"" }; ListElement { category:"B" }; ListElement { category:"b" } }
            section.property:"modelData"; section.delegate:Item { required property string section; height:3 }
            delegate:Item { required property int index; property string group:ListView.section; height:8 }
            function report() { return itemAtIndex(0).group + "/" + itemAtIndex(1).group + "/" + itemAtIndex(2).group + "/" + itemAtIndex(3).group + "/" + contentHeight }
        }"#,
    );
    let rows = ui.id("rows").unwrap();
    for (i, name, expected) in [
        (0, "A", "A//B/B/41"),
        (1, "a", "A/A/B/B/38"),
        (2, "a", "A/A/A/b/38"),
        (0, "a", "a/a/a/b/38"),
        (3, "a", "a/a/a/a/35"),
        (0, "", "/a/a/a/35"),
        (3, "", "/a/a//38"),
    ] {
        ui.call(
            rows,
            "setProperty",
            vec![
                Value::Number(i as f64),
                Value::String("category".into()),
                Value::String(name.into()),
            ],
        )
        .unwrap();
        assert_eq!(
            ui.call(ui.root(), "report", vec![]).unwrap(),
            Value::String(expected.into())
        );
    }
}
