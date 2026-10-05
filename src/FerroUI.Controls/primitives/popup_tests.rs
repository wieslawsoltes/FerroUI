//! Port of the reference `PopupTests`.
//!
//! The reference runs the suite twice: with the platform creating native
//! popups (`PopupTests`) and with the platform creating none, so that
//! popups are shown in the overlay layer (`PopupTestsWithPopupRoot`, whose
//! `UsePopupHost` flag is true). Every test body here takes that flag and
//! is run by both variants.

use super::popup_positioning::{CustomPopupPlacement, PopupAnchor, PopupGravity};
use super::{
    AdornerLayer, LightDismissOverlayLayer, OverlayPopupHost, Popup, PopupRoot, TemplatedControlImpl,
};
use crate::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Border, Canvas, ContentControl, ContentControlImpl, Control, ControlImpl, Decorator, Dock, DockPanel, Panel,
    PlacementMode, StackPanel, TopLevel, Window, WindowResizeReason,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{
    IKeyboardDevice, IKeyboardNavigationHandler, InputElement, InputElementImpl, KeyModifiers, KeyboardDevice,
    KeyboardNavigationHandler, NavigationDirection, Pointer, PointerPointProperties, PointerPressedEventArgs,
    PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, Geometry, GeometryHitTestResult, IBrush};
use ferroui_base::rendering::IHitTester;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn create_services(use_popup_host: bool) -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window().with_windowing_platform(create_mock_windowing_platform(use_popup_host)),
    )
}

fn create_services_with_focus(use_popup_host: bool) -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_windowing_platform(create_mock_windowing_platform(use_popup_host))
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
            .with_keyboard_navigation(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>)),
    )
}

fn create_pointer_pressed_event_args(source: &Ref<Window>, p: Point) -> PointerPressedEventArgs {
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    PointerPressedEventArgs::new(
        source.clone(),
        pointer,
        source,
        p,
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    )
}

fn create_window_impl(use_popup_host: bool) -> Rc<dyn IWindowImpl> {
    let mock = MockWindowingPlatform::create_window_mock();

    let weak = Rc::downgrade(&mock);
    mock.setup_create_popup(move |_| {
        if use_popup_host {
            return None;
        }
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(create_popup_mock(parent))
    });

    mock
}

fn create_mock_windowing_platform(use_popup_host: bool) -> Rc<MockWindowingPlatform> {
    MockWindowingPlatform::with_window_impl(move || create_window_impl(use_popup_host))
}

fn create_popup_mock(parent: Rc<dyn ITopLevelImpl>) -> Rc<dyn IPopupImpl> {
    let mock = MockWindowingPlatform::create_popup_mock(parent);

    let weak = Rc::downgrade(&mock);
    mock.setup_create_popup(move |_| {
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(create_popup_mock(parent))
    });

    mock
}

fn prepared_window(content: Option<Ref<Control>>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(content.map(Control::boxed));
    w.show();
    w.apply_styling();
    w.apply_template();
    w
}

fn host_control(popup: &Popup) -> Ref<Control> {
    popup.host().expect("the popup is open").as_control()
}

/// A focusable control: stands in for the buttons and text boxes of the
/// reference tests, which only use them as focusable elements.
fn focusable() -> Ref<Border> {
    let border = Border::new();
    border.set_focusable(true);
    border
}

fn popup_content_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|control, scope| {
        let popup = Popup::new();
        popup.set_name(Some("popup".to_string()));
        popup.set_placement_target(control.clone());
        let presenter = ContentPresenter::new();
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        popup.set_child(presenter);
        popup.register_in_name_scope(&**scope).upcast()
    })
}

#[repr(C)]
struct PopupContentControl {
    base: ContentControl,
}

ferro_class!(PopupContentControl: ContentControl);
ferro_impl_classes!(
    PopupContentControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl PopupContentControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ContentControl::construct() })
    }
}

#[repr(C)]
struct TestControl {
    base: Decorator,
    data_context_begin_update: Cell<i32>,
}

ferro_class!(TestControl: Decorator);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl StyledElementImpl for TestControl {
    fn on_data_context_begin_update(this: &Self) {
        this.data_context_begin_update.set(this.data_context_begin_update.get() + 1);
        Self::parent_on_data_context_begin_update(this);
    }
}

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Decorator::construct(), data_context_begin_update: Cell::new(0) })
    }
}

/// A hit tester that answers `result` for the first visual at `point`
/// under `root` and nothing otherwise.
struct TestHitTester {
    point: Point,
    root: RefCell<Option<Ref<Visual>>>,
    result: RefCell<Option<Ref<Visual>>>,
}

