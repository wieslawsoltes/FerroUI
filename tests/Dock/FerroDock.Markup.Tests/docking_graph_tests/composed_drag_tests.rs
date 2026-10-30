//! Real pointer input through the source DockControl state and original docking engine.
use super::*;
use ferrodock_ferro::controls::HostWindow;
use ferrodock_model::{controls::IRootDock, core::IDockSerializer};
use ferroui_base::{
    input::{MouseButton, RawInputModifiers},
    Point,
};
use ferroui_controls::Control;
use ferroui_headless::HeadlessWindowExtensions;

fn tab(scene: &Scene, dock: &str, index: i32) -> Ref<Control> {
    scene
        .control
        .find_descendant_of_type_where::<DocumentTabStrip>(false, |strip| {
            strip
                .data_context()
                .and_then(|value| {
                    use ferroui_base::data::core::{ValueType, ValueTypes};
                    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDockable>>())
                        .and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
                })
                .is_some_and(|v| v.id() == dock)
        })
        .unwrap()
        .container_from_index(index)
        .unwrap()
}
fn center(control: &Control, window: &Window) -> Point {
    assert!(
        control.bounds().width > 0.0 && control.bounds().height > 0.0,
        "tab must be realized"
    );
    control
        .translate_point(
            Point::new(control.bounds().width / 2.0, control.bounds().height / 2.0),
            window,
        )
        .unwrap()
}
fn drag(window: &Window, control: &DockControl, start: Point, end: Point) {
    window.mouse_move(start, RawInputModifiers::NONE);
    window.mouse_down(start, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_move(
        Point::new(start.x, start.y + 60.0),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    drain(window);
    assert!(
        control.is_dragging_dock(),
        "moving outside the strip must activate the original dock state"
    );
    window.mouse_move(end, RawInputModifiers::LEFT_MOUSE_BUTTON);
    drain(window);
    window.mouse_move(
        Point::new(end.x + 1.0, end.y),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    drain(window);
    window.mouse_up(
        Point::new(end.x + 1.0, end.y),
        MouseButton::Left,
        RawInputModifiers::NONE,
    );
    drain(window);
    assert!(!control.is_dragging_dock());
}
fn concrete<T: 'static>(value: Rc<dyn IDockable>) -> Rc<T> {
    value
        .boxed_reference()
        .downcast_ref::<Rc<T>>()
        .unwrap()
        .clone()
}
fn model(control: &Control) -> Option<Rc<dyn IDockable>> {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    control.data_context().and_then(|value| {
        ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDockable>>())
            .and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
    })
}
fn check_selector_split_and_resize(document: &str) {
    use ferrodock_controls_proportional_stack_panel::ProportionalStackPanelSplitter;
    use ferrodock_ferro::controls::{DockTarget, ToolDockControl};
    ferrodock_ferro_themes_browser::register_types();
    let scene = Scene::from_document(false, document);
    let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer().unwrap();
    let wire = serializer
        .serialize(scene.root.clone() as Rc<dyn IRootDock>)
        .unwrap();
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    scene.control.set_layout(Some(root.clone()));
    drain(&scene.window);
    let documents = concrete::<DocumentDock>(
        scene
            .factory
            .find_dockable(
                Some(root.clone()),
                Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Documents"))),
            )
            .unwrap(),
    );
    let tools = concrete::<ToolDock>(
        scene
            .factory
            .find_dockable(
                Some(root.clone()),
                Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Tools"))),
            )
            .unwrap(),
    );
    let dragged = documents.visible_dockables().unwrap().get(0).unwrap();
    let source = tab(&scene, "Documents", 0);
    let body = scene
        .control
        .find_descendant_of_type_where::<DocumentControl>(false, |v| {
            model(v).is_some_and(|m| m.id() == "Documents")
        })
        .unwrap();
    let start = center(&source, &scene.window);
    let over = center(&body, &scene.window);
    scene.window.mouse_move(start, RawInputModifiers::NONE);
    scene
        .window
        .mouse_down(start, MouseButton::Left, RawInputModifiers::NONE);
    scene
        .window
        .mouse_move(over, RawInputModifiers::LEFT_MOUSE_BUTTON);
    drain(&scene.window);
    assert!(scene.control.is_dragging_dock());
    let target = scene
        .window
        .find_descendant_of_type::<DockTarget>(false)
        .expect("actual local dock adorner");
    let selector = target
        .get_selector(DockOperation::Right)
        .expect("rendered right selector");
    assert!(selector.is_visible());
    let point = center(&selector, &scene.window);
    scene
        .window
        .mouse_move(point, RawInputModifiers::LEFT_MOUSE_BUTTON);
    drain(&scene.window);
    scene.window.mouse_move(
        Point::new(point.x + 1.0, point.y),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    drain(&scene.window);
    scene.window.mouse_up(
        Point::new(point.x + 1.0, point.y),
        MouseButton::Left,
        RawInputModifiers::NONE,
    );
    drain(&scene.window);
    assert!(!scene.control.is_dragging_dock());
    assert!(
        target.visual_root().is_none(),
        "drop must remove its adorner"
    );
    assert_eq!(documents.visible_dockables().unwrap().count(), 1);
    assert_eq!(
        documents.visible_dockables().unwrap().get(0).unwrap().id(),
        "Doc2"
    );
    let new_owner = dragged.owner().unwrap();
    assert!(!Rc::ptr_eq(
        &new_owner,
        &(documents.clone() as Rc<dyn IDockable>)
    ));
    let new_dock = new_owner.clone().as_document_dock().unwrap();
    assert_eq!(new_dock.visible_dockables().unwrap().count(), 1);
    assert!(Rc::ptr_eq(&new_dock.active_dockable().unwrap(), &dragged));
    assert!(Rc::ptr_eq(
        &dragged.factory().unwrap(),
        &(scene.factory.clone() as Rc<dyn IFactory>)
    ));
    assert!(
        root.windows().is_none_or(|w| w.count() == 0),
        "selector drop must split instead of float"
    );
    let tool_control = scene
        .control
        .find_descendant_of_type_where::<ToolDockControl>(false, |v| {
            model(v).is_some_and(|m| m.id() == "Tools")
        })
        .unwrap();
    let splitter = scene
        .control
        .find_descendant_of_type::<ProportionalStackPanelSplitter>(false)
        .unwrap();
    let before_width = tool_control.bounds().width;
    let before_tools = tools.proportion();
    let before_documents = documents.proportion();
    let unchanged = new_owner.proportion();
    let point = center(&splitter, &scene.window);
    scene.window.mouse_move(point, RawInputModifiers::NONE);
    scene
        .window
        .mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
    scene.window.mouse_move(
        Point::new(point.x + 24.0, point.y),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    scene.window.mouse_up(
        Point::new(point.x + 24.0, point.y),
        MouseButton::Left,
        RawInputModifiers::NONE,
    );
    drain(&scene.window);
    assert!(tools.proportion() > before_tools);
    assert!(documents.proportion() < before_documents);
    assert!((new_owner.proportion() - unchanged).abs() < 1e-9);
    assert!((tool_control.bounds().width - before_width - 24.0).abs() < 1.1);
    let wire = serializer.serialize(root.clone()).unwrap();
    let restored = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(restored).unwrap(), wire);
}

#[test]
fn fluent_rendered_selector_splits_and_native_splitter_updates_restored_model_and_geometry() {
    run(|| check_selector_split_and_resize("DockingGraphConsumer.xaml"));
}
#[test]
fn browser_rendered_selector_splits_and_native_splitter_updates_restored_model_and_geometry() {
    run(|| check_selector_split_and_resize("BrowserDockingGraphConsumer.xaml"));
}
fn click_control(window: &Window, control: &Control) {
    assert!(control.is_visible() && control.is_effectively_enabled());
    let point = center(control, window);
    window.mouse_move(point, RawInputModifiers::NONE);
    window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
    drain(window);
}
fn tool_chrome(scene: &Scene, id: &str) -> Ref<ferrodock_ferro::controls::ToolChromeControl> {
    scene
        .control
        .find_descendant_of_type_where::<ferrodock_ferro::controls::ToolChromeControl>(false, |v| {
            model(v)
                .and_then(|v| v.as_dock())
                .and_then(|v| v.active_dockable())
                .is_some_and(|v| v.id() == id)
        })
        .unwrap()
}
fn chrome_button(scene: &Scene, id: &str, name: &str) -> Ref<ferroui_controls::Button> {
    tool_chrome(scene, id)
        .find_descendant_of_type_where::<ferroui_controls::Button>(false, |v| {
            v.name().as_deref() == Some(name)
        })
        .unwrap()
}
fn select_tool(scene: &Scene, tools: &ToolDock, tool: Rc<dyn IDockable>) {
    let index = i32::try_from(
        tools
            .visible_dockables()
            .unwrap()
            .index_of(&Some(tool))
            .unwrap(),
    )
    .unwrap();
    let strip = scene
        .control
        .find_descendant_of_type::<ToolTabStrip>(false)
        .unwrap();
    let item = strip.container_from_index(index).unwrap();
    click_control(&scene.window, &item);
}
fn reorder_tabs(scene: &Scene, strip: &ferroui_controls::ItemsControl, dock: Rc<dyn IDock>) {
    let first = dock.visible_dockables().unwrap().get(0).unwrap();
    let second = dock.visible_dockables().unwrap().get(1).unwrap();
    let start = center(&strip.container_from_index(0).unwrap(), &scene.window);
    let end = center(&strip.container_from_index(1).unwrap(), &scene.window);
    scene.window.mouse_move(start, RawInputModifiers::NONE);
    scene
        .window
        .mouse_down(start, MouseButton::Left, RawInputModifiers::NONE);
    scene
        .window
        .mouse_move(end, RawInputModifiers::LEFT_MOUSE_BUTTON);
    drain(&scene.window);
    assert!(!scene.control.is_dragging_dock());
    assert!(Rc::ptr_eq(
        &dock.visible_dockables().unwrap().get(0).unwrap(),
        &first
    ));
    scene
        .window
        .mouse_up(end, MouseButton::Left, RawInputModifiers::NONE);
    drain(&scene.window);
    assert!(Rc::ptr_eq(
        &dock.visible_dockables().unwrap().get(0).unwrap(),
        &second
    ));
    assert!(Rc::ptr_eq(
        &dock.visible_dockables().unwrap().get(1).unwrap(),
        &first
    ));
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &first));
}
fn check_tools(document: &str) {
    use ferrodock_ferro::controls::ToolPinItemControl;
    ferrodock_ferro_themes_browser::register_types();
    let scene = Scene::from_document(false, document);
    scene.tools.set_alignment(Alignment::Left);
    let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer().unwrap();
    let wire = serializer
        .serialize(scene.root.clone() as Rc<dyn IRootDock>)
        .unwrap();
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    scene.control.set_layout(Some(root.clone()));
    drain(&scene.window);
    let tools = concrete::<ToolDock>(
        scene
            .factory
            .find_dockable(
                Some(root.clone()),
                Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Tools"))),
            )
            .unwrap(),
    );
    let tool = tools.visible_dockables().unwrap().get(0).unwrap();
    let strip = scene
        .control
        .find_descendant_of_type::<ToolTabStrip>(false)
        .unwrap();
    reorder_tabs(&scene, &strip, tools.clone());
    assert_eq!(strip.selected_index(), 1);
    let pin = chrome_button(&scene, "Tool1", "PART_PinButton");
    click_control(&scene.window, &pin);
    assert_eq!(tools.visible_dockables().unwrap().count(), 1);
    assert!(root
        .left_pinned_dockables()
        .unwrap()
        .contains(&Some(tool.clone())));
    let wire = serializer.serialize(root.clone()).unwrap();
    let saved = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    assert_eq!(saved.left_pinned_dockables().unwrap().count(), 1);
    assert_eq!(serializer.serialize(saved).unwrap(), wire);
    let pinned = scene
        .control
        .find_descendant_of_type_where::<ToolPinItemControl>(false, |v| {
            model(v).is_some_and(|v| v.id() == "Tool1")
        })
        .unwrap();
    let preview = pinned
        .find_descendant_of_type::<ferroui_controls::Button>(false)
        .unwrap();
    click_control(&scene.window, &preview);
    assert!(Rc::ptr_eq(
        &root.pinned_dock().unwrap().active_dockable().unwrap(),
        &tool
    ));
    let unpin = chrome_button(&scene, "Tool1", "PART_PinButton");
    click_control(&scene.window, &unpin);
    assert!(root.left_pinned_dockables().is_none_or(|v| v.count() == 0));
    assert!(tools
        .visible_dockables()
        .unwrap()
        .contains(&Some(tool.clone())));
    assert!(Rc::ptr_eq(
        &tool.owner().unwrap(),
        &(tools.clone() as Rc<dyn IDockable>)
    ));
    scene.factory.hide_dockable(Some(tool.clone()));
    drain(&scene.window);
    assert_eq!(tools.visible_dockables().unwrap().count(), 1);
    assert!(root
        .hidden_dockables()
        .unwrap()
        .contains(&Some(tool.clone())));
    assert!(Rc::ptr_eq(
        &tool.original_owner().unwrap(),
        &(tools.clone() as Rc<dyn IDockable>)
    ));
    let wire = serializer.serialize(root.clone()).unwrap();
    let saved = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    assert_eq!(saved.hidden_dockables().unwrap().count(), 1);
    // The unchanged source resolver honors IgnoreDataMember on OriginalOwner.
    assert!(!wire.contains("\"OriginalOwner\""));
    assert!(saved
        .hidden_dockables()
        .unwrap()
        .get(0)
        .unwrap()
        .original_owner()
        .is_none());
    assert_eq!(serializer.serialize(saved).unwrap(), wire);
    scene.factory.restore_dockable(Some(tool.clone()));
    drain(&scene.window);
    assert!(root.hidden_dockables().is_none_or(|v| v.count() == 0));
    assert_eq!(tools.visible_dockables().unwrap().count(), 2);
    assert!(tool.original_owner().is_none());
    select_tool(&scene, &tools, tool.clone());
    assert!(Rc::ptr_eq(&tools.active_dockable().unwrap(), &tool));
    scene.factory.set_hide_tools_on_close(true);
    let close = chrome_button(&scene, "Tool1", "PART_CloseButton");
    click_control(&scene.window, &close);
    assert_eq!(tools.visible_dockables().unwrap().count(), 1);
    assert!(root
        .hidden_dockables()
        .unwrap()
        .contains(&Some(tool.clone())));
    scene.factory.restore_dockable(Some(tool.clone()));
    drain(&scene.window);
    select_tool(&scene, &tools, tool.clone());
    scene.factory.set_hide_tools_on_close(false);
    let close = chrome_button(&scene, "Tool1", "PART_CloseButton");
    click_control(&scene.window, &close);
    assert_eq!(tools.visible_dockables().unwrap().count(), 1);
    assert!(!tools.visible_dockables().unwrap().contains(&Some(tool)));
    assert!(root.hidden_dockables().is_none_or(|v| v.count() == 0));
    assert!(scene
        .factory
        .find_dockable(
            Some(root.clone()),
            Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Tool1")))
        )
        .is_none());
    assert_eq!(scene.factory.dock_controls().count(), 1);
    assert!(!scene.control.is_dragging_dock());
}
#[test]
fn fluent_restored_tool_controls_pin_preview_unpin_hide_restore_and_close_the_original_models() {
    run(|| check_tools("DockingGraphConsumer.xaml"));
}
#[test]
fn browser_restored_tool_controls_pin_preview_unpin_hide_restore_and_close_the_original_models() {
    run(|| check_tools("BrowserDockingGraphConsumer.xaml"));
}
fn check_move(document: &str) {
    ferrodock_ferro_themes_browser::register_types();
    let browser = document.starts_with("Browser");
    let scene = Scene::from_document(false, document);
    let split = scene
        .root
        .visible_dockables()
        .unwrap()
        .get(0)
        .unwrap()
        .as_dock()
        .unwrap();
    let target = DocumentDock::new();
    target.set_id("TargetDocuments".into());
    let document = Document::new();
    document.set_id("Doc3".into());
    document.set_title("Doc3".into());
    target.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        document.clone() as Rc<dyn IDockable>
    )])));
    target.set_active_dockable(Some(document));
    scene
        .factory
        .add_dockable(Some(split.clone()), Some(ProportionalDockSplitter::new()));
    scene
        .factory
        .add_dockable(Some(split), Some(target.clone()));
    // The source panel has already assigned the original panes their initial shares.
    // A manually-added third pane needs an explicit share of the available width.
    scene.tools.set_proportion(1.0 / 3.0);
    scene.documents.set_proportion(1.0 / 3.0);
    target.set_proportion(1.0 / 3.0);
    drain(&scene.window);
    let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer().unwrap();
    let wire = serializer
        .serialize(scene.root.clone() as Rc<dyn IRootDock>)
        .unwrap();
    let root = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    scene.control.set_layout(Some(root.clone()));
    drain(&scene.window);
    let find = |id: &str| {
        let id = id.to_owned();
        concrete::<DocumentDock>(
            scene
                .factory
                .find_dockable(
                    Some(root.clone()),
                    Some(Rc::new(move |v| v.is_some_and(|v| v.id() == id))),
                )
                .unwrap(),
        )
    };
    let documents = find("Documents");
    let target = find("TargetDocuments");
    assert!(!Rc::ptr_eq(&documents, &scene.documents));
    let hosts: HostWindowLocatorMap = Rc::new(std::cell::RefCell::new(
        ferroui_system::collections::generic::Dictionary::new(),
    ));
    hosts.borrow_mut().add(
        "IDockWindow".into(),
        Some(Rc::new(move || {
            let host = HostWindow::new();
            if browser {
                host.styles()
                    .add(ferrodock_ferro_themes_browser::BrowserTabTheme::new().as_style());
            } else {
                host.styles()
                    .add(ferrodock_ferro_themes_fluent::DockFluentTheme::new().as_style());
            }
            Some(HostWindow::as_host_window(host))
        })),
    );
    scene.factory.set_host_window_locator(Some(hosts));
    let dragged = documents.visible_dockables().unwrap().get(0).unwrap();
    assert_eq!(scene.factory.dock_controls().count(), 1);
    let strip = scene
        .control
        .find_descendant_of_type_where::<DocumentTabStrip>(false, |v| {
            model(v).is_some_and(|v| v.id() == "Documents")
        })
        .unwrap();
    reorder_tabs(&scene, &strip, documents.clone());
    assert_eq!(strip.selected_index(), 1);
    let source = tab(&scene, "Documents", 1);
    let destination = tab(&scene, "TargetDocuments", 0);
    let start = center(&source, &scene.window);
    let end = center(&destination, &scene.window);
    drag(&scene.window, &scene.control, start, end);
    assert_eq!(documents.visible_dockables().unwrap().count(), 1);
    assert_eq!(target.visible_dockables().unwrap().count(), 2);
    assert!(target
        .visible_dockables()
        .unwrap()
        .iter()
        .flatten()
        .any(|v| Rc::ptr_eq(&v, &dragged)));
    assert!(Rc::ptr_eq(
        &dragged.owner().unwrap(),
        &(target.clone() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(&target.active_dockable().unwrap(), &dragged));
    assert!(Rc::ptr_eq(
        &dragged.factory().unwrap(),
        &(scene.factory.clone() as Rc<dyn IFactory>)
    ));
    let index = i32::try_from(
        target
            .visible_dockables()
            .unwrap()
            .index_of(&Some(dragged.clone()))
            .unwrap(),
    )
    .unwrap();
    let source = tab(&scene, "TargetDocuments", index);
    drag(
        &scene.window,
        &scene.control,
        center(&source, &scene.window),
        Point::new(1100.0, 200.0),
    );
    assert_eq!(target.visible_dockables().unwrap().count(), 1);
    assert_eq!(root.windows().unwrap().count(), 1);
    let floating = root.windows().unwrap().get(0).unwrap();
    let host = floating.host().unwrap();
    let native = host
        .clone()
        .boxed_interface(std::any::TypeId::of::<Ref<Window>>())
        .unwrap();
    let native = native.downcast_ref::<Ref<Window>>().unwrap().clone();
    drain(&native);
    assert!(native.is_visible());
    assert_eq!(scene.factory.dock_controls().count(), 2);
    let floating_control = native
        .find_descendant_of_type::<DockControl>(false)
        .unwrap();
    let floating_tabs = floating_control
        .find_descendant_of_type::<DocumentTabStrip>(false)
        .unwrap();
    assert_eq!(
        floating_tabs
            .find_descendant_of_type::<ferrodock_ferro::controls::DelayedUniformTabPanel>(false)
            .is_some(),
        browser
    );
    let source = floating_tabs.container_from_index(0).unwrap();
    assert_eq!(
        source
            .data_context()
            .and_then(|v| {
                ferroui_base::data::core::ValueTypes::try_cast(
                    &v,
                    ferroui_base::data::core::ValueType::of::<Rc<dyn IDockable>>(),
                )
                .and_then(|v| v.downcast_ref::<Rc<dyn IDockable>>().cloned())
            })
            .unwrap()
            .id(),
        "Doc1"
    );
    let floating_wire = serializer.serialize(root.clone()).unwrap();
    let saved = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&floating_wire))
        .unwrap()
        .unwrap();
    assert_eq!(saved.windows().unwrap().count(), 1);
    assert_eq!(serializer.serialize(saved).unwrap(), floating_wire);
    let destination = tab(&scene, "TargetDocuments", 0);
    let screen = scene
        .window
        .point_to_screen(center(&destination, &scene.window));
    let end = native.point_to_client(screen);
    drag(&native, &floating_control, center(&source, &native), end);
    drain(&scene.window);
    assert!(Rc::ptr_eq(
        &dragged.owner().unwrap(),
        &(target.clone() as Rc<dyn IDockable>)
    ));
    assert_eq!(target.visible_dockables().unwrap().count(), 2);
    assert!(root.windows().as_ref().is_none_or(|v| v.count() == 0));
    assert!(!native.is_visible());
    assert!(!scene.control.is_dragging_dock());
    assert_eq!(scene.factory.dock_controls().count(), 1);
    assert!(host.as_host_window_close_state().unwrap().is_closed());
    let last = tab(&scene, "Documents", 0);
    scene
        .window
        .mouse_move(center(&last, &scene.window), RawInputModifiers::NONE);
    drain(&scene.window);
    let close = last
        .find_descendant_of_type_where::<ferroui_controls::Button>(false, |v| {
            v.is_effectively_visible()
        })
        .unwrap();
    click_control(&scene.window, &close);
    assert!(documents.visible_dockables().is_none_or(|v| v.count() == 0));
    assert!(scene
        .factory
        .find_dockable(
            Some(root.clone()),
            Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Doc2")))
        )
        .is_none());
    assert_eq!(scene.factory.dock_controls().count(), 1);
}
#[test]
fn fluent_restored_pointer_drag_moves_floats_and_redocks_through_the_original_engine() {
    run(|| check_move("DockingGraphConsumer.xaml"));
}
#[test]
fn browser_restored_pointer_drag_moves_floats_and_redocks_through_the_original_engine() {
    run(|| check_move("BrowserDockingGraphConsumer.xaml"));
}
