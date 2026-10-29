use crate::{
    PreviewEnvironment, PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeElementChoice,
    PreviewPrototypeFlow, PreviewPrototypeFlowEditor, PreviewPrototypeHotspot,
    PreviewPrototypeLink, PreviewPrototypeLinkEditor, PreviewPrototypePlayerAction,
    PreviewPrototypePlayerView, PreviewPrototypeView, PreviewPrototypeViewAction,
    PreviewSourceAnchor, PreviewStageConfiguration, PreviewStageInfo, PreviewStageSize,
    PreviewStageStatus, PreviewStageView,
};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Point, Rect, Ref, Size};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{
    Button, Canvas, CheckBox, ComboBox, Control, Grid, ScrollViewer, TextBox, Window,
};
use ferroui_designer_core::Document;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn services() -> TestServices {
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1200.0, 760.0);
        native.max_auto_size_hint.set(Size::new(2000.0, 1600.0));
        native.setup_create_popup(|_| None);
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services
}

fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing prototype control {name}"))
}
fn has_named(root: &Control, name: &str) -> bool {
    root.get_logical_descendants()
        .any(|control| control.name().as_deref() == Some(name))
}
fn window(content: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1200.0);
    window.set_height(760.0);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window.update_layout();
    window
}
fn key(control: &Control, key: Key, up: bool) -> bool {
    key_with_modifiers(control, key, KeyModifiers::NONE, up)
}
fn key_with_modifiers(control: &Control, key: Key, modifiers: KeyModifiers, up: bool) -> bool {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(if up {
        InputElement::key_up_event()
    } else {
        InputElement::key_down_event()
    }));
    event.set_source(control.to_ref());
    control.raise_event(&event);
    event.handled()
}
fn key_trap_parent(control: Ref<Control>) -> (Ref<Grid>, Rc<Cell<usize>>) {
    let root = Grid::new();
    root.children().add(control);
    let escaped = Rc::new(Cell::new(0));
    let count = escaped.clone();
    root.add_handler(InputElement::key_down_event(), move |_, _| {
        count.set(count.get() + 1)
    });
    (root, escaped)
}
fn assert_authoring_keys_stay_local(control: &Control, escaped: &Cell<usize>) {
    assert!(key(control, Key::Delete, false));
    assert!(key_with_modifiers(
        control,
        Key::Z,
        KeyModifiers::CONTROL,
        false
    ));
    assert_eq!(
        escaped.get(),
        0,
        "Prototype panel keys must not reach the parent author's command handler"
    );
}
fn pointer_click(control: &Control, window: &Window, press: Point, release: Point) {
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    control.raise_event(&PointerPressedEventArgs::new(
        control.to_ref(),
        pointer.clone(),
        window,
        press,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    ));
    control.raise_event(&PointerReleasedEventArgs::new(
        control.to_ref(),
        pointer.clone(),
        window,
        release,
        2,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Left,
    ));
    pointer.capture(None);
}

