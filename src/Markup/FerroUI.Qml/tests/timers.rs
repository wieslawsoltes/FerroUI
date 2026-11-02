use ferroui_qml::{ir, Instance, TypeRegistry, Value};
use std::{collections::BTreeMap, time::Duration};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn timer_methods_coalesce_overruns_and_property_changes_reset_elapsed_time() {
    let doc=ir::parse("import QtQml\nTimer { id: t; interval: 30; triggeredOnStart: true; property int count: 0; onTriggered: count++ }").unwrap();
    let mut engine = Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
    let t = engine.root();
    engine.call(t, "start", vec![]).unwrap();
    assert_eq!(engine.get(t, "count").unwrap(), Value::Number(1.));
    engine.advance_time(ms(10)).unwrap();
    assert_eq!(engine.next_timer_deadline(), Some(ms(20)));
    engine.call(t, "start", vec![]).unwrap();
    assert_eq!(engine.next_timer_deadline(), Some(ms(20)));
    engine.set(t, "interval", Value::Number(40.)).unwrap();
    assert_eq!(engine.next_timer_deadline(), Some(ms(40)));
    engine.advance_time(ms(100)).unwrap();
    assert_eq!(engine.next_timer_deadline(), None);
    assert_eq!(engine.get(t, "count").unwrap(), Value::Number(2.));
    engine.set(t, "repeat", Value::Bool(true)).unwrap();
    engine.call(t, "restart", vec![]).unwrap();
    engine.advance_time(ms(170)).unwrap();
    assert_eq!(engine.get(t, "count").unwrap(), Value::Number(4.));
    assert_eq!(engine.next_timer_deadline(), Some(ms(40)));
    engine.call(t, "stop", vec![]).unwrap();
    assert_eq!(engine.next_timer_deadline(), None);
}

#[test]
fn failed_timeout_restores_running_value_deadline_and_clock() {
    let doc=ir::parse("import QtQml\nTimer { interval: 5; running: true; property bool fail: true; property int count: 0; onTriggered: { count++; if (fail) { unknown = 2 } } }").unwrap();
    let mut engine = Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
    let t = engine.root();
    assert!(engine.advance_time(ms(5)).is_err());
    assert_eq!(engine.get(t, "count").unwrap(), Value::Number(0.));
    assert_eq!(engine.get(t, "running").unwrap(), Value::Bool(true));
    assert_eq!(engine.next_timer_deadline(), Some(ms(5)));
    engine.set(t, "fail", Value::Bool(false)).unwrap();
    engine.advance_time(ms(5)).unwrap();
    assert_eq!(engine.get(t, "count").unwrap(), Value::Number(1.));
    assert_eq!(engine.next_timer_deadline(), None);
}

#[test]
fn overdue_timers_keep_creation_order_when_an_earlier_callback_reschedules_a_sibling() {
    let doc = ir::parse(r#"import QtQml
        QtObject { id: root; property string fired: ""
            property Timer first: Timer { running: true; interval: 10; onTriggered: { root.fired += "a"; root.second.restart() } }
            property Timer second: Timer { running: true; interval: 3; onTriggered: root.fired += "b" }
        }"#).unwrap();
    let mut ui = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
    ui.advance_time(ms(12)).unwrap();
    assert_eq!(
        ui.get(ui.root(), "fired").unwrap(),
        Value::String("a".into())
    );
    assert_eq!(ui.next_timer_deadline(), Some(ms(3)));
    ui.advance_time(ms(3)).unwrap();
    assert_eq!(
        ui.get(ui.root(), "fired").unwrap(),
        Value::String("ab".into())
    );
}

#[test]
fn failed_animation_completion_restores_driver_index_and_ticking_state() {
    let mut types = TypeRegistry::new();
    types.register_quick_states().unwrap();
    let doc=ir::parse(r#"import QtQml
        import QtQuick
        QtObject { id: root; property real x:0; property bool fail:true
            property NumberAnimation motion: NumberAnimation { id: moveX; target: root; property:"x"; from:0; to:10; duration:16; onFinished: { if(root.fail) { missing() } } }
        }"#).unwrap();
    let mut ui = Instance::new(&doc, &types, Default::default()).unwrap();
    let motion = ui.id("moveX").unwrap();
    ui.call(motion, "start", vec![]).unwrap();
    assert!(ui.advance_time(ms(16)).is_err());
    assert_eq!(ui.next_timer_deadline(), Some(ms(16)));
    ui.call(motion, "pause", vec![]).unwrap();
    assert_eq!(ui.next_timer_deadline(), None);
    ui.call(motion, "resume", vec![]).unwrap();
    ui.set(ui.root(), "fail", Value::Bool(false)).unwrap();
    ui.advance_time(ms(16)).unwrap();
    assert_eq!(ui.get(ui.root(), "x").unwrap(), Value::Number(10.));
    assert_eq!(ui.next_timer_deadline(), None);
}
