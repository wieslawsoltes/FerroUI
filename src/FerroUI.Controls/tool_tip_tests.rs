//! Port of the reference `ToolTipTests`.
//!
//! The reference runs the suite twice: with the platform creating native
//! popups (`ToolTipTests_Popup`) and with the platform creating none, so
//! that tooltips are shown in the overlay layer (`ToolTipTests_Overlay`).
//! Every test body here takes that flag and is run by both variants; the
//! two tests of `ToolTipTests_Popup` only exist in that variant.

use crate::presentation_source::PresentationSource;
use crate::primitives::PopupOverlayLayer;
use crate::test_support::{boxed_str, string_of};
use crate::testing::{
    MockWindowImpl, MockWindowingPlatform, NullRenderer, TestServices, UnitTestApplication,
    UnitTestApplicationScope,
};
use crate::{Control, Decorator, IToolTipService, Panel, StackPanel, ToolTip, TopLevel, Window};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputDevice, IInputRoot, MouseDevice, Pointer, PointerType, RawInputModifiers};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::IHitTester;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_model, FerroLocator, FerroObject, LocatorExtensions, Point, Rect, Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The state the reference test classes set up around every test.
struct Fixture {
    overlay: bool,
    tool_tip_open_subscription: Option<Rc<dyn IDisposable>>,
}

impl Fixture {
    fn new(overlay: bool) -> Fixture {
        // The overlay variant measures the popup overlay layer whenever a
        // tooltip opens or closes.
        let tool_tip_open_subscription = overlay.then(|| {
            ToolTip::is_open_property().changed().subscribe(|e| {
                let Some(visual) = e.sender().downcast_ref::<Visual>() else { return };
                let Some(root) = TopLevel::get_top_level(Some(visual)) else { return };
                if let Some(layer) = PopupOverlayLayer::get_popup_overlay_layer(visual) {
                    layer.measure(root.client_size());
                }
            })
        });
        Fixture { overlay, tool_tip_open_subscription }
    }

    fn configure_services(&self, base_services: TestServices) -> TestServices {
        if self.overlay {
            base_services.with_windowing_platform(MockWindowingPlatform::with(None, Some(Rc::new(|_| None)), None))
        } else {
            base_services
        }
    }

    fn setup_window_mock(&self, window_impl: &MockWindowImpl) {
        if self.overlay {
            window_impl.setup_create_popup(|_| None);
        }
    }

    fn verify_tool_tip_type(&self, control: &Control) {
        let tool_tip = control.get_value(ToolTip::tool_tip_property()).expect("the control has a tooltip");
        let host = tool_tip.popup_host().expect("the tooltip is open");
        if self.overlay {
            assert!(host.as_overlay_popup_host().is_some());
            assert_eq!(tool_tip.visual_root(), control.visual_root());
        } else {
            let popup_root = host.as_popup_root().expect("the tooltip is shown in a popup root");
            assert_eq!(TopLevel::get_top_level(Some(&tool_tip)), Some(popup_root.upcast()));
        }
    }

    fn assert_tool_tip_open(&self, control: &Control) {
        assert!(ToolTip::get_is_open(control));
        self.verify_tool_tip_type(control);
    }

    fn setup_window(&self, window_content: &Ref<Control>, test_name: &str) -> ToolTipTestScope {
        let window_impl = MockWindowingPlatform::create_window_mock();
        self.setup_window_mock(&window_impl);

        let hit_tester = Rc::new(TestHitTester::default());

        let window = Window::with_impl(window_impl.clone());
        window.set_hit_tester_override(Some(hit_tester.clone()));
        window.set_content(Some(Control::boxed(window_content)));
        window.set_title(Some(test_name.to_string()));

        window.apply_styling();
        window.apply_template();
        window.presenter().expect("the window has a presenter").apply_template();
        window.show();

        assert!(window_content.is_attached_to_visual_tree());
        assert!(window_content.is_measure_valid());
        assert!(window_content.is_visible());

        ToolTipTestScope {
            window,
            window_impl,
            hit_tester,
            mouse_device: MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true)),
            control_ids: RefCell::new(Vec::new()),
            last_root: RefCell::new(None),
            next_timestamp: Cell::new(1),
        }
    }

    fn setup_window_and_activate_tool_tip(&self, window_content: &Ref<Control>, test_name: &str) {
        self.setup_window(window_content, test_name).mouse_enter(Some(window_content));
    }

    fn setup_window_and_activate_tool_tip_of(
        &self,
        window_content: &Ref<Control>,
        target_override: &Ref<Control>,
        test_name: &str,
    ) {
        self.setup_window(window_content, test_name).mouse_enter(Some(target_override));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(subscription) = self.tool_tip_open_subscription.take() {
            subscription.dispose();
        }
    }
}