impl IHitTester for TestHitTester {
    fn hit_test(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        Vec::new()
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(
        &self,
        p: Point,
        root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<Ref<Visual>> {
        let expected_root = self.root.borrow().clone();
        if p == self.point && expected_root.is_some_and(|expected_root| expected_root == root.to_ref()) {
            return self.result.borrow().clone();
        }
        None
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

thread_local! {
    /// A bubbling routed event (the reference tests borrow the click event
    /// of buttons for it).
    static TEST_CLICK_EVENT: &'static RoutedEvent<RoutedEventArgs> =
        RoutedEvent::register::<Popup, RoutedEventArgs>("TestClick", RoutingStrategies::BUBBLE);
}

/// Watches the position of the host of `popup`: `raised` is set when a
/// native popup root reports a position (that satisfies `expected`, which
/// is asserted) or when the canvas position of an overlay popup host
/// changes (to `expected`, when given).
fn watch_host_position(popup: &Popup, expected: Option<PixelPoint>) -> Rc<Cell<bool>> {
    let raised = Rc::new(Cell::new(false));
    let host = popup.host().expect("the popup is open");
    if let Some(popup_root) = host.as_popup_root() {
        let raised = raised.clone();
        let _ = popup_root.position_changed(move |args| {
            if let Some(expected) = expected {
                assert_eq!(expected, args.point());
            }
            raised.set(true);
        });
    } else if let Some(overlay_popup_host) = host.as_overlay_popup_host() {
        let raised = raised.clone();
        let weak = overlay_popup_host.downgrade();
        let _ = overlay_popup_host.property_changed(move |args| {
            if args.property() == Canvas::top_property().as_property()
                || args.property() == Canvas::left_property().as_property()
            {
                match expected {
                    None => raised.set(true),
                    Some(expected) => {
                        let Some(overlay_popup_host) = weak.upgrade() else { return };
                        if Canvas::get_left(&overlay_popup_host) == expected.x as f64
                            && Canvas::get_top(&overlay_popup_host) == expected.y as f64
                        {
                            raised.set(true);
                        }
                    }
                }
            }
        });
    }
    raised
}

/// An open popup whose placement target is its window forms a cycle with
/// the window (opened popups -> popup -> placement target). Closing the
/// window closes the popup, after which nothing keeps either alive.
fn closing_window_closes_its_open_popup_and_frees_both(use_popup_host: bool) {
    let _app = create_services(use_popup_host);

    let (weak_window, weak_popup, closed) = {
        let window = prepared_window(None);
        let popup = Popup::new();
        popup.set_child(Border::new());
        window.set_content(Some(Control::boxed(popup.clone())));
        window.layout_manager().execute_initial_layout_pass();

        popup.open();
        assert!(popup.is_open());
        assert_eq!(1, window.opened_popups().len());

        let closed = Rc::new(Cell::new(false));
        let _ = popup.closed({
            let closed = closed.clone();
            move || closed.set(true)
        });

        window.close();
        assert!(!popup.is_open());
        assert!(window.opened_popups().is_empty());

        (window.downgrade(), popup.downgrade(), closed)
    };
    Dispatcher::ui_thread().run_jobs(None);

    assert!(closed.get());
    assert!(weak_window.upgrade().is_none());
    assert!(weak_popup.upgrade().is_none());
}

/// The handlers an open popup adds to the notifications of its parent's
/// platform implementation are removed when it closes.
fn reopening_a_popup_does_not_accumulate_platform_handlers(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let window = prepared_window(None);
    let popup = Popup::new();
    popup.set_child(Border::new());
    window.set_content(Some(Control::boxed(popup.clone())));
    window.layout_manager().execute_initial_layout_pass();

    let window_impl = window.platform_impl().unwrap();
    let lost_focus = ITopLevelImpl::lost_focus(&*window_impl).map(|f| Rc::as_ptr(&f) as *const ());
    let deactivated = window_impl.deactivated().map(|f| Rc::as_ptr(&f) as *const ());
    let position_changed = window_impl.position_changed().map(|f| Rc::as_ptr(&f) as *const ());

    for _ in 0..3 {
        popup.open();
        popup.close();
    }

    // The callbacks of the implementation are the ones the window set.
    assert_eq!(lost_focus, ITopLevelImpl::lost_focus(&*window_impl).map(|f| Rc::as_ptr(&f) as *const ()));
    assert_eq!(deactivated, window_impl.deactivated().map(|f| Rc::as_ptr(&f) as *const ()));
    assert_eq!(position_changed, window_impl.position_changed().map(|f| Rc::as_ptr(&f) as *const ()));
}

fn centered_placement_target() -> Ref<Panel> {
    let placement_target = Panel::new();
    placement_target.set_width(10.0);
    placement_target.set_height(10.0);
    placement_target.set_horizontal_alignment(HorizontalAlignment::Center);
    placement_target.set_vertical_alignment(VerticalAlignment::Center);
    placement_target
}

fn popup_open_without_target_should_attach_itself_later(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let opened_event = Rc::new(Cell::new(0));
    let target = Popup::new();
    let count = opened_event.clone();
    let _ = target.opened(move || count.set(count.get() + 1));
    target.set_is_open(true);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();
    assert_eq!(1, opened_event.get());
}

fn popup_without_top_level_shouldnt_call_open(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let opened_event = Rc::new(Cell::new(0));
    let target = Popup::new();
    let count = opened_event.clone();
    let _ = target.opened(move || count.set(count.get() + 1));
    target.set_is_open(true);

    assert_eq!(0, opened_event.get());
}

fn opening_popup_shouldnt_throw_when_not_in_visual_tree(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    target.set_is_open(true);
}

fn opening_popup_shouldnt_throw_when_in_tree_without_top_level(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let c = Control::new();
    let target = Popup::new();
    target.set_parent(c);
    target.set_is_open(true);
}

fn setting_child_should_set_child_controls_logical_parent(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();

    target.set_child(child.clone());

    assert_eq!(child.parent(), Some(target.upcast()));
}

fn clearing_child_should_clear_child_controls_parent(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();

    target.set_child(child.clone());
    target.set_child(None);

    assert!(child.parent().is_none());
}

fn child_control_should_appear_in_logical_children(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();

    target.set_child(child.clone());

    assert_eq!(vec![child.upcast::<StyledElement>()], target.logical_children().to_vec());
}

fn clearing_child_should_remove_from_logical_children(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();

    target.set_child(child);
    target.set_child(None);

    assert!(target.logical_children().to_vec().is_empty());
}

fn setting_child_should_fire_logical_children_collection_changed(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();
    let called = Rc::new(Cell::new(false));

    let flag = called.clone();
    target
        .logical_children()
        .add_collection_changed(Rc::new(move |e| flag.set(e.action == NotifyCollectionChangedAction::Add)));

    target.set_child(child);

    assert!(called.get());
}

fn clearing_child_should_fire_logical_children_collection_changed(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();
    let called = Rc::new(Cell::new(false));

    target.set_child(child);

    let flag = called.clone();
    target
        .logical_children()
        .add_collection_changed(Rc::new(move |e| flag.set(e.action == NotifyCollectionChangedAction::Remove)));

    target.set_child(None);

    assert!(called.get());
}

fn changing_child_should_fire_logical_children_collection_changed(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child1 = Control::new();
    let child2 = Control::new();
    let called = Rc::new(Cell::new(false));

    target.set_child(child1);

    let flag = called.clone();
    target.logical_children().add_collection_changed(Rc::new(move |_| flag.set(true)));

    target.set_child(child2);

    assert!(called.get());
}

fn setting_child_should_not_set_childs_visual_parent(_use_popup_host: bool) {
    let _dispatcher = Dispatcher::unit_test_scope();
    let target = Popup::new();
    let child = Control::new();

    target.set_child(child.clone());

    assert!(child.visual_parent().is_none());
}

fn popup_root_should_initially_be_null(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();

    assert!(target.host().is_none());
}

fn popup_root_should_have_popup_as_logical_parent(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_placement_target(prepared_window(None));

    target.open();

    assert_eq!(Some(target.clone().upcast::<StyledElement>()), host_control(&target).parent());
}

fn popup_root_should_be_detached_from_logical_tree_when_popup_is_detached(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);
    let root = prepared_window(Some(target.clone().upcast()));

    target.open();

    let popup_root = host_control(&target);

    assert!(popup_root.is_attached_to_logical_tree());
    root.set_content(None);
    assert!(!target.is_attached_to_logical_tree());
}

fn should_close_when_control_detaches(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let button = Border::new();
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);
    target.set_placement_target(button.clone());
    let root = prepared_window(Some(button.upcast()));

    target.open();

    assert!(target.is_open());
    root.set_content(None);
    assert!(!target.is_open());
}

fn popup_open_should_raise_single_opened_event(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let window = prepared_window(None);
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);

    window.set_content(Some(Control::boxed(target.clone())));

    let opened_count = Rc::new(Cell::new(0));

    let count = opened_count.clone();
    let _ = target.opened(move || count.set(count.get() + 1));

    target.open();

    assert_eq!(1, opened_count.get());
}

fn popup_close_should_raise_single_closed_event(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let window = prepared_window(None);
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);

