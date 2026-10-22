use super::*;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::{AppBuilder, Application, Border, Button, StackPanel, TextBox, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_simple::SimpleTheme;
use std::cell::Cell;

fn app() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_skia()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..Default::default()
        })
        .after_setup(|b| {
            b.instance()
                .unwrap()
                .styles()
                .add(SimpleTheme::new().as_style());
        })
}
fn run(f: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(app);
    session
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
    session.dispose();
}
fn fixture() -> (Ref<Window>, Ref<Button>, Ref<TextBox>, Inspector) {
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(200.0);
    let panel = StackPanel::new();
    let button = Button::new();
    button.set_name(Some("Action".into()));
    button.set_content(Some(Rc::new("Run".to_owned())));
    let text = TextBox::new();
    text.set_name(Some("Editor".into()));
    panel.children().add(button.clone().upcast::<Control>());
    panel.children().add(text.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    window.capture_rendered_frame();
    let mut inspector = Inspector::new(Permissions {
        allow_input: true,
        allow_property_writes: true,
    });
    inspector.register_root(&window);
    (window, button, text, inspector)
}
#[test]
fn stable_weak_ids_do_not_keep_controls_alive() {
    run(|| {
        let mut inspector = Inspector::new(Permissions::default());
        let node = Border::new();
        let weak = node.downgrade();
        let id = inspector.node_id(&node);
        assert_eq!(id, inspector.node_id(&node));
        drop(node);
        assert!(weak.upgrade().is_none());
        assert!(inspector.resolve(id).is_err());
        let next = Border::new();
        assert_ne!(id, inspector.node_id(&next));
    });
}
#[test]
fn trees_search_and_layout_use_real_controls() {
    run(|| {
        let (window, button, _, mut inspector) = fixture();
        let id = inspector.node_id(&button);
        for tree in ["visual", "logical", "automation"] {
            let snapshot = inspector
                .execute(
                    "get_tree",
                    &json!({"tree":tree,"depth":3,"properties":["Width"]}),
                )
                .unwrap();
            assert_eq!(snapshot.as_array().unwrap().len(), 1);
            assert!(snapshot[0]["child_count"].as_u64().unwrap() > 0);
        }
        let found = inspector
            .execute(
                "find_elements",
                &json!({"selector":"Button#Action","text":"Run"}),
            )
            .unwrap();
        assert_eq!(found[0]["node_id"], id);
        assert!(found[0]["bounds"]["width"].as_f64().unwrap() > 0.0);
        assert_eq!(
            inspector
                .execute("find_elements", &json!({"selector":"Button:disabled"}))
                .unwrap(),
            json!([])
        );
        button.set_is_enabled(false);
        assert_eq!(
            inspector
                .execute("find_elements", &json!({"selector":"Button:disabled"}))
                .unwrap()[0]["node_id"],
            id
        );
        let shallow = inspector.execute("get_tree", &json!({"depth":0})).unwrap();
        assert!(shallow[0].get("children").is_none());
        window.close();
    });
}
#[test]
fn property_permissions_conversion_validation_and_sources() {
    run(|| {
        let (_window, button, _text, mut inspector) = fixture();
        let id = inspector.node_id(&button);
        inspector.permissions.allow_property_writes = false;
        assert!(inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Width","value":75})
            )
            .is_err());
        inspector.permissions.allow_property_writes = true;
        let value = inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Width","value":75}),
            )
            .unwrap();
        assert_eq!(button.width(), 75.0);
        assert_eq!(value[0]["priority"], "LocalValue");
        assert!(inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Width","value":-1})
            )
            .is_err());
        assert!(inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Bounds","value":"0,0,1,1"})
            )
            .is_err());
        inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Width","clear":true}),
            )
            .unwrap();
        assert!(button.width().is_nan());
        assert!(inspector
            .execute(
                "set_property",
                &json!({"node_id":id,"property":"Name","value":"Renamed"})
            )
            .is_err());
        let editor = inspector.node_id(&_text);
        inspector
            .execute(
                "set_property",
                &json!({"node_id":editor,"property":"Text","value":"value"}),
            )
            .unwrap();
        assert_eq!(_text.text().as_deref(), Some("value"));
        inspector
            .execute(
                "set_property",
                &json!({"node_id":editor,"property":"Text","value":null}),
            )
            .unwrap();
        assert!(_text.text().is_none());
    });
}
#[test]
fn raw_click_keyboard_text_and_automation_drive_the_ui() {
    run(|| {
        let (window, button, text, mut inspector) = fixture();
        let id = inspector.node_id(&button);
        let editor = inspector.node_id(&text);
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        let _subscription = button.click(move |_, _| count.set(count.get() + 1));
        inspector.permissions.allow_input = false;
        assert!(inspector.execute("click", &json!({"node_id":id})).is_err());
        assert_eq!(clicks.get(), 0);
        inspector.permissions.allow_input = true;
        inspector.execute("click", &json!({"node_id":id})).unwrap();
        assert_eq!(clicks.get(), 1);
        inspector
            .execute("invoke_pattern", &json!({"node_id":id,"pattern":"invoke"}))
            .unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(clicks.get(), 2);
        inspector
            .execute("type_text", &json!({"node_id":editor,"text":"hello"}))
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("hello"));
        inspector
            .execute("press_key", &json!({"node_id":editor,"key":"Ctrl+A"}))
            .unwrap();
        inspector
            .execute("type_text", &json!({"node_id":editor,"text":"replacement"}))
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("replacement"));
        assert!(inspector
            .execute("invoke_pattern", &json!({"node_id":id,"pattern":"toggle"}))
            .is_err());
        window.close();
    });
}
#[test]
fn resources_styles_screenshots_waits_and_stats() {
    run(|| {
        let (window, button, _, mut inspector) = fixture();
        let id = inspector.node_id(&button);
        window.resources().add_value("Greeting", "hello".to_owned());
        let resources = inspector
            .execute(
                "get_resources",
                &json!({"node_id":id,"keys":["Greeting","Missing"]}),
            )
            .unwrap();
        assert_eq!(resources[0]["value"], "hello");
        assert_eq!(resources[1]["found"], false);
        assert!(!inspector
            .execute("get_styles", &json!({"node_id":id}))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
        let image = inspector
            .execute("screenshot", &json!({"scale":0.5}))
            .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(image["data"].as_str().unwrap())
            .unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(image["width"], 160.0);
        assert_eq!(
            inspector
                .execute(
                    "wait_for",
                    &json!({"selector":"#Action","condition":"enabled"})
                )
                .unwrap()["matched"],
            true
        );
        assert_eq!(
            inspector
                .execute("wait_for", &json!({"name":"Missing","condition":"absent"}))
                .unwrap()["matched"],
            true
        );
        assert!(inspector
            .execute("wait_for", &json!({"condition":"property_equals"}))
            .is_err());
        assert_eq!(
            inspector
                .execute("get_frame_stats", &json!({}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        window.close();
    });
}
#[test]
fn schemas_reject_invalid_input_before_mutating() {
    assert_eq!(tools().len(), 16);
    for args in [
        json!({"node_id":0}),
        json!({"node_id":-1}),
        json!({"node_id":"1"}),
        json!({"node_id":1,"typo":true}),
    ] {
        assert!(validate("click", &args).is_err());
    }
    assert!(validate("get_tree", &json!({"depth":100})).is_err());
    assert!(validate("screenshot", &json!({"scale":0})).is_err());
    assert!(validate("unknown", &json!({})).is_err());
}