/// A hit tester that answers the visual set up for a point as the first
/// visual at that point, and nothing otherwise.
#[derive(Default)]
struct TestHitTester {
    results: RefCell<Vec<(Point, Option<Ref<Visual>>)>>,
}

impl TestHitTester {
    fn setup(&self, point: Point, control: Option<&Ref<Control>>) {
        let result = control.map(|control| control.clone().upcast::<Visual>());
        let mut results = self.results.borrow_mut();
        match results.iter_mut().find(|(p, _)| *p == point) {
            Some(entry) => entry.1 = result,
            None => results.push((point, result)),
        }
    }
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
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<Ref<Visual>> {
        self.results.borrow().iter().find(|(point, _)| *point == p).and_then(|(_, result)| result.clone())
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

struct ToolTipTestScope {
    window: Ref<Window>,
    window_impl: Rc<MockWindowImpl>,
    hit_tester: Rc<TestHitTester>,
    mouse_device: Rc<MouseDevice>,
    control_ids: RefCell<Vec<Ref<Control>>>,
    last_root: RefCell<Option<Rc<PresentationSource>>>,
    next_timestamp: Cell<u64>,
}

impl ToolTipTestScope {
    fn timestamp(&self) -> u64 {
        let timestamp = self.next_timestamp.get();
        self.next_timestamp.set(timestamp + 1);
        timestamp
    }

    fn device(&self) -> Rc<dyn IInputDevice> {
        self.mouse_device.clone()
    }

    /// The presentation source of the tree `control` is in; the one of the
    /// window for no control.
    fn source_of(&self, control: Option<&Ref<Control>>) -> Rc<PresentationSource> {
        control
            .and_then(|control| TopLevel::get_top_level(Some(control)))
            .map(|top_level| top_level.presentation_source().clone())
            .unwrap_or_else(|| self.window.presentation_source().clone())
    }

    /// Returns a pointer position which hit tests to `control`, or to
    /// nothing when it's `None`.
    fn get_pointer_position(&self, control: Option<&Ref<Control>>) -> Point {
        let point = match control {
            None => Point::default(),
            Some(control) => {
                let mut control_ids = self.control_ids.borrow_mut();
                let id = match control_ids.iter().position(|c| c == control) {
                    Some(id) => id,
                    None => {
                        control_ids.push(control.clone());
                        control_ids.len() - 1
                    }
                };
                Point::new(id as f64, f64::from(i32::MAX))
            }
        };

        self.hit_tester.setup(point, control);

        point
    }

    /// Moves the pointer over `control`, leaving the previous root if it
    /// changed.
    fn mouse_enter(&self, control: Option<&Ref<Control>>) {
        let point = self.get_pointer_position(control);
        let source = self.source_of(control);
        let root: Rc<dyn IInputRoot> = source.clone();
        let timestamp = self.timestamp();

        let input = crate::platform::ITopLevelImpl::input(&*self.window_impl).expect("the window handles input");
        input(Rc::new(RawPointerEventArgs::new(
            self.device(),
            timestamp,
            root,
            RawPointerEventType::Move,
            point,
            RawInputModifiers::NONE,
        )));

        let last_root = self.last_root.borrow().clone();
        if let Some(last_root) = last_root.filter(|last_root| !Rc::ptr_eq(last_root, &source)) {
            let input = last_root.platform_impl().and_then(|platform_impl| platform_impl.input());
            if let Some(input) = input {
                input(Rc::new(RawPointerEventArgs::new(
                    self.device(),
                    timestamp,
                    last_root.clone(),
                    RawPointerEventType::LeaveWindow,
                    Point::new(-1.0, -1.0),
                    RawInputModifiers::NONE,
                )));
            }
        }

        *self.last_root.borrow_mut() = Some(source);

        assert!(control.is_none_or(|control| control.is_pointer_over()));
    }

    /// Sends a raw pointer event, as a platform backend would do.
    fn send_raw_pointer_event(&self, type_: RawPointerEventType, root: Rc<dyn IInputRoot>, position: Point) {
        let input = crate::platform::ITopLevelImpl::input(&*self.window_impl).expect("the window handles input");
        input(Rc::new(RawPointerEventArgs::new(
            self.device(),
            self.timestamp(),
            root,
            type_,
            position,
            RawInputModifiers::NONE,
        )));
    }
}

/// The running application of a test, with the fonts and the render
/// interface that the text of the tips is laid out with (the reference
/// services of these tests carry a font manager and a render interface).
struct TestApplication {
    // Dropped in this order: the application ends before the text services.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn start(services: TestServices) -> TestApplication {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(services.with_render_interface(render_interface));
    TestApplication { _app: app, _text: text }
}

struct ToolTipViewModel;

ferro_model!(ToolTipViewModel, |b| b.read_only::<Value<String>>("Tip", |_| "Tip".to_string()));

/// A decorator with the tip "Tip" and the given show delay.
fn target_with_tip(show_delay: i32) -> Ref<Decorator> {
    let target = Decorator::new();
    ToolTip::set_tip(&target, boxed_str("Tip"));
    ToolTip::set_show_delay(&target, show_delay);
    target
}

/// A tooltip control with the given text as its content.
fn tool_tip_with_content(content: &str) -> Ref<ToolTip> {
    let tip = ToolTip::new();
    tip.set_content(boxed_str(content));
    tip
}

fn tool_tip_of(control: &Control) -> Ref<ToolTip> {
    control.get_value(ToolTip::tool_tip_property()).expect("the control has a tooltip")
}

fn content_of(control: &Control) -> Option<String> {
    control.get_value(ToolTip::tool_tip_property()).and_then(|tool_tip| tool_tip.content()).and_then(|c| string_of(&c))
}

fn should_close_when_control_detaches(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let panel = Panel::new();

    let target = target_with_tip(0);

    panel.children().add(target.clone());

    fixture.setup_window_and_activate_tool_tip_of(&panel.clone().upcast(), &target.clone().upcast(), "Should_Close_When_Control_Detaches");

    fixture.assert_tool_tip_open(&target);

    panel.children().remove(target.clone());

    assert!(!ToolTip::get_is_open(&target));
}

fn should_close_when_tip_is_opened_and_detached_from_visual_tree(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = Decorator::new();
    target.bind_binding(ToolTip::tip_property().as_property(), &ReflectionBinding::new("Tip"));
    ToolTip::set_show_delay(&target, 0);

    let panel = Panel::new();
    panel.children().add(target.clone());

    let scope = fixture
        .setup_window(&panel.clone().upcast(), "Should_Close_When_Tip_Is_Opened_And_Detached_From_Visual_Tree");

    panel.set_data_context(Some(Model::new_model(ToolTipViewModel)));

    scope.mouse_enter(Some(&target.clone().upcast()));

    fixture.assert_tool_tip_open(&target);

    panel.children().remove(target.clone());

    assert!(!ToolTip::get_is_open(&target));
}

fn should_open_on_pointer_enter(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    fixture.setup_window_and_activate_tool_tip(&target.clone().upcast(), "Should_Open_On_Pointer_Enter");

    fixture.assert_tool_tip_open(&target);
}

fn content_should_update_when_tip_property_changes_and_already_open(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    fixture.setup_window_and_activate_tool_tip(
        &target.clone().upcast(),
        "Content_Should_Update_When_Tip_Property_Changes_And_Already_Open",
    );

    fixture.assert_tool_tip_open(&target);
    assert_eq!(Some("Tip".to_string()), content_of(&target));

    ToolTip::set_tip(&target, boxed_str("Tip1"));
    assert_eq!(Some("Tip1".to_string()), content_of(&target));
}

fn should_open_on_pointer_enter_with_delay(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(1);

    fixture.setup_window_and_activate_tool_tip(&target.clone().upcast(), "Should_Open_On_Pointer_Enter_With_Delay");

    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.len());
    let timer = timers[0].clone();
    assert_eq!(Duration::from_millis(1), timer.interval());
    assert!(!ToolTip::get_is_open(&target));

    Dispatcher::force_fire_timer_for_unit_tests(&timer);

    fixture.assert_tool_tip_open(&target);
}

/// A window whose content is a decorator with `tool_tip` as its tip, with
/// styles and templates applied.
fn window_with_tool_tip(window: &Ref<Window>, tool_tip: &Ref<ToolTip>) -> Ref<Decorator> {
    let decorator = Decorator::new();
    ToolTip::set_tip(&decorator, Some(Control::boxed(tool_tip)));

    window.set_content(Some(Control::boxed(&decorator)));

    window.apply_styling();
    window.apply_template();
    window.presenter().expect("the window has a presenter").apply_template();

    decorator
}

fn open_class_should_not_initially_be_added(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::styled_window()));

    let tool_tip = ToolTip::new();
    let window = Window::new();

    let _decorator = window_with_tool_tip(&window, &tool_tip);

    assert!(!tool_tip.classes().contains(":open"));
}