    window.set_content(Some(Control::boxed(target.clone())));
    window.apply_template();
    target.open();

    let closed_count = Rc::new(Cell::new(0));

    let count = closed_count.clone();
    let _ = target.closed(move || count.set(count.get() + 1));

    target.close();

    assert_eq!(1, closed_count.get());
}

fn popup_close_on_closed_popup_should_not_raise_closed_event(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let window = prepared_window(None);
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);

    window.set_content(Some(Control::boxed(target.clone())));
    window.apply_template();

    let closed_count = Rc::new(Cell::new(0));

    let count = closed_count.clone();
    let _ = target.closed(move || count.set(count.get() + 1));

    target.close();
    target.close();
    target.close();
    target.close();

    assert_eq!(0, closed_count.get());
}

fn content_control_with_popup_in_template_should_set_templated_parent(use_popup_host: bool) {
    // Test uses OverlayPopupHost default template
    let _app = create_services(use_popup_host);
    let target = PopupContentControl::new();
    target.set_content(Some(Control::boxed(Border::new())));
    target.set_template(Some(popup_content_control_template()));
    let root = prepared_window(Some(target.clone().upcast()));
    root.show();

    target.apply_template();

    let popup = target
        .get_template_descendants()
        .into_iter()
        .find(|x| x.name().as_deref() == Some("popup"))
        .and_then(|x| x.cast::<Popup>())
        .unwrap();
    popup.open();

    let popup_root = host_control(&popup);
    popup_root.measure(Size::INFINITY);
    popup_root.arrange(Rect::from_size(popup_root.desired_size()));

    let children: Vec<Ref<Visual>> = popup_root.get_visual_descendants().collect();
    let types: Vec<&str> = children.iter().map(|x| x.get_type().name()).collect();

    if use_popup_host {
        assert_eq!(
            vec!["LayoutTransformControl", "VisualLayerManager", "ContentPresenter", "ContentPresenter", "Border"],
            types
        );
    } else {
        assert_eq!(
            vec![
                "LayoutTransformControl",
                "Panel",
                "Border",
                "VisualLayerManager",
                "ContentPresenter",
                "ContentPresenter",
                "Border",
            ],
            types
        );
    }

    let templated_parents: Vec<Option<Ref<FerroObject>>> =
        children.iter().filter_map(|x| x.cast::<Control>()).map(|x| x.templated_parent()).collect();

    let popup_root: Option<Ref<FerroObject>> = Some(popup_root.upcast());
    let target: Option<Ref<FerroObject>> = Some(target.upcast());

    if use_popup_host {
        assert_eq!(
            vec![popup_root.clone(), popup_root.clone(), popup_root, target, None],
            templated_parents
        );
    } else {
        assert_eq!(
            vec![
                popup_root.clone(),
                popup_root.clone(),
                popup_root.clone(),
                popup_root.clone(),
                popup_root,
                target,
                None,
            ],
            templated_parents
        );
    }
}

fn data_context_begin_update_should_not_be_called_for_controls_that_dont_inherit(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let child = TestControl::new();
    let popup = Popup::new();
    popup.set_child(child.clone());
    popup.set_data_context(Some(Rc::new("foo".to_string())));
    popup.set_placement_target(prepared_window(None));

    child.data_context_begin_update.set(0);

    // Test for #1245. Here, the child's logical parent is the popup but it's not yet
    // attached to a visual tree because the popup hasn't been opened.
    assert_eq!(Some(popup.clone().upcast::<StyledElement>()), child.parent());
    assert_eq!(Some(popup.clone().upcast::<FerroObject>()), child.inheritance_parent());
    assert!(child.visual_root().is_none());

    popup.open();

    // #1245 was caused by the fact that DataContextBeginUpdate was called on `target`
    // when the PopupRoot was created, even though PopupRoot isn't the
    // InheritanceParent of child.
    assert_eq!(0, child.data_context_begin_update.get());
}

fn popup_host_type_should_match_platform_preference(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_placement_target(prepared_window(None));

    target.open();
    if use_popup_host {
        assert!(host_control(&target).get_type() == OverlayPopupHost::TYPE);
    } else {
        assert!(host_control(&target).get_type() == PopupRoot::TYPE);
    }
}

