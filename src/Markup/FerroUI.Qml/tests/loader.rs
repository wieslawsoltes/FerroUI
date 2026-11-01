use ferroui_qml::{ir::parse, Instance, ObjectId, TypeDefinition, TypeRegistry, Value};

fn model(source: &str) -> Instance {
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("width", "real", Value::Undefined)
                .property("height", "real", Value::Undefined)
                .property("data", "list<QtObject>", Value::List(vec![]))
                .default_property("data")
                .visual_children("data"),
        )
        .unwrap();
    registry
        .register("QtQuick", "Loader", TypeDefinition::quick_loader())
        .unwrap();
    registry
        .register("QtQuick", "Repeater", TypeDefinition::quick_repeater())
        .unwrap();
    registry.reexport("QtQuick", "QtQml").unwrap();
    let doc = parse(source).unwrap();
    Instance::new(&doc, &registry, Default::default()).unwrap()
}

fn item(model: &Instance, loader: ObjectId) -> ObjectId {
    let Value::Object(id) = model.get(loader, "item").unwrap() else {
        panic!("Loader has no item")
    };
    id
}

#[test]
fn switching_or_deactivating_an_inline_loader_releases_the_previous_subtree() {
    let mut model = model(
        r#"import QtQuick
        Item { id: root
          property int loads: 0
          Component { id: tile; Item { property int role: 7 } }
          Loader { id: loader; sourceComponent: tile; onLoaded: root.loads++ }
        }"#,
    );
    let loader = model.id("loader").unwrap();
    let first = item(&model, loader);
    assert_eq!(model.get(first, "role").unwrap(), Value::Number(7.));
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(model.get(model.root(), "loads").unwrap(), Value::Number(1.));
    model.set(loader, "active", Value::Bool(false)).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(model.get(loader, "data").unwrap(), Value::List(vec![]));
    assert!(model.type_name(first).is_none());
    model.set(loader, "active", Value::Bool(true)).unwrap();
    assert_ne!(first, item(&model, loader));
    model.set(loader, "sourceComponent", Value::Null).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(model.get(loader, "progress").unwrap(), Value::Number(0.));
}

#[test]
fn inactive_and_replaced_deferred_loaders_do_not_publish_stale_items() {
    let mut model = model(
        r#"import QtQuick
        Item { Component { id: tile; Item {} }
           Loader { id: loader; asynchronous: true; sourceComponent: tile }
        }"#,
    );
    let loader = model.id("loader").unwrap();
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(2.));
    model.set(loader, "active", Value::Bool(false)).unwrap();
    model.process_deferred(8).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    model.set(loader, "active", Value::Bool(true)).unwrap();
    model.process_deferred(8).unwrap();
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(1.));
}

#[test]
fn explicit_and_content_driven_loader_axes_follow_qt_sizing_rules() {
    let mut model = model(
        r#"import QtQuick
        Loader { id: host; sourceComponent: Component { Item {width:40; height:12; property real implicitWidth:25 } } }
    "#,
    );
    let root = model.root();
    let child = item(&model, root);
    let n = Value::Number;
    assert_eq!(model.get(root, "width").unwrap(), n(40.));
    model.set(child, "width", n(33.)).unwrap();
    assert_eq!(model.get(root, "implicitWidth").unwrap(), n(33.));
    model.set(root, "width", n(77.)).unwrap();
    assert_eq!(model.get(child, "width").unwrap(), n(77.));
    assert_eq!(model.get(root, "implicitWidth").unwrap(), n(25.));
    assert_eq!(model.get(root, "height").unwrap(), n(12.));
    model.set(child, "width", n(31.)).unwrap();
    assert_eq!(model.get(child, "width").unwrap(), n(31.));
    assert_eq!(model.get(root, "width").unwrap(), n(77.));
    model.set(root, "width", Value::Undefined).unwrap();
    assert_eq!(model.get(root, "width").unwrap(), n(31.));
    model.set(root, "active", Value::Bool(false)).unwrap();
    assert_eq!(model.get(root, "width").unwrap(), n(31.));
}

