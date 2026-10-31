use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Canvas, StackPanel, TextBlock, TextBox};
use ferroui_qml_compiler::parse;

fn mount(source: &str) -> QmlView {
    let doc = parse(source).unwrap();
    QmlView::with_error_handler(
        &doc,
        &ControlRegistry::standard().unwrap(),
        BTreeMap::new(),
        |err| panic!("unexpected QML event error: {err}"),
    )
    .unwrap()
}

#[test]
fn native_click_updates_the_same_text_control_and_rust_can_drive_the_binding() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick 6.12
        import QtQuick.Controls
        Column {
            id: page
            property int clicks: 1
            spacing: 7
            Button { id: trigger; text: "Increment"; onClicked: page.clicks = page.clicks + 1 }
            Text { id: result; text: "Clicked " + page.clicks }
        }
    "#,
    );
    let root = view.root().cast::<StackPanel>().unwrap();
    let result = view
        .control(view.id("result").unwrap())
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    let button = root.children().get(0).cast::<Button>().unwrap();
    assert_eq!(root.spacing(), 7.0);
    assert_eq!(root.children().count(), 2);
    assert_eq!(root.children().get(1), result.clone().upcast::<Control>());
    assert_eq!(result.text().as_deref(), Some("Clicked 1"));
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(result.text().as_deref(), Some("Clicked 2"));
    view.set(view.root_id(), "clicks", Value::Number(8.0))
        .unwrap();
    assert_eq!(result.text().as_deref(), Some("Clicked 8"));
}

#[test]
fn text_input_roundtrips_without_replacing_bindings_on_native_echo() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick
        import QtQuick.Controls
        Column {
            id: page
            property string seed: "one"
            TextField { id: editor; text: page.seed }
            Text { id: mirror; text: editor.text }
        }
    "#,
    );
    let editor = view
        .control(view.id("editor").unwrap())
        .unwrap()
        .cast::<TextBox>()
        .unwrap();
    let mirror = view
        .control(view.id("mirror").unwrap())
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    for seed in ["two", "three"] {
        view.set(view.root_id(), "seed", Value::String(seed.into()))
            .unwrap();
        assert_eq!(editor.text().as_deref(), Some(seed));
    }
    editor.set_text(Some("typed"));
    assert_eq!(mirror.text().as_deref(), Some("typed"));

    let subscriptions = editor.property_changed_subscriber_count();
    drop(view);
    assert_eq!(
        editor.property_changed_subscriber_count(),
        subscriptions - 1
    );
    editor.set_text(Some("after drop"));
    assert_eq!(mirror.text().as_deref(), Some("typed"));
}

#[test]
fn nested_visuals_are_distinct_from_object_property_ownership() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQml
        import QtQuick
        Item {
            property QtObject model: QtObject { property string title: "ready" }
            property Item spare: Item { width: 99 }
            Text { id: label; x: 12; y: 22; text: parent.model.title }
        }
    "#,
    );
    let canvas = view.root().cast::<Canvas>().unwrap();
    assert_eq!(canvas.children().count(), 1);
    let label = canvas.children().get(0).cast::<TextBlock>().unwrap();
    assert_eq!(label.text().as_deref(), Some("ready"));
    assert_eq!(Canvas::get_left(&label), 12.0);
    assert_eq!(Canvas::get_top(&label), 22.0);
}
