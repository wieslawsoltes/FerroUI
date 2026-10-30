use super::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, Tool, ToolDock},
    Factory, FactoryImpl,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::layout::Orientation;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::primitives::{TabStrip, TabStripItem};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    AppBuilder, Application, Border, IItemsList, ItemsChangedEventArgs, ItemsChangedHandler,
    ItemsSource, ItemsView, Panel, TextBlock, TypedItems, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::rc::Weak;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-item-drag",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(c: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(600.0);
        w.set_height(180.0);
        w.set_content(Some(Control::boxed(c)));
        w.show();
        let scene = Self(w);
        scene.update();
        scene
    }
    fn update(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.0.update_layout();
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
fn helper(
    owner: &Control,
    items: &ItemsControl,
    orientation: Orientation,
    outside: Option<DragOutside>,
) -> Rc<ItemDragHelper> {
    let weak = items.to_ref().downgrade();
    let h = ItemDragHelper::new(
        owner,
        Rc::new(move || weak.upgrade()),
        Rc::new(move || orientation),
        None,
        None,
        outside,
        None,
    );
    h.attach();
    h
}
fn item(strip: &ItemsControl, index: i32) -> Ref<Control> {
    strip
        .container_from_index(index)
        .expect("real native item container")
}
fn point(control: &Control, window: &Window) -> Point {
    control
        .translate_point(
            Point::new(control.bounds().width / 2.0, control.bounds().height / 2.0),
            window,
        )
        .unwrap()
}
fn offset(control: &Control) -> Point {
    control
        .render_transform()
        .unwrap()
        .value()
        .transform(Point::default())
}
fn press(scene: &Scene, owner: &Control) {
    let start = point(owner, &scene.0);
    scene.0.mouse_move(start, RawInputModifiers::empty());
    scene
        .0
        .mouse_down(start, MouseButton::Left, RawInputModifiers::empty());
}
fn move_to(scene: &Scene, p: Point) {
    scene.0.mouse_move(p, RawInputModifiers::LEFT_MOUSE_BUTTON);
}
fn release(scene: &Scene, p: Point) {
    scene
        .0
        .mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
    scene.update();
}
fn string_tabs() -> (Ref<TabStrip>, Scene) {
    let tabs = TabStrip::new();
    for text in ["First", "Second", "Third"] {
        let item = TabStripItem::new();
        item.set_width(100.0);
        item.set_height(40.0);
        item.set_content(Some(Rc::new(text.to_owned())));
        tabs.items().add(Some(Control::boxed(item)));
    }
    let scene = Scene::new(tabs.clone().upcast());
    (tabs, scene)
}
#[test]
fn real_pointer_reorders_item_collection_only_on_release_and_resets_source_visuals() {
    run(|| {
        let (tabs, scene) = string_tabs();
        let first = item(&tabs, 0);
        let second = item(&tabs, 1);
        let h = helper(&first, &tabs, Orientation::Horizontal, None);
        let end = point(&second, &scene.0);
        press(&scene, &first);
        assert!(h.enable_drag.get());
        assert!(first.classes().contains(":dragging"));
        assert_eq!(first.z_index(), 1);
        move_to(&scene, point(&first, &scene.0) + Vector::new(12.0, 0.0));
        move_to(&scene, end);
        assert!(h.drag_started.get());
        assert_eq!(h.target_index.get(), 1);
        assert_eq!(tabs.index_from_container(&first), 0);
        assert_eq!(offset(&second), Point::new(-100.0, 0.0));
        release(&scene, end);
        assert_eq!(tabs.index_from_container(&first), 1);
        assert_eq!(tabs.selected_index(), 1);
        assert_eq!(offset(&first), Point::default());
        assert!(!first.classes().contains(":dragging"));
        assert_eq!(first.z_index(), 0);
        assert!(!h.enable_drag.get());
        h.detach();
    });
}
#[test]
fn live_native_items_source_mutates_same_ferro_list_through_original_ilist_bridge() {
    run(|| {
        let list = Rc::new(FerroList::from_items(vec![
            "First".to_owned(),
            "Second".to_owned(),
        ]));
        let tabs = TabStrip::new();
        tabs.set_item_template(Some(FuncDataTemplate::for_type::<String>(
            |text, _| {
                let b = TextBlock::new();
                b.set_width(80.0);
                b.set_text(Some(text));
                Some(b.upcast())
            },
            false,
        )));
        tabs.set_items_source(Some(ItemsSource::from(list.clone())));
        let scene = Scene::new(tabs.clone().upcast());
        let first = item(&tabs, 0);
        let end = point(&item(&tabs, 1), &scene.0);
        let h = helper(&first, &tabs, Orientation::Horizontal, None);
        press(&scene, &first);
        move_to(&scene, point(&first, &scene.0) + Vector::new(12.0, 0.0));
        move_to(&scene, end);
        release(&scene, end);
        assert_eq!(&*list.snapshot(), &["Second", "First"]);
        assert_eq!(tabs.selected_index(), 1);
        h.detach();
    });
}
struct PolicyFactory {
    component: Factory,
    drag: Cell<bool>,
    reorder: Cell<bool>,
}
impl PolicyFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            component: Factory::construct(this.clone()),
            drag: Cell::new(true),
            reorder: Cell::new(true),
        })
    }
}
impl ferrodock_model::FactoryBaseImpl for PolicyFactory {
    fn factory_base(&self) -> &ferrodock_model::FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for PolicyFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn as_dock_item_drag_policy(this: Rc<Self>) -> Option<Rc<dyn IDockItemDragPolicy>> {
        Some(this)
    }
}
impl IDockItemDragPolicy for PolicyFactory {
    fn can_drag_item(&self, _: Rc<dyn IDockable>) -> bool {
        self.drag.get()
    }
    fn can_reorder_item(&self, _: Rc<dyn IDockable>) -> bool {
        self.reorder.get()
    }
}
ferrodock_model_reactiveui::impl_factory!(PolicyFactory);
struct DockItems(ListHandle<Option<Rc<dyn IDockable>>>);
impl IItemsList for DockItems {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, i: usize) -> Option<BoxedValue> {
        self.0
            .get(i)
            .map(crate::model_projection::to_dockable_value)
    }
    fn is_notifying(&self) -> bool {
        self.0.as_observable_collection().is_some()
    }
    fn add_collection_changed(&self, h: Rc<ItemsChangedHandler>) -> Option<u64> {
        self.0.as_observable_collection().map(|list| {
            list.add_collection_changed(Rc::new(move |e| {
                let new = TypedItems(e.new_items);
                let old = TypedItems(e.old_items);
                h(&ItemsChangedEventArgs {
                    action: e.action,
                    new_items: ItemsView::from_typed(&new),
                    old_items: ItemsView::from_typed(&old),
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                });
            }))
        })
    }
    fn remove_collection_changed(&self, token: u64) {
        if let Some(list) = self.0.as_observable_collection() {
            list.remove_collection_changed(token);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
struct Workspace {
    first: Rc<dyn IDockable>,
    second: Rc<dyn IDockable>,
    pane: Rc<dyn IDock>,
    _root: Rc<RootDock>,
    _factory: Rc<dyn IFactory>,
    tabs: Ref<TabStrip>,
    scene: Scene,
    owner: Ref<Control>,
    target: Ref<Control>,
    helper: Rc<ItemDragHelper>,
    end: Point,
}
impl Workspace {
    fn new(tool: bool, factory: Rc<dyn IFactory>) -> Self {
        let first: Rc<dyn IDockable> = if tool { Tool::new() } else { Document::new() };
        let second: Rc<dyn IDockable> = if tool { Tool::new() } else { Document::new() };
        first.set_id("first".into());
        first.set_title("First".into());
        second.set_id("second".into());
        second.set_title("Second".into());
        let pane: Rc<dyn IDock> = if tool {
            ToolDock::new()
        } else {
            DocumentDock::new()
        };
        let list = ListHandle::from_items(vec![Some(first.clone()), Some(second.clone())]);
        pane.set_visible_dockables(Some(list.clone()));
        pane.set_active_dockable(Some(first.clone()));
        let root = RootDock::new();
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            pane.clone() as Rc<dyn IDockable>
        )])));
        root.set_active_dockable(Some(pane.clone()));
        root.set_default_dockable(Some(pane.clone()));
        factory.init_layout(root.clone());
        let tabs = TabStrip::new();
        tabs.set_items_source(Some(ItemsSource::new(Rc::new(DockItems(list)))));
        tabs.set_item_template(Some(FuncDataTemplate::new(
            |_| true,
            |_, _| {
                let t = TextBlock::new();
                t.set_width(90.0);
                t.set_height(35.0);
                Some(t.upcast())
            },
            false,
        )));
        let scene = Scene::new(tabs.clone().upcast());
        let owner = item(&tabs, 0);
        let target = item(&tabs, 1);
        let end = point(&target, &scene.0);
        let helper = helper(&owner, &tabs, Orientation::Horizontal, None);
        Self {
            first,
            second,
            pane,
            _root: root,
            _factory: factory,
            tabs,
            scene,
            owner,
            target,
            helper,
            end,
        }
    }
    fn move_first(&self) {
        press(&self.scene, &self.owner);
        move_to(
            &self.scene,
            point(&self.owner, &self.scene.0) + Vector::new(12.0, 0.0),
        );
        move_to(&self.scene, self.end);
    }
    fn release(&self) {
        release(&self.scene, self.end);
    }
    fn order(&self) -> Vec<String> {
        self.pane
            .visible_dockables()
            .unwrap()
            .iter()
            .map(|d| d.expect("nonnull drag fixture").id())
            .collect()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        self.helper.detach();
    }
}
#[test]
fn source_policy_matrix_uses_original_factories_for_tools_and_documents() {
    run(|| {
        for tool in [false, true] {
            for policy in 0..4 {
                let factory = PolicyFactory::new();
                factory.drag.set(policy != 3);
                factory.reorder.set(policy != 2);
                let f: Rc<dyn IFactory> = if policy == 0 { Factory::new() } else { factory };
                let w = Workspace::new(tool, f);
                w.move_first();
                assert_eq!(w.order(), vec!["first", "second"]);
                w.release();
                assert_eq!(
                    w.order(),
                    if policy < 2 {
                        vec!["second", "first"]
                    } else {
                        vec!["first", "second"]
                    }
                );
            }
        }
    });
}
#[test]
fn source_policy_change_before_release_and_during_move_cancels_pending_move() {
    run(|| {
        for tool in [false, true] {
            for drag in [false, true] {
                let f = PolicyFactory::new();
                let w = Workspace::new(tool, f.clone());
                w.move_first();
                if drag {
                    f.drag.set(false);
                } else {
                    f.reorder.set(false);
                }
                w.release();
                assert_eq!(w.order(), vec!["first", "second"]);
            }
            let f = PolicyFactory::new();
            let w = Workspace::new(tool, f.clone());
            w.move_first();
            f.drag.set(false);
            move_to(&w.scene, w.end + Vector::new(1.0, 0.0));
            f.drag.set(true);
            w.release();
            assert_eq!(w.order(), vec!["first", "second"]);
        }
    });
}
#[test]
fn source_noninteractive_targets_are_excluded_during_movement_and_at_release() {
    run(|| {
        for tool in [false, true] {
            for after_move in [false, true] {
                let w = Workspace::new(tool, Factory::new());
                if !after_move {
                    w.target.set_is_hit_test_visible(false);
                }
                w.move_first();
                if after_move {
                    w.target.set_is_hit_test_visible(false);
                }
                w.release();
                assert_eq!(w.order(), vec!["first", "second"]);
            }
        }
    });
}
#[test]
fn mutable_list_adapter_preserves_live_identity_indexing_type_checks_and_mutations() {
    run(|| {
        let list = FerroList::from_items(vec!["a".to_owned(), "b".to_owned()]);
        let view = crate::mutable_items_list::from_ferro_list(list.clone());
        let second = crate::mutable_items_list::from_ferro_list(list.clone());
        assert!(view == second);
        assert_eq!(view.count(), 2);
        assert_eq!(view.index_of(&Some(Rc::new("b".to_owned()))), Some(1));
        view.set(0, Some(Rc::new("c".to_owned())));
        view.insert(1, Some(Rc::new("d".to_owned())));
        assert_eq!(&*list.snapshot(), &["c", "d", "b"]);
        view.remove_at(1);
        assert_eq!(view.get(0).unwrap().downcast_ref::<String>().unwrap(), "c");
        view.clear();
        assert_eq!(list.count(), 0);
    });
}
#[test]
fn both_threshold_delegates_are_evaluated_in_order_with_strict_orientation_threshold() {
    run(|| {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let (tabs, scene) = string_tabs();
            if orientation == Orientation::Vertical {
                tabs.set_items_panel(ferroui_controls::templates::FuncTemplate::new(|| {
                    let panel = ferroui_controls::StackPanel::new();
                    panel.set_orientation(Orientation::Vertical);
                    Some(panel.upcast::<Panel>())
                }));
                scene.update();
            }
            let owner = item(&tabs, 0);
            let weak = tabs.downgrade();
            let calls = Rc::new(RefCell::new(Vec::new()));
            let horizontal = calls.clone();
            let vertical = calls.clone();
            let h = ItemDragHelper::new(
                &owner,
                Rc::new(move || weak.upgrade().map(|t| t.upcast())),
                Rc::new(move || orientation),
                Some(Rc::new(move || {
                    horizontal.borrow_mut().push('h');
                    10.0
                })),
                Some(Rc::new(move || {
                    vertical.borrow_mut().push('v');
                    10.0
                })),
                None,
                None,
            );
            h.attach();
            press(&scene, &owner);
            let start = point(&owner, &scene.0);
            let axis = |n| {
                if orientation == Orientation::Horizontal {
                    Vector::new(n, 0.0)
                } else {
                    Vector::new(0.0, n)
                }
            };
            move_to(&scene, start + axis(10.0));
            assert!(!h.drag_started.get());
            assert_eq!(&*calls.borrow(), &['h', 'v']);
            move_to(&scene, start + axis(11.0));
            assert!(h.drag_started.get());
            assert_eq!(offset(&owner), Point::default() + axis(11.0));
            assert_eq!(&*calls.borrow(), &['h', 'v', 'h', 'v']);
            let end = point(&item(&tabs, 1), &scene.0);
            move_to(&scene, end);
            assert_eq!(h.target_index.get(), 1);
            assert_eq!(&*calls.borrow(), &['h', 'v', 'h', 'v']);
            release(&scene, end);
            assert_eq!(tabs.index_from_container(&owner), 1);
            h.detach();
        }
    });
}
#[test]
fn drag_outside_selects_and_resets_before_callback_and_shares_original_press_state() {
    run(|| {
        let (tabs, scene) = string_tabs();
        tabs.set_height(40.0);
        tabs.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
        scene.update();
        let owner = item(&tabs, 1);
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let weak_owner = owner.downgrade();
        let weak_tabs = tabs.downgrade();
        let weak_helper = Rc::new(RefCell::new(Weak::<ItemDragHelper>::new()));
        let callback_helper = weak_helper.clone();
        let h = helper(
            &owner,
            &tabs,
            Orientation::Horizontal,
            Some(Rc::new(move |press, moved| {
                let h = callback_helper.borrow().upgrade().unwrap();
                let owner = weak_owner.upgrade().unwrap();
                assert!(!h.enable_drag.get());
                assert!(!h.captured.get());
                assert!(h.items.borrow().is_none());
                assert!(!owner.classes().contains(":dragging"));
                assert_eq!(offset(&owner), Point::default());
                assert_eq!(weak_tabs.upgrade().unwrap().selected_index(), 1);
                let press = press.unwrap();
                assert_eq!(
                    press.timestamp(),
                    h.pressed_args.borrow().as_ref().unwrap().timestamp()
                );
                press.set_handled(true);
                assert!(h.pressed_args.borrow().as_ref().unwrap().handled());
                assert!(moved.get_position(Some(&weak_tabs.upgrade().unwrap())).y > 40.0);
                called.set(called.get() + 1);
            })),
        );
        weak_helper.replace(Rc::downgrade(&h));
        press(&scene, &owner);
        let start = point(&owner, &scene.0);
        move_to(&scene, Point::new(start.x, 80.0));
        assert_eq!(calls.get(), 1);
        assert!(h.pressed_args.borrow().is_none());
        assert_eq!(tabs.index_from_container(&owner), 1);
        release(&scene, Point::new(start.x, 80.0));
        assert_eq!(calls.get(), 1);
        h.detach();
    });
}
#[test]
fn capture_loss_body_commits_pending_reorder_and_source_tunnel_registration_is_preserved() {
    use ferroui_base::input::{KeyModifiers, PointerCaptureLostEventArgs};
    run(|| {
        for nonleft in [false, true] {
            let (tabs, scene) = string_tabs();
            let owner = item(&tabs, 0);
            let h = helper(&owner, &tabs, Orientation::Horizontal, None);
            let end = point(&item(&tabs, 1), &scene.0);
            press(&scene, &owner);
            move_to(&scene, end);
            assert!(h.drag_started.get());
            let pointer = Pointer::new(100, PointerType::Mouse, true);
            if nonleft {
                let e = PointerReleasedEventArgs::new(
                    owner.clone(),
                    pointer.clone(),
                    &scene.0,
                    end,
                    1,
                    PointerPointProperties::new(
                        RawInputModifiers::empty(),
                        PointerUpdateKind::RightButtonReleased,
                    ),
                    KeyModifiers::empty(),
                    MouseButton::Right,
                );
                owner.raise_event(&e);
                assert!(!h.captured.get());
                assert!(h.pressed_args.borrow().is_none());
                assert!(h.enable_drag.get());
                assert!(owner.classes().contains(":dragging"));
                assert_eq!(tabs.index_from_container(&owner), 0);
            }
            let lost = PointerCaptureLostEventArgs::new(owner.clone(), pointer);
            // The source subscribes with Tunnel while the native event is Direct.
            owner.raise_event(&lost);
            assert!(h.enable_drag.get());
            h.pointer_capture_lost(&lost);
            assert!(!h.enable_drag.get());
            assert!(!h.captured.get());
            assert!(h.pressed_args.borrow().is_none());
            assert_eq!(tabs.index_from_container(&owner), 1);
            release(&scene, end);
            h.detach();
        }
    });
}
#[test]
fn detach_stops_timer_without_inventing_gesture_reset_and_duplicate_attach_removes_one_set() {
    run(|| {
        let (tabs, scene) = string_tabs();
        let owner = item(&tabs, 0);
        let h = helper(&owner, &tabs, Orientation::Horizontal, None);
        h.attach();
        assert_eq!(h.subscriptions.borrow().len(), 2);
        h.detach();
        press(&scene, &owner);
        assert!(h.enable_drag.get());
        h.start_auto_scroll(ScrollViewer::new(), Vector::new(200.0, 0.0));
        let timer = h.auto_scroll.borrow().as_ref().unwrap().timer.clone();
        h.detach();
        assert!(!timer.is_enabled());
        assert!(h.auto_scroll.borrow().is_none());
        assert!(h.enable_drag.get());
        assert!(h.captured.get());
        release(&scene, point(&owner, &scene.0));
        assert!(h.enable_drag.get());
        h.released();
        assert!(!h.enable_drag.get());
    });
}
fn scroll_scene() -> (Ref<ScrollViewer>, Scene) {
    let scroll = ScrollViewer::new();
    scroll.set_horizontal_scroll_bar_visibility(
        ferroui_controls::primitives::ScrollBarVisibility::Auto,
    );
    scroll.set_vertical_scroll_bar_visibility(
        ferroui_controls::primitives::ScrollBarVisibility::Auto,
    );
    let content = Border::new();
    content.set_width(2000.0);
    content.set_height(1000.0);
    scroll.set_content(Some(Control::boxed(content)));
    let scene = Scene::new(scroll.clone().upcast());
    assert!(scroll.extent().width > scroll.viewport().width);
    assert!(scroll.extent().height > scroll.viewport().height);
    (scroll, scene)
}
#[test]
fn auto_scroll_uses_source_edges_speed_fixed_delta_clamping_and_timer_replacement() {
    run(|| {
        let (scroll, _scene) = scroll_scene();
        let owner = Border::new();
        let h = ItemDragHelper::new(
            &owner,
            Rc::new(|| None),
            Rc::new(|| Orientation::Horizontal),
            None,
            None,
            None,
            None,
        );
        let viewport = scroll.viewport();
        assert_eq!(
            h.calculate_auto_scroll_velocity(Point::new(20.0, 20.0), &scroll),
            Vector::default()
        );
        assert_eq!(
            h.calculate_auto_scroll_velocity(Point::new(10.0, 10.0), &scroll),
            Vector::new(-100.0, -100.0)
        );
        assert_eq!(
            h.calculate_auto_scroll_velocity(Point::new(-10.0, -10.0), &scroll),
            Vector::new(-300.0, -300.0)
        );
        assert_eq!(
            h.calculate_auto_scroll_velocity(Point::new(viewport.width, viewport.height), &scroll),
            Vector::new(200.0, 200.0)
        );
        h.start_auto_scroll(scroll.clone(), Vector::new(200.0, 200.0));
        let first = h.auto_scroll.borrow().as_ref().unwrap().timer.clone();
        assert_eq!(first.interval(), Duration::from_millis(16));
        h.start_auto_scroll(scroll.clone(), Vector::new(200.0, 200.0));
        assert!(Rc::ptr_eq(
            &first,
            &h.auto_scroll.borrow().as_ref().unwrap().timer
        ));
        Dispatcher::force_fire_timer_for_unit_tests(&first);
        assert_eq!(scroll.offset(), Vector::new(3.2, 3.2));
        h.start_auto_scroll(scroll.clone(), Vector::new(-400.0, -400.0));
        assert!(!first.is_enabled());
        let second = h.auto_scroll.borrow().as_ref().unwrap().timer.clone();
        Dispatcher::force_fire_timer_for_unit_tests(&second);
        assert_eq!(scroll.offset(), Vector::default());
        scroll.set_offset(Vector::new(100000.0, 100000.0));
        let end = scroll.offset();
        h.start_auto_scroll(scroll.clone(), Vector::new(400.0, 400.0));
        h.on_auto_scroll_tick();
        assert_eq!(scroll.offset(), end);
        h.stop_auto_scroll();
        assert!(h.scroll.borrow().is_none());
        assert_eq!(h.velocity.get(), Vector::default());
        let other = ScrollViewer::new();
        h.start_auto_scroll(other.clone(), Vector::new(f64::NAN, 0.0));
        let nan_timer = h.auto_scroll.borrow().as_ref().unwrap().timer.clone();
        h.start_auto_scroll(other, Vector::new(f64::NAN, 0.0));
        assert!(Rc::ptr_eq(
            &nan_timer,
            &h.auto_scroll.borrow().as_ref().unwrap().timer
        ));
        h.stop_auto_scroll();
        assert!(min(f64::NAN, 0.0).is_nan());
        assert!(max(0.0, f64::NAN).is_nan());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert!(max(-0.0, 0.0).is_sign_positive());
    });
}
#[test]
fn unregistered_native_source_stays_unchanged_until_mutable_composition_cast_is_registered() {
    run(|| {
        for mutable in [false, true] {
            let list = Rc::new(FerroList::from_items(vec![11_i32, 22]));
            if mutable {
                ValueTypes::register_cast::<FerroList<i32>, ListHandle<Option<BoxedValue>>>(|l| {
                    crate::mutable_items_list::from_ferro_list(l.clone())
                });
            }
            let tabs = TabStrip::new();
            tabs.set_item_template(Some(FuncDataTemplate::for_type::<i32>(
                |_, _| {
                    let b = Border::new();
                    b.set_width(100.0);
                    b.set_height(40.0);
                    Some(b.upcast())
                },
                false,
            )));
            tabs.set_items_source(Some(ItemsSource::from(list.clone())));
            let scene = Scene::new(tabs.clone().upcast());
            let owner = item(&tabs, 0);
            let h = helper(&owner, &tabs, Orientation::Horizontal, None);
            let end = point(&item(&tabs, 1), &scene.0);
            press(&scene, &owner);
            move_to(&scene, end);
            release(&scene, end);
            assert_eq!(
                &*list.snapshot(),
                if mutable { &[22, 11] } else { &[11, 22] }
            );
            h.detach();
        }
    });
}
#[test]
fn z_index_ignores_dragged_item_and_release_clears_value_and_overwrites_old_transform() {
    run(|| {
        let (tabs, scene) = string_tabs();
        let owner = item(&tabs, 0);
        let other = item(&tabs, 1);
        owner.set_z_index(999);
        other.set_z_index(4);
        translate(&owner, 10.0, 20.0);
        assert_eq!(get_dragged_z_index(&tabs, &owner), 5);
        other.set_z_index(i32::MAX);
        assert_eq!(get_dragged_z_index(&tabs, &owner), 1);
        other.set_z_index(4);
        let h = helper(&owner, &tabs, Orientation::Horizontal, None);
        press(&scene, &owner);
        assert_eq!(owner.z_index(), 5);
        assert_eq!(offset(&owner), Point::default());
        release(&scene, point(&owner, &scene.0));
        assert_eq!(owner.z_index(), 0);
        assert!(owner.render_transform().is_some());
        assert_eq!(offset(&owner), Point::default());
        h.detach();
    });
}
#[test]
fn original_pinned_move_path_mutates_pinned_collection_and_not_visible_dockables() {
    run(|| {
        let w = Workspace::new(true, Factory::new());
        w._factory.pin_dockable(Some(w.first.clone()));
        w._factory.pin_dockable(Some(w.second.clone()));
        let pinned = w._root.left_pinned_dockables().unwrap();
        assert_eq!(pinned.count(), 2);
        assert!(w._factory.is_dockable_pinned(Some(w.first.clone()), None));
        w.helper.detach();
        w.tabs
            .set_items_source(Some(ItemsSource::new(Rc::new(DockItems(pinned.clone())))));
        w.scene.update();
        let owner = item(&w.tabs, 0);
        let h = helper(&owner, &w.tabs, Orientation::Horizontal, None);
        let end = point(&item(&w.tabs, 1), &w.scene.0);
        press(&w.scene, &owner);
        move_to(&w.scene, end);
        release(&w.scene, end);
        assert_eq!(
            pinned
                .iter()
                .map(|d| d.expect("nonnull drag fixture").id())
                .collect::<Vec<_>>(),
            ["second", "first"]
        );
        assert_eq!(w.pane.visible_dockables().unwrap().count(), 0);
        h.detach();
    });
}
#[test]
fn bounds_transform_descendant_scroll_extent_and_container_bounds_are_all_applied() {
    use ferroui_base::Thickness;
    use ferroui_controls::templates::FuncControlTemplate;
    run(|| {
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(
            ferroui_controls::primitives::ScrollBarVisibility::Auto,
        );
        scroll.set_vertical_scroll_bar_visibility(
            ferroui_controls::primitives::ScrollBarVisibility::Auto,
        );
        let content = Border::new();
        content.set_width(2000.0);
        content.set_height(1000.0);
        scroll.set_content(Some(Control::boxed(content)));
        let items = ItemsControl::new();
        let template_scroll = scroll.clone();
        items.set_template(Some(FuncControlTemplate::for_type::<ItemsControl>(
            move |_, _| template_scroll.clone().upcast(),
        )));
        items.set_margin(Thickness::uniform(30.0));
        let bounds = Border::new();
        bounds.set_child(Some(items.clone().upcast()));
        let scene = Scene::new(bounds.clone().upcast());
        assert!(items
            .find_descendant_of_type::<ScrollViewer>(false)
            .is_some());
        let weak_bounds = bounds.downgrade();
        let owner = Border::new();
        let h = ItemDragHelper::new(
            &owner,
            Rc::new(|| None),
            Rc::new(|| Orientation::Horizontal),
            None,
            None,
            None,
            Some(Rc::new(move || weak_bounds.upgrade().map(|b| b.upcast()))),
        );
        assert!(h.position_within_drag_bounds(Point::new(50.0, 50.0), &items));
        assert!(h.auto_scroll.borrow().is_none());
        let edge = Point::new(scroll.viewport().width, 50.0);
        assert!(h.position_within_drag_bounds(edge, &items));
        assert!(h.auto_scroll.borrow().is_some());
        assert!(!h.position_within_drag_bounds(Point::new(2100.0, 50.0), &items));
        // The source starts scrolling before testing either bounds limit.
        assert!(h.auto_scroll.borrow().is_some());
        scroll.set_offset(Vector::new(100.0, 0.0));
        assert!(h.position_within_drag_bounds(Point::new(-10.0, 50.0), &items));
        assert!(!h.position_within_drag_bounds(Point::new(-31.0, 50.0), &items));
        assert!(!h.position_within_drag_bounds(Point::new(-101.0, 50.0), &items));
        items.set_template(None);
        scene.update();
        assert!(h.position_within_drag_bounds(Point::new(-10.0, 50.0), &items));
        assert!(h.auto_scroll.borrow().is_none());
        h.stop_auto_scroll();
    });
}
async fn dispatcher_delay(duration: Duration) {
    let ready = Rc::new(Cell::new(false));
    let waker = Rc::new(RefCell::new(None::<std::task::Waker>));
    let completed = ready.clone();
    let wake = waker.clone();
    DispatcherTimer::run_once(
        move || {
            completed.set(true);
            if let Some(waker) = wake.borrow_mut().take() {
                waker.wake();
            }
        },
        duration,
        ferroui_base::threading::DispatcherPriority::BACKGROUND,
    );
    std::future::poll_fn(move |cx| {
        if ready.get() {
            std::task::Poll::Ready(())
        } else {
            *waker.borrow_mut() = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    })
    .await;
}
#[test]
fn native_auto_scroll_timer_advances_and_detached_inactive_helper_is_released() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch_task(
            || async {
                crate::register_types();
                let (scroll, _scene) = scroll_scene();
                let owner = Border::new();
                let h = ItemDragHelper::new(
                    &owner,
                    Rc::new(|| None),
                    Rc::new(|| Orientation::Horizontal),
                    None,
                    None,
                    None,
                    None,
                );
                h.attach();
                h.start_auto_scroll(scroll.clone(), Vector::new(200.0, 0.0));
                let until = std::time::Instant::now() + Duration::from_secs(2);
                while scroll.offset().x == 0.0 {
                    assert!(
                        std::time::Instant::now() < until,
                        "native drag timer did not tick"
                    );
                    dispatcher_delay(Duration::from_millis(10)).await;
                }
                assert!(scroll.offset().x >= 3.2);
                h.detach();
                let stopped = scroll.offset();
                let weak = Rc::downgrade(&h);
                drop(h);
                assert!(weak.upgrade().is_none());
                dispatcher_delay(Duration::from_millis(40)).await;
                assert_eq!(scroll.offset(), stopped);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
#[test]
fn ignored_press_and_missing_items_keep_source_initial_state() {
    run(|| {
        let (tabs, scene) = string_tabs();
        let owner = item(&tabs, 0);
        let h = helper(&owner, &tabs, Orientation::Horizontal, None);
        assert_eq!((h.dragged_index.get(), h.target_index.get()), (0, 0));
        let start = point(&owner, &scene.0);
        scene
            .0
            .mouse_down(start, MouseButton::Right, RawInputModifiers::empty());
        assert!(!h.enable_drag.get());
        assert!(owner.render_transform().is_none());
        scene
            .0
            .mouse_up(start, MouseButton::Right, RawInputModifiers::empty());
        h.detach();
        let h = ItemDragHelper::new(
            &owner,
            Rc::new(|| None),
            Rc::new(|| Orientation::Horizontal),
            None,
            None,
            None,
            None,
        );
        h.attach();
        press(&scene, &owner);
        assert!(!h.enable_drag.get());
        assert!(!h.captured.get());
        assert!(h.pressed_args.borrow().is_none());
        release(&scene, start);
        h.detach();
    });
}
#[test]
fn mutable_adapter_preserves_native_control_boxing_null_items_and_rejects_wrong_types() {
    run(|| {
        let first = Border::new();
        let second = Border::new();
        let list = FerroList::<Option<BoxedValue>>::from_items(vec![
            Some(Control::boxed(first.clone())),
            None,
        ]);
        let bridge = crate::mutable_items_list::from_ferro_list(list.clone());
        assert!(bridge.get(1).is_none());
        assert_eq!(bridge.index_of(&None), Some(1));
        assert!(
            ferroui_base::metadata::from_markup_value::<Ref<Control>>(&bridge.get(0))
                .unwrap()
                .ptr_eq(&first)
        );
        bridge.set(0, Some(Control::boxed(second.clone())));
        bridge.insert(1, None);
        assert_eq!(list.count(), 3);
        assert!(
            ferroui_base::metadata::from_markup_value::<Ref<Control>>(&list.get(0))
                .unwrap()
                .ptr_eq(&second)
        );
        let boxed = FerroList::<BoxedValue>::from_items(vec![Control::boxed(first.clone())]);
        let bridge = crate::mutable_items_list::from_ferro_list(boxed.clone());
        bridge.insert(1, Some(Control::boxed(second.clone())));
        assert!(
            ferroui_base::metadata::from_markup_value::<Ref<Control>>(&bridge.get(1))
                .unwrap()
                .ptr_eq(&second)
        );
        let strings = FerroList::from_items(vec!["a".to_owned()]);
        let bridge = crate::mutable_items_list::from_ferro_list(strings.clone());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || bridge.set(0, Some(Rc::new(17_i32)))
        ))
        .is_err());
        assert_eq!(strings.get(0), "a");
    });
}
