// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Standard JSON layout reload followed by actual Fluent/Browser control interaction.
use super::*;
use ferrodock_ferro::controls::DocumentTabStripItem;
use ferrodock_model::controls::*;
use ferroui_base::{
    input::{MouseButton, RawInputModifiers},
    Point,
};
use ferroui_headless::HeadlessWindowExtensions;

fn concrete<T: 'static>(value: Rc<dyn IDockable>) -> Rc<T> {
    value
        .boxed_reference()
        .downcast_ref::<Rc<T>>()
        .unwrap()
        .clone()
}

fn check_tool_menu(scene: &Scene) {
    let chrome = scene
        .control
        .find_descendant_of_type::<ferrodock_ferro::controls::ToolChromeControl>(false)
        .unwrap();
    assert!(
        chrome.tool_flyout().is_some(),
        "restored chrome keeps its flyout"
    );
    let menu_button = chrome
        .find_descendant_of_type_where::<ferroui_controls::Button>(false, |button| {
            button.name().as_deref() == Some("PART_MenuButton")
        })
        .unwrap();
    assert!(
        menu_button.flyout().is_some(),
        "restored template binds the flyout"
    );
    let point = menu_button
        .translate_point(
            Point::new(
                menu_button.bounds().width / 2.0,
                menu_button.bounds().height / 2.0,
            ),
            &scene.window,
        )
        .unwrap();
    scene.window.mouse_move(point, RawInputModifiers::NONE);
    scene
        .window
        .mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
    scene
        .window
        .mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
    drain(&scene.window);
    let flyout = menu_button.flyout().unwrap();
    assert!(
        flyout.is_open(),
        "restored tool menu opens with pointer input"
    );
    flyout.hide();
    drain(&scene.window);
}

fn check(document: &str) {
    ferrodock_ferro_themes_browser::register_types();
    let scene = Scene::from_document(false, document);
    scene.tools.set_alignment(Alignment::Left);
    let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer().unwrap();
    let wire = serializer
        .serialize(scene.root.clone() as Rc<dyn IRootDock>)
        .unwrap();
    let restored = serializer
        .deserialize::<Rc<dyn IRootDock>>(Some(&wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(restored.clone()).unwrap(), wire);
    assert!(!Rc::ptr_eq(
        &(scene.root.clone() as Rc<dyn IRootDock>),
        &restored
    ));
    scene.control.set_layout(Some(restored.clone()));
    drain(&scene.window);
    let split = concrete::<ProportionalDock>(restored.visible_dockables().unwrap().get(0).unwrap());
    let documents = concrete::<DocumentDock>(split.visible_dockables().unwrap().get(2).unwrap());
    let tools = concrete::<ToolDock>(split.visible_dockables().unwrap().get(0).unwrap());
    assert!(Rc::ptr_eq(
        &documents.factory().unwrap(),
        &(scene.factory.clone() as Rc<dyn IFactory>)
    ));
    check_tool_menu(&scene);
    let tabs = scene
        .control
        .find_descendant_of_type::<DocumentTabStrip>(false)
        .unwrap();
    assert_eq!(tabs.items().count(), 2);
    let item = tabs
        .container_from_index(1)
        .unwrap()
        .cast::<DocumentTabStripItem>()
        .unwrap();
    let point = item
        .translate_point(
            Point::new(item.bounds().width / 2.0, item.bounds().height / 2.0),
            &scene.window,
        )
        .unwrap();
    scene.window.mouse_move(point, RawInputModifiers::NONE);
    scene
        .window
        .mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
    scene
        .window
        .mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
    drain(&scene.window);
    assert_eq!(documents.active_dockable().unwrap().id(), "Doc2");
    assert_eq!(tabs.selected_index(), 1);
    let tool = tools.visible_dockables().unwrap().get(0).unwrap();
    scene.factory.pin_dockable(Some(tool.clone()));
    drain(&scene.window);
    assert!(restored
        .left_pinned_dockables()
        .unwrap()
        .iter()
        .flatten()
        .any(|v| Rc::ptr_eq(&v, &tool)));
    scene.factory.unpin_dockable(Some(tool.clone()));
    drain(&scene.window);
    assert!(tools
        .visible_dockables()
        .unwrap()
        .iter()
        .flatten()
        .any(|v| Rc::ptr_eq(&v, &tool)));
    check_tool_menu(&scene);
    assert_eq!(scene.documents.active_dockable().unwrap().id(), "Doc1");
}

#[test]
fn fluent_restored_layout_uses_original_factory_and_native_tab_and_pin_operations() {
    run(|| check("DockingGraphConsumer.xaml"));
}

#[test]
fn browser_restored_layout_uses_original_factory_and_native_tab_and_pin_operations() {
    run(|| check("BrowserDockingGraphConsumer.xaml"));
}
