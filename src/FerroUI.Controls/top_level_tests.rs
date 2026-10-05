//! Port of the reference `TopLevelTests`.

use crate::embedding::EmbeddableControlRoot;
use crate::platform::ITopLevelImpl;
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControlImpl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockCall, MockImplKind, MockWindowImpl, NullRenderer, TestServices, UnitTestApplication};
use crate::{
    Application, Border, ContentControl, ContentControlImpl, Control, ControlImpl, Panel, TopLevel, TopLevelImpl,
    WindowResizeReason, WindowTransparencyLevel,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{
    IInputManager, InputElementImpl, InputManager, Key, RawInputModifiers, KeyDeviceType, KeyboardDevice, MouseDevice, PhysicalKey,
    Pointer, PointerType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::IObservable;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct TestTopLevel {
    base: TopLevel,
    measured: Cell<i32>,
    arranged: Cell<i32>,
}

ferro_class!(TestTopLevel: TopLevel);
ferro_impl_classes!(
    TestTopLevel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl LayoutableImpl for TestTopLevel {
    fn measure_core(this: &Self, available_size: Size) -> Size {
        this.measured.set(this.measured.get() + 1);
        Self::parent_measure_core(this, available_size)
    }

    fn arrange_core(this: &Self, final_rect: Rect) {
        this.arranged.set(this.arranged.get() + 1);
        Self::parent_arrange_core(this, final_rect)
    }
}

impl TestTopLevel {
    fn new(platform_impl: &Rc<MockWindowImpl>) -> Ref<Self> {
        let platform_impl: Rc<dyn ITopLevelImpl> = platform_impl.clone();
        instantiate(Self { base: TopLevel::construct(platform_impl), measured: Cell::new(0), arranged: Cell::new(0) })
    }
}

fn create_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    }))
}

fn create_mock_top_level_impl() -> Rc<MockWindowImpl> {
    MockWindowImpl::bare(MockImplKind::TopLevel)
}

fn resized(platform_impl: &MockWindowImpl, size: Size) {
    ITopLevelImpl::resized(platform_impl).expect("the resized callback is set")(size, WindowResizeReason::Unspecified);
}

fn closed(platform_impl: &MockWindowImpl) {
    // The callback is released once the top-level has closed.
    if let Some(closed) = ITopLevelImpl::closed(platform_impl) {
        closed();
    }
}

#[test]
fn is_attached_to_logical_tree_is_true() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);

    assert!(target.is_attached_to_logical_tree());
}

#[test]
fn client_size_should_be_set_on_construction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.client_size.set(Size::new(123.0, 456.0));

    let target = TestTopLevel::new(&platform_impl);

    assert_eq!(Size::new(123.0, 456.0), target.client_size());
}

#[test]
fn width_should_not_be_set_on_construction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.client_size.set(Size::new(123.0, 456.0));

    let target = TestTopLevel::new(&platform_impl);

    assert!(target.width().is_nan());
}

#[test]
fn height_should_not_be_set_on_construction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.client_size.set(Size::new(123.0, 456.0));

    let target = TestTopLevel::new(&platform_impl);

    assert!(target.height().is_nan());
}

#[test]
fn layout_pass_should_not_be_automatically_scheduled() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let target = TestTopLevel::new(&platform_impl);

    // The layout pass should be scheduled by the derived class.
    assert_eq!(0, target.measured.get());
    assert_eq!(0, target.arranged.get());
}

#[test]
fn bounds_should_be_set_after_layout_pass() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let target = TestTopLevel::new(&platform_impl);
    target.set_is_visible(true);
    target.set_template(create_template());
    let content = Border::new();
    content.set_width(321.0);
    content.set_height(432.0);
    target.set_content(Some(Control::boxed(content)));

    target.layout_manager().execute_initial_layout_pass();

    assert_eq!(Rect::new(0.0, 0.0, 321.0, 432.0), target.bounds());
}

#[test]
fn width_and_height_should_not_be_set_after_layout_pass() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.client_size.set(Size::new(123.0, 456.0));

    let target = TestTopLevel::new(&platform_impl);
    target.layout_manager().execute_layout_pass();

    assert!(target.width().is_nan());
    assert!(target.height().is_nan());
}

#[test]
fn width_and_height_should_be_set_after_window_resize_notification() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.client_size.set(Size::new(123.0, 456.0));

    // The user has resized the window, so we can no longer auto-size.
    let target = TestTopLevel::new(&platform_impl);
    resized(&platform_impl, Size::new(100.0, 200.0));

    assert_eq!(100.0, target.width());
    assert_eq!(200.0, target.height());
}