fn setting_is_open_should_add_open_class(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::styled_window()));

    let tool_tip = ToolTip::new();
    let window = Window::new();

    let decorator = window_with_tool_tip(&window, &tool_tip);

    ToolTip::set_is_open(&decorator, true);

    assert!(tool_tip.classes().contains(":open"));
    fixture.verify_tool_tip_type(&decorator);
}

fn clearing_is_open_should_remove_open_class(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::styled_window()));

    let tool_tip = ToolTip::new();

    let window_impl = MockWindowingPlatform::create_window_mock();
    fixture.setup_window_mock(&window_impl);
    let window = Window::with_impl(window_impl);

    let decorator = window_with_tool_tip(&window, &tool_tip);

    ToolTip::set_is_open(&decorator, true);
    fixture.assert_tool_tip_open(&decorator);
    ToolTip::set_is_open(&decorator, false);

    assert!(!tool_tip.classes().contains(":open"));
}

fn should_close_on_null_tip(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    fixture.setup_window_and_activate_tool_tip(&target.clone().upcast(), "Should_Close_On_Null_Tip");

    fixture.assert_tool_tip_open(&target);

    ToolTip::set_tip(&target, None);

    assert!(!ToolTip::get_is_open(&target));
}

fn should_not_close_when_pointer_is_moved_over_tool_tip(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    let scope =
        fixture.setup_window(&target.clone().upcast(), "Should_Not_Close_When_Pointer_Is_Moved_Over_ToolTip");

    scope.mouse_enter(Some(&target.clone().upcast()));

    fixture.assert_tool_tip_open(&target);

    let tooltip = tool_tip_of(&target);

    scope.mouse_enter(Some(&tooltip.upcast()));

    fixture.assert_tool_tip_open(&target);
}

