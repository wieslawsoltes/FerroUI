//! Tests of the host. A stand-in application in Rust speaks the protocol of the probe (a count
//! and a note) so that the tree and the reconciler are tested without Osprey; with an application
//! linked (`OSPREY_NATIVE_APP_DIR`, the configuration `osprey_app`) and `OSPREY_NATIVE_APP_NAME=probe`,
//! `the_linked_probe_counts` drives Osprey's own probe through the real boundary and the tree.

use std::rc::Rc;

use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, ContentControl, Control, Panel, TextBlock, TextBox};
use ferroui_view::reactive::Owner;
use serde_json::{json, Value};

use crate::boundary::Boundary;
use crate::host::Host;
use crate::protocol;
use crate::session::Session;
use crate::store::Store;

/// The probe's protocol in Rust: a count in the root region and a note in the region `note`.
struct StandIn;

fn envelope(count: i64, note: &str, theme: bool) -> String {
    let root = json!({"kind": "stack", "axis": "vertical", "children": [
        {"kind": "text", "text": format!("Count {count}"), "role": "body", "tone": "primary"},
        {"kind": "button", "label": "Add", "symbol": "add", "role": "filled", "enabled": true, "event": 0},
        {"kind": "slot", "key": "note"},
    ]});
    let note_node = json!({"kind": "stack", "axis": "vertical", "children": [
        {"kind": "field", "label": "Note", "value": note, "prompt": "Type here", "secure": false,
         "multiline": false, "commit": "edit", "event": 0},
    ]});
    let mut envelope = json!({
        "version": 1,
        "state": json!({"count": count, "note": note}).to_string(),
        "patches": [
            {"key": "root", "revision": count as u64 + 1, "node": root},
            {"key": "note", "revision": 1, "node": note_node},
        ],
        "requests": [],
    });
    if theme {
        envelope["theme"] = json!({"tokens": [{"name": "accent", "light": "#2563EB", "dark": "#60A5FA"}],
            "spacing": [4, 8, 12], "feedback": 120, "settle": 220});
    }
    envelope.to_string()
}

impl Boundary for StandIn {
    fn start(&mut self, _context: &str) -> Result<String, String> {
        Ok(envelope(0, "macos", true))
    }

    fn dispatch(&mut self, state: &str, event: &str) -> Result<String, String> {
        let state: Value = serde_json::from_str(state).map_err(|problem| problem.to_string())?;
        let event: Value = serde_json::from_str(event).map_err(|problem| problem.to_string())?;
        let mut count = state["count"].as_i64().unwrap_or(0);
        let mut note = state["note"].as_str().unwrap_or("").to_owned();
        match (event["region"].as_str(), &event["value"]) {
            (Some("root"), Value::Null) => count += 1,
            (Some("note"), Value::String(text)) => note = text.clone(),
            _ => {}
        }
        Ok(envelope(count, &note, false))
    }
}

fn walk(control: &Ref<Control>, found: &mut Vec<Ref<Control>>) {
    found.push(control.clone());
    if let Some(panel) = control.cast::<Panel>() {
        for child in panel.children().to_vec() {
            walk(&child, found);
        }
    }
    if let Some(border) = control.cast::<ferroui_controls::Border>() {
        if let Some(child) = border.child() {
            walk(&child, found);
        }
    }
    if let Some(content) = control.cast::<ContentControl>() {
        if let Some(child) = content
            .content()
            .and_then(|content: BoxedValue| content.downcast_ref::<Ref<Control>>().cloned())
        {
            walk(&child, found);
        }
    }
}

fn controls(root: &Ref<Control>) -> Vec<Ref<Control>> {
    let mut found = Vec::new();
    walk(root, &mut found);
    found
}

fn texts(root: &Ref<Control>) -> Vec<String> {
    controls(root)
        .iter()
        .filter_map(|control| control.cast::<TextBlock>())
        .filter_map(|text| text.text())
        .collect()
}

fn button_labelled(root: &Ref<Control>, label: &str) -> Ref<Control> {
    controls(root)
        .into_iter()
        .find(|control| {
            control.cast::<Button>().is_some() && texts(control).iter().any(|text| text == label)
        })
        .unwrap_or_else(|| panic!("no button {label} in {:?}", texts(root)))
}