#[test]
fn impl_close_should_call_raise_closed_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let raised = Rc::new(Cell::new(false));
    let target = TestTopLevel::new(&platform_impl);
    let _ = target.closed({
        let raised = raised.clone();
        move || raised.set(true)
    });

    closed(&platform_impl);

    assert!(raised.get());
}

#[test]
fn impl_close_should_raise_closed_event_only_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let raised = Rc::new(Cell::new(0));
    let target = TestTopLevel::new(&platform_impl);
    let _ = target.closed({
        let raised = raised.clone();
        move || raised.set(raised.get() + 1)
    });

    closed(&platform_impl);
    closed(&platform_impl);

    assert_eq!(1, raised.get());
}

#[test]
fn embeddable_control_root_dispose_should_raise_closed_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let raised = Rc::new(Cell::new(0));
    let target = EmbeddableControlRoot::with_impl(platform_impl.clone());
    let _ = target.closed({
        let raised = raised.clone();
        move || raised.set(raised.get() + 1)
    });

    target.dispose();

    assert_eq!(1, raised.get());
}

#[test]
fn embeddable_control_root_dispose_after_impl_close_should_raise_closed_event_only_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let raised = Rc::new(Cell::new(0));
    let target = EmbeddableControlRoot::with_impl(platform_impl.clone());
    let _ = target.closed({
        let raised = raised.clone();
        move || raised.set(raised.get() + 1)
    });

    closed(&platform_impl);
    target.dispose();

    assert_eq!(1, raised.get());
}

#[test]
fn embeddable_control_root_dispose_should_dispose_impl_before_teardown() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let closed_raised = Rc::new(Cell::new(false));
    let impl_disposed_before_closed = Rc::new(Cell::new(false));
    platform_impl.setup_dispose({
        let closed_raised = closed_raised.clone();
        let impl_disposed_before_closed = impl_disposed_before_closed.clone();
        move |_| impl_disposed_before_closed.set(!closed_raised.get())
    });

    let target = EmbeddableControlRoot::with_impl(platform_impl.clone());
    let _ = target.closed({
        let closed_raised = closed_raised.clone();
        move || closed_raised.set(true)
    });

    target.dispose();

    assert_eq!(1, platform_impl.count_of(&MockCall::Dispose));
    assert!(impl_disposed_before_closed.get());
    assert!(closed_raised.get());
}

#[test]
fn impl_close_should_raise_detached_from_logical_tree_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let target = TestTopLevel::new(&platform_impl);
    let raised = Rc::new(Cell::new(0));

    let _ = target.detached_from_logical_tree({
        let raised = raised.clone();
        let target = target.downgrade();
        move |e| {
            let target: Ref<StyledElement> = target.upgrade().unwrap().upcast();
            assert!(target.ptr_eq(e.root()));
            assert!(target.ptr_eq(e.source()));
            assert!(e.parent().is_none());
            raised.set(raised.get() + 1);
        }
    });

    closed(&platform_impl);

    assert_eq!(1, raised.get());
}

struct RecordingInputManager {
    inner: InputManager,
    processed: RefCell<Vec<Rc<dyn IRawInputEventArgs>>>,
}

impl IInputManager for RecordingInputManager {
    fn pre_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        self.inner.pre_process()
    }

    fn process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        self.inner.process()
    }

    fn post_process(&self) -> Rc<dyn IObservable<Rc<dyn IRawInputEventArgs>>> {
        self.inner.post_process()
    }

    fn process_input(&self, e: Rc<dyn IRawInputEventArgs>) {
        self.processed.borrow_mut().push(e);
    }
}

#[test]
fn impl_input_should_pass_input_to_input_manager() {
    let input_manager = Rc::new(RecordingInputManager { inner: InputManager::new(), processed: RefCell::new(Vec::new()) });

    let services = TestServices::styled_window().with_input_manager(input_manager.clone());

    let _app = UnitTestApplication::start(services);
    let platform_impl = create_mock_top_level_impl();

    let target = TestTopLevel::new(&platform_impl);

    let input: Rc<dyn IRawInputEventArgs> = Rc::new(RawKeyEventArgs::new(
        KeyboardDevice::new(),
        0,
        target.input_root(),
        RawKeyEventType::KeyDown,
        Key::A,
        RawInputModifiers::NONE,
        PhysicalKey::A,
        Some("a".to_string()),
        KeyDeviceType::Keyboard,
    ));

    ITopLevelImpl::input(&*platform_impl).expect("the input callback is set")(input.clone());

    let processed = input_manager.processed.borrow();
    assert_eq!(1, processed.len());
    assert!(Rc::ptr_eq(&processed[0], &input));
}