fn should_not_close_when_pointer_is_moved_from_tool_tip_to_original_control(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    let scope = fixture.setup_window(
        &target.clone().upcast(),
        "Should_Not_Close_When_Pointer_Is_Moved_From_ToolTip_To_Original_Control",
    );

    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    let tooltip = tool_tip_of(&target);
    scope.mouse_enter(Some(&tooltip.upcast()));

    fixture.assert_tool_tip_open(&target);

    scope.mouse_enter(Some(&target.clone().upcast()));

    fixture.assert_tool_tip_open(&target);
}

fn should_close_when_pointer_is_moved_from_tool_tip_to_another_control(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    let other = Decorator::new();

    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(other.clone());

    let scope = fixture.setup_window(
        &panel.clone().upcast(),
        "Should_Close_When_Pointer_Is_Moved_From_ToolTip_To_Another_Control",
    );

    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    let tooltip = tool_tip_of(&target);
    scope.mouse_enter(Some(&tooltip.upcast()));

    fixture.assert_tool_tip_open(&target);

    scope.mouse_enter(Some(&other.clone().upcast()));

    assert!(!ToolTip::get_is_open(&target));
}

fn new_tool_tip_replaces_other_tool_tip_immediately(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    // one hour
    let other = target_with_tip(60 * 60 * 1000);

    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(other.clone());

    let scope = fixture.setup_window(&panel.clone().upcast(), "New_ToolTip_Replaces_Other_ToolTip_Immediately");
    let target_control: Ref<Control> = target.clone().upcast();
    let other_control: Ref<Control> = other.clone().upcast();

    scope.mouse_enter(Some(&other_control));
    assert!(!ToolTip::get_is_open(&other)); // long delay

    scope.mouse_enter(Some(&target_control));
    fixture.assert_tool_tip_open(&target); // no delay

    scope.mouse_enter(Some(&other_control));
    assert!(ToolTip::get_is_open(&other)); // delay skipped, a tooltip was already open

    // Now disable the between-show system

    scope.mouse_enter(Some(&target_control));
    fixture.assert_tool_tip_open(&target);

    ToolTip::set_between_show_delay(&other, -1);

    scope.mouse_enter(Some(&other_control));
    assert!(!ToolTip::get_is_open(&other));
}

