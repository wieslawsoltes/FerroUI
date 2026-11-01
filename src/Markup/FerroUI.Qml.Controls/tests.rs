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
    view.set(
        backdrop_id,
        "anchors.fill",
        Value::Object(view.id("middle").unwrap()),
    )
    .unwrap();
    layout(140., 80.);
    assert_eq!(backdrop.bounds(), Rect::new(71., 40., 2., 0.));
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

#[test]
fn repeater_moves_native_peers_in_place_and_retires_removed_button_handlers() {
    use ferroui_base::{Rect, Size};
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"
        import QtQuick
        import QtQuick.Controls
        import QtQml.Models
        Column {
            id: page
            spacing: 7
            property string clickedTitle: ""
            ListModel {
                id: rows
                ListElement { title: "A" }
                ListElement { title: "B" }
            }
            Item { height: 10; width: 20 }
            Repeater {
                id: entries
                model: rows
                Button {
                    required property string title
                    width: 20; height: 12
                    text: title
                    onClicked: page.clickedTitle = title
                }
            }
            Item { id: footer; height: 14; width: 20 }
            function reorder() { rows.move(0, 1, 1) }
            function removeFirst() { rows.remove(0) }
        }"#,
    );
    let column = view.root().cast::<StackPanel>().unwrap();
    let entries = view.id("entries").unwrap();
    let item_at = |index| {
        let Value::Object(item) = view
            .call(entries, "itemAt", vec![Value::Number(index)])
            .unwrap()
        else {
            panic!("expected a live delegate");
        };
        item
    };
    let first = item_at(0.);
    let second = item_at(1.);
    let a = view.control(first).unwrap().cast::<Button>().unwrap();
    let b = view.control(second).unwrap().cast::<Button>().unwrap();
    let footer = view.control(view.id("footer").unwrap()).unwrap();
    let layout = || {
        column.measure(Size::new(100., 100.));
        column.arrange(Rect::new(0., 0., 100., 100.));
    };
    layout();
    assert_eq!(
        (a.bounds().y, b.bounds().y, footer.bounds().y),
        (17., 36., 55.)
    );
    view.call(view.root_id(), "reorder", vec![]).unwrap();
    assert_eq!(item_at(0.), second);
    assert_eq!(view.control(second).unwrap(), b);
    layout();
    assert_eq!(
        (b.bounds().y, a.bounds().y, footer.bounds().y),
        (17., 36., 55.)
    );
    b.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "clickedTitle").unwrap(),
        Value::String("B".into())
    );
    view.call(view.root_id(), "removeFirst", vec![]).unwrap();
    assert!(view.control(second).is_none());
    layout();
    assert_eq!((a.bounds().y, footer.bounds().y), (17., 36.));
    a.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    b.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        view.get(view.root_id(), "clickedTitle").unwrap(),
        Value::String("A".into())
    );
}

#[test]
fn state_transition_samples_the_original_native_peer_on_the_view_clock() {
    use std::time::Duration;
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(
        r#"import QtQuick
        Item { id: scene; Rectangle { id: box; width: 20; height: 10 }
            states: State { name: "wide"; PropertyChanges { box.width: 100 } }
            transitions: Transition { NumberAnimation { properties: "width"; duration: 80 } }
        }"#,
    );
    let id = view.id("box").unwrap();
    let peer = view.control(id).unwrap();
    view.set(view.root_id(), "state", Value::String("wide".into()))
        .unwrap();
    view.advance_time(Duration::from_millis(40)).unwrap();
    assert_eq!(peer.width(), 60.);
    view.advance_time(Duration::from_millis(40)).unwrap();
    assert_eq!(view.control(id).unwrap(), peer);
    assert_eq!(peer.width(), 100.);
}

