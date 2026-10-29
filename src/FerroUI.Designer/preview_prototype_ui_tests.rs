use crate::{
    PreviewEnvironment, PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeElementChoice,
    PreviewPrototypeFlow, PreviewPrototypeFlowEditor, PreviewPrototypeHotspot,
    PreviewPrototypeLink, PreviewPrototypeLinkEditor, PreviewPrototypeOverlayOptions,
    PreviewPrototypeOverlayPlacement, PreviewPrototypePlayerAction, PreviewPrototypePlayerLayer,
    PreviewPrototypePlayerView, PreviewPrototypeView, PreviewPrototypeViewAction,
    PreviewSourceAnchor, PreviewStageConfiguration, PreviewStageInfo, PreviewStageSize,
    PreviewStageStatus, PreviewStageView,
};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers, TextInputEventArgs,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Point, Rect, Ref, Size};
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use ferroui_controls::{
    Border, Button, Canvas, CheckBox, ComboBox, Control, Grid, ScrollViewer, TextBlock, TextBox,
    Window,
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
    let name = named::<TextBox>(&editor.control(), "PrototypeLinkName");
    assert!(name.focus());
    assert!(key(&name, Key::Tab, false));
    assert!(source.is_keyboard_focus_within());
    assert!(key_with_modifiers(
        &source,
        Key::Tab,
        KeyModifiers::SHIFT,
        false
    ));
    assert!(name.is_keyboard_focus_within());
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
    let flow_name = named::<TextBox>(&flow_editor.control(), "PrototypeFlowName");
    assert!(flow_name.focus());
    assert!(key(&flow_name, Key::Tab, false));
    assert!(start.is_keyboard_focus_within());
    assert!(key_with_modifiers(
        &start,
        Key::Tab,
        KeyModifiers::SHIFT,
        false
    ));
    assert!(flow_name.is_keyboard_focus_within());
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

fn layer(
    stage: u64,
    host: Ref<Control>,
    bounds: Rect,
    overlay: Option<PreviewPrototypeOverlayOptions>,
) -> PreviewPrototypePlayerLayer {
    PreviewPrototypePlayerLayer {
        stage,
        host,
        bounds,
        overlay,
    }
}

fn overlay_content(name: &str, width: f64, height: f64) -> (Ref<Canvas>, Ref<TextBox>) {
    let host = Canvas::new();
    host.set_name(Some(name.into()));
    host.set_width(width);
    host.set_height(height);
    let input = TextBox::new();
    input.set_name(Some(format!("{name}Input")));
    input.set_width(140.0);
    input.set_height(32.0);
    input.set_text(Some("Retained input"));
    Canvas::set_left(&input, 10.0);
    Canvas::set_top(&input, 10.0);
    host.children().add(input.clone().upcast::<Control>());
    (host, input)
}

#[test]
fn prototype_overlay_keeps_underlying_controls_and_blocks_input_until_focus_is_restored() {
    let _app = UnitTestApplication::start(services());
    let view = PreviewPrototypePlayerView::new(&Grid::new());
    let (base, base_input) = overlay_content("Base", 400.0, 300.0);
    let (dialog, dialog_input) = overlay_content("Dialog", 200.0, 150.0);
    let base_action = Button::new();
    base_action.set_name(Some("BaseAction".into()));
    base_action.set_width(100.0);
    base_action.set_height(35.0);
    Canvas::set_left(&base_action, 20.0);
    Canvas::set_top(&base_action, 200.0);
    base.children().add(base_action.clone().upcast::<Control>());
    let native_calls = Rc::new(Cell::new(0));
    let calls = native_calls.clone();
    base_action.click(move |_, _| calls.set(calls.get() + 1));
    let commands = Rc::new(RefCell::new(Vec::new()));
    let captured = commands.clone();
    view.on_action(move |action, epoch, _| {
        captured.borrow_mut().push((action, epoch));
        Ok(())
    });
    let size = PreviewStageSize {
        width: 400.0,
        height: 300.0,
    };
    let base_layer = layer(
        1,
        base.clone().upcast(),
        Rect::new(0.0, 0.0, 400.0, 300.0),
        None,
    );
    let base_hotspot = PreviewPrototypeHotspot {
        link: 0,
        name: "Open dialog".into(),
        bounds: Rect::new(20.0, 200.0, 100.0, 35.0),
    };
    view.update_layers(
        size,
        &[base_layer.clone()],
        "Flow · Base",
        &[base_hotspot.clone()],
        false,
        1,
    )
    .unwrap();
    let window = window(view.control());
    base_input.set_text(Some("Original native value"));
    base_input.focus();
    let base_parent = base.visual_parent();
    let overlay = layer(
        2,
        dialog.clone().upcast(),
        Rect::new(100.0, 75.0, 200.0, 150.0),
        Some(PreviewPrototypeOverlayOptions {
            dim_background: false,
            ..Default::default()
        }),
    );
    let close = PreviewPrototypeHotspot {
        link: 1,
        name: "Close dialog".into(),
        bounds: Rect::new(20.0, 100.0, 100.0, 30.0),
    };
    view.update_layers(
        size,
        &[base_layer.clone(), overlay.clone()],
        "Flow · Dialog",
        &[close.clone()],
        true,
        2,
    )
    .unwrap();
    window.update_layout();
    assert_eq!(
        base.visual_parent(),
        base_parent,
        "Opening an overlay must keep the native base mounted"
    );
    assert!(
        base.is_effectively_enabled(),
        "Modal input containment must not disable application controls"
    );
    assert!(dialog_input.is_focused());
    assert!(key(&dialog_input, Key::Tab, false));
    let next = window.focus_manager().get_focused_element().unwrap();
    assert_ne!(next, dialog_input.clone().upcast::<InputElement>());
    let plane = named::<Control>(&view.control(), "PrototypePlayerPlane2");
    for modifiers in [
        KeyModifiers::NONE,
        KeyModifiers::SHIFT,
        KeyModifiers::NONE,
        KeyModifiers::NONE,
    ] {
        let focused = window.focus_manager().get_focused_element().unwrap();
        assert!(plane.is_visual_ancestor_of(&focused));
        assert!(key_with_modifiers(
            &focused.cast::<Control>().unwrap(),
            Key::Tab,
            modifiers,
            false
        ));
        let focused = window.focus_manager().get_focused_element().unwrap();
        assert!(
            plane.is_visual_ancestor_of(&focused),
            "Native Tab navigation must cycle inside the active modal plane"
        );
    }
    dialog_input.focus();
    assert_eq!(
        dialog.translate_point(Point::default(), &base),
        Some(Point::new(100.0, 75.0))
    );
    let hit = named::<Button>(&view.control(), "PrototypeHotspot1");
    assert_eq!(
        hit.translate_point(Point::default(), &dialog),
        Some(Point::new(20.0, 100.0))
    );
    assert!(key(&base_action, Key::Enter, false));
    assert_eq!(
        native_calls.get(),
        0,
        "Tunneled input from a retained background control must be rejected"
    );
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    pointer.capture(Some(&base_action.clone().upcast::<InputElement>()));
    let captured_release = PointerReleasedEventArgs::new(
        base_action.to_ref(),
        pointer.clone(),
        &*window,
        Point::new(10.0, 10.0),
        4,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Left,
    );
    base_action.raise_event(&captured_release);
    assert!(captured_release.handled());
    assert!(
        pointer.captured().is_none(),
        "Covered application pointer capture must be retired"
    );
    assert_eq!(native_calls.get(), 0);
    let mut text = TextInputEventArgs::new();
    text.text = Some("blocked".into());
    text.set_routed_event(Some(InputElement::text_input_event()));
    text.set_source(base_input.clone());
    base_input.raise_event(&text);
    assert!(text.handled());
    assert_eq!(base_input.text().as_deref(), Some("Original native value"));
    base_input.focus();
    assert!(
        !base_input.is_focused(),
        "Programmatic focus must not enter a blocked native screen"
    );
    assert!(dialog_input.is_focused());
    dialog_input.set_text(Some("Dialog state"));
    let scrim = named::<Border>(&view.control(), "PrototypeOverlayScrim2");
    let inside_blank = dialog
        .translate_point(Point::new(175.0, 75.0), &window)
        .unwrap();
    let inside_target = window.input_hit_test(inside_blank).unwrap();
    assert!(
        inside_target == plane.clone().upcast::<InputElement>()
            || plane.is_visual_ancestor_of(&inside_target),
        "Blank space inside a transparent overlay must remain inside its native plane"
    );
    assert_ne!(
        inside_target,
        scrim.clone().upcast::<InputElement>(),
        "Blank space inside a transparent overlay must not hit the outside-dismiss scrim"
    );
    let outside = base
        .translate_point(Point::new(50.0, 210.0), &window)
        .unwrap();
    let target = window.input_hit_test(outside).unwrap();
    assert_eq!(
        target,
        scrim.clone().upcast::<InputElement>(),
        "Even a transparent scrim must own outside input"
    );
    pointer_click(&scrim, &window, outside, outside);
    assert!(commands.borrow().is_empty(), "Outside dismissal is opt-in");
    assert_eq!(native_calls.get(), 0);
    assert!(key(&dialog_input, Key::Escape, false));
    assert_eq!(
        *commands.borrow(),
        [(PreviewPrototypePlayerAction::CloseOverlay, 2)]
    );
    let mut dismissible = overlay;
    dismissible.overlay.as_mut().unwrap().dismiss_on_outside = true;
    view.update_layers(
        size,
        &[base_layer.clone(), dismissible],
        "Flow · Dialog",
        &[close],
        true,
        3,
    )
    .unwrap();
    window.update_layout();
    hit.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        commands.borrow().len(),
        1,
        "The previous overlay frame's hotspot is retired"
    );
    pointer_click(&scrim, &window, outside, outside);
    assert_eq!(
        commands.borrow().last(),
        Some(&(PreviewPrototypePlayerAction::DismissOverlay, 3))
    );
    view.update_layers(
        size,
        &[base_layer],
        "Flow · Base",
        &[base_hotspot],
        false,
        4,
    )
    .unwrap();
    window.update_layout();
    assert_eq!(base.visual_parent(), base_parent);
    assert!(
        base_input.is_focused(),
        "Closing the overlay restores the original native input focus"
    );
    assert_eq!(base_input.text().as_deref(), Some("Original native value"));
    assert_eq!(dialog_input.text().as_deref(), Some("Dialog state"));
    let count = commands.borrow().len();
    pointer_click(&scrim, &window, outside, outside);
    assert_eq!(
        commands.borrow().len(),
        count,
        "A detached scrim cannot dismiss a replacement frame"
    );
    window.close();
}