fn tool_tip_events_order_is_defined(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let tip = tool_tip_with_content("Tip");
    let target = Decorator::new();
    ToolTip::set_tip(&target, Some(Control::boxed(&tip)));
    ToolTip::set_show_delay(&target, 0);

    type Events = Vec<(&'static str, Ref<FerroObject>, Option<Ref<FerroObject>>)>;
    let events_order: Rc<RefCell<Events>> = Rc::new(RefCell::new(Vec::new()));

    ToolTip::add_tool_tip_opening_handler(&target, {
        let events_order = events_order.clone();
        move |sender, args| events_order.borrow_mut().push(("Opening", sender.to_ref().upcast(), args.source()))
    });
    ToolTip::add_tool_tip_closing_handler(&target, {
        let events_order = events_order.clone();
        move |sender, args| events_order.borrow_mut().push(("Closing", sender.to_ref().upcast(), args.source()))
    });

    fixture.setup_window_and_activate_tool_tip(&target.clone().upcast(), "ToolTip_Events_Order_Is_Defined");

    fixture.assert_tool_tip_open(&target);

    ToolTip::set_tip(&target, None);

    assert!(!ToolTip::get_is_open(&target));

    let target_object: Ref<FerroObject> = target.clone().upcast();
    let expected: Events = vec![
        ("Opening", target_object.clone(), Some(target_object.clone())),
        ("Closing", target_object.clone(), Some(target_object)),
    ];
    assert!(expected == *events_order.borrow());
}

fn tool_tip_is_not_opened_if_opening_event_handled(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let tip = tool_tip_with_content("Tip");
    let target = Decorator::new();
    ToolTip::set_tip(&target, Some(Control::boxed(&tip)));
    ToolTip::set_show_delay(&target, 0);

    ToolTip::add_tool_tip_opening_handler(&target, |_, args| args.set_cancel(true));

    fixture.setup_window_and_activate_tool_tip(
        &target.clone().upcast(),
        "ToolTip_Is_Not_Opened_If_Opening_Event_Handled",
    );

    assert!(!ToolTip::get_is_open(&target));
}

fn tool_tip_can_be_replaced_on_the_fly_via_opening_event(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let tip1 = tool_tip_with_content("Hi");
    let tip2 = tool_tip_with_content("Bye");
    let target = Decorator::new();
    ToolTip::set_tip(&target, Some(Control::boxed(&tip1)));
    ToolTip::set_show_delay(&target, 0);

    ToolTip::add_tool_tip_opening_handler(&target, {
        let weak_target = target.downgrade();
        move |_, _| {
            if let Some(target) = weak_target.upgrade() {
                ToolTip::set_tip(&target, Some(Control::boxed(&tip2)));
            }
        }
    });

    fixture.setup_window_and_activate_tool_tip(
        &target.clone().upcast(),
        "ToolTip_Can_Be_Replaced_On_The_Fly_Via_Opening_Event",
    );

    fixture.assert_tool_tip_open(&target);

    ToolTip::set_tip(&target, None);

    assert!(!ToolTip::get_is_open(&target));
}

fn should_close_when_pointer_leaves_window(overlay: bool) {
    let fixture = Fixture::new(overlay);
    // The reference starts this test with the unconfigured services.
    let _app = start(TestServices::focusable_window());

    let target = target_with_tip(0);

    let scope = fixture.setup_window(&target.clone().upcast(), "Should_Close_When_Pointer_Leaves_Window");

    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    let top_level = TopLevel::get_top_level(Some(&target)).expect("the target is in a top-level");
    let input = top_level.platform_impl().and_then(|platform_impl| platform_impl.input()).expect("input is handled");
    input(Rc::new(RawPointerEventArgs::new(
        scope.device(),
        scope.timestamp(),
        top_level.input_root(),
        RawPointerEventType::LeaveWindow,
        Point::default(),
        RawInputModifiers::NONE,
    )));

    Dispatcher::ui_thread().run_jobs(None);

    assert!(!ToolTip::get_is_open(&target));
}

fn should_not_close_when_pointer_is_over_tool_tip_window_without_hit_test_result() {
    let fixture = Fixture::new(false);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    let scope = fixture.setup_window(
        &target.clone().upcast(),
        "Should_Not_Close_When_Pointer_Is_Over_ToolTip_Window_Without_Hit_Test_Result",
    );

    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    let tool_tip = tool_tip_of(&target);
    let tool_tip_root = tool_tip
        .popup_host()
        .and_then(|host| host.as_popup_root())
        .expect("the tooltip is shown in a popup root")
        .get_input_root()
        .expect("the popup root has an input root");

    scope.send_raw_pointer_event(RawPointerEventType::Move, tool_tip_root, scope.get_pointer_position(None));

    fixture.assert_tool_tip_open(&target);
}

fn should_not_close_when_leaving_window_right_after_tool_tip_opened_under_pointer() {
    let fixture = Fixture::new(false);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let target = target_with_tip(0);

    let scope = fixture.setup_window(
        &target.clone().upcast(),
        "Should_Not_Close_When_Leaving_Window_Right_After_ToolTip_Opened_Under_Pointer",
    );

    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    // The pointer is still over the adorned control: this leave event is only caused by the tooltip window
    // being displayed on top of it (macOS case).
    scope.send_raw_pointer_event(
        RawPointerEventType::LeaveWindow,
        scope.window.input_root(),
        scope.get_pointer_position(Some(&target.clone().upcast())),
    );

    fixture.assert_tool_tip_open(&target);
}

/// Not in the reference: a tooltip and the popup it is shown in hold each
/// other only while the tooltip is open, so that a control that goes away
/// takes its closed tooltip with it.
fn closed_tool_tip_is_freed_with_its_control(overlay: bool) {
    let fixture = Fixture::new(overlay);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));

    let panel = Panel::new();
    let target = target_with_tip(0);
    panel.children().add(target.clone());

    let scope = fixture.setup_window(&panel.clone().upcast(), "Closed_ToolTip_Is_Freed_With_Its_Control");
    scope.mouse_enter(Some(&target.clone().upcast()));
    fixture.assert_tool_tip_open(&target);

    let tool_tip = tool_tip_of(&target).downgrade();
    let weak_target = target.downgrade();

    scope.mouse_enter(Some(&panel.clone().upcast()));
    assert!(!ToolTip::get_is_open(&target));

    panel.children().remove(target.clone());
    drop(target);
    scope.control_ids.borrow_mut().clear();
    scope.hit_tester.results.borrow_mut().clear();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(weak_target.upgrade().is_none());
    assert!(tool_tip.upgrade().is_none());
}