#[test]
fn prototype_player_uses_scaled_native_hotspots_and_preserves_focus_and_unlinked_input() {
    let _app = UnitTestApplication::start(services());
    let view = PreviewPrototypePlayerView::new(&Grid::new());
    let host = Canvas::new();
    host.set_width(400.0);
    host.set_height(240.0);
    let input = TextBox::new();
    input.set_width(180.0);
    input.set_height(40.0);
    input.set_text(Some("Live input"));
    Canvas::set_left(&input, 20.0);
    Canvas::set_top(&input, 150.0);
    host.children().add(input.clone().upcast::<Control>());
    let hotspots = vec![PreviewPrototypeHotspot {
        link: 4,
        name: "Open detail".into(),
        bounds: Rect::new(30.0, 25.0, 140.0, 50.0),
    }];
    let events = Rc::new(RefCell::new(Vec::new()));
    let calls = events.clone();
    view.on_action(move |action, epoch, _| {
        calls.borrow_mut().push((action, epoch));
        Ok(())
    });
    let size = PreviewStageSize {
        width: 400.0,
        height: 240.0,
    };
    view.update(
        host.clone().upcast(),
        size,
        "Checkout · Cart",
        &hotspots,
        false,
        7,
    );
    let (parent, escaped) = key_trap_parent(view.control());
    let window = window(parent.upcast());
    let hit = named::<Button>(&view.control(), "PrototypeHotspot4");
    assert_authoring_keys_stay_local(&hit, &escaped);
    assert_eq!(hit.bounds().size(), Size::new(140.0, 50.0));
    let origin = host.translate_point(Point::default(), &window).unwrap();
    let shifted = host
        .translate_point(Point::new(100.0, 0.0), &window)
        .unwrap();
    assert!(
        shifted.x - origin.x > 150.0,
        "The native player must be measurably scaled for this regression"
    );
    let center = host
        .translate_point(Point::new(100.0, 50.0), &window)
        .unwrap();
    pointer_click(&hit, &window, center, center);
    assert_eq!(
        *events.borrow(),
        [(PreviewPrototypePlayerAction::Activate { link: 4 }, 7)]
    );
    let outside = host
        .translate_point(Point::new(300.0, 200.0), &window)
        .unwrap();
    pointer_click(&hit, &window, center, outside);
    assert_eq!(
        events.borrow().len(),
        1,
        "Releasing outside a captured hotspot must cancel activation"
    );
    input.set_text(Some("Retained native input"));
    assert!(hit.focus());
    view.update(
        host.clone().upcast(),
        size,
        "Checkout · Cart",
        &hotspots,
        false,
        7,
    );
    window.update_layout();
    assert_eq!(named::<Button>(&view.control(), "PrototypeHotspot4"), hit);
    assert!(
        hit.is_focused(),
        "An unchanged native frame must retain hotspot keyboard focus"
    );
    assert!(key(&hit, Key::Enter, false));
    assert_eq!(events.borrow().len(), 2);
    assert!(key(&hit, Key::Space, false));
    assert!(key(&hit, Key::Space, true));
    assert_eq!(events.borrow().len(), 3);
    named::<CheckBox>(&view.control(), "PrototypePlayerHints").set_is_checked(Some(true));
    window.update_layout();
    assert_eq!(hit.bounds().size(), Size::new(140.0, 50.0));
    assert!(hit.border_thickness().left > 0.0);
    assert_eq!(input.text().as_deref(), Some("Retained native input"));
    input.focus();
    assert!(
        key(&input, Key::Delete, false),
        "Unhandled player shortcuts must stop before the authoring workspace"
    );
    view.update(host.upcast(), size, "Checkout · Cart", &hotspots, true, 8);
    hit.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        events.borrow().len(),
        3,
        "A retired hotspot cannot navigate a later frame"
    );
    let current = named::<Button>(&view.control(), "PrototypeHotspot4");
    view.clear();
    current.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        events.borrow().len(),
        3,
        "Closing playback must retire native callbacks"
    );
    window.close();
}

fn stage(id: u64, name: &str, uri: &str) -> PreviewStageInfo {
    PreviewStageInfo {
        id,
        configuration: PreviewStageConfiguration::new(name, uri, PreviewEnvironment::default()),
        epoch: 1,
        status: PreviewStageStatus::Ready { revision: 0 },
        layout_size: PreviewStageSize {
            width: 400.0,
            height: 240.0,
        },
        rendered_revision: Some(0),
        source_revision: Some(0),
        notices: Vec::new(),
    }
}