fn overlay_dismiss_event_pass_through_should_pass_event_to_window_contents(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let hit_tester =
        Rc::new(TestHitTester { point: Point::new(10.0, 15.0), root: RefCell::new(None), result: RefCell::new(None) });

    let window = Window::with_impl(create_window_impl(use_popup_host));
    window.set_hit_tester_override(Some(hit_tester.clone()));
    window.apply_styling();
    window.apply_template();

    let target = Popup::new();
    target.set_placement_target(window.clone());
    target.set_is_light_dismiss_enabled(true);
    target.set_overlay_dismiss_event_pass_through(true);

    let raised = Rc::new(Cell::new(0));
    let border = Border::new();
    window.set_content(Some(Control::boxed(border.clone())));

    *hit_tester.root.borrow_mut() = window.visual_root();
    *hit_tester.result.borrow_mut() = Some(border.clone().upcast());

    let count = raised.clone();
    let expected_source: Ref<FerroObject> = border.clone().upcast();
    border.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
        assert_eq!(Some(expected_source.clone()), e.source());
        count.set(count.get() + 1);
    });

    target.open();
    assert!(target.is_open());

    let e = create_pointer_pressed_event_args(&window, Point::new(10.0, 15.0));
    let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window);
    let overlay = overlay.expect("the window has a light dismiss overlay layer");
    overlay.raise_event(&e);

    assert_eq!(1, raised.get());
    assert!(!target.is_open());
}

fn focusable_controls_in_popup_should_get_focus(use_popup_host: bool) {
    let _app = create_services_with_focus(use_popup_host);
    let window_panel = Panel::new();
    window_panel.children().add(Border::new());
    let window = prepared_window(Some(window_panel.upcast()));

    let text_box = focusable();
    let button = focusable();
    let stack_panel = StackPanel::new();
    stack_panel.children().add(text_box.clone());
    stack_panel.children().add(button.clone());
    let popup = Popup::new();
    popup.set_placement_target(window.clone());
    popup.set_child(stack_panel);

    popup.set_parent(popup.placement_target().unwrap());
    window.show();
    popup.open();

    button.focus();

    let host = popup.host().unwrap();
    let input_root = host.as_control().get_input_root();

    let focus_manager = input_root.unwrap().focus_manager().unwrap();
    assert_eq!(Some(button.clone().upcast::<InputElement>()), focus_manager.get_focused_element());

    //Ensure focus remains in the popup
    let handler = if let Some(popup_root) = host.as_popup_root() {
        popup_root.tests_keyboard_navigation_handler()
    } else if let Some(overlay_popup_host) = host.as_overlay_popup_host() {
        overlay_popup_host.tests_keyboard_navigation_handler()
    } else {
        panic!("Unknown popup host type")
    }
    .unwrap();

    handler.move_(focus_manager.get_focused_element().as_ref(), NavigationDirection::Next, KeyModifiers::NONE, None);
    assert_eq!(Some(text_box.upcast::<InputElement>()), focus_manager.get_focused_element());

    popup.close();
}

fn popup_should_clear_keyboard_focus_from_children_when_closed(use_popup_host: bool) {
    let _app = create_services_with_focus(use_popup_host);
    let win_button = focusable();
    let window_panel = Panel::new();
    window_panel.children().add(win_button.clone());
    let window = prepared_window(Some(window_panel.upcast()));

    let border1 = Border::new();
    let border2 = Border::new();
    let button = focusable();
    border1.set_child(border2.clone());
    border2.set_child(button.clone());
    let stack_panel = StackPanel::new();
    stack_panel.children().add(border1.clone());
    let popup = Popup::new();
    popup.set_placement_target(window.clone());
    popup.set_child(stack_panel);

    popup.set_parent(popup.placement_target().unwrap());
    window.show();
    win_button.focus();
    popup.open();

    button.focus();

    let input_root = host_control(&popup).get_input_root();

    let focus_manager = input_root.unwrap().focus_manager().unwrap();
    assert_eq!(Some(button.upcast::<InputElement>()), focus_manager.get_focused_element());

    border1.set_child(None);

    win_button.focus();

    assert!(!border2.is_keyboard_focus_within());
}

fn closing_popup_sets_focus_on_placement_target(use_popup_host: bool) {
    let _app = create_services_with_focus(use_popup_host);
    let window = prepared_window(None);
    window.set_focusable(true);

    let tb = focusable();
    let p = Popup::new();
    p.set_placement_target(window.clone());
    p.set_child(tb.clone());

    window.set_content(Some(Control::boxed(p.clone())));
    window.show();
    window.focus();
    p.open();

    if let Some(host) = p.host().unwrap().as_overlay_popup_host() {
        //Need to measure/arrange for visual children to show up
        //in OverlayPopupHost
        host.measure(Size::INFINITY);
        host.arrange(Rect::from_size(host.desired_size()));
    }

    tb.focus();

    p.close();

    let focus_manager = window.focus_manager();
    let focus = focus_manager.get_focused_element();
    assert_eq!(Some(window.upcast::<InputElement>()), focus);
}

fn prog_close_popup_no_light_dismiss_doesnt_move_focus_to_placement_target(use_popup_host: bool) {
    let _app = create_services_with_focus(use_popup_host);
    let window = prepared_window(None);

    let window_tb = focusable();
    window.set_content(Some(Control::boxed(window_tb.clone())));

    let popup_tb = focusable();
    let p = Popup::new();
    p.set_placement_target(window.clone());
    p.set_is_light_dismiss_enabled(false);
    p.set_child(popup_tb.clone());
    p.set_parent(p.placement_target().unwrap());
    window.show();

    p.open();

    if let Some(host) = p.host().unwrap().as_overlay_popup_host() {
        //Need to measure/arrange for visual children to show up
        //in OverlayPopupHost
        host.measure(Size::INFINITY);
        host.arrange(Rect::from_size(host.desired_size()));
    }

    popup_tb.focus();

    window_tb.focus();

    let focus_manager = window.focus_manager();
    let focus = focus_manager.get_focused_element();

    assert_eq!(Some(window_tb.clone().upcast::<InputElement>()), focus);

    p.close();

    assert_eq!(Some(window_tb.upcast::<InputElement>()), focus);
}