macro_rules! tool_tip_tests {
    ($($name:ident,)*) => {
        /// The reference `ToolTipTests_Popup`: the platform creates native
        /// popups.
        mod tool_tip_tests_popup {
            $(
                #[test]
                fn $name() {
                    super::$name(false);
                }
            )*

            #[test]
            fn should_not_close_when_pointer_is_over_tool_tip_window_without_hit_test_result() {
                super::should_not_close_when_pointer_is_over_tool_tip_window_without_hit_test_result();
            }

            #[test]
            fn should_not_close_when_leaving_window_right_after_tool_tip_opened_under_pointer() {
                super::should_not_close_when_leaving_window_right_after_tool_tip_opened_under_pointer();
            }
        }

        /// The reference `ToolTipTests_Overlay`: the platform creates no
        /// popups, so tooltips are shown in the overlay layer.
        mod tool_tip_tests_overlay {
            $(
                #[test]
                fn $name() {
                    super::$name(true);
                }
            )*
        }
    };
}

tool_tip_tests! {
    should_close_when_control_detaches,
    should_close_when_tip_is_opened_and_detached_from_visual_tree,
    should_open_on_pointer_enter,
    content_should_update_when_tip_property_changes_and_already_open,
    should_open_on_pointer_enter_with_delay,
    open_class_should_not_initially_be_added,
    setting_is_open_should_add_open_class,
    clearing_is_open_should_remove_open_class,
    should_close_on_null_tip,
    should_not_close_when_pointer_is_moved_over_tool_tip,
    should_not_close_when_pointer_is_moved_from_tool_tip_to_original_control,
    should_close_when_pointer_is_moved_from_tool_tip_to_another_control,
    new_tool_tip_replaces_other_tool_tip_immediately,
    tool_tip_events_order_is_defined,
    tool_tip_is_not_opened_if_opening_event_handled,
    tool_tip_can_be_replaced_on_the_fly_via_opening_event,
    should_close_when_pointer_leaves_window,
    closed_tool_tip_is_freed_with_its_control,
}

