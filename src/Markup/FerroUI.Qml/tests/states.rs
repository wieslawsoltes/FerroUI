use ferroui_qml::{ir, Instance, TypeDefinition, TypeRegistry, Value};
use std::collections::BTreeMap;

fn model(body: &str) -> Instance {
    let mut registry = TypeRegistry::new();
    registry.register_quick_states().unwrap();
    registry
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .with_quick_states()
                .property("x", "real", Value::Number(0.))
                .property("data", "list<QtObject>", Value::List(vec![]))
                .default_property("data"),
        )
        .unwrap();
    registry.reexport("QtQuick", "QtQml").unwrap();
    let doc = ir::parse(&format!("import QtQuick\nItem {{ id: root; {body} }}")).unwrap();
    Instance::new(&doc, &registry, BTreeMap::new()).unwrap()
}
#[test]
fn named_automatic_and_extended_states_restore_bindings_and_respect_target_scope() {
    let mut m = model(
        r#"
        property int n: 2; property bool active: false
        Item { id: target; property int n: 5; x: root.n * 3 }
        states: [
          State { name: "one"; PropertyChanges { target: target; x: n + 1 } },
          State { name: "two"; when: root.active; extend: "one"; PropertyChanges { target { x: n + 9 } } }
        ]
    "#,
    );
    let r = m.root();
    let target = m.id("target").unwrap();
    let get = |m: &Instance| m.get(target, "x").unwrap();
    m.set(r, "state", Value::String("one".into())).unwrap();
    assert_eq!(get(&m), Value::Number(6.));
    m.set(r, "active", Value::Bool(true)).unwrap();
    assert_eq!(m.get(r, "state").unwrap(), Value::String("two".into()));
    assert_eq!(get(&m), Value::Number(11.));
    m.set(r, "n", Value::Number(9.)).unwrap();
    assert_eq!(get(&m), Value::Number(18.));
    m.set(r, "active", Value::Bool(false)).unwrap();
    assert_eq!(get(&m), Value::Number(27.));
}

#[test]
fn explicit_values_are_sampled_once_and_failed_state_entries_roll_back() {
    let mut m = model(
        r#"
        property int n: 2
        states: [
          State { name: "snapshot"; PropertyChanges { target: root; explicit: true; restoreEntryValues: false; x: n } },
          State { name: "bad"; PropertyChanges { root.x: "invalid" } }
        ]
    "#,
    );
    let r = m.root();
    m.set(r, "state", Value::String("snapshot".into())).unwrap();
    m.set(r, "n", Value::Number(8.)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(2.));
    assert!(m.set(r, "state", Value::String("bad".into())).is_err());
    assert_eq!(m.get(r, "state").unwrap(), Value::String("snapshot".into()));
    m.set(r, "state", Value::String(String::new())).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(2.));
}

#[test]
fn behavior_retargets_binding_from_current_sample_and_retires_at_completion() {
    use std::time::Duration;
    let mut m = model(
        r#"
        property real destination: 10; x: destination
        Behavior on x { id: behavior; NumberAnimation { id: easing; duration: 100; easing.type: Easing.InQuad } }
    "#,
    );
    let r = m.root();
    let driver = m.id("easing").unwrap();
    let get = |m: &Instance| m.get(r, "x").unwrap();
    m.set(r, "destination", Value::Number(110.)).unwrap();
    assert_eq!(get(&m), Value::Number(10.));
    assert_eq!(m.get(driver, "running").unwrap(), Value::Bool(true));
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(get(&m), Value::Number(35.));
    m.set(r, "destination", Value::Number(135.)).unwrap();
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(get(&m), Value::Number(60.));
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(get(&m), Value::Number(135.));
    assert_eq!(m.get(driver, "running").unwrap(), Value::Bool(false));
    assert!(m.next_timer_deadline().is_none());
}

#[test]
fn state_transition_precedes_behavior_and_reverses_from_the_sampled_position() {
    use std::time::Duration;
    let mut m = model(
        r#"
        x: 3; property real dest: 103
        Behavior on x { NumberAnimation { duration: 1000 } }
        states: State { name: "wide"; PropertyChanges { root.x: root.dest } }
        transitions: Transition { id: tr; from: ""; to: "wide"; reversible: true
            NumberAnimation { properties: "x"; duration: 100 } }
    "#,
    );
    let r = m.root();
    let tr = m.id("tr").unwrap();
    m.set(r, "state", Value::String("wide".into())).unwrap();
    assert_eq!(m.get(tr, "running").unwrap(), Value::Bool(true));
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(53.));
    m.set(r, "state", Value::String(String::new())).unwrap();
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(28.));
    m.advance_time(Duration::from_millis(100)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(3.));
    assert_eq!(m.get(tr, "running").unwrap(), Value::Bool(false));
}

#[test]
fn number_animation_as_a_value_source_runs_loops_on_the_owner_clock() {
    use std::time::Duration;
    let mut m = model(
        r#"
        property int completions: 0
        NumberAnimation on x { id: anim; from: 10; to: 30; duration: 100; loops: 2; onFinished: root.completions++ }
    "#,
    );
    let r = m.root();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(10.));
    m.advance_time(Duration::from_millis(150)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(20.));
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(30.));
    assert_eq!(m.get(r, "completions").unwrap(), Value::Number(1.));
}

#[test]
fn pause_resume_restart_and_complete_preserve_the_sampled_time() {
    use std::time::Duration;
    let mut m = model(r#"NumberAnimation on x { id: driver; from:10; to:110; duration:100 }"#);
    let r = m.root();
    let driver = m.id("driver").unwrap();
    m.advance_time(Duration::from_millis(25)).unwrap();
    m.call(driver, "pause", vec![]).unwrap();
    assert!(m.next_timer_deadline().is_none());
    m.advance_time(Duration::from_secs(1)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(35.));
    m.call(driver, "resume", vec![]).unwrap();
    m.advance_time(Duration::from_millis(25)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(60.));
    m.call(driver, "complete", vec![]).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(110.));
    m.call(driver, "restart", vec![]).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(10.));
    m.call(driver, "stop", vec![]).unwrap();
    assert!(m.next_timer_deadline().is_none());
}