#[test]
fn prototype_nested_overlays_retain_planes_restore_hotspot_focus_and_reject_duplicate_hosts() {
    let _app = UnitTestApplication::start(services());
    let view = PreviewPrototypePlayerView::new(&Grid::new());
    let (base, _) = overlay_content("Screen", 400.0, 300.0);
    let (outer, outer_input) = overlay_content("Outer", 240.0, 180.0);
    let (inner, inner_input) = overlay_content("Inner", 180.0, 100.0);
    let size = PreviewStageSize {
        width: 400.0,
        height: 300.0,
    };
    let a = layer(10, base.upcast(), Rect::new(0.0, 0.0, 400.0, 300.0), None);
    let b = layer(
        20,
        outer.clone().upcast(),
        Rect::new(80.0, 60.0, 240.0, 180.0),
        Some(Default::default()),
    );
    let c = layer(
        30,
        inner.clone().upcast(),
        Rect::new(110.0, 100.0, 180.0, 100.0),
        Some(Default::default()),
    );
    let opener = PreviewPrototypeHotspot {
        link: 2,
        name: "Open nested overlay".into(),
        bounds: Rect::new(20.0, 90.0, 150.0, 40.0),
    };
    view.on_action(|_, _, _| Ok(()));
    view.update_layers(
        size,
        &[a.clone(), b.clone()],
        "Outer",
        &[opener.clone()],
        true,
        1,
    )
    .unwrap();
    let window = window(view.control());
    let old_hotspot = named::<Button>(&view.control(), "PrototypeHotspot2");
    old_hotspot.focus();
    outer_input.set_text(Some("Retained outer"));
    let parent = outer.visual_parent();
    view.update_layers(
        size,
        &[a.clone(), b.clone(), c.clone()],
        "Inner",
        &[],
        true,
        2,
    )
    .unwrap();
    window.update_layout();
    assert_eq!(outer.visual_parent(), parent);
    assert!(inner_input.is_focused());
    assert!(!named::<Control>(&view.control(), "PrototypePlayerLayer20").is_hit_test_visible());
    assert!(named::<Control>(&view.control(), "PrototypePlayerLayer30").is_hit_test_visible());
    let mut duplicate = c.clone();
    duplicate.stage = 40;
    assert!(view
        .update_layers(
            size,
            &[a.clone(), b.clone(), c, duplicate],
            "Invalid",
            &[],
            true,
            3
        )
        .is_err());
    assert!(inner_input.is_focused());
    assert!(has_named(&view.control(), "PrototypePlayerLayer30"));
    view.update_layers(size, &[a, b], "Outer", &[opener], true, 4)
        .unwrap();
    window.update_layout();
    assert_eq!(outer_input.text().as_deref(), Some("Retained outer"));
    assert_eq!(outer.visual_parent(), parent);
    let restored = named::<Button>(&view.control(), "PrototypeHotspot2");
    assert_ne!(restored, old_hotspot);
    assert!(
        restored.is_focused(),
        "Closing a nested overlay restores its rebuilt opener hotspot"
    );
    assert!(named::<Control>(&view.control(), "PrototypePlayerLayer20").is_hit_test_visible());
    view.clear();
    assert!(outer.visual_parent().is_none());
    assert!(inner.visual_parent().is_none());
    window.close();
}