fn popup_should_not_follow_placement_target_on_window_move_if_pointer(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let popup = Popup::new();
    popup.set_width(400.0);
    popup.set_height(200.0);
    popup.set_placement(PlacementMode::Pointer);
    let window = prepared_window(Some(popup.clone().upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));

    let raised = watch_host_position(&popup, None);
    window.set_position(PixelPoint::new(10, 10));
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));
    assert!(!raised.get());
}

fn popup_should_follow_placement_target_on_window_resize(use_popup_host: bool) {
    let _app = create_services(use_popup_host);

    let placement_target = centered_placement_target();
    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_width(10.0);
    popup.set_height(10.0);
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(placement_target.clone().upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));

    // The target's initial placement is (395,295) which is a 10x10 panel centered in a 800x600 window
    assert_eq!(placement_target.bounds(), Rect::new(395.0, 295.0, 10.0, 10.0));

    // Resizing the window to 700x500 must move the popup to (345,255) as this is the new
    // location of the placement target
    let raised = watch_host_position(&popup, Some(PixelPoint::new(345, 255)));
    if let Some(platform_impl) = window.platform_impl() {
        platform_impl.resize(Size::new(700.0, 500.0), WindowResizeReason::Unspecified);
    }
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));
    assert!(raised.get());
}

fn popup_should_not_follow_placement_target_on_window_resize_if_pointer_if_pointer(use_popup_host: bool) {
    let _app = create_services(use_popup_host);

    let placement_target = centered_placement_target();
    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_placement(PlacementMode::Pointer);
    popup.set_width(10.0);
    popup.set_height(10.0);
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(placement_target.clone().upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));

    // The target's initial placement is (395,295) which is a 10x10 panel centered in a 800x600 window
    assert_eq!(placement_target.bounds(), Rect::new(395.0, 295.0, 10.0, 10.0));

    let raised = watch_host_position(&popup, None);
    if let Some(platform_impl) = window.platform_impl() {
        platform_impl.resize(Size::new(700.0, 500.0), WindowResizeReason::Unspecified);
    }
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));
    assert!(!raised.get());
}

fn popup_should_follow_placement_target_on_target_moved(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let placement_target = centered_placement_target();
    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_width(10.0);
    popup.set_height(10.0);
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(placement_target.clone().upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    // The target's initial placement is (395,295) which is a 10x10 panel centered in a 800x600 window
    assert_eq!(placement_target.bounds(), Rect::new(395.0, 295.0, 10.0, 10.0));

    // Margin will move placement target
    let raised = watch_host_position(&popup, Some(PixelPoint::new(400, 305)));
    placement_target.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(raised.get());
}

fn popup_should_not_follow_placement_target_on_target_moved_if_pointer(use_popup_host: bool) {
    let _app = create_services(use_popup_host);

    let placement_target = centered_placement_target();
    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_placement(PlacementMode::Pointer);
    popup.set_width(10.0);
    popup.set_height(10.0);
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(placement_target.clone().upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    // The target's initial placement is (395,295) which is a 10x10 panel centered in a 800x600 window
    assert_eq!(placement_target.bounds(), Rect::new(395.0, 295.0, 10.0, 10.0));

    let raised = watch_host_position(&popup, None);
    placement_target.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!raised.get());
}

fn popup_should_follow_popup_root_placement_target(use_popup_host: bool) {
    // When the placement target of a popup is another popup (e.g. nested menu items), the child popup must
    // follow the parent popup if it moves (due to root window movement or resize)
    let _app = create_services(use_popup_host);
    // The child popup is placed directly over the parent popup for position testing
    let parent_popup = Popup::new();
    parent_popup.set_width(10.0);
    parent_popup.set_height(10.0);
    let child_popup = Popup::new();
    child_popup.set_width(20.0);
    child_popup.set_height(20.0);
    child_popup.set_placement_target(parent_popup.clone());
    child_popup.set_placement(PlacementMode::AnchorAndGravity);
    child_popup.set_placement_anchor(PopupAnchor::TOP_LEFT);
    child_popup.set_placement_gravity(PopupGravity::BOTTOM_RIGHT);
    child_popup.set_parent(child_popup.placement_target().unwrap());

    let window = prepared_window(Some(parent_popup.clone().upcast()));
    window.show();
    parent_popup.open();
    child_popup.open();

    if let Some(popup_root) = child_popup.host().unwrap().as_popup_root() {
        let raised = Rc::new(Cell::new(false));
        let flag = raised.clone();
        let _ = popup_root.position_changed(move |args| {
            // The parent's initial placement is (395,295) which is a 10x10 popup centered
            // in a 800x600 window. When the window is moved, the child's final placement is (405, 305)
            // which is the parent's placement moved 10 pixels left and down.
            assert_eq!(PixelPoint::new(405, 305), args.point());
            flag.set(true);
        });

        window.set_position(PixelPoint::new(10, 10));
        assert!(raised.get());
    }
}

fn child_margin_should_not_affect_popup_position(use_popup_host: bool) {
    let _app = create_services(use_popup_host);

    let placement_target = centered_placement_target();

    let popup_child = Border::new();
    popup_child.set_width(10.0);
    popup_child.set_height(10.0);

    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_placement(PlacementMode::BottomEdgeAlignedLeft);
    popup.set_child(popup_child.clone());
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(placement_target.upcast()));
    window.show();
    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let get_popup_position = || -> Point {
        let host = popup.host().unwrap();
        if use_popup_host {
            host.as_overlay_popup_host().unwrap().translate_point(Point::default(), &window).unwrap()
        } else {
            let platform_impl = host.as_popup_root().unwrap().platform_impl();
            let platform_impl = platform_impl.expect("the popup root has a platform implementation");
            platform_impl.position().to_point(platform_impl.render_scaling())
        }
    };

    let initial_position = get_popup_position();

    const CHILD_MARGIN_LENGTH: f64 = 20.0;

    popup_child.set_margin(Thickness::uniform(CHILD_MARGIN_LENGTH));

    // The popup's bounds will now include the child's margin, but the positioning system should have substracted this
    // to keep the child's bounds stable. Thus the popup as a whole should have moved upwards and to the left.
    let expected = initial_position - Point::new(CHILD_MARGIN_LENGTH, CHILD_MARGIN_LENGTH);

    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(expected, get_popup_position());
}

