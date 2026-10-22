use super::*;
use ferroui_base::input::InputManager;
use ferroui_base::{Ref, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Panel, TextBox, TopLevel, Window};
use serde_json::{json, Value};
use std::rc::Rc;
use std::sync::Mutex;
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn fixture() -> (
    UnitTestApplicationScope,
    Ref<Window>,
    Ref<Panel>,
    Ref<Button>,
    Ref<TextBox>,
) {
    let mut services = TestServices::styled_window();
    let focus = TestServices::focusable_window();
    services.input_manager = Some(Rc::new(InputManager::new()));
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    let app = UnitTestApplication::start(services);
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    let panel = Panel::new();
    panel.set_name(Some("container".into()));
    let button = Button::new();
    button.set_name(Some("clickMe".into()));
    button.classes().add("action");
    button.set_width(120.0);
    button.set_height(40.0);
    let text = TextBox::new();
    text.set_name(Some("editor".into()));
    text.set_text(Some("initial"));
    panel.children().add(button.clone());
    panel.children().add(text.clone());
    window.set_content(Some(Rc::new(panel.clone())));
    window.show();
    window.measure(Size::new(400.0, 300.0));
    window.arrange(ferroui_base::Rect::new(0.0, 0.0, 400.0, 300.0));
    (app, window, panel, button, text)
}
fn command(session: &mut CdpSession, method: &str, params: Value) -> Value {
    let messages = session.handle(&json!({"id":1,"method":method,"params":params}).to_string());
    assert!(
        messages[0].get("error").is_none(),
        "{method}: {}",
        messages[0]
    );
    messages[0]["result"].clone()
}
fn find(session: &mut CdpSession, selector: &str) -> u64 {
    command(
        session,
        "DOM.querySelector",
        json!({"nodeId":1,"selector":selector}),
    )["nodeId"]
        .as_u64()
        .filter(|id| *id > 0)
        .unwrap()
}

#[test]
fn trees_selectors_and_stale_ids() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, panel, button, _) = fixture();
    let mut session = CdpSession::new(&window.upcast::<TopLevel>(), "1");
    let doc = command(&mut session, "DOM.getDocument", json!({"depth":-1}));
    assert_eq!(doc["root"]["nodeType"], 9);
    let id = find(&mut session, "#clickMe");
    assert_eq!(find(&mut session, "Button.action"), id);
    assert_eq!(find(&mut session, "Panel > Button"), id);
    command(&mut session, "DOM.getDocument", json!({"tree":"logical"}));
    assert_eq!(find(&mut session, "#clickMe"), id);
    panel.children().remove(&button);
    let messages = session
        .handle(&json!({"id":2,"method":"DOM.getAttributes","params":{"nodeId":id}}).to_string());
    assert_eq!(messages[0]["error"]["code"], -32000);
    panel.children().add(button);
    assert_ne!(find(&mut session, "#clickMe"), id);
}

#[test]
fn edits_styles_and_box_model_are_live() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, button, _) = fixture();
    let mut session = CdpSession::new(&window.upcast::<TopLevel>(), "1");
    let id = find(&mut session, "#clickMe");
    command(
        &mut session,
        "DOM.setAttributeValue",
        json!({"nodeId":id,"name":"Width","value":"150"}),
    );
    assert_eq!(button.width(), 150.0);
    let computed = command(
        &mut session,
        "CSS.getComputedStyleForNode",
        json!({"nodeId":id}),
    );
    assert!(computed["computedStyle"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == "Width" && p["value"] == "150"));
    let styles = command(
        &mut session,
        "CSS.getMatchedStylesForNode",
        json!({"nodeId":id}),
    );
    assert!(styles["matchedCSSRules"].is_array());
    let model = command(&mut session, "DOM.getBoxModel", json!({"nodeId":id}));
    assert_eq!(model["model"]["border"].as_array().unwrap().len(), 8);
    let messages=session.handle(&json!({"id":2,"method":"DOM.setAttributeValue","params":{"nodeId":id,"name":"Width","value":"invalid"}}).to_string());
    assert!(messages[0].get("error").is_some());
    assert_eq!(button.width(), 150.0);
}