#[test]
fn prototype_overlay_focus_survives_native_preparation_and_rebuilt_source_hotspots() {
    let _app = UnitTestApplication::start(services());
    let view = PreviewPrototypePlayerView::new(&Grid::new());
    let (base, _) = overlay_content("Base", 400.0, 300.0);
    let (outer, outer_input) = overlay_content("Outer", 240.0, 180.0);
    let (inner, inner_input) = overlay_content("Inner", 180.0, 100.0);
    let source = Button::new();
    source.set_name(Some("NativeOpener".into()));
    source.set_width(150.0);
    source.set_height(40.0);
    Canvas::set_left(&source, 20.0);
    Canvas::set_top(&source, 90.0);
    outer.children().add(source.clone().upcast::<Control>());
    let native_calls = Rc::new(Cell::new(0));
    let count = native_calls.clone();
    source.click(move |_, _| count.set(count.get() + 1));
    let size = PreviewStageSize {
        width: 400.0,
        height: 300.0,
    };
    let a = layer(10, base.upcast(), Rect::new(0.0, 0.0, 400.0, 300.0), None);
    let b = layer(
        20,
        outer.upcast(),
        Rect::new(80.0, 60.0, 240.0, 180.0),
        Some(Default::default()),
    );
    let c = layer(
        30,
        inner.upcast(),
        Rect::new(110.0, 100.0, 180.0, 100.0),
        Some(Default::default()),
    );
    let opener = PreviewPrototypeHotspot {
        link: 2,
        name: "Open nested overlay".into(),
        bounds: Rect::new(20.0, 90.0, 150.0, 40.0),
    };
    let actions = Rc::new(RefCell::new(Vec::new()));
    let captured = actions.clone();
    view.on_action(move |action, _, _| {
        captured.borrow_mut().push(action);
        Ok(())
    });
    let native_targets = [(2, source.clone().upcast::<InputElement>())];
    view.update_layers(
        size,
        &[a.clone(), b.clone()],
        "Outer",
        &[opener.clone()],
        true,
        1,
    )
    .unwrap();
    let window = window(view.control());

    // A native source control may already have focus when a refreshed source
    // map first associates it with a prototype interaction. It must restore to
    // the current hotspot, so Enter follows the prototype action path.
    assert!(source.focus());
    view.set_hotspot_focus_targets(&native_targets);
    view.prepare_layers(size, &[a.clone(), b.clone()], "Outer", &[], true, 2)
        .unwrap();
    window.update_layout();
    view.set_hotspot_focus_targets(&native_targets);
    view.update_layers(
        size,
        &[a.clone(), b.clone()],
        "Outer",
        &[opener.clone()],
        true,
        3,
    )
    .unwrap();
    assert!(named::<Button>(&view.control(), "PrototypeHotspot2").is_focused());
    assert!(!source.is_focused());
    assert!(
        !source.focus(),
        "The source control must redirect focus to its interaction"
    );
    let focused = window.focus_manager().get_focused_element().unwrap();
    assert!(key(&focused.cast::<Control>().unwrap(), Key::Enter, false));
    assert_eq!(
        *actions.borrow(),
        [PreviewPrototypePlayerAction::Activate { link: 2 }]
    );
    assert_eq!(native_calls.get(), 0);
    assert!(key_with_modifiers(
        &focused.cast::<Control>().unwrap(),
        Key::Tab,
        KeyModifiers::SHIFT,
        false
    ));
    assert!(
        outer_input.is_focused(),
        "Shift+Tab must pass the native alias of the focused hotspot"
    );
    assert!(key(&outer_input, Key::Tab, false));
    let hit = named::<Button>(&view.control(), "PrototypeHotspot2");
    assert!(hit.is_focused());
    assert!(key(&hit, Key::Tab, false));
    assert!(
        outer_input.is_focused(),
        "Forward Tab must still cycle inside the active overlay"
    );

    for (index, command) in [false, true].into_iter().enumerate() {
        let name = if command {
            "PrototypeActivate2"
        } else {
            "PrototypeHotspot2"
        };
        let opener_control = named::<Button>(&view.control(), name);
        assert!(opener_control.focus());
        let epoch = 10 + index as u64 * 10;
        view.prepare_layers(
            size,
            &[a.clone(), b.clone(), c.clone()],
            "Inner",
            &[],
            true,
            epoch,
        )
        .unwrap();
        window.update_layout();
        view.set_hotspot_focus_targets(&[]);
        view.update_layers(
            size,
            &[a.clone(), b.clone(), c.clone()],
            "Inner",
            &[],
            true,
            epoch + 1,
        )
        .unwrap();
        assert!(inner_input.is_focused());

        view.prepare_layers(size, &[a.clone(), b.clone()], "Outer", &[], true, epoch + 2)
            .unwrap();
        window.update_layout();
        // Layout can require multiple preparations before the final source map.
        // The empty provisional hotspot set must not replace the saved opener.
        view.prepare_layers(size, &[a.clone(), b.clone()], "Outer", &[], true, epoch + 2)
            .unwrap();
        window.update_layout();
        view.set_hotspot_focus_targets(&native_targets);
        view.update_layers(
            size,
            &[a.clone(), b.clone()],
            "Outer",
            &[opener.clone()],
            true,
            epoch + 3,
        )
        .unwrap();
        let restored = named::<Button>(&view.control(), name);
        assert_ne!(restored, opener_control);
        assert!(
            restored.is_focused(),
            "The exact opener kind must regain focus"
        );
    }
    window.close();
}