fn click(control: &Ref<Control>) {
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

/// Mounts the tree of a session; the answer keeps the owner alive.
fn mount(session: Session) -> (Rc<Host>, Ref<Control>, Owner) {
    let owner = Owner::new_root();
    let (host, root) = owner.with(|| {
        let host = Host::new(session);
        let root = host.element().mount();
        (host, root)
    });
    (host, root, owner)
}

#[test]
fn a_tap_reaches_the_application_and_the_tree_shows_its_answer() {
    let _thread = Dispatcher::unit_test_scope();
    let session = Session::start(Box::new(StandIn), "{}", Store::none()).expect("start");
    let (_host, root, owner) = mount(session);
    assert!(
        texts(&root).contains(&"Count 0".to_owned()),
        "{:?}",
        texts(&root)
    );
    let add = button_labelled(&root, "Add");
    click(&add);
    assert!(
        texts(&root).contains(&"Count 1".to_owned()),
        "{:?}",
        texts(&root)
    );
    assert!(
        add.ptr_eq(&button_labelled(&root, "Add")),
        "the button keeps its control across the patch"
    );
    owner.dispose();
}

#[test]
fn a_field_keeps_its_control_while_its_text_goes_to_the_application() {
    let _thread = Dispatcher::unit_test_scope();
    let session = Session::start(Box::new(StandIn), "{}", Store::none()).expect("start");
    let (host, root, owner) = mount(session);
    let field = controls(&root)
        .into_iter()
        .find(|control| control.cast::<TextBox>().is_some())
        .expect("a field");
    field.cast::<TextBox>().unwrap().set_text(Some("hello"));
    let state: Value = serde_json::from_str(&host_state(&host)).unwrap();
    assert_eq!(state["note"], "hello", "the edit reached the application");
    let again = controls(&root)
        .into_iter()
        .find(|control| control.cast::<TextBox>().is_some())
        .unwrap();
    assert!(
        field.ptr_eq(&again),
        "the field keeps its control, so its caret and focus stay"
    );
    owner.dispose();
}

fn host_state(host: &Rc<Host>) -> String {
    let session = host.session();
    let note = session.region("note").unwrap();
    // The state is opaque; the stand-in's note is in the field's value too.
    json!({"note": note.node["children"][0]["value"]}).to_string()
}

#[test]
fn an_envelope_with_an_unknown_kind_is_refused_whole() {
    let text = r#"{"version":1,"state":"s","requests":[],"patches":[
        {"key":"root","revision":1,"node":{"kind":"stack","axis":"vertical","children":[{"kind":"marquee"}]}}]}"#;
    let problem = protocol::parse(text).expect_err("refused");
    assert!(problem.contains("unknown node kind marquee"), "{problem}");
}

#[test]
fn a_control_without_its_ordinal_is_refused() {
    let text = r#"{"version":1,"state":"s","requests":[],"patches":[
        {"key":"root","revision":1,"node":{"kind":"button","label":"Go","symbol":"","role":"plain","enabled":true}}]}"#;
    let problem = protocol::parse(text).expect_err("refused");
    assert!(problem.contains("no event"), "{problem}");
}

#[test]
fn another_protocol_version_is_refused() {
    let problem = protocol::parse(r#"{"version":2,"state":"","patches":[],"requests":[]}"#)
        .expect_err("refused");
    assert!(problem.contains("version 2"), "{problem}");
}

#[test]
fn a_malformed_scene_primitive_refuses_the_envelope() {
    let scene = |shapes: &str| {
        format!(
            r#"{{"version":1,"state":"","requests":[],"patches":[{{"key":"root","revision":1,"node":
            {{"kind":"scene","width":10.0,"height":10.0,"zoom":1.0,"atlas":[],"shapes":{shapes},
            "streamed":0,"event":0}}}}]}}"#
        )
    };
    assert!(protocol::parse(&scene("[[1,5.0,5.0,2.0,4278190335,0]]")).is_ok());
    for bad in [
        "[[1,5.0,5.0,2.0,4278190335]]",
        "[[9,1.0]]",
        "[[1,5.0,5.0,\"r\",1,0]]",
        "[[6,\"a\",0,0,9,1,0,7,400]]",
    ] {
        assert!(protocol::parse(&scene(bad)).is_err(), "{bad} was accepted");
    }
}