#[test]
fn missing_required_input_reports_error_without_retaining_a_failed_instance() {
    let model = model(
        r#"import QtQuick
        Loader { sourceComponent: Component { Item {required property int amount} } }
    "#,
    );
    assert_eq!(
        model.get(model.root(), "status").unwrap(),
        Value::Number(3.)
    );
    assert_eq!(model.get(model.root(), "item").unwrap(), Value::Null);
    assert_eq!(model.objects().count(), 2);
}

#[test]
fn component_valued_object_shorthand_defers_its_private_ids_and_initialization() {
    let mut model = model(
        r#"import QtQuick
        Loader {
            active: false
            sourceComponent: Item { id: privateChild; property int amount:21 }
        }"#,
    );
    let root = model.root();
    assert_eq!(model.get(root, "item").unwrap(), Value::Null);
    assert!(model.id("privateChild").is_none());
    assert_eq!(model.objects().count(), 2);
    model.set(root, "active", Value::Bool(true)).unwrap();
    assert_eq!(
        model.get(item(&model, root), "amount").unwrap(),
        Value::Number(21.)
    );
    assert!(model.id("privateChild").is_none());
}

#[test]
fn an_aborted_reload_restores_the_previously_loaded_identity_and_selection() {
    let mut model = model(
        r#"import QtQuick
        Item { id: page; property bool reject:false
            Component {id: a; Item{property bool first:true}}
            Component {id: b; Item{property bool first:false}}
            Loader {id: slot; sourceComponent:a; onLoaded: { if (page.reject) page.unknownMethod() }}
        }"#,
    );
    let loader = model.id("slot").unwrap();
    let first = item(&model, loader);
    let prior = model.structure_revision();
    model
        .set(model.root(), "reject", Value::Bool(true))
        .unwrap();
    assert!(model
        .set(
            loader,
            "sourceComponent",
            Value::Object(model.id("b").unwrap())
        )
        .is_err());
    assert_eq!(item(&model, loader), first);
    assert_eq!(model.get(first, "first").unwrap(), Value::Bool(true));
    assert_eq!(model.structure_revision(), prior);
    model
        .set(model.root(), "reject", Value::Bool(false))
        .unwrap();
    model
        .set(
            loader,
            "sourceComponent",
            Value::Object(model.id("b").unwrap()),
        )
        .unwrap();
    assert!(model.type_name(first).is_none());
    assert_eq!(
        model.get(item(&model, loader), "first").unwrap(),
        Value::Bool(false)
    );
}

fn wide_loader() -> Instance {
    let row = "Item { Item { Component.onCompleted: page.completions++ } }\n";
    model(&format!(
        "import QtQuick\nItem {{ id: page; property int completions: 0
            Component {{ id: wide; Item {{ {} }} }}
            Component {{ id: small; Item {{ width: 10 }} }}
            Loader {{ id: slot; asynchronous: true; sourceComponent: wide }}
        }}",
        row.repeat(96)
    ))
}

#[test]
fn preparing_a_large_async_loader_hides_objects_and_runs_completion_only_at_publication() {
    let mut app = wide_loader();
    let root = app.root();
    let loader = app.id("slot").unwrap();
    let before = app.objects().count();
    let revision = app.structure_revision();
    assert_eq!(app.process_deferred_while(16, || false).unwrap(), 0);
    let mut turns = 0;
    while app.get(loader, "item").unwrap() == Value::Null {
        assert_eq!(app.objects().count(), before);
        assert_eq!(app.structure_revision(), revision);
        assert_eq!(app.get(root, "completions").unwrap(), Value::Number(0.));
        assert_eq!(app.get(loader, "status").unwrap(), Value::Number(2.));
        assert_eq!(app.process_deferred(usize::MAX).unwrap(), 1);
        turns += 1;
        assert!(turns < 1000);
    }
    assert!(turns > 1);
    assert_eq!(app.objects().count(), before + 193);
    assert_eq!(app.get(root, "completions").unwrap(), Value::Number(96.));
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(1.));
    assert!(!app.has_deferred_work());
}