// The hooks of the top-level that keep the tooltip service up to date
// without pointer input. Not in the reference suite.

/// A tooltip service that records the updates it gets.
#[derive(Default)]
struct RecordingToolTipService {
    updates: RefCell<Vec<(Rc<dyn IInputRoot>, Option<Ref<Visual>>)>>,
}

impl IToolTipService for RecordingToolTipService {
    fn update(&self, root: &Rc<dyn IInputRoot>, candidate_tool_tip_host: Option<Ref<Visual>>) {
        self.updates.borrow_mut().push((root.clone(), candidate_tool_tip_host));
    }
}

impl RecordingToolTipService {
    /// Makes a new recording service the tooltip service of the top-levels
    /// created from now on.
    fn register() -> Rc<RecordingToolTipService> {
        let service = Rc::new(RecordingToolTipService::default());
        FerroLocator::current_mutable().bind::<dyn IToolTipService>().to_constant(service.clone());
        service
    }

    /// The candidates of the updates recorded so far, which must all be for
    /// the input root of `window`; forgets them.
    fn take_candidates(&self, window: &Window) -> Vec<Option<Ref<Visual>>> {
        let input_root = window.input_root();
        self.updates
            .take()
            .into_iter()
            .map(|(root, candidate)| {
                assert!(std::ptr::addr_eq(Rc::as_ptr(&input_root), Rc::as_ptr(&root)));
                candidate
            })
            .collect()
    }
}

/// Moves the pointer to `position` of the window of `scope`, where it hits
/// `control`, and returns the position the top-level then knows, in its
/// client coordinates.
fn rest_pointer_at(scope: &ToolTipTestScope, position: Point, control: &Ref<Control>) -> Point {
    scope.hit_tester.setup(position, Some(control));
    scope.send_raw_pointer_event(RawPointerEventType::Move, scope.window.input_root(), position);

    assert!(scope.window.is_pointer_over());
    let last_position = scope.window.last_pointer_position().expect("the pointer is over the window");
    let client_point = Visual::point_to_client(&scope.window, last_position);
    assert_eq!(position, client_point);
    client_point
}

