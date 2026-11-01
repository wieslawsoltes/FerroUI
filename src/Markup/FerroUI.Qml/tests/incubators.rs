use ferroui_qml::{
    ir::parse, IncubationMode, IncubationStatus, Incubator, InitialProperty, Instance,
    TypeRegistry, Value,
};
use std::collections::BTreeMap;

fn instance(source: &str) -> Instance {
    Instance::new(
        &parse(source).unwrap(),
        &TypeRegistry::new(),
        Default::default(),
    )
    .unwrap()
}

#[test]
fn pending_handles_keep_no_live_subtree_and_drop_cancelled_initial_values() {
    let source = format!("import QtQml\nQtObject {{ property Component factory: Component {{ QtObject {{ required property int amount; {} }} }} }}",
        (0..100).map(|index| format!("property int p{index}: {index}\n")).collect::<String>());
    let mut app = instance(&source);
    let root = app.root();
    let Value::Object(factory) = app.get(root, "factory").unwrap() else {
        panic!("factory")
    };
    let make = |app: &mut Instance| {
        app.incubate_object(
            factory,
            Some(root),
            BTreeMap::from([("amount".into(), InitialProperty::Value(Value::Number(8.)))]),
            IncubationMode::Asynchronous,
        )
        .unwrap()
        .unwrap()
    };
    let original = app.objects().count();
    let a = make(&mut app);
    let b = make(&mut app);
    assert_eq!(app.process_deferred(4).unwrap(), 2);
    assert!(app.has_incubating_work());
    assert_eq!(a.status(&app).unwrap(), IncubationStatus::Loading);
    assert_eq!(a.object(&app).unwrap(), None);
    assert_eq!(app.objects().count(), original);
    a.cancel(&mut app).unwrap();
    assert_eq!(a.status(&app).unwrap(), IncubationStatus::Error);
    while app.has_deferred_work() {
        app.process_deferred(4).unwrap();
    }
    let id = b.object(&app).unwrap().unwrap();
    assert_eq!(b.status(&app).unwrap(), IncubationStatus::Ready);
    assert_eq!(app.get(id, "amount").unwrap(), Value::Number(8.));
    assert_eq!(app.objects().count(), original + 1);
    b.force_completion(&mut app).unwrap();
    app.destroy_object(id).unwrap();
    assert_eq!(b.object(&app).unwrap(), None);
    assert!(app
        .set_record_property(b.record(), "status", Value::Number(2.))
        .is_err());
    let mut other = instance("import QtQml\nQtObject {}");
    assert!(b.force_completion(&mut other).is_err());
}

#[test]
fn force_completion_inside_a_failed_caller_restores_the_pending_job() {
    let mut app = instance(
        r#"import QtQml
        QtObject { id: root; property var job
            property Component factory: Component {QtObject {property int idValue: 23}}
            function start() {root.job = root.factory.incubateObject(root)}
            function bad() {root.job.forceCompletion(); root.unknown()}
        }"#,
    );
    app.call(app.root(), "start", vec![]).unwrap();
    let Value::Record(record) = app.get(app.root(), "job").unwrap() else {
        panic!("incubator")
    };
    let job = Incubator::from_record(record).unwrap();
    let before = app.objects().count();
    assert!(app.call(app.root(), "bad", vec![]).is_err());
    assert_eq!(app.objects().count(), before);
    assert!(app.has_deferred_work());
    assert_eq!(job.status(&app).unwrap(), IncubationStatus::Loading);
    app.process_deferred(4).unwrap();
    assert_eq!(job.status(&app).unwrap(), IncubationStatus::Ready);
}

#[test]
fn completion_handlers_can_create_a_synchronous_incubator_immediately() {
    let mut app = instance(
        r#"import QtQml
        QtObject {id: root; property var child; property int nestedStatus: -1
            property Component leaf: Component {QtObject {}}
            property Component outer: Component {QtObject {Component.onCompleted: {
                root.child = root.leaf.incubateObject(root,{},Qt.Synchronous)
                root.nestedStatus = root.child.status
            }}}
            function begin() {root.outer.incubateObject(root)}
        }"#,
    );
    app.call(app.root(), "begin", vec![]).unwrap();
    assert_eq!(app.process_deferred(4).unwrap(), 1);
    assert_eq!(
        app.get(app.root(), "nestedStatus").unwrap(),
        Value::Number(1.)
    );
    let Value::Record(r) = app.get(app.root(), "child").unwrap() else {
        panic!("child")
    };
    let child = Incubator::from_record(r).unwrap();
    assert!(!app.has_deferred_work());
    assert_eq!(child.status(&app).unwrap(), IncubationStatus::Ready);
}

#[test]
fn callback_failure_preserves_the_activated_tree_and_rolls_back_callback_writes() {
    let mut app = instance(
        r#"import QtQml
        QtObject {id: root; property var job; property int calls:0
            property Component part: Component {QtObject {}}
            function fail(status:int) {root.calls++; missing()}
            function start() {root.job = root.part.incubateObject(root); root.job.onStatusChanged = root.fail}
        }"#,
    );
    app.call(app.root(), "start", vec![]).unwrap();
    assert!(app.process_deferred(1).is_err());
    let Value::Record(r) = app.get(app.root(), "job").unwrap() else {
        panic!("job")
    };
    let job = Incubator::from_record(r).unwrap();
    assert!(job.object(&app).unwrap().is_some());
    assert_eq!(job.status(&app).unwrap(), IncubationStatus::Ready);
    assert_eq!(app.get(app.root(), "calls").unwrap(), Value::Number(0.));
    assert!(!app.has_deferred_work());
}

#[test]
fn the_requested_parent_is_weak_while_creation_is_pending() {
    let mut app = instance(
        r#"import QtQml
        QtObject { id: root; property Component part: Component {QtObject {}}}
    "#,
    );
    let Value::Object(part) = app.get(app.root(), "part").unwrap() else {
        panic!("factory")
    };
    let owner = app
        .create_object(part, Some(app.root()), Default::default())
        .unwrap();
    let job = app
        .incubate_object(
            part,
            Some(owner),
            Default::default(),
            IncubationMode::Asynchronous,
        )
        .unwrap()
        .unwrap();
    app.destroy_object(owner).unwrap();
    app.process_deferred(8).unwrap();
    assert_eq!(job.status(&app).unwrap(), IncubationStatus::Ready);
    assert_eq!(app.parent(job.object(&app).unwrap().unwrap()), None);
}