#[test]
fn prototype_overlay_close_falls_back_from_a_filtered_command_to_its_hotspot() {
    let _app = UnitTestApplication::start(services());
    let view = PreviewPrototypePlayerView::new(&Grid::new());
    let (base, base_input) = overlay_content("Base", 400.0, 300.0);
    let (dialog, dialog_input) = overlay_content("Dialog", 200.0, 150.0);
    let size = PreviewStageSize {
        width: 400.0,
        height: 300.0,
    };
    let a = layer(1, base.upcast(), Rect::new(0.0, 0.0, 400.0, 300.0), None);
    let b = layer(
        2,
        dialog.upcast(),
        Rect::new(100.0, 75.0, 200.0, 150.0),
        Some(Default::default()),
    );
    let opener = PreviewPrototypeHotspot {
        link: 7,
        name: "Open dialog".into(),
        bounds: Rect::new(20.0, 200.0, 100.0, 35.0),
    };
    view.update_layers(size, &[a.clone()], "Base", &[opener.clone()], false, 1)
        .unwrap();
    let window = window(view.control());
    assert!(named::<Button>(&view.control(), "PrototypeActivate7").focus());
    view.update_layers(size, &[a.clone(), b.clone()], "Dialog", &[], true, 2)
        .unwrap();
    assert!(dialog_input.is_focused());
    named::<TextBox>(&view.control(), "PrototypePlayerSearch").set_text(Some("unmatched"));
    view.update_layers(size, &[a.clone()], "Base", &[opener], false, 3)
        .unwrap();
    assert!(!named::<Button>(&view.control(), "PrototypeActivate7").is_visible());
    assert!(named::<Button>(&view.control(), "PrototypeHotspot7").is_focused());

    // Removing the interaction entirely must restore usable native input rather
    // than leaving focus on a detached button from the previous frame.
    view.update_layers(size, &[a.clone(), b], "Dialog", &[], true, 4)
        .unwrap();
    assert!(dialog_input.is_focused());
    view.update_layers(size, &[a], "Base", &[], false, 5)
        .unwrap();
    assert!(base_input.is_focused());
    window.close();
}