fn events_should_be_routed_to_popup_parent(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let popup_content = Border::new();
    let popup = Popup::new();
    popup.set_child(popup_content.clone());
    let popup_parent = Border::new();
    popup_parent.set_child(popup.clone());
    let root = prepared_window(Some(popup_parent.clone().upcast()));
    let raised = Rc::new(Cell::new(0));

    root.layout_manager().execute_initial_layout_pass();
    popup.open();
    root.layout_manager().execute_layout_pass();

    let click_event = TEST_CLICK_EVENT.with(|event| *event);
    let ev = RoutedEventArgs::with_event(click_event);

    let count = raised.clone();
    popup_parent.add_handler(click_event, move |_, _| count.set(count.get() + 1));
    popup_content.raise_event(&ev);

    assert_eq!(1, raised.get());
}

fn get_position_on_control_in_popup_called_from_parent_should_return_valid_coordinates(use_popup_host: bool) {
    // This test only applies when using a PopupRoot host and not an overlay popup.
    if use_popup_host {
        return;
    }

    let _app = create_services(use_popup_host);
    let popup_content = Border::new();
    popup_content.set_height(100.0);
    popup_content.set_width(100.0);
    let red: Rc<dyn IBrush> = Brushes::red();
    popup_content.set_background(Some(red));
    let popup = Popup::new();
    popup.set_child(popup_content.clone());
    popup.set_horizontal_offset(40.0);
    popup.set_vertical_offset(40.0);
    popup.set_placement(PlacementMode::AnchorAndGravity);
    popup.set_placement_anchor(PopupAnchor::TOP_LEFT);
    popup.set_placement_gravity(PopupGravity::BOTTOM_RIGHT);
    let popup_parent = Border::new();
    popup_parent.set_child(popup.clone());
    let _root = prepared_window(Some(popup_parent.clone().upcast()));

    popup.open();

    // Verify that the popup is positioned at 40,40 as descibed by the Horizontal/
    // VerticalOffset: 10,10 becomes 50,50 in screen coordinates.
    assert_eq!(PixelPoint::new(50, 50), popup_content.point_to_screen(Point::new(10.0, 10.0)));

    // The popup parent is positioned at 0,0 in screen coordinates so client and
    // screen coordinates are the same.
    assert_eq!(PixelPoint::new(10, 10), popup_parent.point_to_screen(Point::new(10.0, 10.0)));

    // The event will be raised on the popup content at 50,50 (90,90 in screen coordinates)
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let popup_root = TopLevel::get_top_level(Some(&popup_content)).and_then(|tl| tl.cast::<PopupRoot>()).unwrap();
    let ev = PointerPressedEventArgs::new(
        popup_content.clone(),
        pointer,
        &popup_root,
        Point::new(50.0, 50.0),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    );

    let content_raised = Rc::new(Cell::new(0));
    let parent_raised = Rc::new(Cell::new(0));

    // The event is raised on the popup content in popup coordinates.
    let count = content_raised.clone();
    let relative_to = popup_content.clone();
    popup_content.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
        count.set(count.get() + 1);
        assert_eq!(Point::new(50.0, 50.0), e.get_position(Some(&relative_to)));
    });

    // The event is raised on the parent in root coordinates (which in this case are
    // the same as screen coordinates).
    let count = parent_raised.clone();
    let relative_to = popup_parent.clone();
    popup_parent.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
        count.set(count.get() + 1);
        assert_eq!(Point::new(90.0, 90.0), e.get_position(Some(&relative_to)));
    });

    popup_content.raise_event(&ev);

    assert_eq!(1, content_raised.get());
    assert_eq!(1, parent_raised.get());
}

fn popup_attached_to_adorner_respects_adorner_position(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let popup_target = Border::new();
    popup_target.set_height(30.0);
    let red: Rc<dyn IBrush> = Brushes::red();
    popup_target.set_background(Some(red));
    DockPanel::set_dock(&popup_target, Dock::Top);
    let popup_content = Border::new();
    popup_content.set_height(30.0);
    popup_content.set_width(50.0);
    let yellow: Rc<dyn IBrush> = Brushes::yellow();
    popup_content.set_background(Some(yellow));
    let popup = Popup::new();
    popup.set_child(popup_content.clone());
    popup.set_placement(PlacementMode::AnchorAndGravity);
    popup.set_placement_target(popup_target.clone());
    popup.set_placement_anchor(PopupAnchor::BOTTOM_RIGHT);
    popup.set_placement_gravity(PopupGravity::BOTTOM_RIGHT);
    let adorner = DockPanel::new();
    adorner.children().add(popup_target);
    adorner.children().add(popup.clone());
    adorner.set_horizontal_alignment(HorizontalAlignment::Left);
    adorner.set_width(40.0);
    adorner.set_margin(Thickness::new(50.0, 5.0, 0.0, 0.0));

    let adorned = Border::new();
    adorned.set_width(100.0);
    adorned.set_height(100.0);
    let blue: Rc<dyn IBrush> = Brushes::blue();
    adorned.set_background(Some(blue));
    Canvas::set_left(&adorned, 20.0);
    Canvas::set_top(&adorned, 40.0);
    let window_content = Canvas::new();
    window_content.children().add(adorned.clone());

    let root = prepared_window(Some(window_content.upcast()));

    let adorner_layer = AdornerLayer::get_adorner_layer(&adorned);
    let adorner_layer = adorner_layer.expect("the window has an adorner layer");
    adorner_layer.children().add(adorner.clone());
    AdornerLayer::set_adorned_element(&adorner, adorned);

    root.layout_manager().execute_initial_layout_pass();
    popup.open();
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::AFTER_RENDER));

    // X: Adorned Canvas.Left + Adorner Margin Left + Adorner Width
    // Y: Adorned Canvas.Top + Adorner Margin Top + Adorner Height
    assert_eq!(PixelPoint::new(110, 75), popup_content.point_to_screen(Point::new(0.0, 0.0)));
}