#[test]
fn scene_primitives_decode_in_depth_order() {
    let node = json!({"kind": "scene", "width": 100.0, "height": 50.0, "zoom": 1.0, "atlas": [],
    "streamed": 0, "event": 0,
    "shapes": [
        [1, 5.0, 5.0, 2.0, 0xFF0000FFi64, 3],
        [0, 10.0, 10.0, 4.0, 2.0, 0.0, 0x00FF00FFi64, 1],
        [2, 2, 0.0, 0.0, 10.0, 10.0, 0x0000FFFFi64, 1.5, 2],
        [6, "Hi", 1.0, 2.0, 12.0, 0x000000FFi64, 0, 1, 600.0],
        [5, [[0, 0.0, 0.0], [1, 4.0, 0.0], [6]], [1, 2.0, 0], [0, [0, 1.0, 0.0, 0.0, 1.0]], [], 4]
    ]});
    let scene = crate::scene::decode(&node, &[]).expect("decodes");
    let depths: Vec<i64> = scene.shapes.iter().map(|shape| shape.depth).collect();
    assert_eq!(depths, vec![0, 1, 2, 3, 4]);
    assert!(matches!(
        scene.shapes[0].primitive,
        crate::scene::Primitive::Lettering { anchor: 1, .. }
    ));
}

#[test]
fn a_capability_this_host_does_not_answer_completes_with_an_error_naming_it() {
    struct Asks;
    impl Boundary for Asks {
        fn start(&mut self, _: &str) -> Result<String, String> {
            Ok(r#"{"version":1,"state":"0","patches":[],"requests":[{"ticket":"t1","kind":"http","url":"x"},
                {"ticket":"t2","kind":"animate","on":true}]}"#
                .to_owned())
        }
        fn dispatch(&mut self, _: &str, event: &str) -> Result<String, String> {
            let event: Value = serde_json::from_str(event).unwrap();
            let ok = event["ok"].as_bool().unwrap();
            let error = event["error"].as_str().unwrap_or("").to_owned();
            Ok(json!({"version": 1, "state": format!("{ok}:{error}"), "patches": [], "requests": []}).to_string())
        }
    }
    let session = Session::start(Box::new(Asks), "{}", Store::none()).unwrap();
    assert!(session
        .diagnostics()
        .iter()
        .any(|message| message.contains("unsupported capability http")));
    assert!(session.animating(), "animate turns the frame clock on");
}

/// Osprey's probe, linked: the real boundary, then the same tap through the FerroUI tree.
#[cfg(osprey_app)]
#[test]
fn the_linked_probe_counts() {
    if std::env::var("OSPREY_NATIVE_APP_NAME").as_deref() != Ok("probe") {
        eprintln!(
            "the linked application is not the probe (OSPREY_NATIVE_APP_NAME=probe); skipped"
        );
        return;
    }
    let context = crate::launch_context(&[]);
    // One test drives the library: every export is called from one thread.
    crate::smoke::probe(
        Box::new(crate::boundary::Linked::new().expect("linked")),
        &context,
    )
    .expect("boundary smoke");
    let session = Session::start(
        Box::new(crate::boundary::Linked::new().unwrap()),
        &context,
        Store::none(),
    )
    .expect("start");
    let _thread = Dispatcher::unit_test_scope();
    let (_host, root, owner) = mount(session);
    assert!(
        texts(&root).contains(&"Count 0".to_owned()),
        "{:?}",
        texts(&root)
    );
    click(&button_labelled(&root, "Add"));
    assert!(
        texts(&root).contains(&"Count 1".to_owned()),
        "{:?}",
        texts(&root)
    );
    owner.dispose();
}

/// A linked application's recorded smoke script (`OSPREY_NATIVE_SMOKE` names its `smoke.jsonl`),
/// replayed through the real boundary: Osprey's flock and painting examples ship one.
#[cfg(osprey_app)]
#[test]
fn the_linked_application_replays_its_smoke_script() {
    let Some(path) = std::env::var_os("OSPREY_NATIVE_SMOKE") else {
        eprintln!("OSPREY_NATIVE_SMOKE names no smoke.jsonl; skipped");
        return;
    };
    let script = std::fs::read_to_string(&path).expect("the smoke script");
    let context = crate::launch_context(&[]);
    let steps = crate::smoke::replay(
        Box::new(crate::boundary::Linked::new().expect("linked")),
        &context,
        &script,
    )
    .expect("the smoke script passes");
    assert!(steps > 0);
}