#[test]
fn prototype_overlay_editor_preserves_detached_options_and_rejects_invalid_manual_positions() {
    let _app = UnitTestApplication::start(services());
    let document =
        Document::new("<Button xmlns='https://github.com/ferroui' Name='Open'/>").unwrap();
    let anchor = PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap();
    let options = PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::BottomCenter,
        ..Default::default()
    };
    let initial = PreviewPrototypeLink {
        name: "Open sheet".into(),
        source: 10,
        trigger: anchor.clone(),
        action: PreviewPrototypeAction::OpenOverlay { stage: 20, options },
    };
    let stages = vec![
        stage(10, "Home", "memory://home.xaml"),
        stage(20, "Sheet", "memory://sheet.xaml"),
    ];
    let elements = vec![PreviewPrototypeElementChoice {
        stage: 10,
        label: "Open".into(),
        anchor,
    }];
    let editor = PreviewPrototypeLinkEditor::new(
        Some(initial.clone()),
        None,
        stages.clone(),
        elements.clone(),
    );
    assert_eq!(editor.value().unwrap(), initial);
    let root = editor.control();
    let action = named::<ComboBox>(&root, "PrototypeLinkAction");
    let placement = named::<ComboBox>(&root, "PrototypeLinkOverlayPlacement");
    assert_eq!(action.selected_index(), 2);
    assert_eq!(placement.selected_index(), 7);
    assert!(named::<Control>(&root, "PrototypeLinkOverlayOptions").is_visible());
    assert!(!named::<Control>(&root, "PrototypeLinkOverlayManual").is_visible());
    placement.set_selected_index(9);
    assert!(named::<Control>(&root, "PrototypeLinkOverlayManual").is_visible());
    let x = named::<TextBox>(&root, "PrototypeLinkOverlayX");
    let y = named::<TextBox>(&root, "PrototypeLinkOverlayY");
    x.set_text(Some("-18"));
    y.set_text(Some("31"));
    named::<CheckBox>(&root, "PrototypeLinkOverlayDismiss").set_is_checked(Some(true));
    named::<CheckBox>(&root, "PrototypeLinkOverlayDim").set_is_checked(Some(false));
    let expected = PreviewPrototypeOverlayOptions {
        placement: PreviewPrototypeOverlayPlacement::Manual { x: -18, y: 31 },
        dismiss_on_outside: true,
        dim_background: false,
    };
    assert_eq!(
        editor.value().unwrap().action,
        PreviewPrototypeAction::OpenOverlay {
            stage: 20,
            options: expected
        }
    );
    action.set_selected_index(3);
    assert_eq!(
        editor.value().unwrap().action,
        PreviewPrototypeAction::SwapOverlay {
            stage: 20,
            options: expected
        }
    );
    let applied = Rc::new(RefCell::new(Vec::new()));
    let captured = applied.clone();
    editor.on_apply(move |value| {
        captured.borrow_mut().push(value);
        Ok(())
    });
    x.set_text(Some("1.5"));
    assert!(editor.apply().unwrap_err().contains("whole horizontal"));
    assert!(applied.borrow().is_empty());
    x.set_text(Some("16385"));
    assert!(editor.apply().is_err());
    assert!(applied.borrow().is_empty());
    action.set_selected_index(4);
    assert_eq!(
        editor.value().unwrap().action,
        PreviewPrototypeAction::CloseOverlay
    );
    assert!(!named::<Control>(&root, "PrototypeLinkOverlayOptions").is_visible());
    editor.apply().unwrap();
    assert_eq!(
        applied.borrow()[0].action,
        PreviewPrototypeAction::CloseOverlay
    );
    assert!(editor.apply().is_err());
    let mut missing = initial;
    missing.action = PreviewPrototypeAction::OpenOverlay { stage: 99, options };
    let detached = PreviewPrototypeLinkEditor::new(Some(missing), None, stages, elements);
    assert_eq!(
        named::<ComboBox>(&detached.control(), "PrototypeLinkDestination").selected_index(),
        -1
    );
    assert!(detached.value().unwrap_err().contains("Choose an overlay"));
}