#[test]
#[should_panic]
fn adding_top_level_as_child_should_throw_exception() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);
    let child = TestTopLevel::new(&create_mock_top_level_impl());

    target.set_template(create_template());
    target.set_content(Some(Control::boxed(child)));
    target.apply_template();
    target.presenter().unwrap().apply_template();
}

#[test]
fn adding_resource_to_application_should_raise_resources_changed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);
    let raised = Rc::new(Cell::new(false));

    let _ = target.resources_changed({
        let raised = raised.clone();
        move |_| raised.set(true)
    });
    Application::current().unwrap().resources().add_value("foo", "bar".to_string());

    assert!(raised.get());
}

#[test]
fn x_button1_down_should_raise_back_requested() {
    // Regression test: the pre-process subscription has to compare the root
    // of the event against the presentation source (the input root), not
    // against the top-level itself.
    let services = TestServices::styled_window().with_input_manager(Rc::new(InputManager::new()));

    let _app = UnitTestApplication::start(services);
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);

    let raised = Rc::new(Cell::new(false));
    target.back_requested({
        let raised = raised.clone();
        move |_, _| raised.set(true)
    });

    let mouse_device = MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true));
    ITopLevelImpl::input(&*platform_impl).expect("the input callback is set")(Rc::new(RawPointerEventArgs::new(
        mouse_device,
        0,
        target.input_root(),
        RawPointerEventType::XButton1Down,
        Point::default(),
        RawInputModifiers::NONE,
    )));

    assert!(raised.get());
}

#[test]
fn top_level_should_unfocus_when_impl_focus_is_lost() {
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let platform_impl = create_mock_top_level_impl();
    let content = Border::new();
    content.set_focusable(true);
    let target = TestTopLevel::new(&platform_impl);
    target.set_template(create_template());
    target.set_focusable(true);
    target.set_content(Some(Control::boxed(content.clone())));

    target.layout_manager().execute_initial_layout_pass();

    content.focus();
    assert!(content.is_focused());

    ITopLevelImpl::lost_focus(&*platform_impl).expect("the lost focus callback is set")();

    assert!(!content.is_focused());
}

#[test]
fn reacts_to_changes_in_global_styles() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();

    let child = Border::new();
    child.classes().add("foo");
    let target = TestTopLevel::new(&platform_impl);
    target.set_template(create_template());
    target.set_content(Some(Control::boxed(child.clone())));

    target.layout_manager().execute_initial_layout_pass();

    assert_eq!(Thickness::uniform(0.0), child.border_thickness());

    let style = Style::with_selector(Selectors::of_type::<Border>().class("foo"));
    style.add_setter(Setter::new(Border::border_thickness_property(), Thickness::uniform(2.0)));

    let application = Application::current().unwrap();
    application.styles().add(&style);
    target.layout_manager().execute_initial_layout_pass();

    assert_eq!(Thickness::uniform(2.0), child.border_thickness());

    application.styles().remove(&style);

    assert_eq!(Thickness::uniform(0.0), child.border_thickness());
}

// --- Tests of the port: the renderer seam and the platform wiring -----------

fn null_renderer(target: &TopLevel) -> Rc<dyn crate::presentation_source::ITopLevelRenderer> {
    target.renderer()
}

#[test]
fn renderer_is_created_through_the_factory_and_gets_the_host_as_root() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    platform_impl.transparency_level.set(WindowTransparencyLevel::blur());
    let target = TestTopLevel::new(&platform_impl);

    let renderer = null_renderer(&target);
    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().expect("the test renderer is a null renderer");

    let root = renderer.root().expect("the renderer has a root");
    assert!(root.ptr_eq(&target.visual_root().unwrap()));
    assert!(target.visual_parent().unwrap().ptr_eq(&root));
    assert_eq!(CompositionTransparencyLevel::Blur, renderer.transparency_level());
    assert_eq!(WindowTransparencyLevel::blur(), target.actual_transparency_level());
}