fn custom_placement_callback_is_executed(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let callback_executed = Rc::new(Cell::new(0));
    let popup_content = Border::new();
    popup_content.set_width(100.0);
    popup_content.set_height(100.0);
    let popup = Popup::new();
    popup.set_child(popup_content.clone());
    popup.set_placement(PlacementMode::Custom);
    popup.set_horizontal_offset(42.0);
    popup.set_vertical_offset(21.0);
    let popup_parent = Border::new();
    popup_parent.set_child(popup.clone());
    let root = prepared_window(Some(popup_parent.upcast()));

    let count = callback_executed.clone();
    let weak_popup = popup.downgrade();
    let weak_root = root.downgrade();
    popup.set_custom_popup_placement_callback(Some(Rc::new(move |parameters: &mut CustomPopupPlacement| {
        let popup = weak_popup.upgrade().unwrap();
        let root = weak_root.upgrade().unwrap();

        assert_eq!(popup_content.width(), parameters.popup_size().width);
        assert_eq!(popup_content.height(), parameters.popup_size().height);

        assert_eq!(root.width(), parameters.anchor_rectangle.width);
        assert_eq!(root.height(), parameters.anchor_rectangle.height);

        assert_eq!(popup.horizontal_offset(), parameters.offset.x);
        assert_eq!(popup.vertical_offset(), parameters.offset.y);

        count.set(count.get() + 1);

        parameters.set_anchor(PopupAnchor::TOP);
        parameters.set_gravity(PopupGravity::BOTTOM);
    })));

    root.layout_manager().execute_initial_layout_pass();
    popup.open();
    root.layout_manager().execute_layout_pass();

    assert_eq!(1, callback_executed.get());
}

fn popup_is_hit_test_visible_defaults_to_true(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    assert!(Popup::new().is_hit_test_visible());
}

fn popup_forwards_is_hit_test_visible_to_host_on_open(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_is_hit_test_visible(false);
    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.open();

    assert!(!target.host().unwrap().is_hit_test_visible());
}

fn popup_forwards_is_hit_test_visible_changes_to_open_host(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.open();
    assert!(target.host().unwrap().is_hit_test_visible());

    target.set_is_hit_test_visible(false);

    assert!(!target.host().unwrap().is_hit_test_visible());
}

fn popup_open_with_correct_is_using_overlay_layer_and_disabled_overlay_layer(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_is_open(true);
    target.set_should_use_overlay_layer(false);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    assert_eq!(use_popup_host, target.is_using_overlay_layer());
}

fn popup_open_with_correct_is_using_overlay_layer_and_enabled_overlay_layer(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    target.set_is_open(true);
    target.set_should_use_overlay_layer(true);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    assert!(target.is_using_overlay_layer());
}

fn closing_previous_light_dismiss_popup_should_not_affect_overlay_for_next_popup(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let placement_target = Border::new();
    let window = prepared_window(Some(placement_target.clone().upcast()));
    let first = Popup::new();
    first.set_placement_target(placement_target.clone());
    first.set_is_light_dismiss_enabled(true);
    let second = Popup::new();
    second.set_placement_target(placement_target);
    second.set_is_light_dismiss_enabled(true);

    first.open();
    second.open();

    let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window);
    let overlay = overlay.expect("the window has a light dismiss overlay layer");

    first.close();

    assert!(overlay.is_visible());

    overlay.raise_event(&create_pointer_pressed_event_args(&window, Point::new(10.0, 15.0)));

    assert!(!second.is_open());
    assert!(!overlay.is_visible());
}

fn opened_popup_should_be_in_opened_popups(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    let window = prepared_window(Some(target.clone().upcast()));

    target.open();

    assert_eq!(vec![target.clone()], window.opened_popups());

    target.close();

    assert!(window.opened_popups().is_empty());
}

fn closing_popup_with_is_open_should_remove_it_from_opened_popups(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    let window = prepared_window(Some(target.clone().upcast()));

    target.set_is_open(true);

    assert_eq!(vec![target.clone()], window.opened_popups());

    target.set_is_open(false);

    assert!(window.opened_popups().is_empty());
}

fn closing_window_should_clear_opened_popups(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let target = Popup::new();
    let window = prepared_window(Some(target.clone().upcast()));

    target.open();
    window.close();

    assert!(window.opened_popups().is_empty());
}

fn nested_popup_should_be_in_parent_popup_opened_popups(use_popup_host: bool) {
    let _app = create_services(use_popup_host);
    let nested_target = Border::new();
    nested_target.set_width(20.0);
    nested_target.set_height(20.0);
    let nested_child = Border::new();
    nested_child.set_width(10.0);
    nested_child.set_height(10.0);
    let nested_popup = Popup::new();
    nested_popup.set_placement_target(nested_target.clone());
    nested_popup.set_child(nested_child);
    let target = Border::new();
    let popup_panel = Panel::new();
    popup_panel.children().add(nested_target);
    popup_panel.children().add(nested_popup.clone());
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_child(popup_panel);
    let window_panel = Panel::new();
    window_panel.children().add(target);
    window_panel.children().add(popup.clone());
    let window = prepared_window(Some(window_panel.upcast()));

    popup.open();

    if let Some(host) = popup.host().unwrap().as_overlay_popup_host() {
        //Need to measure/arrange for visual children to show up
        //in OverlayPopupHost
        host.measure(Size::INFINITY);
        host.arrange(Rect::from_size(host.desired_size()));
    }

    nested_popup.open();

    assert_eq!(vec![popup.clone()], window.opened_popups());
    assert_eq!(vec![nested_popup.clone()], popup.opened_popups());
    assert!(nested_popup.opened_popups().is_empty());

    if let Some(popup_root) = popup.host().unwrap().as_popup_root() {
        // A popup root exposes the popups opened by its own popup.
        let top_level: &TopLevel = &popup_root;
        assert_eq!(vec![nested_popup.clone()], top_level.opened_popups());
    }

    nested_popup.close();

    assert_eq!(vec![popup.clone()], window.opened_popups());
    assert!(popup.opened_popups().is_empty());

    popup.close();

    assert!(window.opened_popups().is_empty());
}