#[test]
fn scene_invalidation_under_the_pointer_updates_the_tool_tip_service() {
    let fixture = Fixture::new(false);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));
    let service = RecordingToolTipService::register();

    let panel = Panel::new();
    let target = Decorator::new();
    panel.children().add(target.clone());
    let panel: Ref<Control> = panel.upcast();
    let target: Ref<Control> = target.upcast();

    let scope = fixture.setup_window(&panel, "Scene_Invalidation_Under_The_Pointer_Updates_The_ToolTip_Service");
    let renderer = scope.window.renderer();
    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().expect("the renderer of the tests");
    let everything = Rect::new(0.0, 0.0, 10_000.0, 10_000.0);

    // The pointer is not over the top-level yet.
    renderer.raise_scene_invalidated(everything);
    assert!(service.take_candidates(&scope.window).is_empty());

    let pointer = rest_pointer_at(&scope, Point::new(20.0, 30.0), &panel);
    assert!(service.take_candidates(&scope.window).is_empty());

    // The scene changes somewhere else.
    renderer.raise_scene_invalidated(Rect::new(pointer.x + 10.0, pointer.y + 10.0, 5.0, 5.0));
    assert!(service.take_candidates(&scope.window).is_empty());

    // The scene changes under the pointer: another control is there now.
    scope.hit_tester.setup(pointer, Some(&target));
    renderer.raise_scene_invalidated(Rect::new(pointer.x - 1.0, pointer.y - 1.0, 2.0, 2.0));
    assert_eq!(vec![Some(target.clone().upcast::<Visual>())], service.take_candidates(&scope.window));

    // Nothing is under the pointer any more: the same invalidation makes
    // the input system take the pointer off the top-level, which then has
    // nothing to tell the service.
    scope.hit_tester.setup(pointer, None);
    renderer.raise_scene_invalidated(everything);
    assert!(!scope.window.is_pointer_over());
    assert!(service.take_candidates(&scope.window).is_empty());
}

#[test]
fn enabling_the_tool_tip_service_of_the_control_under_the_pointer_updates_the_service() {
    let fixture = Fixture::new(false);
    let _app = start(fixture.configure_services(TestServices::focusable_window()));
    let service = RecordingToolTipService::register();

    let under_pointer = Decorator::new();
    under_pointer.set_width(100.0);
    under_pointer.set_height(100.0);
    let elsewhere = Decorator::new();
    elsewhere.set_width(100.0);
    elsewhere.set_height(100.0);
    // The controls sit in the top left corner of the window. The area the
    // top-level checks is the bounds of the control moved by the position
    // of the control in the top-level, as in the reference, so it is the
    // area of the control only for a control at the origin of its parent
    // and of the top-level.
    let panel = StackPanel::new();
    panel.set_horizontal_alignment(HorizontalAlignment::Left);
    panel.set_vertical_alignment(VerticalAlignment::Top);
    panel.children().add(under_pointer.clone());
    panel.children().add(elsewhere.clone());
    let panel: Ref<Control> = panel.upcast();
    let under_pointer: Ref<Control> = under_pointer.upcast();

    let scope = fixture.setup_window(&panel, "Enabling_The_ToolTip_Service_Updates_The_Service");

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), under_pointer.bounds());
    assert_eq!(Some(Point::default()), under_pointer.translate_point(Point::default(), &scope.window));
    assert_eq!(Rect::new(0.0, 100.0, 100.0, 100.0), elsewhere.bounds());

    // The pointer rests on the first control.
    rest_pointer_at(&scope, Point::new(10.0, 10.0), &under_pointer);
    assert!(service.take_candidates(&scope.window).is_empty());

    // Disabling the service updates nothing.
    ToolTip::set_service_enabled(&under_pointer, false);
    assert!(service.take_candidates(&scope.window).is_empty());

    // Enabling it for a control the pointer is not over updates nothing.
    ToolTip::set_service_enabled(&elsewhere, false);
    ToolTip::set_service_enabled(&elsewhere, true);
    assert!(service.take_candidates(&scope.window).is_empty());

    // Enabling it for the control under the pointer does.
    ToolTip::set_service_enabled(&under_pointer, true);
    assert_eq!(vec![Some(under_pointer.clone().upcast::<Visual>())], service.take_candidates(&scope.window));
}