#[test]
fn failed_animation_tick_rolls_back_clock_output_and_transition_ownership() {
    use std::time::Duration;
    let mut m = model(
        r#"
        property bool fail: false
        x: 0
        states: State { name:"move"; PropertyChanges { root.x: 10 } }
        transitions: Transition { id: tr; NumberAnimation { properties:"x"; duration:100 } }
        onXChanged: { if (fail) root.missing() }
    "#,
    );
    let r = m.root();
    let tr = m.id("tr").unwrap();
    m.set(r, "state", Value::String("move".into())).unwrap();
    m.set(r, "fail", Value::Bool(true)).unwrap();
    assert!(m.advance_time(Duration::from_millis(100)).is_err());
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(0.));
    assert_eq!(m.get(tr, "running").unwrap(), Value::Bool(true));
    m.set(r, "fail", Value::Bool(false)).unwrap();
    m.advance_time(Duration::from_millis(40)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(4.));
}

#[test]
fn retained_state_expression_is_still_live_and_can_be_replaced_after_exit() {
    let mut m = model(
        r#"
        property real n: 2
        states: State { name:"retain"; PropertyChanges { target: root; restoreEntryValues:false; x: n * 2 } }
    "#,
    );
    let r = m.root();
    m.set(r, "state", Value::String("retain".into())).unwrap();
    m.set(r, "state", Value::String(String::new())).unwrap();
    m.set(r, "n", Value::Number(12.)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(24.));
    m.set(r, "x", Value::Number(7.)).unwrap();
    m.set(r, "n", Value::Number(13.)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(7.));
}

#[test]
fn reversible_transition_reverses_asymmetric_easing() {
    use std::time::Duration;
    let mut m = model(
        r#"
        states: State { name:"move"; PropertyChanges { root.x: 100 } }
        transitions: Transition { from:""; to:"move"; reversible:true
            NumberAnimation { properties:"x"; duration:100; easing.type:Easing.InQuad } }
    "#,
    );
    let r = m.root();
    m.set(r, "state", Value::String("move".into())).unwrap();
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(25.));
    m.advance_time(Duration::from_millis(50)).unwrap();
    m.set(r, "state", Value::String(String::new())).unwrap();
    m.advance_time(Duration::from_millis(50)).unwrap();
    assert_eq!(m.get(r, "x").unwrap(), Value::Number(25.));
}

#[test]
fn nested_behavior_waits_then_retargets_color_and_preserves_alpha() {
    use std::time::Duration;
    let mut m = model(
        r##"
        property color goal: "#40100000"; property color accent: goal
        Behavior on accent { SequentialAnimation {
            PauseAnimation { duration: 40 }
            ColorAnimation { duration: 60 }
        } }
    "##,
    );
    let r = m.root();
    m.set(r, "goal", Value::String("#c0306080".into())).unwrap();
    m.advance_time(Duration::from_millis(30)).unwrap();
    assert_eq!(
        m.get(r, "accent").unwrap(),
        Value::String("#40100000".into())
    );
    m.advance_time(Duration::from_millis(40)).unwrap();
    assert_eq!(
        m.get(r, "accent").unwrap(),
        Value::String("#80203040".into())
    );
    m.set(r, "goal", Value::String("red".into())).unwrap();
    m.advance_time(Duration::from_millis(100)).unwrap();
    assert_eq!(m.get(r, "accent").unwrap(), Value::String("red".into()));
}

#[test]
fn nested_reversible_color_transition_waits_and_finishes_from_its_current_sample() {
    use std::time::Duration;
    let mut m = model(
        r##"
        property color accent: "#000000"
        states: State { name:"bright"; PropertyChanges { root.accent:"#ffffff" } }
        transitions: Transition { from:""; to:"bright"; reversible:true
            SequentialAnimation { PauseAnimation {duration:20}; ColorAnimation { properties:"accent"; duration:80 } }
        }
    "##,
    );
    let r = m.root();
    m.set(r, "state", Value::String("bright".into())).unwrap();
    m.advance_time(Duration::from_millis(60)).unwrap();
    assert_eq!(m.get(r, "accent").unwrap(), Value::String("#7f7f7f".into()));
    m.set(r, "state", Value::String(String::new())).unwrap();
    m.advance_time(Duration::from_millis(40)).unwrap();
    assert_eq!(m.get(r, "accent").unwrap(), Value::String("#3f3f3f".into()));
    m.advance_time(Duration::from_millis(60)).unwrap();
    assert_eq!(m.get(r, "accent").unwrap(), Value::String("#000000".into()));
}

#[test]
fn pause_only_and_nested_loops_complete_without_flattening_repetitions() {
    use std::time::Duration;
    let mut m = model(
        r#"
        property int done:0
        SequentialAnimation { id: waitGroup; loops:2; onFinished: root.done++
            PauseAnimation { duration:30; loops:3 }
        }
    "#,
    );
    let r = m.root();
    let id = m.id("waitGroup").unwrap();
    m.call(id, "start", vec![]).unwrap();
    m.advance_time(Duration::from_millis(179)).unwrap();
    assert_eq!(m.get(r, "done").unwrap(), Value::Number(0.));
    m.advance_time(Duration::from_millis(1)).unwrap();
    assert_eq!(m.get(r, "done").unwrap(), Value::Number(1.));
    assert!(!m.has_timers());
}