#[test]
fn text_input_and_accessibility_use_framework_services() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, _, text) = fixture();
    let mut session = CdpSession::new(&window.upcast::<TopLevel>(), "1");
    let id = find(&mut session, "#editor");
    command(&mut session, "DOM.focus", json!({"nodeId":id}));
    text.set_selection_start(0);
    text.set_selection_end(7);
    command(&mut session, "Input.insertText", json!({"text":"CDP text"}));
    assert_eq!(text.text().as_deref(), Some("CDP text"));
    let attrs = command(&mut session, "DOM.getAttributes", json!({"nodeId":id}));
    assert!(attrs["attributes"]
        .as_array()
        .unwrap()
        .windows(2)
        .any(|p| p[0] == "Text" && p[1] == "CDP text"));
    let ax = command(&mut session, "Accessibility.getFullAXTree", json!({}));
    assert!(!ax["nodes"].as_array().unwrap().is_empty());
    let events = session.handle(
        &json!({"id":3,"method":"Runtime.evaluate","params":{"expression":"1+1"}}).to_string(),
    );
    assert_eq!(events[0]["error"]["code"], -32000);
}

#[test]
fn document_invalidation_and_protocol_errors() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, panel, button, _) = fixture();
    let mut session = CdpSession::new(&window.upcast::<TopLevel>(), "1");
    command(&mut session, "DOM.enable", json!({}));
    command(&mut session, "DOM.getDocument", json!({}));
    assert!(session.poll().is_empty());
    button.classes().add("changed");
    let events = session.poll();
    assert!(events
        .iter()
        .any(|e| e["method"] == "DOM.attributeModified" && e["params"]["name"] == "class"));
    assert!(!events.iter().any(|e| e["method"] == "DOM.documentUpdated"));
    panel.children().add(Button::new());
    assert!(session
        .poll()
        .iter()
        .any(|e| e["method"] == "DOM.documentUpdated"));
    assert_eq!(session.handle("{")[0]["error"]["code"], -32700);
    assert_eq!(session.handle("{}")[0]["error"]["code"], -32600);
    assert_eq!(
        session.handle(r#"{"id":1,"method":"Missing.method"}"#)[0]["error"]["code"],
        -32601
    );
    assert_eq!(
        session.handle(r#"{"id":2,"method":"DOM.getDocument","params":{"depth":-2}}"#)[0]["error"]
            ["code"],
        -32602
    );
}

#[test]
fn targets_flattened_sessions_and_closed_windows() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, _, _) = fixture();
    let mut browser = Browser::new();
    let target = browser.register(&window.clone().upcast());
    let connection = browser.connect(None).unwrap();
    let messages=browser.handle(connection,&json!({"id":1,"method":"Target.attachToTarget","params":{"targetId":target,"flatten":true}}).to_string());
    let session = messages[0]["result"]["sessionId"].as_str().unwrap();
    assert_eq!(messages[1]["method"], "Target.attachedToTarget");
    let messages = browser.handle(
        connection,
        &json!({"id":2,"sessionId":session,"method":"DOM.getDocument"}).to_string(),
    );
    assert_eq!(messages[0]["sessionId"], session);
    assert_eq!(messages[0]["result"]["root"]["nodeId"], 1);
    window.close();
    let messages = browser.poll(connection);
    assert!(messages
        .iter()
        .any(|e| e["method"] == "Target.detachedFromTarget"));
    assert_eq!(browser.discovery(9222), json!([]));
    browser.disconnect(connection);
}

#[test]
fn log_capture_forwards_and_restores_previous_sink() {
    use ferroui_base::logging::{LogEventLevel, Logger};
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, _, _) = fixture();
    let previous = Logger::sink();
    {
        let mut browser = Browser::new();
        let target = browser.register(&window.upcast());
        let connection = browser.connect(Some(&target)).unwrap();
        browser.handle(connection, r#"{"id":1,"method":"Log.enable"}"#);
        Logger::try_get(LogEventLevel::Warning, "CDP-test")
            .unwrap()
            .log(None, "hello");
        assert!(browser
            .poll(connection)
            .iter()
            .any(|e| e["method"] == "Log.entryAdded"
                && e["params"]["entry"]["text"] == "[CDP-test] hello"));
    }
    assert_eq!(Logger::sink().is_some(), previous.is_some());
}