#[test]
fn paint_and_resize_reach_the_renderer_and_close_disposes_it() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);
    let renderer = null_renderer(&target);
    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().unwrap();

    ITopLevelImpl::paint(&*platform_impl).unwrap()(Rect::new(1.0, 2.0, 3.0, 4.0));
    resized(&platform_impl, Size::new(100.0, 200.0));

    assert_eq!(vec![Rect::new(1.0, 2.0, 3.0, 4.0)], renderer.paint_calls());
    assert_eq!(vec![Size::new(100.0, 200.0)], renderer.resized_calls());

    target.start_rendering();
    assert!(renderer.is_started());
    assert!(ferroui_base::media::MediaContext::instance().is_top_level_active(target.media_context_key()));

    closed(&platform_impl);

    assert!(!renderer.is_started());
    assert!(renderer.is_disposed());
    assert!(target.platform_impl().is_none());
    assert!(!target.is_attached_to_visual_tree());
    assert!(ITopLevelImpl::input(&*platform_impl).is_none());
}

#[test]
fn scaling_change_updates_render_scaling_and_raises_scaling_changed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);
    let raised = Rc::new(Cell::new(0));
    let _ = target.scaling_changed({
        let raised = raised.clone();
        move || raised.set(raised.get() + 1)
    });

    ITopLevelImpl::scaling_changed(&*platform_impl).unwrap()(2.0);

    assert_eq!(2.0, target.render_scaling());
    assert_eq!(2.0, ferroui_base::rendering::IPresentationSource::render_scaling(&**target.presentation_source()));
    assert_eq!(1, raised.get());
}

#[test]
fn transparency_level_hint_is_forwarded_to_the_impl() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);

    assert_eq!(1, platform_impl.count_of(&MockCall::SetTransparencyLevelHint(vec![])));

    target.set_transparency_level_hint(crate::WindowTransparencyLevelCollection::new(vec![
        WindowTransparencyLevel::mica(),
        WindowTransparencyLevel::blur(),
    ]));

    assert_eq!(
        1,
        platform_impl.count_of(&MockCall::SetTransparencyLevelHint(vec![
            WindowTransparencyLevel::mica(),
            WindowTransparencyLevel::blur()
        ]))
    );
}

#[test]
fn transparency_fallback_border_follows_the_transparency_level() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = EmbeddableControlRoot::with_impl(platform_impl.clone());
    target.prepare();

    let border = target
        .get_visual_children()
        .first()
        .and_then(|panel| panel.clone().cast::<Panel>())
        .and_then(|panel| panel.children().get(0).cast::<Border>())
        .expect("the test theme template was applied");
    assert_eq!(Some("PART_TransparencyFallback".to_string()), border.name());
    assert!(border.background().is_some());

    ITopLevelImpl::transparency_level_changed(&*platform_impl).unwrap()(WindowTransparencyLevel::transparent());

    assert!(border.background().is_none());
    assert_eq!(WindowTransparencyLevel::transparent(), target.actual_transparency_level());
}

#[test]
fn get_top_level_finds_the_hosting_top_level() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let child = Border::new();
    let target = TestTopLevel::new(&platform_impl);
    target.set_template(create_template());
    target.set_content(Some(Control::boxed(child.clone())));
    target.layout_manager().execute_initial_layout_pass();

    let found = TopLevel::get_top_level(Some(&child)).expect("the child is hosted in the top-level");
    assert!(std::ptr::eq::<TopLevel>(&*found, &**target));
    assert!(TopLevel::get_top_level(Some(&Border::new())).is_none());
}

#[test]
fn top_level_without_application_theme_host_is_a_theme_variant_root() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let target = TestTopLevel::new(&platform_impl);

    assert_eq!(Some(ferroui_base::styling::ThemeVariant::light()), target.actual_theme_variant());

    target.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
    assert_eq!(Some(ferroui_base::styling::ThemeVariant::dark()), target.actual_theme_variant());
    assert_eq!(
        Some(MockCall::SetFrameThemeVariant(Some(crate::platform::PlatformThemeVariant::Dark))),
        platform_impl.calls().into_iter().rev().find(|call| matches!(call, MockCall::SetFrameThemeVariant(_)))
    );

    target.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::default()));
    assert_eq!(Some(ferroui_base::styling::ThemeVariant::light()), target.actual_theme_variant());
}

#[test]
fn platform_impl_keeps_the_top_level_alive_until_it_closes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_top_level_impl();
    let weak = TestTopLevel::new(&platform_impl).downgrade();

    assert!(weak.upgrade().is_some());

    closed(&platform_impl);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);

    assert!(weak.upgrade().is_none());
}