#[test]
fn cancellation_replacement_and_synchronous_completion_discard_private_preparations() {
    let mut app = wide_loader();
    let loader = app.id("slot").unwrap();
    let before = app.objects().count();

    assert_eq!(app.process_deferred(1).unwrap(), 1);
    app.set(loader, "active", Value::Bool(false)).unwrap();
    assert!(!app.has_deferred_work());
    assert_eq!(app.objects().count(), before);

    app.set(loader, "active", Value::Bool(true)).unwrap();
    app.process_deferred(1).unwrap();
    let small = app.id("small").unwrap();
    app.set(loader, "sourceComponent", Value::Object(small))
        .unwrap();
    app.process_deferred(1).unwrap();
    assert_eq!(
        app.get(item(&app, loader), "width").unwrap(),
        Value::Number(10.)
    );
    assert_eq!(
        app.get(app.root(), "completions").unwrap(),
        Value::Number(0.)
    );

    let wide = app.id("wide").unwrap();
    app.set(loader, "sourceComponent", Value::Object(wide))
        .unwrap();
    app.process_deferred(1).unwrap();
    app.set(loader, "asynchronous", Value::Bool(false)).unwrap();
    assert!(!app.has_deferred_work());
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(
        app.get(app.root(), "completions").unwrap(),
        Value::Number(96.)
    );
}

#[test]
fn an_async_completion_exception_rolls_back_before_the_loader_reports_error() {
    let mut app = model(
        r#"import QtQuick
        Item { id: page; property int calls: 0
            Loader { id: slot; asynchronous: true
                sourceComponent: Item {
                    Component.onCompleted: { page.calls++; page.unknownMethod() }
                }
            }
        }"#,
    );
    let loader = app.id("slot").unwrap();
    let before = app.objects().count();
    assert!(app.process_deferred(1).is_err());
    assert_eq!(app.get(app.root(), "calls").unwrap(), Value::Number(0.));
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(3.));
    assert_eq!(app.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(app.objects().count(), before);
    assert!(!app.has_deferred_work());
}

#[test]
fn a_host_can_interrupt_one_loader_before_starting_the_next_queued_job() {
    let mut app = model(
        r#"import QtQuick
        Item { Component { id: tile; Item {width:17} }
            Loader { id: a; asynchronous:true; sourceComponent:tile }
            Loader { id: b; asynchronous:true; sourceComponent:tile }
        }"#,
    );
    let mut probes = 0;
    let ran = app
        .process_deferred_while(10, || {
            probes += 1;
            probes == 1
        })
        .unwrap();
    assert_eq!(ran, 1);
    assert!(app.has_incubating_loader());
    for name in ["a", "b"] {
        assert_eq!(app.get(app.id(name).unwrap(), "item").unwrap(), Value::Null);
    }
    assert_eq!(app.process_deferred(10).unwrap(), 2);
    for name in ["a", "b"] {
        let id = item(&app, app.id(name).unwrap());
        assert_eq!(app.get(id, "width").unwrap(), Value::Number(17.));
    }
    assert!(!app.has_deferred_work());
}

#[test]
fn destroying_a_dynamic_loader_discards_its_prepared_subtree_and_private_ids() {
    let source = format!(
        "import QtQuick\nItem {{ Component {{ id: host;
            Loader {{ asynchronous: true; sourceComponent: Item {{ id: privateRoot; {} }} }}
        }} }}",
        "Item {}\n".repeat(96)
    );
    let mut app = model(&source);
    let before = app.objects().count();
    let loader = app
        .create_object(
            app.id("host").unwrap(),
            Some(app.root()),
            Default::default(),
        )
        .unwrap();
    app.process_deferred(1).unwrap();
    assert!(app.has_incubating_loader());
    app.destroy_object(loader).unwrap();
    assert_eq!(app.objects().count(), before);
    assert!(app.id("privateRoot").is_none());
    assert!(!app.has_deferred_work());
}