#[test]
fn prototype_overview_maps_reachability_and_retains_guarded_keyboard_selection() {
    // The base test theme supplies only ContentControl's inherited template to
    // ordinary ScrollViewers. Install the native presenter and scrollbars so
    // viewport/extent assertions exercise real scrolling geometry.
    let mut native_services = services();
    native_services.theme = Some(Rc::new(
        ferroui_controls::testing::create_split_view_list_box_theme,
    ));
    let _app = UnitTestApplication::start(native_services);
    let document = Document::new("<Button xmlns='https://github.com/ferroui' Name='Go'/>").unwrap();
    let trigger = PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap();
    let graph = PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Checkout".into(),
            start: 10,
        }],
        links: vec![
            PreviewPrototypeLink {
                name: "Open order".into(),
                source: 10,
                trigger: trigger.clone(),
                action: PreviewPrototypeAction::Navigate { stage: 20 },
            },
            PreviewPrototypeLink {
                name: "Return".into(),
                source: 20,
                trigger,
                action: PreviewPrototypeAction::Back,
            },
        ],
    };
    let stages = vec![
        stage(10, "Cart", "memory://shop.xaml"),
        stage(20, "Order", "memory://shop.xaml"),
        stage(30, "Help", "memory://help.xaml"),
    ];
    let view = PreviewPrototypeView::new(&Grid::new());
    let events = Rc::new(RefCell::new(Vec::new()));
    let calls = events.clone();
    view.on_action(move |action, epoch, _| {
        calls.borrow_mut().push((action, epoch));
        Ok(())
    });
    view.update(&graph, &stages, 1);
    let (parent, escaped) = key_trap_parent(view.control());
    let window = window(parent.upcast());
    assert_eq!(view.selected_flow(), Some(0));
    assert!(has_named(&view.control(), "PrototypeScreen10"));
    assert!(has_named(&view.control(), "PrototypeScreen20"));
    assert!(!has_named(&view.control(), "PrototypeScreen30"));
    let connections = named::<Control>(&view.control(), "PrototypeConnections");
    assert!(connections.bounds().width > 500.0 && connections.bounds().height > 100.0);
    let select = named::<Button>(&view.control(), "PrototypeSelect10");
    assert_authoring_keys_stay_local(&select, &escaped);
    assert_authoring_keys_stay_local(
        &named::<ComboBox>(&view.control(), "PrototypeFlowChoice"),
        &escaped,
    );
    select.focus();
    assert!(key(&select, Key::Enter, false));
    window.update_layout();
    assert!(named::<Button>(&view.control(), "PrototypeSelect10").is_focused());
    assert!(has_named(&view.control(), "PrototypeLink0"));
    assert!(!has_named(&view.control(), "PrototypeLink1"));
    let old_remove = named::<Button>(&view.control(), "PrototypeRemoveLink0");
    view.update(&graph, &stages, 2);
    old_remove.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(events.borrow().is_empty());
    named::<Button>(&view.control(), "PrototypeEditLink0")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        *events.borrow(),
        [(PreviewPrototypeViewAction::EditLink { index: 0 }, 2)]
    );
    named::<CheckBox>(&view.control(), "PrototypeReachable").set_is_checked(Some(false));
    assert!(has_named(&view.control(), "PrototypeScreen30"));
    view.set_query("help.xaml");
    assert!(has_named(&view.control(), "PrototypeScreen30"));
    assert!(!has_named(&view.control(), "PrototypeScreen10"));
    view.set_query("shop.xaml");
    assert!(has_named(&view.control(), "PrototypeLink0"));
    assert!(has_named(&view.control(), "PrototypeLink1"));
    view.set_query("");
    view.select_flow(None);
    window.update_layout();
    assert!(!named::<Button>(&view.control(), "PrototypePlay").is_enabled());
    for id in [10, 20, 30] {
        let card = named::<Control>(&view.control(), &format!("PrototypeScreen{id}"));
        let map = named::<Control>(&view.control(), "PrototypeMap");
        let position = card.translate_point(Point::default(), &map).unwrap();
        assert!(
            position.x >= 0.0
                && position.y >= 0.0
                && position.x + card.bounds().width <= map.bounds().width
                && position.y + card.bounds().height <= map.bounds().height
        );
    }
    let second_document =
        Document::new("<Button xmlns='https://github.com/ferroui' Name='Settings'/>").unwrap();
    let mut branched = graph.clone();
    branched.links.push(PreviewPrototypeLink {
        name: "Open help".into(),
        source: 10,
        trigger: PreviewSourceAnchor::capture(&second_document, second_document.ast().root())
            .unwrap(),
        action: PreviewPrototypeAction::Navigate { stage: 30 },
    });
    view.update(&branched, &stages, 3);
    view.select_flow(Some(0));
    window.update_layout();
    let map = named::<Control>(&view.control(), "PrototypeMap");
    let viewport = named::<ScrollViewer>(&view.control(), "PrototypeMapScroll");
    assert!(
        viewport.viewport().height + 2.0 >= map.bounds().height,
        "A compact three-screen flow must show the full connector extent without nested scrolling: viewport={:?}, scroll bounds={:?}, diagram={:?}, extent={:?}",
        viewport.viewport(),
        viewport.bounds(),
        map.bounds(),
        viewport.extent()
    );
    let board = PreviewStageView::new(&Grid::new());
    let (parent, stage_keys) = key_trap_parent(board.control());
    window.set_content(Some(Control::boxed(parent.upcast::<Control>())));
    window.update_layout();
    assert_authoring_keys_stay_local(
        &named::<ComboBox>(&board.control(), "StageScope"),
        &stage_keys,
    );
    assert_authoring_keys_stay_local(
        &named::<Button>(&board.control(), "StagePrototype"),
        &stage_keys,
    );
    window.close();
}