#[test]
fn mirrored_positioners_keep_declaration_order_and_wrap_each_line() {
    use ferroui_base::layout::ILayoutManager;
    use ferroui_controls::Window;
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    let view = mount(include_str!(
        "../../../tests/QmlCompiled/anchors/Positioners.qml"
    ));
    let window = Window::new();
    window.set_content(Some(Control::boxed(view.root())));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    let positions = || {
        window.layout_manager().execute_layout_pass();
        ["first", "second", "a", "b", "c"]
            .map(|name| view.control(view.id(name).unwrap()).unwrap().bounds().x)
    };
    assert_eq!(positions(), [70., 48., 30., 18., 10.]);
    view.call(view.root_id(), "reverseDirections", vec![])
        .unwrap();
    assert_eq!(
        view.get(view.id("row").unwrap(), "effectiveLayoutDirection")
            .unwrap(),
        Value::Number(0.)
    );
    assert_eq!(positions(), [0., 12., 0., 22., 0.]);
    window.close();
}

#[test]
fn baseline_and_mirrored_anchor_geometry_match_qt() {
    use ferroui_base::{Rect, Size};
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(include_str!(
        "../../../tests/QmlCompiled/anchors/Mirrored.qml"
    ));
    let root = view.root();
    let frames = || {
        root.measure(Size::new(101., 60.));
        root.arrange(Rect::new(0., 0., 101., 60.));
        let bounds = |name| view.control(view.id(name).unwrap()).unwrap().bounds();
        let label = bounds("label");
        let inset = bounds("inset");
        [label.x, label.y, inset.x, inset.width, bounds("middle").x]
    };
    assert_eq!(frames(), [-2., 19., 7., 92., 42.]);
    view.call(view.root_id(), "change", vec![]).unwrap();
    assert_eq!(frames(), [32., 21., 2., 92., 50.]);
    let label = view.id("label").unwrap();
    assert!(view
        .set(
            label,
            "anchors.top",
            Value::AnchorLine(view.root_id(), ferroui_qml::AnchorEdge::Top)
        )
        .is_err());
    assert_eq!(frames(), [32., 21., 2., 92., 50.]);
}

#[test]
fn sibling_anchor_order_margins_and_failed_cycles_are_transactional() {
    use ferroui_base::{Rect, Size};
    use ferroui_qml::AnchorEdge;
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(include_str!("../../../tests/QmlCompiled/anchors/Main.qml"));
    let root = view.root();
    let follower = view.id("follower").unwrap();
    let marker = view.id("marker").unwrap();
    let label = view.id("label").unwrap();
    let frames = || {
        root.measure(Size::new(200., 100.));
        root.arrange(Rect::new(0., 0., 200., 100.));
        (
            view.control(follower).unwrap().bounds(),
            view.control(label).unwrap().bounds(),
        )
    };
    assert_eq!(
        frames(),
        (
            Rect::new(63., 42., 132., 52.),
            Rect::new(124., 23., 10., 6.)
        )
    );
    view.call(view.root_id(), "moveMarker", vec![]).unwrap();
    assert_eq!(
        frames(),
        (
            Rect::new(83., 52., 112., 42.),
            Rect::new(134., 33., 10., 6.)
        )
    );
    let target = Value::AnchorLine(follower, AnchorEdge::Right);
    assert!(view
        .set(marker, "anchors.left", target)
        .unwrap_err()
        .message
        .contains("cycle"));
    assert_eq!(view.get(marker, "anchors.left").unwrap(), Value::Undefined);
    assert!(view
        .set(
            marker,
            "anchors.left",
            Value::AnchorLine(follower, AnchorEdge::Bottom)
        )
        .is_err());
    view.call(view.root_id(), "resetLeft", vec![]).unwrap();
    assert_eq!(
        frames(),
        (Rect::new(3., 52., 192., 42.), Rect::new(94., 33., 10., 6.))
    );
}