#[test]
fn cancelling_inside_an_aborted_transaction_preserves_the_inflight_preparation() {
    let source = format!(
        "import QtQuick\nItem {{ id: page; function cancelThenFail() {{ slot.active = false; page.unknownMethod() }}
            Loader {{ id: slot; asynchronous: true; sourceComponent: Item {{ {} }} }}
        }}", "Item {}\n".repeat(96)
    );
    let mut app = model(&source);
    let loader = app.id("slot").unwrap();
    app.process_deferred(1).unwrap();
    assert!(app.has_incubating_loader());
    assert!(app.call(app.root(), "cancelThenFail", Vec::new()).is_err());
    assert!(app.has_incubating_loader());
    while app.has_deferred_work() {
        app.process_deferred(1).unwrap();
    }
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(app.children(item(&app, loader)).len(), 96);
}

#[test]
fn selecting_an_inline_component_replaces_an_owned_url_factory_during_incubation() {
    let mut app = model(
        r#"import QtQuick
        Item {
            Component { id: fallback; Item { width: 37 } }
            Loader { id: slot; asynchronous: true; source: "Missing.qml" }
        }"#,
    );
    let loader = app.id("slot").unwrap();
    let fallback = app.id("fallback").unwrap();
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(3.));
    let Value::Object(old_factory) = app.get(loader, "sourceComponent").unwrap() else {
        panic!("expected URL factory");
    };
    app.set(loader, "sourceComponent", Value::Object(fallback))
        .unwrap();
    assert_eq!(
        app.get(loader, "sourceComponent").unwrap(),
        Value::Object(fallback)
    );
    assert!(app.type_name(old_factory).is_none());
    assert_eq!(app.process_deferred(1).unwrap(), 1);
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(
        app.get(item(&app, loader), "width").unwrap(),
        Value::Number(37.)
    );
}

#[test]
fn an_error_status_handler_can_replace_a_missing_file_with_an_async_fallback() {
    let mut app = model(
        r#"import QtQuick
        Item {
            Component { id: fallback; Item { width: 19 } }
            Loader { id: slot; asynchronous: true
                onStatusChanged: { if (slot.status === Loader.Error) slot.sourceComponent = fallback }
            }
        }"#,
    );
    let loader = app.id("slot").unwrap();
    app.set(loader, "source", Value::String("Missing.qml".into()))
        .unwrap();
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(2.));
    app.process_deferred(1).unwrap();
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(
        app.get(item(&app, loader), "width").unwrap(),
        Value::Number(19.)
    );
}

#[test]
fn a_loaded_component_captures_delegate_inputs_only_from_its_declaration() {
    let mut app = model(
        r#"import QtQuick
        Item { property int index: 50
            Component { id: outside; Item { property int seen: index + 7 } }
            Repeater { id: local; model: 2
                Loader { asynchronous: true; sourceComponent: Item { property int seen: index + 7 } }
            }
            Repeater { id: external; model: 2
                Loader { asynchronous: true; sourceComponent: outside }
            }
        }"#,
    );
    while app.has_deferred_work() {
        app.process_deferred(8).unwrap();
    }
    for (rep, expected) in [("local", [7., 8.]), ("external", [57., 57.])] {
        for (i, wanted) in expected.into_iter().enumerate() {
            let Value::Object(loader) = app
                .call(
                    app.id(rep).unwrap(),
                    "itemAt",
                    vec![Value::Number(i as f64)],
                )
                .unwrap()
            else {
                panic!("missing delegate");
            };
            assert_eq!(
                app.get(item(&app, loader), "seen").unwrap(),
                Value::Number(wanted)
            );
        }
    }
}

#[test]
fn a_throwing_error_observer_cannot_leave_a_failed_loader_permanently_loading() {
    let mut app = model(
        r#"import QtQuick
        Item { id: page; property int writes: 0
            Loader { id: slot; asynchronous: true
                sourceComponent: Item { Component.onCompleted: page.failActivation() }
                onStatusChanged: { if (slot.status === Loader.Error) { page.writes++; page.failObserver() } }
            }
        }"#,
    );
    let loader = app.id("slot").unwrap();
    let error = app.process_deferred(1).unwrap_err();
    assert_eq!(app.get(loader, "status").unwrap(), Value::Number(3.));
    assert_eq!(app.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(app.get(app.root(), "writes").unwrap(), Value::Number(0.));
    assert!(error.message.contains("failActivation"));
    assert!(error.message.contains("failObserver"));
    assert!(!app.has_deferred_work());
}
