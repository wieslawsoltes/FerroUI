use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Canvas, StackPanel, TextBlock, TextBox};
use ferroui_qml::ir::parse;

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

#[test]
fn native_projection_can_reject_a_transaction_without_splitting_model_and_view() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick
        Item {
            id: page
            property string shade: "red"
            Rectangle { id: boxItem; width: 100; color: page.shade }
        }
    "#,
    );
    let root = view.root_id();
    let child = view.id("boxItem").unwrap();
    let err = view
        .set(root, "shade", Value::String("unparseable-color".into()))
        .unwrap_err();
    assert!(err.to_string().contains("color"));
    assert_eq!(
        view.get(root, "shade").unwrap(),
        Value::String("red".into())
    );
    assert_eq!(
        view.get(child, "color").unwrap(),
        Value::String("red".into())
    );

    let err = view
        .set(root, "data", Value::List(vec![Value::Object(root)]))
        .unwrap_err();
    assert!(err.to_string().contains("root"));
    assert_eq!(
        view.get(root, "data").unwrap(),
        Value::List(vec![Value::Object(child)])
    );
    assert_eq!(view.root().cast::<Canvas>().unwrap().children().count(), 1);
}

#[test]
fn reflected_controls_use_native_owners_defaults_validation_and_input_notifications() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let mut types = ControlRegistry::default();
    types
        .register_ferro_control("Workbench", StackPanel::new)
        .unwrap();
    types
        .register_ferro_control("Workbench", TextBlock::new)
        .unwrap();
    types
        .register_ferro_control("Workbench", TextBox::new)
        .unwrap();
    let document = parse(
        r#"
        import Workbench
        StackPanel {
            spacing: 10
            TextBox { id: input; text: "editable" }
            TextBlock { id: output; text: input.text; fontSize: 22 }
        }
    "#,
    )
    .unwrap();
    let view = QmlView::new(&document, &types, BTreeMap::new()).unwrap();
    let panel = view.root().cast::<StackPanel>().unwrap();
    let input = panel.children().get(0).cast::<TextBox>().unwrap();
    let output = panel.children().get(1).cast::<TextBlock>().unwrap();
    assert_eq!(panel.spacing(), 10.0);
    assert!(!output.is_set(ferroui_base::layout::Layoutable::width_property().as_property()));
    assert_eq!(output.font_size(), 22.0);
    assert_eq!(output.text().as_deref(), Some("editable"));
    input.set_text(Some("draft"));
    assert_eq!(output.text().as_deref(), Some("draft"));
    let id = view.id("output").unwrap();
    let err = view.set(id, "width", Value::Number(-42.0)).unwrap_err();
    assert!(err.to_string().contains("Width"));
    assert!(output.width().is_nan());
}

#[test]
fn parent_anchors_follow_native_arrangement_and_unfill_restores_explicit_dimensions() {
    use ferroui_base::{Rect, Size};
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick
        Item {
            Rectangle {
                id: backdrop; anchors.fill: parent; anchors.margins: 4
                width: 24; height: 12
            }
            Item {
                id: middle; width: 10; height: 8
                anchors.centerIn: parent; anchors.horizontalCenterOffset: 2
            }
        }
    "#,
    );
    let root = view.root();
    let backdrop_id = view.id("backdrop").unwrap();
    let backdrop = view.control(backdrop_id).unwrap();
    let middle = view.control(view.id("middle").unwrap()).unwrap();
    let layout = |w, h| {
        root.measure(Size::new(w, h));
        root.arrange(Rect::new(0., 0., w, h));
    };
    layout(100., 60.);
    assert_eq!(backdrop.bounds(), Rect::new(4., 4., 92., 52.));
    assert_eq!(middle.bounds(), Rect::new(47., 26., 10., 8.));
    layout(140., 80.);
    assert_eq!(backdrop.bounds(), Rect::new(4., 4., 132., 72.));
    assert_eq!(middle.bounds(), Rect::new(67., 36., 10., 8.));
    view.set(backdrop_id, "width", Value::Number(32.)).unwrap();
    view.set(backdrop_id, "anchors.fill", Value::Null).unwrap();
    layout(140., 80.);
    assert_eq!(backdrop.bounds(), Rect::new(0., 0., 32., 12.));
    assert!(view
        .set(
            backdrop_id,
            "anchors.fill",
            Value::Object(view.id("middle").unwrap())
        )
        .is_err());
    assert_eq!(view.get(backdrop_id, "anchors.fill").unwrap(), Value::Null);
}
