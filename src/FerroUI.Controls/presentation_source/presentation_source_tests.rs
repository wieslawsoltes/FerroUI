//! Port of the reference `PresentationSourceTests`.

use crate::chrome::{WindowDecorationProperties, WindowDrawnDecorations, WindowDrawnDecorationsContent};
use crate::platform::PlatformRequestedDrawnDecoration;
use crate::top_level_host_decorations::LayerWrapper;
use crate::testing::{
    decorations_template_theme, FuncWindowDrawnDecorationsTemplate, MockWindowingPlatform, TestServices,
    UnitTestApplication,
};
use crate::platform::ITopLevelImpl;
use crate::{Application, Border, Control, StackPanel, Window, WindowDecorations};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    Cursor, IInputRoot, IPointer, InputElement, InputManager, MouseDevice, PointerPressedEventArgs,
    RawInputModifiers, StandardCursorType, WindowDecorationsElementRole,
};
use ferroui_base::layout::Orientation;
use ferroui_base::media::Brushes;
use ferroui_base::platform::ICursorImpl;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn closing_should_detach_platform_input_handler() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let _window = Window::with_impl(window_impl.clone());

    assert!(ITopLevelImpl::input(&*window_impl).is_some());

    ITopLevelImpl::closed(&*window_impl).expect("the closed callback is set")();

    assert!(ITopLevelImpl::input(&*window_impl).is_none());
}

#[test]
fn chrome_hit_test_prefers_overlay_over_content() {
    let overlay = Border::new();
    overlay.set_background(Some(Brushes::red()));
    WindowDecorationProperties::set_element_role(&overlay, WindowDecorationsElementRole::TitleBar);

    let content = Border::new();
    content.set_background(Some(Brushes::blue()));

    do_chrome_hit_test(
        None,
        Some(content),
        Some(overlay.clone()),
        Some(overlay.upcast()),
        Some(WindowDecorationsElementRole::TitleBar),
    );
}

#[test]
fn chrome_hit_test_prefers_content_over_underlay() {
    let underlay = Border::new();
    underlay.set_background(Some(Brushes::red()));
    WindowDecorationProperties::set_element_role(&underlay, WindowDecorationsElementRole::TitleBar);

    let content = Border::new();
    content.set_background(Some(Brushes::blue()));

    do_chrome_hit_test(Some(underlay), Some(content.clone()), None, Some(content.upcast()), None);
}

fn do_chrome_hit_test(
    underlay: Option<Ref<Border>>,
    content: Option<Ref<Border>>,
    overlay: Option<Ref<Border>>,
    expected_chrome_visual: Option<Ref<Visual>>,
    expected_role: Option<WindowDecorationsElementRole>,
) {
    const WIDTH: f64 = 100.0;
    const HEIGHT: f64 = 100.0;

    for control in [&underlay, &content, &overlay].into_iter().flatten() {
        control.set_width(WIDTH);
        control.set_height(HEIGHT);
    }

    let _app = UnitTestApplication::start(TestServices::styled_window());

    let decorations = WindowDrawnDecorationsContent::new();
    decorations.set_underlay(underlay.map(Ref::upcast::<Control>));
    decorations.set_overlay(overlay.map(Ref::upcast::<Control>));

    Application::current().expect("the application is running").resources().add_value(
        ResourceKey::Type(WindowDrawnDecorations::TYPE),
        decorations_template_theme(FuncWindowDrawnDecorationsTemplate::from_content(&decorations)),
    );

    let window_impl = MockWindowingPlatform::create_window_mock_with_size(WIDTH, HEIGHT);
    window_impl.is_client_area_extended_to_decorations.set(true);
    window_impl.requested_drawn_decorations.set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    window_impl.needs_managed_decorations.set(true);

    let window = Window::with_impl(window_impl);
    window.set_window_decorations(WindowDecorations::Full);
    window.set_extend_client_area_to_decorations_hint(true);
    window.set_content(content.clone().map(Control::boxed));

    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    let hit_test_point = Point::new(WIDTH / 2.0, HEIGHT / 2.0);

    let client_visual = window.get_visual_at(hit_test_point);
    assert_eq!(content.map(Ref::upcast::<Visual>), client_visual);

    let source = window.presentation_source();
    // The managed hit tester of the tests hits every visual by its bounds, while the reference
    // renderer only hits what is drawn: the decoration layers without content are skipped.
    let chrome_visual = source.root_visual().expect("the window is open").get_visual_at_filtered(
        hit_test_point,
        &|visual: &Visual| visual.is_visible() && !(visual.is::<LayerWrapper>() && visual.visual_children_count() == 0),
    );
    assert_eq!(expected_chrome_visual, chrome_visual);

    let input_root: &dyn IInputRoot = &**source;
    let chrome_role = input_root.hit_test_chrome_element(hit_test_point);
    assert_eq!(expected_role, chrome_role);
}