#[test]
fn list_view_scrolls_million_row_native_canvas_from_both_sides() {
    use ferroui_base::{layout::ILayoutManager, Vector};
    use ferroui_controls::{ScrollViewer, Window};
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    // Install the same native scroll presenter template that the test theme uses for lists.
    ferroui_controls::testing::add_autocomplete_themes(
        &ferroui_controls::Application::current().unwrap().styles(),
    );
    let view = mount(
        r#"import QtQuick
        ListView { width: 100; height: 60; cacheBuffer: 0; model: 1000000
            delegate: Item { required property int index; width: 100; height: 20 }
        }"#,
    );
    let id = view.root_id();
    let scroll = view.root().cast::<ScrollViewer>().unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(view.root())));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    assert_eq!(scroll.extent().height, 20_000_000.);
    assert_eq!(scroll.viewport().height, 60.);

    view.call(
        id,
        "positionViewAtIndex",
        vec![Value::Number(500_000.), Value::Number(0.)],
    )
    .unwrap();
    window.layout_manager().execute_layout_pass();
    assert_eq!(scroll.offset().y, 10_000_000.);
    let Value::Object(row) = view
        .call(id, "itemAtIndex", vec![Value::Number(500_000.)])
        .unwrap()
    else {
        panic!("visible delegate")
    };
    assert_eq!(view.control(row).unwrap().bounds().y, 10_000_000.);
    scroll.set_offset(Vector::new(0., 40.));
    window.layout_manager().execute_layout_pass();
    assert_eq!(view.get(id, "contentY").unwrap(), Value::Number(40.));
    assert!(view.control(row).is_none());
    assert!(matches!(view.get(id,"data").unwrap(),Value::List(items) if items.len()<12));
    window.close();
}

#[test]
fn reversed_native_list_translates_scroll_origin_and_keeps_reused_peer() {
    use ferroui_base::{layout::ILayoutManager, Vector};
    use ferroui_controls::{ScrollViewer, Window};
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    ferroui_controls::testing::add_autocomplete_themes(
        &ferroui_controls::Application::current().unwrap().styles(),
    );
    let view = mount(
        r#"import QtQuick
        ListView {width:45; height:45; orientation:ListView.Horizontal; layoutDirection:Qt.RightToLeft
            model:5; reuseItems:true; spacing:3
            header: Item {width:7;height:7}
            footer: Item {width:9;height:9}
            delegate: Item {required property int index; width:20;height:20}
        }"#,
    );
    let id = view.root_id();
    let scroll = view.root().cast::<ScrollViewer>().unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(view.root())));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    assert_eq!(scroll.extent().width, 128.);
    assert_eq!(view.get(id, "contentX").unwrap(), Value::Number(-38.));
    assert_eq!(scroll.offset().x, 83.);
    let Value::Object(old) = view
        .call(id, "itemAtIndex", vec![Value::Number(1.)])
        .unwrap()
    else {
        panic!("row 1")
    };
    let peer = view.control(old).unwrap();
    scroll.set_offset(Vector::new(0., 0.));
    window.layout_manager().execute_layout_pass();
    assert_eq!(view.get(id, "contentX").unwrap(), Value::Number(-121.));
    assert_eq!(view.get(old, "index").unwrap(), Value::Number(3.));
    assert_eq!(view.control(old).unwrap(), peer);
    view.call(id, "positionViewAtBeginning", vec![]).unwrap();
    window.layout_manager().execute_layout_pass();
    assert_eq!(scroll.offset().x, 83.);
    window.close();
}

#[test]
fn network_completion_wakes_a_mounted_loader_on_its_dispatcher() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_qml::network::{ComponentReply, ComponentTransport, Url};
    #[derive(Default)]
    struct Download(RefCell<Option<ComponentReply>>);
    impl ComponentTransport for Download {
        fn request(&self, url: &Url, reply: ComponentReply) {
            assert_eq!(url.path(), "/Panel.qml");
            self.0.replace(Some(reply));
        }
    }
    let _scope = UnitTestApplication::start(TestServices::new());
    let download = Rc::new(Download::default());
    let mut types = ControlRegistry::standard().unwrap();
    types.types_mut().set_component_transport(download.clone());
    let doc = parse(
        r#"import QtQuick
        Loader { source: "https://widgets.test/Panel.qml" }"#,
    )
    .unwrap();
    let view = QmlView::new(&doc, &types, Default::default()).unwrap();
    let reply = download.0.take().unwrap();
    std::thread::spawn(move || {
        reply.complete(Ok(br#"import QtQuick
        Text { text: "Fetched and mounted" }"#
            .to_vec()))
    })
    .join()
    .unwrap();
    assert_eq!(view.get(view.root_id(), "item").unwrap(), Value::Null);
    Dispatcher::current_dispatcher().run_jobs(None);
    let Value::Object(item) = view.get(view.root_id(), "item").unwrap() else {
        panic!("remote text")
    };
    assert_eq!(
        view.control(item)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Fetched and mounted")
    );
}

