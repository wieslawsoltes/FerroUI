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