/// Not in the reference suite: covers the transform tracking of a popup
/// that inherits the transform of its placement target.
fn inherits_transform_scales_the_host_with_the_placement_target(use_popup_host: bool) {
    use ferroui_base::media::{ITransform, ScaleTransform};

    let _app = create_services(use_popup_host);
    let placement_target = Border::new();
    placement_target.set_width(10.0);
    placement_target.set_height(10.0);
    let scaled = Decorator::new();
    let scale: Rc<dyn ITransform> = ScaleTransform::with_scale(2.0, 2.0).into();
    scaled.set_render_transform(Some(scale));
    scaled.set_child(placement_target.clone());
    let popup = Popup::new();
    popup.set_placement_target(placement_target.clone());
    popup.set_inherits_transform(true);
    popup.set_width(100.0);
    popup.set_height(50.0);
    popup.set_parent(popup.placement_target().unwrap());

    let window = prepared_window(Some(scaled.clone().upcast()));
    window.show();
    popup.open();

    let host = popup.host().unwrap();
    assert_eq!(200.0, host.width());
    assert_eq!(100.0, host.height());
    assert_eq!(Some(Matrix::create_scale(2.0, 2.0)), host.transform().map(|transform| transform.value()));

    // Moving the target changes its transform to the root: the host is sized again.
    popup.set_width(110.0);
    assert_eq!(220.0, host.width());
    placement_target.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(220.0, host.width());

    popup.set_inherits_transform(false);
    popup.close();
    popup.open();
    let host = popup.host().unwrap();
    assert_eq!(110.0, host.width());
    assert!(host.transform().is_none());
}

macro_rules! popup_tests {
    ($($name:ident,)*) => {
        /// The reference `PopupTests`: the platform creates native popups.
        mod popup_tests {
            $(
                #[test]
                fn $name() {
                    super::$name(false);
                }
            )*
        }

        /// The reference `PopupTestsWithPopupRoot`: the platform creates no
        /// popups, so they are shown in the overlay layer.
        mod popup_tests_with_popup_root {
            $(
                #[test]
                fn $name() {
                    super::$name(true);
                }
            )*
        }
    };
}

popup_tests! {
    popup_open_without_target_should_attach_itself_later,
    popup_without_top_level_shouldnt_call_open,
    opening_popup_shouldnt_throw_when_not_in_visual_tree,
    opening_popup_shouldnt_throw_when_in_tree_without_top_level,
    setting_child_should_set_child_controls_logical_parent,
    clearing_child_should_clear_child_controls_parent,
    child_control_should_appear_in_logical_children,
    clearing_child_should_remove_from_logical_children,
    setting_child_should_fire_logical_children_collection_changed,
    clearing_child_should_fire_logical_children_collection_changed,
    changing_child_should_fire_logical_children_collection_changed,
    setting_child_should_not_set_childs_visual_parent,
    popup_root_should_initially_be_null,
    popup_root_should_have_popup_as_logical_parent,
    popup_root_should_be_detached_from_logical_tree_when_popup_is_detached,
    should_close_when_control_detaches,
    popup_open_should_raise_single_opened_event,
    popup_close_should_raise_single_closed_event,
    popup_close_on_closed_popup_should_not_raise_closed_event,
    content_control_with_popup_in_template_should_set_templated_parent,
    data_context_begin_update_should_not_be_called_for_controls_that_dont_inherit,
    popup_host_type_should_match_platform_preference,
    overlay_dismiss_event_pass_through_should_pass_event_to_window_contents,
    focusable_controls_in_popup_should_get_focus,
    popup_should_clear_keyboard_focus_from_children_when_closed,
    closing_popup_sets_focus_on_placement_target,
    prog_close_popup_no_light_dismiss_doesnt_move_focus_to_placement_target,
    popup_should_not_follow_placement_target_on_window_move_if_pointer,
    popup_should_follow_placement_target_on_window_resize,
    popup_should_not_follow_placement_target_on_window_resize_if_pointer_if_pointer,
    popup_should_follow_placement_target_on_target_moved,
    popup_should_not_follow_placement_target_on_target_moved_if_pointer,
    popup_should_follow_popup_root_placement_target,
    child_margin_should_not_affect_popup_position,
    events_should_be_routed_to_popup_parent,
    get_position_on_control_in_popup_called_from_parent_should_return_valid_coordinates,
    popup_attached_to_adorner_respects_adorner_position,
    custom_placement_callback_is_executed,
    popup_is_hit_test_visible_defaults_to_true,
    popup_forwards_is_hit_test_visible_to_host_on_open,
    popup_forwards_is_hit_test_visible_changes_to_open_host,
    popup_open_with_correct_is_using_overlay_layer_and_disabled_overlay_layer,
    popup_open_with_correct_is_using_overlay_layer_and_enabled_overlay_layer,
    closing_previous_light_dismiss_popup_should_not_affect_overlay_for_next_popup,
    opened_popup_should_be_in_opened_popups,
    closing_popup_with_is_open_should_remove_it_from_opened_popups,
    closing_window_should_clear_opened_popups,
    nested_popup_should_be_in_parent_popup_opened_popups,
    inherits_transform_scales_the_host_with_the_placement_target,
    closing_window_closes_its_open_popup_and_frees_both,
    reopening_a_popup_does_not_accumulate_platform_handlers,
}