#[test]
fn native_toggles_share_qml_groups_and_accessible_activation() {
    use ferroui_controls::{
        automation::{peers::ToggleButtonAutomationPeer, AutomationProperties},
        CheckBox, RadioButton, ToggleSwitch,
    };
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(include_str!("../../../tests/QmlCompiled/buttons/Main.qml"));
    let root = view.root_id();
    let report = || view.call(root, "report", vec![]).unwrap();
    let call = |name| view.call(root, name, vec![]).unwrap();
    assert_eq!(
        report(),
        Value::String("2/false/0/false/false/false/0/0/".into())
    );
    call("choose");
    call("choose");
    assert_eq!(
        report(),
        Value::String("2/true/1/true/false/false/0/0/TCGCG".into())
    );
    call("setSecond");
    let first = view
        .control(view.id("first").unwrap())
        .unwrap()
        .cast::<RadioButton>()
        .unwrap();
    let second = view
        .control(view.id("second").unwrap())
        .unwrap()
        .cast::<RadioButton>()
        .unwrap();
    assert_eq!(first.is_checked(), Some(false));
    assert_eq!(second.is_checked(), Some(true));
    let check = view
        .control(view.id("mixed").unwrap())
        .unwrap()
        .cast::<CheckBox>()
        .unwrap();
    ToggleButtonAutomationPeer::new(&check).toggle().unwrap();
    assert_eq!(check.is_checked(), None);
    assert_eq!(
        report(),
        Value::String("2/false/1/false/true/false/1/0/TCGCGK".into())
    );
    call("nextCheck");
    assert_eq!(check.is_checked(), Some(true));
    let switch = view
        .control(view.id("power").unwrap())
        .unwrap()
        .cast::<ToggleSwitch>()
        .unwrap();
    assert_eq!(
        AutomationProperties::get_name(&switch).as_deref(),
        Some("Main power")
    );
    ToggleButtonAutomationPeer::new(&switch).toggle().unwrap();
    assert_eq!(switch.is_checked(), Some(true));
    call("groupAll");
    assert_eq!(first.is_checked(), Some(true));
    assert_eq!(second.is_checked(), Some(true));
    assert_eq!(
        report(),
        Value::String("2/false/2/true/true/true/2/1/TCGCGKKS".into())
    );
}

#[test]
fn native_ranges_preserve_descending_bounds_and_propagate_user_input() {
    use ferroui_base::{layout::Orientation, utilities::Decimal};
    use ferroui_controls::{NumericUpDown, ProgressBar, Slider};
    let _scope = UnitTestApplication::start(TestServices::new());
    let view = mount(include_str!("../../../tests/QmlCompiled/ranges/Main.qml"));
    let level = view
        .control(view.id("level").unwrap())
        .unwrap()
        .cast::<Slider>()
        .unwrap();
    let bar = view
        .control(view.id("progress").unwrap())
        .unwrap()
        .cast::<ProgressBar>()
        .unwrap();
    let spin = view
        .control(view.id("spin").unwrap())
        .unwrap()
        .cast::<NumericUpDown>()
        .unwrap();
    assert_eq!(level.orientation(), Orientation::Vertical);
    assert!(level.is_direction_reversed());
    assert_eq!(
        (level.minimum(), level.maximum(), level.value()),
        (0., 10., 6.)
    );
    assert_eq!(spin.increment(), Decimal::from(-3));
    level.set_range_value(4.);
    spin.set_numeric_value(Some(Decimal::from(2)));
    assert_eq!(bar.value(), 4.);
    let id = view.root_id();
    assert_eq!(view.get(id, "moves").unwrap(), Value::Number(1.));
    assert_eq!(view.get(id, "edits").unwrap(), Value::Number(1.));
    view.call(id, "step", vec![]).unwrap();
    assert_eq!(level.value(), 7.);
    assert_eq!(bar.value(), 7.);
    assert_eq!(spin.value(), Some(Decimal::from(10)));
    assert_eq!(view.get(id, "moves").unwrap(), Value::Number(1.));
}