#[test]
fn browser_sessions_route_attachment_events_to_the_requesting_client() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, _, _) = fixture();
    let mut browser = Browser::new();
    let target = browser.register(&window.upcast());
    let connection = browser.connect(None).unwrap();
    let result = browser.handle(
        connection,
        r#"{"id":1,"method":"Target.attachToBrowserTarget"}"#,
    );
    let parent = result[0]["result"]["sessionId"].as_str().unwrap();
    let result = browser.handle(connection, &json!({"id":2,"sessionId":parent,"method":"Target.attachToTarget","params":{"targetId":target,"flatten":true}}).to_string());
    assert_eq!(result[0]["sessionId"], parent);
    assert_eq!(result[1]["sessionId"], parent);
    let child = result[0]["result"]["sessionId"].as_str().unwrap();
    let result = browser.handle(
        connection,
        &json!({"id":3,"sessionId":child,"method":"DOM.getDocument"}).to_string(),
    );
    assert_eq!(result[0]["result"]["root"]["nodeType"], 9);
    let result = browser.handle(
        connection,
        r#"{"id":4,"method":"Target.setAutoAttach","params":{"autoAttach":true,"flatten":false}}"#,
    );
    assert_eq!(result[0]["error"]["code"], -32602);
    assert!(browser.poll(connection).is_empty());
}

#[test]
fn multiple_endpoints_share_the_log_tee_and_restore_it_after_the_last_drop() {
    use ferroui_base::logging::Logger;
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let previous = Logger::sink();
    let first = Browser::new();
    let sink = Logger::sink().unwrap();
    let second = Browser::new();
    assert!(std::sync::Arc::ptr_eq(&sink, &Logger::sink().unwrap()));
    drop(first);
    assert!(std::sync::Arc::ptr_eq(&sink, &Logger::sink().unwrap()));
    drop(second);
    assert_eq!(Logger::sink().is_some(), previous.is_some());
    if let Some(previous) = previous {
        assert!(std::sync::Arc::ptr_eq(&previous, &Logger::sink().unwrap()));
    }
}

#[test]
fn immutable_names_and_invalid_input_do_not_mutate_controls() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, button, _) = fixture();
    let mut session = CdpSession::new(&window.clone().upcast::<TopLevel>(), "1");
    let id = find(&mut session, "#clickMe");
    let result = session.handle(&json!({"id":1,"method":"DOM.setAttributeValue","params":{"nodeId":id,"name":"Name","value":"changed"}}).to_string());
    assert!(result[0].get("error").is_some());
    assert_eq!(button.name().as_deref(), Some("clickMe"));
    let result = session.handle(r#"{"id":2,"method":"Input.dispatchMouseEvent","params":{"type":"mousePressed","button":"left","y":0}}"#);
    assert_eq!(result[0]["error"]["code"], -32602);
    let result = session.handle(r#"{"id":3,"method":"Input.dispatchTouchEvent","params":{"type":"touchStart","touchPoints":[]}}"#);
    assert_eq!(result[0]["error"]["code"], -32602);
    window.close();
    let result = session.handle(r#"{"id":4,"method":"DOM.getDocument"}"#);
    assert_eq!(result[0]["error"]["code"], -32000);
}

#[test]
fn children_arrive_before_the_response_and_unsafe_style_queries_are_rejected() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_app, window, _, _, _) = fixture();
    let mut session = CdpSession::new(&window.upcast::<TopLevel>(), "1");
    let panel = find(&mut session, "#container");
    let pushed = session.handle(&json!({"id":5,"method":"DOM.pushNodesByBackendIdsToFrontend","params":{"backendNodeIds":[panel,999999]}}).to_string());
    assert_eq!(pushed[0]["method"], "DOM.setChildNodes");
    assert_eq!(
        pushed.last().unwrap()["result"]["nodeIds"],
        json!([panel, 0])
    );
    let restored = session
        .handle(r#"{"id":6,"method":"DOM.pushNodeByPathToFrontend","params":{"path":"0,Window"}}"#);
    assert_eq!(restored.last().unwrap()["result"]["nodeId"], 2);
    let messages = session.handle(
        &json!({"id":1,"method":"DOM.requestChildNodes","params":{"nodeId":panel}}).to_string(),
    );
    assert_eq!(messages[0]["method"], "DOM.setChildNodes");
    assert_eq!(messages[0]["params"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(messages.last().unwrap()["id"], 1);
    for selector in ["^", ":not(^)", "> Button", "Button >", "Button, > TextBox"] {
        let result = session.handle(&json!({"id":2,"method":"DOM.querySelectorAll","params":{"nodeId":1,"selector":selector}}).to_string());
        assert!(result[0].get("error").is_some(), "{selector}: {result:?}");
    }
}