#[test]
fn prototype_link_editor_never_retargets_a_missing_or_filtered_authored_hotspot() {
    let _app = UnitTestApplication::start(services());
    let document = Document::new("<Canvas xmlns='https://github.com/ferroui'><Button Name='Old'/><Button Name='Stable'/><Button Name='Other'/></Canvas>").unwrap();
    let anchor = |name| {
        PreviewSourceAnchor::capture(&document, document.ast().find_name(name).unwrap()).unwrap()
    };
    let stable = anchor("Stable");
    let other = anchor("Other");
    let stages = vec![
        stage(10, "First", "memory://first.xaml"),
        stage(20, "Second", "memory://second.xaml"),
        stage(30, "Alternate first", "memory://first.xaml"),
    ];
    let initial = PreviewPrototypeLink {
        name: "Existing interaction".into(),
        source: 10,
        trigger: anchor("Old"),
        action: PreviewPrototypeAction::Navigate { stage: 20 },
    };
    let elements = vec![
        PreviewPrototypeElementChoice {
            stage: 10,
            label: "Button · Stable".into(),
            anchor: stable.clone(),
        },
        PreviewPrototypeElementChoice {
            stage: 20,
            label: "Button · Other".into(),
            anchor: other,
        },
        PreviewPrototypeElementChoice {
            stage: 30,
            label: "Button · Stable".into(),
            anchor: stable.clone(),
        },
    ];
    let flow_editor = PreviewPrototypeFlowEditor::new(
        Some(PreviewPrototypeFlow {
            name: "Flow".into(),
            start: 10,
        }),
        stages.clone(),
    );
    let editor = PreviewPrototypeLinkEditor::new(Some(initial), None, stages, elements);
    let applied = Rc::new(RefCell::new(Vec::new()));
    let calls = applied.clone();
    editor.on_apply(move |link| {
        calls.borrow_mut().push(link);
        Ok(())
    });
    let (parent, escaped) = key_trap_parent(editor.control());
    let window = window(parent.upcast());
    let element = named::<ComboBox>(&editor.control(), "PrototypeLinkElement");
    let source = named::<ComboBox>(&editor.control(), "PrototypeLinkSource");
    assert_authoring_keys_stay_local(&source, &escaped);
    assert_eq!(element.selected_index(), -1);
    assert!(editor.apply().is_err());
    assert!(applied.borrow().is_empty());
    element.set_selected_index(0);
    assert_eq!(editor.value().unwrap().trigger, stable);
    source.set_selected_index(1);
    assert_eq!(
        element.selected_index(),
        -1,
        "A source change cannot silently choose the first unrelated hotspot"
    );
    assert!(editor.apply().is_err());
    source.set_selected_index(0);
    assert_eq!(element.selected_index(), -1);
    element.set_selected_index(0);
    source.set_selected_index(2);
    assert_eq!(
        element.selected_index(),
        0,
        "An explicitly chosen anchor survives when still eligible in the selected stage"
    );
    assert_eq!(editor.value().unwrap().trigger, stable);
    named::<ComboBox>(&editor.control(), "PrototypeLinkAction").set_selected_index(1);
    window.update_layout();
    let apply = named::<Button>(&editor.control(), "PrototypeLinkApply");
    let position = apply
        .translate_point(Point::default(), &editor.control())
        .unwrap();
    assert!(
        position.x >= 0.0
            && position.y >= 0.0
            && position.x + apply.bounds().width <= editor.control().bounds().width + 0.5
            && position.y + apply.bounds().height <= editor.control().bounds().height + 0.5,
        "The detached editor footer must remain within its native bounds"
    );
    editor.apply().unwrap();
    assert_eq!(applied.borrow().len(), 1);
    assert_eq!(applied.borrow()[0].source, 30);
    assert_eq!(applied.borrow()[0].action, PreviewPrototypeAction::Back);
    assert!(editor.apply().is_err());
    let (parent, flow_keys) = key_trap_parent(flow_editor.control());
    window.set_content(Some(Control::boxed(parent.upcast::<Control>())));
    window.update_layout();
    let start = named::<ComboBox>(&flow_editor.control(), "PrototypeFlowStart");
    assert_authoring_keys_stay_local(&start, &flow_keys);
    let closed = Rc::new(Cell::new(0));
    let close_count = closed.clone();
    flow_editor.on_close(move || close_count.set(close_count.get() + 1));
    assert!(key(&start, Key::Escape, false));
    assert_eq!(
        closed.get(),
        1,
        "The shared key guard must still allow the editor's Escape cancellation"
    );
    assert!(flow_editor.apply().is_err());
    window.close();
}