#[test]
fn prototype_overview_shows_overlay_reachability_and_distinct_action_descriptions() {
    let _app = UnitTestApplication::start(services());
    let document =
        Document::new("<Button xmlns='https://github.com/ferroui' Name='Open'/>").unwrap();
    let trigger = PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap();
    let graph = PreviewPrototype {
        flows: vec![PreviewPrototypeFlow {
            name: "Review".into(),
            start: 10,
        }],
        links: vec![
            PreviewPrototypeLink {
                name: "Open details".into(),
                source: 10,
                trigger: trigger.clone(),
                action: PreviewPrototypeAction::OpenOverlay {
                    stage: 20,
                    options: Default::default(),
                },
            },
            PreviewPrototypeLink {
                name: "Replace details".into(),
                source: 20,
                trigger: trigger.clone(),
                action: PreviewPrototypeAction::SwapOverlay {
                    stage: 30,
                    options: Default::default(),
                },
            },
            PreviewPrototypeLink {
                name: "Dismiss review".into(),
                source: 30,
                trigger,
                action: PreviewPrototypeAction::CloseOverlay,
            },
        ],
    };
    let stages = vec![
        stage(10, "Home", "memory://home.xaml"),
        stage(20, "Details", "memory://details.xaml"),
        stage(30, "Review", "memory://review.xaml"),
    ];
    let view = PreviewPrototypeView::new(&Grid::new());
    view.update(&graph, &stages, 1);
    for id in [10, 20, 30] {
        assert!(has_named(&view.control(), &format!("PrototypeScreen{id}")));
    }
    let descriptions = view
        .control()
        .get_logical_descendants()
        .filter_map(|control| control.cast::<TextBlock>())
        .filter_map(|text| text.text())
        .collect::<Vec<_>>();
    assert!(descriptions
        .iter()
        .any(|text| text.contains("Open overlay Details")));
    assert!(descriptions
        .iter()
        .any(|text| text.contains("Replace overlay with Review")));
    assert!(descriptions
        .iter()
        .any(|text| text.contains("Close top overlay")));
    view.set_query("replace overlay");
    assert!(has_named(&view.control(), "PrototypeLink1"));
    assert!(!has_named(&view.control(), "PrototypeLink0"));
}
