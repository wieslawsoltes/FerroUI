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

#[test]
fn invalid_native_edit_restores_the_originating_widget_and_reports_one_error() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let document = parse(
        r#"
        import QtQuick
        import QtQuick.Controls
        Column {
            TextField { id: editor; text: "red" }
            Rectangle { id: swatch; color: editor.text }
        }
    "#,
    )
    .unwrap();
    let errors = Rc::new(RefCell::new(vec![]));
    let received = errors.clone();
    let view = QmlView::with_error_handler(
        &document,
        &ControlRegistry::standard().unwrap(),
        BTreeMap::new(),
        move |error| received.borrow_mut().push(error),
    )
    .unwrap();
    let editor_id = view.id("editor").unwrap();
    let editor = view.control(editor_id).unwrap().cast::<TextBox>().unwrap();
    editor.set_text(Some("not-a-color"));
    assert_eq!(editor.text().as_deref(), Some("red"));
    assert_eq!(
        view.get(editor_id, "text").unwrap(),
        Value::String("red".into())
    );
    assert_eq!(errors.borrow().len(), 1);
    editor.set_text(Some("blue"));
    assert_eq!(
        view.get(view.id("swatch").unwrap(), "color").unwrap(),
        Value::String("blue".into())
    );
}

#[test]
fn positioners_keep_quick_item_cross_axis_geometry_and_native_alignment_on_resize() {
    use ferroui_base::{layout::HorizontalAlignment, Rect, Size};
    let _scope = UnitTestApplication::start(TestServices::new());
    let mut registry = ControlRegistry::standard().unwrap();
    registry
        .register_ferro_control("Workbench", Canvas::new)
        .unwrap();
    let document = parse(
        r#"
        import QtQuick
        import Workbench as Native
        Column {
            Item { id: top; width: 24; height: 10; x: 3 }
            Row {
                id: row; height: 40
                Item { id: middle; width: 16; height: 12; y: 5 }
            }
            Native.Canvas { id: native; width: 20; height: 8 }
        }
    "#,
    )
    .unwrap();
    let view = QmlView::new(&document, &registry, BTreeMap::new()).unwrap();
    let root = view.root();
    let element = |name: &str| view.control(view.id(name).unwrap()).unwrap();
    let top = element("top");
    let middle = element("middle");
    let native = element("native");
    native.set_horizontal_alignment(HorizontalAlignment::Right);
    for width in [120., 190.] {
        root.measure(Size::new(width, 100.));
        root.arrange(Rect::new(0., 0., width, 100.));
        assert_eq!(top.bounds(), Rect::new(3., 0., 24., 10.));
        assert_eq!(middle.bounds(), Rect::new(0., 5., 16., 12.));
        assert_eq!(native.bounds(), Rect::new(width - 20., 50., 20., 8.));
    }
    view.set(view.id("top").unwrap(), "x", Value::Number(11.))
        .unwrap();
    root.arrange(Rect::new(0., 0., 190., 100.));
    assert_eq!(top.bounds().x, 11.);
}

#[test]
fn dynamic_visual_lifetime_validates_before_mount_and_disconnects_before_retirement() {
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick
        import QtQuick.Controls
        Column {
            id: app
            property int activations: 0
            Component {
                id: factory
                Rectangle {
                    required property string paint
                    color: paint
                    Button { text: "Run"; onClicked: app.activations++ }
                }
            }
        }
    "#,
    );
    let column = view.root().cast::<StackPanel>().unwrap();
    let factory = view.id("factory").unwrap();
    let paint = |value: &str| BTreeMap::from([("paint".into(), Value::String(value.into()))]);
    assert_eq!(column.children().count(), 0);
    let child = view
        .create_object(factory, Some(view.root_id()), paint("red"))
        .unwrap();
    let peer = view.control(child).unwrap();
    assert_eq!(column.children().get(0), peer);
    let button = {
        let instance = view.0.instance.borrow();
        view.control(instance.children(child)[0])
            .unwrap()
            .cast::<Button>()
            .unwrap()
    };
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "activations").unwrap(),
        Value::Number(1.0)
    );
    assert!(view
        .create_object(factory, Some(view.root_id()), paint("invalid paint"))
        .is_err());
    assert_eq!(column.children().count(), 1);
    assert_eq!(column.children().get(0), peer);
    view.destroy_object(child).unwrap();
    assert_eq!(column.children().count(), 0);
    assert!(view.control(child).is_none());
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "activations").unwrap(),
        Value::Number(1.0)
    );
}

#[test]
fn native_loader_sizes_and_disconnects_a_retained_loaded_control() {
    use ferroui_base::{layout::ILayoutManager, Rect};
    use ferroui_controls::Window;
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    let view = mount(
        r#"
        import QtQuick
        import QtQuick.Controls
        Item { id: app; width:120; height:80; property int clicks:0
            Component { id: factory; Button { width:40; height:24; onClicked: app.clicks++ } }
            Loader { id: dest; sourceComponent:factory }
        }
    "#,
    );
    let id = view.id("dest").unwrap();
    let host = view.control(id).unwrap().cast::<Canvas>().unwrap();
    let Value::Object(item) = view.get(id, "item").unwrap() else {
        panic!("loaded button")
    };
    let button = view.control(item).unwrap().cast::<Button>().unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(view.root())));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    assert_eq!(host.bounds(), Rect::new(0., 0., 40., 24.));
    assert_eq!(button.bounds(), Rect::new(0., 0., 40., 24.));
    view.set(id, "width", Value::Number(75.)).unwrap();
    assert_eq!(view.get(item, "width").unwrap(), Value::Number(75.));
    assert_eq!(button.width(), 75.);
    window.layout_manager().execute_layout_pass();
    assert_eq!(button.bounds().width, 75.);
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "clicks").unwrap(),
        Value::Number(1.)
    );
    view.set(id, "active", Value::Bool(false)).unwrap();
    assert_eq!(host.children().count(), 0);
    assert!(view.control(item).is_none());
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "clicks").unwrap(),
        Value::Number(1.)
    );
}