fn same_cursor(expected: &Option<Rc<Cursor>>, actual: Option<Rc<dyn ICursorImpl>>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => Rc::ptr_eq(expected.platform_impl(), &actual),
        _ => false,
    }
}

#[test]
fn cursor_should_follow_captured_element() {
    let _app =
        UnitTestApplication::start(TestServices::styled_window().with_input_manager(Rc::new(InputManager::new())));

    let captured = Border::new();
    captured.set_background(Some(Brushes::red()));
    captured.set_width(20.0);
    captured.set_cursor(Some(Cursor::new(StandardCursorType::SizeWestEast)));

    let other1 = Border::new();
    other1.set_background(Some(Brushes::blue()));
    other1.set_width(100.0);
    other1.set_cursor(Some(Cursor::new(StandardCursorType::Ibeam)));

    let other2 = Border::new();
    other2.set_background(Some(Brushes::blue()));
    other2.set_width(100.0);
    other2.set_cursor(Some(Cursor::new(StandardCursorType::Cross)));

    // The window double keeps the cursor set last.
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(200.0, 100.0);

    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    panel.children().add(captured.clone().upcast::<Control>());
    panel.children().add(other1.clone().upcast::<Control>());
    panel.children().add(other2.clone().upcast::<Control>());

    let window = Window::with_impl(window_impl.clone());
    window.set_content(Some(Control::boxed(panel)));

    let pointer: Rc<RefCell<Option<Rc<dyn IPointer>>>> = Rc::new(RefCell::new(None));
    captured.add_handler(InputElement::pointer_pressed_event(), {
        let pointer = pointer.clone();
        let captured: Ref<InputElement> = captured.clone().upcast();
        move |_, e: &PointerPressedEventArgs| {
            e.pointer().capture(Some(&captured));
            *pointer.borrow_mut() = Some(e.pointer().clone());
        }
    });

    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    let mouse = MouseDevice::new();
    let root = window.presentation_source().clone();
    let input = ITopLevelImpl::input(&*window_impl).expect("the input callback is set");
    let captured_element: Ref<InputElement> = captured.clone().upcast();

    // Press inside the first border: the pointer becomes captured and the cursor is its own.
    input(Rc::new(RawPointerEventArgs::new(
        mouse.clone(),
        1,
        root.clone(),
        RawPointerEventType::LeftButtonDown,
        Point::new(10.0, 50.0),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    )));

    let pointer = pointer.borrow().clone().expect("the pointer was pressed on the first border");
    assert_eq!(Some(captured_element.clone()), pointer.captured());
    assert!(same_cursor(&captured.cursor(), window_impl.cursor()));
    let cursor_while_captured = window_impl.cursor().expect("a cursor is set");

    // Drag over the other border. With the pointer still captured by the first border,
    // the pointer-over element of the source changes (it becomes none), but the displayed
    // cursor must keep coming from the captured element rather than following the new
    // pointer-over element.
    input(Rc::new(RawPointerEventArgs::new(
        mouse.clone(),
        2,
        root.clone(),
        RawPointerEventType::Move,
        Point::new(70.0, 50.0),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    )));

    assert_eq!(Some(captured_element), pointer.captured());
    assert!(Rc::ptr_eq(&cursor_while_captured, &window_impl.cursor().expect("a cursor is set")));

    // Changing the captured element's cursor should still work.
    let new_cursor = Cursor::new(StandardCursorType::Hand);
    captured.set_cursor(Some(new_cursor.clone()));
    assert!(same_cursor(&Some(new_cursor), window_impl.cursor()));

    // Changing the capture explicitly should update the cursor.
    pointer.capture(Some(&other1.clone().upcast()));
    assert!(same_cursor(&other1.cursor(), window_impl.cursor()));

    // Move the pointer to an unrelated element and release the capture:
    // it should reset the cursor to match that new element.
    let input_root: &dyn IInputRoot = &*root;
    input_root.set_pointer_over_element(Some(other2.clone().upcast()));

    input(Rc::new(RawPointerEventArgs::new(
        mouse,
        2,
        root.clone(),
        RawPointerEventType::Move,
        Point::new(120.0, 50.0),
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    )));

    pointer.capture(None);
    assert!(same_cursor(&other2.cursor(), window_impl.cursor()));
}
