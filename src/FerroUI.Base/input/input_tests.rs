//! Tests for the input core: input elements, focus, devices, pointer-over
//! tracking, gestures and hit testing.

use super::raw::*;
use super::*;
use crate::interactivity::{Interactive, InteractiveImpl, RoutedEventArgs, RoutingStrategies};
use crate::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl};
use crate::platform::{DefaultPlatformSettings, ICursorFactory, ICursorImpl, IPlatformSettings};
use crate::rendering::{IHitTester, IPresentationSource, IRenderer, ManagedHitTester};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// --- test classes ---------------------------------------------------------------

/// An input element that can be a logical root and/or a focus scope and can
/// veto being enabled.
#[repr(C)]
struct TestControl {
    base: InputElement,
    label: &'static str,
    logical_root: bool,
    focus_scope: bool,
    should_enable: Cell<bool>,
    /// When set, the control handles its own keyboard navigation: the flag
    /// tells whether it handles requests and the element is what it returns.
    custom_navigation: RefCell<Option<(bool, Option<Ref<InputElement>>)>>,
}

ferro_class!(TestControl: InputElement);
ferro_impl_classes!(TestControl: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl StyledElementImpl for TestControl {
    fn is_logical_root(this: &Self) -> bool {
        this.logical_root
    }
}

impl InputElementImpl for TestControl {
    fn is_enabled_core(this: &Self) -> bool {
        this.is_enabled() && this.should_enable.get()
    }

    fn is_focus_scope(this: &Self) -> bool {
        this.focus_scope
    }

    fn as_custom_keyboard_navigation(this: &Self) -> Option<&dyn ICustomKeyboardNavigation> {
        if this.custom_navigation.borrow().is_some() {
            Some(this)
        } else {
            None
        }
    }
}

impl ICustomKeyboardNavigation for TestControl {
    fn get_next(&self, _element: &Ref<InputElement>, _direction: NavigationDirection) -> (bool, Option<Ref<InputElement>>) {
        self.custom_navigation.borrow().clone().expect("custom navigation is enabled")
    }
}

impl TestControl {
    fn create(label: &'static str, logical_root: bool, focus_scope: bool) -> Ref<Self> {
        instantiate(Self {
            base: InputElement::construct(),
            label,
            logical_root,
            focus_scope,
            should_enable: Cell::new(true),
            custom_navigation: RefCell::new(None),
        })
    }

    /// A plain, non-focusable element.
    fn new(label: &'static str) -> Ref<Self> {
        Self::create(label, false, false)
    }

    /// A focusable element, like a button.
    fn focusable(label: &'static str) -> Ref<Self> {
        let result = Self::new(label);
        result.set_focusable(true);
        result
    }

    /// A non-focusable focus scope.
    fn scope(label: &'static str) -> Ref<Self> {
        Self::create(label, false, true)
    }

    fn add_child(&self, child: &Ref<TestControl>) {
        self.logical_children().add(child.clone().upcast());
        self.visual_children().add(child.clone().upcast());
    }

    fn remove_child(&self, child: &Ref<TestControl>) {
        self.visual_children().remove(&child.clone().upcast());
        self.logical_children().remove(&child.clone().upcast());
    }

    fn set_should_enable(&self, value: bool) {
        self.should_enable.set(value);
        self.update_is_effectively_enabled();
    }

    fn has_class(&self, name: &str) -> bool {
        self.classes().contains(name)
    }
}

/// Fluent tree construction on handles.
trait TestControlExt {
    fn with_children(self, children: &[&Ref<TestControl>]) -> Self;
    fn at(self, x: f64, y: f64, width: f64, height: f64) -> Self;
}

impl TestControlExt for Ref<TestControl> {
    fn with_children(self, children: &[&Ref<TestControl>]) -> Self {
        for child in children {
            self.add_child(child);
        }
        self
    }

    fn at(self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.set_bounds(Rect::new(x, y, width, height));
        self
    }
}

fn label_of(sender: &Interactive) -> &'static str {
    sender.downcast_ref::<TestControl>().map_or("?", |c| c.label)
}

#[derive(Default)]
struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<crate::rendering::RendererDiagnostics> {
        crate::rendering::RendererDiagnostics::new()
    }
    fn scene_invalidated(
        &self,
        _handler: Rc<dyn Fn(&crate::rendering::SceneInvalidatedEventArgs)>,
    ) -> Rc<dyn crate::reactive::IDisposable> {
        crate::reactive::Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<crate::rendering::composition::RenderInterfaceFeature> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {}
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

/// A minimal presentation source: hosts a root element, hit tests it with
/// the managed hit tester and feeds raw input through an input manager, the
/// way a real presentation source does.
struct TestHost {
    this: Weak<TestHost>,
    root: Ref<TestControl>,
    renderer: Rc<TestRenderer>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
    focus_manager: Rc<FocusManager>,
    hit_tester: Rc<ManagedHitTester>,
    platform_settings: RefCell<Option<Rc<dyn IPlatformSettings>>>,
    pointer_over_element: RefCell<Option<Ref<InputElement>>>,
    cursor_element: RefCell<Option<Ref<InputElement>>>,
    input_manager: Rc<InputManager>,
    pointer_over: RefCell<Option<Rc<PointerOverPreProcessor>>>,
    screen_offset: Cell<PixelPoint>,
    input_method: RefCell<Option<Rc<dyn text_input::ITextInputMethodImpl>>>,
}

impl TestHost {
    fn new(root: &Ref<TestControl>) -> Rc<Self> {
        let host = Rc::new_cyclic(|this: &Weak<TestHost>| TestHost {
            this: this.clone(),
            root: root.clone(),
            renderer: Rc::new(TestRenderer),
            layout_manager: RefCell::new(None),
            focus_manager: FocusManager::new(),
            hit_tester: Rc::new(ManagedHitTester::new()),
            platform_settings: RefCell::new(Some(Rc::new(DefaultPlatformSettings::default()))),
            pointer_over_element: RefCell::new(None),
            cursor_element: RefCell::new(None),
            input_manager: Rc::new(InputManager::new()),
            pointer_over: RefCell::new(None),
            screen_offset: Cell::new(PixelPoint::new(0, 0)),
            input_method: RefCell::new(None),
        });

        let as_layout_root: Rc<dyn ILayoutRoot> = host.clone();
        *host.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));

        let pointer_over = Rc::new(PointerOverPreProcessor::new(host.clone()));
        host.input_manager.pre_process().subscribe(pointer_over.clone());
        *host.pointer_over.borrow_mut() = Some(pointer_over);

        if root.bounds() == Rect::default() {
            root.set_bounds(Rect::new(0.0, 0.0, 200.0, 200.0));
        }
        root.set_presentation_source_for_root_visual(Some(host.clone()));
        host
    }

    fn as_input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }

    fn first_enabled_ancestor(hit_test_element: Option<Ref<InputElement>>) -> Option<Ref<InputElement>> {
        let mut candidate = hit_test_element;
        while let Some(current) = &candidate {
            if current.is_effectively_enabled() {
                break;
            }
            candidate = current.get_visual_parent_of_type::<InputElement>();
        }
        candidate
    }

    /// Handles raw input the way a presentation source does: hit tests
    /// pointer events and passes the event to the input manager.
    fn input(&self, e: Rc<dyn IRawInputEventArgs>) {
        if let Some(pointer_args) = e.downcast_ref::<RawPointerEventArgs>() {
            let hit_test_element = self.root.input_hit_test_with(pointer_args.position(), false);
            pointer_args
                .set_input_hit_test_result(hit_test_element.clone(), Self::first_enabled_ancestor(hit_test_element));
        }

        self.input_manager.process_input(e);
    }

    fn pointer(&self, device: &Rc<MouseDevice>, type_: RawPointerEventType, position: Point, timestamp: u64) -> bool {
        let modifiers = match type_ {
            RawPointerEventType::LeftButtonDown => RawInputModifiers::LEFT_MOUSE_BUTTON,
            RawPointerEventType::RightButtonDown => RawInputModifiers::RIGHT_MOUSE_BUTTON,
            _ => RawInputModifiers::NONE,
        };
        let args = Rc::new(RawPointerEventArgs::new(
            device.clone(),
            timestamp,
            self.as_input_root(),
            type_,
            position,
            modifiers,
        ));
        self.input(args.clone());
        args.handled()
    }

    fn mouse_move(&self, device: &Rc<MouseDevice>, position: Point) {
        self.pointer(device, RawPointerEventType::Move, position, 0);
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        self.input_manager.dispose();
    }
}

impl IPresentationSource for TestHost {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        Some(self.root.clone().upcast())
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        self.root.bounds().size()
    }
    fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        self.platform_settings.borrow().clone()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        self.hit_tester.clone()
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }
    fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
        Some(PixelPoint::from_point(point, 1.0) + self.screen_offset.get())
    }
    fn point_to_client(&self, point: PixelPoint) -> Option<Point> {
        Some((point - self.screen_offset.get()).to_point(1.0))
    }
}

impl ILayoutRoot for TestHost {
    fn layout_scaling(&self) -> f64 {
        1.0
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root.clone().upcast()
    }
}

impl IInputRoot for TestHost {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        Some(self.focus_manager.clone())
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        self.pointer_over_element.borrow().clone()
    }
    fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>) {
        drop(self.pointer_over_element.replace(value));
    }
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        self.cursor_element.borrow().clone()
    }
    fn set_cursor_element(&self, value: Option<Ref<InputElement>>) {
        drop(self.cursor_element.replace(value));
    }
    fn input_method(&self) -> Option<Rc<dyn text_input::ITextInputMethodImpl>> {
        self.input_method.borrow().clone()
    }
    fn root_element(&self) -> Ref<InputElement> {
        self.root.clone().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root.clone().upcast()
    }
    fn pointer_over_invalidated(&self) {
        let pointer_over = self.pointer_over.borrow().clone();
        if let Some(pointer_over) = pointer_over {
            pointer_over.scene_invalidated(Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY));
        }
    }
}

/// A root element (logical root and focus scope) with its host.
struct TestRoot {
    root: Ref<TestControl>,
    host: Rc<TestHost>,
}

impl TestRoot {
    fn new() -> Self {
        let root = TestControl::create("root", true, true);
        let host = TestHost::new(&root);
        Self { root, host }
    }

    fn with_child(child: &Ref<TestControl>) -> Self {
        let result = Self::new();
        result.root.add_child(child);
        result
    }

    fn focus_manager(&self) -> &Rc<FocusManager> {
        &self.host.focus_manager
    }
}

/// Installs a keyboard device for the current test thread.
fn real_focus() -> Rc<KeyboardDevice> {
    let keyboard = KeyboardDevice::new();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard.clone());
    keyboard
}

fn log() -> Rc<RefCell<Vec<String>>> {
    Rc::new(RefCell::new(Vec::new()))
}

// --- enabled ---------------------------------------------------------------------

#[test]
fn is_effectively_enabled_follows_is_enabled() {
    let target = TestControl::new("target");

    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());

    target.set_is_enabled(false);

    assert!(!target.is_enabled());
    assert!(!target.is_effectively_enabled());
}

#[test]
fn is_effectively_enabled_follows_ancestor_is_enabled() {
    let grandchild = TestControl::new("grandchild");
    let child = TestControl::new("child").with_children(&[&grandchild]);
    let target = TestControl::new("target").with_children(&[&child]);

    assert!(target.is_enabled() && target.is_effectively_enabled());
    assert!(child.is_enabled() && child.is_effectively_enabled());
    assert!(grandchild.is_enabled() && grandchild.is_effectively_enabled());

    target.set_is_enabled(false);

    assert!(!target.is_enabled());
    assert!(!target.is_effectively_enabled());
    assert!(child.is_enabled());
    assert!(!child.is_effectively_enabled());
    assert!(grandchild.is_enabled());
    assert!(!grandchild.is_effectively_enabled());
}

#[test]
fn disabled_pseudoclass_follows_is_effectively_enabled() {
    let child = TestControl::new("child");
    let target = TestControl::new("target").with_children(&[&child]);

    assert!(!child.has_class(":disabled"));

    target.set_is_enabled(false);

    assert!(child.has_class(":disabled"));
}

#[test]
fn is_effectively_enabled_respects_is_enabled_core() {
    let child = TestControl::new("child");
    let target = TestControl::new("target").with_children(&[&child]);

    target.set_should_enable(false);

    assert!(target.is_enabled());
    assert!(!target.is_effectively_enabled());
    assert!(child.is_enabled());
    assert!(!child.is_effectively_enabled());
}

#[test]
fn is_effectively_enabled_is_updated_when_attached_to_tree() {
    let target = TestControl::new("target");
    let container = TestControl::new("container");
    container.set_is_enabled(false);
    let root = TestRoot::with_child(&container);

    // Only the visual tree attachment updates the state.
    container.add_child(&target);

    assert!(!target.is_effectively_enabled());
    assert!(root.root.is_effectively_enabled());
}

// --- properties ------------------------------------------------------------------

#[test]
fn input_element_defaults() {
    let target = InputElement::new();

    assert!(!target.focusable());
    assert!(target.is_enabled());
    assert!(target.is_effectively_enabled());
    assert!(target.cursor().is_none());
    assert!(!target.is_keyboard_focus_within());
    assert!(!target.is_focused());
    assert!(target.is_hit_test_visible());
    assert!(!target.is_pointer_over());
    assert!(target.is_tab_stop());
    assert_eq!(target.tab_index(), i32::MAX);
    assert!(!target.is_focus_scope());

    target.set_tab_index(3);
    target.set_is_tab_stop(false);
    assert_eq!(KeyboardNavigation::get_tab_index(&target), 3);
    assert!(!KeyboardNavigation::get_is_tab_stop(&target));
    assert!(InputElement::get_is_holding_enabled(&target));
    assert!(!InputElement::get_is_hold_with_mouse_enabled(&target));
}

#[test]
fn direct_properties_raise_change_notifications() {
    let target = TestControl::new("target");
    let changes = log();
    let recorded = changes.clone();
    target.property_changed(move |e| recorded.borrow_mut().push(e.property().name().to_string()));

    target.set_is_pointer_over(true);
    target.set_is_keyboard_focus_within(true);

    assert_eq!(*changes.borrow(), ["IsPointerOver", "IsKeyboardFocusWithin"]);
    assert!(target.has_class(":pointerover"));
    assert!(target.has_class(":focus-within"));
    assert!(target.get_direct_value(InputElement::is_pointer_over_property()));
}

struct TestCursorImpl;

impl ICursorImpl for TestCursorImpl {
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct TestCursorFactory;

impl ICursorFactory for TestCursorFactory {
    fn create_cursor(&self, _cursor: &crate::media::imaging::Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(TestCursorImpl)
    }

    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(TestCursorImpl)
    }
}

#[test]
fn cursor_is_inherited_and_parsed() {
    FerroLocator::current_mutable().bind::<dyn ICursorFactory>().to_constant(Rc::new(TestCursorFactory));

    let child = TestControl::new("child");
    let parent = TestControl::new("parent").with_children(&[&child]);
    let hand = Cursor::parse("hand").unwrap();

    parent.set_cursor(Some(hand.clone()));

    assert_eq!(hand.to_string(), "Hand");
    assert!(child.cursor().is_some_and(|c| Rc::ptr_eq(&c, &hand)));
    assert!(Cursor::parse("NotACursor").is_err());
    assert!(Rc::ptr_eq(&Cursor::default_cursor(), &Cursor::default_cursor()));
    assert_eq!(Cursor::default_cursor().to_string(), "Arrow");
    assert!(*Cursor::new(StandardCursorType::Arrow) != *Cursor::default_cursor());

}

// --- keyboard device -------------------------------------------------------------

#[test]
fn keypresses_should_be_sent_to_root_if_no_focused_element() {
    let keyboard = real_focus();
    let root = TestRoot::new();
    let raised = Rc::new(Cell::new(0));

    root.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);

    let counter = raised.clone();
    root.root.key_down(move |sender, ev| {
        if label_of(sender) == "root" && ev.routed_event() == Some(InputElement::key_down_event().as_routed_event()) {
            counter.set(counter.get() + 1);
        }
    });

    keyboard.process_raw_event(&RawKeyEventArgs::new(
        keyboard.clone(),
        0,
        root.host.as_input_root(),
        RawKeyEventType::KeyDown,
        Key::A,
        RawInputModifiers::NONE,
        PhysicalKey::A,
        Some("a".to_string()),
        KeyDeviceType::Keyboard,
    ));

    assert_eq!(raised.get(), 1);
}

#[test]
fn keypresses_should_be_sent_to_focused_element() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let root = TestRoot::new();
    let raised = Rc::new(RefCell::new(Vec::new()));

    target.set_focused_element(Some(&focused.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    let recorded = raised.clone();
    focused.key_down(move |_, e| recorded.borrow_mut().push((e.key, e.key_modifiers, e.key_symbol.clone())));

    let args = Rc::new(RawKeyEventArgs::new(
        target.clone(),
        0,
        root.host.as_input_root(),
        RawKeyEventType::KeyDown,
        Key::A,
        RawInputModifiers::CONTROL | RawInputModifiers::LEFT_MOUSE_BUTTON,
        PhysicalKey::A,
        Some("a".to_string()),
        KeyDeviceType::Keyboard,
    ));
    target.process_raw_event(&*args);

    assert_eq!(*raised.borrow(), [(Key::A, KeyModifiers::CONTROL, Some("a".to_string()))]);
    assert!(!args.handled());
}

#[test]
fn key_up_and_handled_state_are_reported_back() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let root = TestRoot::new();

    target.set_focused_element(Some(&focused.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);
    focused.key_up(|_, e| e.set_handled(true));

    let args = RawKeyEventArgs::new(
        target.clone(),
        0,
        root.host.as_input_root(),
        RawKeyEventType::KeyUp,
        Key::A,
        RawInputModifiers::NONE,
        PhysicalKey::A,
        None,
        KeyDeviceType::Keyboard,
    );
    target.process_raw_event(&args);
    assert!(args.handled());

    // A handled raw event is not processed again.
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    focused.key_up(move |_, _| counter.set(counter.get() + 1));
    target.process_raw_event(&args);
    assert_eq!(count.get(), 0);
}

#[test]
fn text_input_should_be_sent_to_root_if_no_focused_element() {
    let keyboard = real_focus();
    let root = TestRoot::new();
    let raised = Rc::new(Cell::new(0));

    root.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);

    let counter = raised.clone();
    root.root.text_input(move |sender, ev| {
        if label_of(sender) == "root" && ev.routed_event() == Some(InputElement::text_input_event().as_routed_event())
        {
            counter.set(counter.get() + 1);
        }
    });

    keyboard.process_raw_event(&RawTextInputEventArgs::new(keyboard.clone(), 0, root.host.as_input_root(), "Foo"));

    assert_eq!(raised.get(), 1);
}

#[test]
fn text_input_should_be_sent_to_focused_element() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let root = TestRoot::new();
    let raised = Rc::new(RefCell::new(Vec::new()));

    target.set_focused_element(Some(&focused.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    let recorded = raised.clone();
    focused.text_input(move |_, e| recorded.borrow_mut().push(e.text.clone()));

    target.process_raw_event(&RawTextInputEventArgs::new(target.clone(), 0, root.host.as_input_root(), "Foo"));

    assert_eq!(*raised.borrow(), [Some("Foo".to_string())]);
}

#[test]
fn control_focus_should_be_set_before_focused_element_raises_property_changed() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let got_focus_raised = Rc::new(Cell::new(0));
    let property_changed_raised = Rc::new(Cell::new(0));

    let counter = got_focus_raised.clone();
    focused.got_focus(move |_, _| counter.set(counter.get() + 1));

    let (got_focus, counter) = (got_focus_raised.clone(), property_changed_raised.clone());
    target.property_changed(move |property_name| {
        if property_name == "FocusedElement" {
            assert_eq!(got_focus.get(), 1);
            counter.set(counter.get() + 1);
        }
    });

    target.set_focused_element(Some(&focused.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    assert_eq!(property_changed_raised.get(), 1);
}

#[test]
fn cancelled_focus_change_should_not_send_got_focus_event() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let focus_cancelled = Rc::new(Cell::new(false));

    let cancelled = focus_cancelled.clone();
    focused.getting_focus(move |_, e| cancelled.set(e.try_cancel()));

    let cancelled = focus_cancelled.clone();
    focused.got_focus(move |_, _| cancelled.set(false));

    target.set_focused_element(Some(&focused.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    assert!(focus_cancelled.get());
    assert!(target.focused_element().is_none());
    assert!(!focused.is_focused());
}

#[test]
fn non_cancellable_focus_change_cannot_be_cancelled() {
    let target = KeyboardDevice::new();
    let focused = TestControl::new("focused");
    let cancelled = Rc::new(Cell::new(true));

    let result = cancelled.clone();
    focused.getting_focus(move |_, e| result.set(e.try_cancel()));

    target.set_focused_element_with(
        Some(&focused.clone().upcast()),
        NavigationMethod::Unspecified,
        KeyModifiers::NONE,
        false,
    );

    assert!(!cancelled.get());
    assert!(focused.is_focused());
}

#[test]
fn redirected_focus_should_change_focused_element() {
    let target = KeyboardDevice::new();
    let first = TestControl::new("first");
    let second = TestControl::new("second");
    let stack = TestControl::new("stack").with_children(&[&first, &second]);
    let _root = TestRoot::with_child(&stack);

    let redirect_to: Ref<InputElement> = second.clone().upcast();
    first.getting_focus(move |_, e| {
        e.try_set_new_focused_element(Some(&redirect_to));
    });

    target.set_focused_element(Some(&first.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    assert!(second.is_focused());
    assert!(!first.is_focused());
}

#[test]
fn focus_events_carry_old_and_new_elements() {
    let target = KeyboardDevice::new();
    let first = TestControl::new("first");
    let second = TestControl::new("second");
    let events = log();

    for control in [&first, &second] {
        let recorded = events.clone();
        control.lost_focus(move |s, e| {
            recorded.borrow_mut().push(format!(
                "{} lost: {:?} -> {:?} {:?}",
                label_of(s),
                e.old_focused_element.as_ref().map(|x| label_of(x)),
                e.new_focused_element.as_ref().map(|x| label_of(x)),
                e.navigation_method
            ));
        });
        let recorded = events.clone();
        control.got_focus(move |s, e| {
            recorded.borrow_mut().push(format!(
                "{} got: {:?} -> {:?} {:?}",
                label_of(s),
                e.old_focused_element.as_ref().map(|x| label_of(x)),
                e.new_focused_element.as_ref().map(|x| label_of(x)),
                e.key_modifiers
            ));
        });
    }

    target.set_focused_element(Some(&first.clone().upcast()), NavigationMethod::Tab, KeyModifiers::SHIFT);
    target.set_focused_element(Some(&second.clone().upcast()), NavigationMethod::Pointer, KeyModifiers::NONE);

    assert_eq!(
        *events.borrow(),
        [
            "first got: None -> Some(\"first\") KeyModifiers(SHIFT)",
            "first lost: Some(\"first\") -> Some(\"second\") Pointer",
            "second got: Some(\"first\") -> Some(\"second\") KeyModifiers(0x0)",
        ]
    );
}

// --- focus -----------------------------------------------------------------------

// Upstream's `InputElement_Focus.cs` is ported with the controls
// (`input_element_focus_tests.rs` of `ferroui-controls`). The tests of this
// section and of "focus manager navigation" are not from upstream.

/// Not a port: a focus scope does not own its focused element. A focused
/// focus scope is its own focused element, and it must still be released
/// when the last reference to it is dropped.
#[test]
fn focused_focus_scope_is_released_when_dropped() {
    let _keyboard = real_focus();
    let scope = TestControl::scope("scope");
    scope.set_focusable(true);
    let root = TestRoot::with_child(&scope);

    assert!(scope.focus());
    assert_eq!(root.focus_manager().get_focused_element(), Some(scope.clone().upcast()));
    assert_eq!(root.focus_manager().get_focused_element_in_scope(&scope), Some(scope.clone().upcast()));

    root.root.remove_child(&scope);
    assert_eq!(root.focus_manager().get_focused_element(), None);

    let weak = scope.downgrade();
    drop(scope);

    assert!(weak.upgrade().is_none());
}

/// Not a port: without a keyboard device nothing is focused.
#[test]
fn focus_without_keyboard_device_does_nothing() {
    let target = TestControl::focusable("target");
    let _root = TestRoot::with_child(&target);

    assert!(!target.focus());
    assert!(!target.is_focused());
}

/// Not a port: a control that is not focusable, and one that is not
/// attached, are not focused.
#[test]
fn non_focusable_and_detached_controls_should_not_receive_focus() {
    let _keyboard = real_focus();
    FerroLocator::current_mutable().bind::<dyn IFocusManager>().to_constant(FocusManager::new());
    let not_focusable = TestControl::new("target");
    let detached = TestControl::focusable("detached");
    let _root = TestRoot::with_child(&not_focusable);

    assert!(!not_focusable.focus());
    // Not attached to a visual tree: handled by the fallback focus manager,
    // which refuses it because it is not visible on any root.
    assert!(!detached.focus());

}

// --- pointer ---------------------------------------------------------------------

/// The tree used by the capture tests:
/// `root -> el -> (initialParent -> initialCapture, newParent -> newCapture)`.
struct CaptureTree {
    root: TestRoot,
    el: Ref<TestControl>,
    initial_parent: Ref<TestControl>,
    initial_capture: Ref<TestControl>,
    new_parent: Ref<TestControl>,
    new_capture: Ref<TestControl>,
}

fn capture_tree() -> CaptureTree {
    let initial_capture = TestControl::new("initialCapture");
    let initial_parent = TestControl::new("initialParent").with_children(&[&initial_capture]);
    let new_capture = TestControl::new("newCapture");
    let new_parent = TestControl::new("newParent").with_children(&[&new_capture]);
    let el = TestControl::new("el").with_children(&[&initial_parent, &new_parent]);
    let root = TestRoot::with_child(&el);
    CaptureTree { root, el, initial_parent, initial_capture, new_parent, new_capture }
}

#[test]
fn on_capture_transfer_pointer_capture_lost_should_propagate_up_to_the_common_parent() {
    let tree = capture_tree();
    let receivers = log();

    for visual in tree.root.root.get_self_and_visual_descendants() {
        let recorded = receivers.clone();
        visual
            .cast::<InputElement>()
            .unwrap()
            .pointer_capture_lost(move |s, _| recorded.borrow_mut().push(label_of(s).to_string()));
    }

    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);

    pointer.capture(Some(&tree.initial_capture.clone().upcast()));
    pointer.capture(Some(&tree.new_capture.clone().upcast()));

    assert_eq!(*receivers.borrow(), ["initialCapture", "initialParent"]);
    assert_eq!(pointer.captured(), Some(tree.new_capture.clone().upcast()));

    receivers.borrow_mut().clear();
    pointer.capture(None);

    assert_eq!(*receivers.borrow(), ["newCapture", "newParent", "el", "root"]);
    assert!(pointer.captured().is_none());
    let _ = (&tree.el, &tree.initial_parent, &tree.new_parent);
}

fn counting_pointer() -> (Rc<Pointer>, Rc<Cell<i32>>) {
    let platform_capture_called = Rc::new(Cell::new(0));
    let counter = platform_capture_called.clone();
    let pointer = Pointer::with_platform_capture(Pointer::get_next_free_id(), PointerType::Mouse, true, move |_| {
        counter.set(counter.get() + 1)
    });
    (pointer, platform_capture_called)
}

#[test]
fn capture_captured_should_not_call_platform() {
    let (pointer, platform_capture_called) = counting_pointer();
    let capture: Ref<InputElement> = TestControl::new("capture").upcast();

    pointer.capture(Some(&capture));
    pointer.capture(Some(&capture));

    assert_eq!(platform_capture_called.get(), 1);

    pointer.capture(None);
    pointer.capture(None);

    assert_eq!(platform_capture_called.get(), 2);
}

#[test]
fn capture_explicit_should_notify_after_implicit() {
    let (pointer, platform_capture_called) = counting_pointer();
    let capture = TestControl::new("capture");
    let capture_element: Ref<InputElement> = capture.clone().upcast();
    let sources = Rc::new(RefCell::new(Vec::new()));

    let recorded = sources.clone();
    capture.pointer_capture_changing(move |_, e| recorded.borrow_mut().push(e.capture_source()));

    pointer.capture_with_source(Some(&capture_element), CaptureSource::Implicit);
    pointer.capture_with_source(Some(&capture_element), CaptureSource::Explicit);

    assert_eq!(*sources.borrow(), [CaptureSource::Implicit, CaptureSource::Explicit]);
    assert_eq!(platform_capture_called.get(), 1);

    // Not ignored, so the captured element will become null.
    pointer.capture_with_source(None, CaptureSource::Implicit);
    // Changing from null to null does not notify anything.
    pointer.capture_with_source(None, CaptureSource::Explicit);

    assert_eq!(*sources.borrow(), [CaptureSource::Implicit, CaptureSource::Explicit, CaptureSource::Implicit]);
    assert_eq!(platform_capture_called.get(), 2);
}

#[test]
fn capture_explicit_should_notify_after_handled_implicit() {
    let (pointer, platform_capture_called) = counting_pointer();
    let capture = TestControl::new("capture");
    let capture_element: Ref<InputElement> = capture.clone().upcast();
    let sources = Rc::new(RefCell::new(Vec::new()));

    let recorded = sources.clone();
    capture.pointer_capture_changing(move |_, e| {
        recorded.borrow_mut().push(e.capture_source());
        e.set_handled(e.capture_source() == CaptureSource::Implicit);
    });

    pointer.capture_with_source(Some(&capture_element), CaptureSource::Implicit);
    pointer.capture_with_source(Some(&capture_element), CaptureSource::Explicit);

    assert_eq!(*sources.borrow(), [CaptureSource::Implicit, CaptureSource::Explicit]);
    assert_eq!(platform_capture_called.get(), 1);

    pointer.capture_with_source(None, CaptureSource::Implicit);
    pointer.capture_with_source(None, CaptureSource::Explicit);

    assert_eq!(
        *sources.borrow(),
        [CaptureSource::Implicit, CaptureSource::Explicit, CaptureSource::Implicit, CaptureSource::Explicit]
    );
    assert_eq!(platform_capture_called.get(), 2);
}

#[test]
fn disposed_pointer_loses_capture_and_cannot_capture() {
    let (pointer, _) = counting_pointer();
    let capture: Ref<InputElement> = TestControl::new("capture").upcast();

    pointer.capture(Some(&capture));
    pointer.set_is_gesture_recognition_skipped(true);
    pointer.dispose();

    assert!(pointer.captured().is_none());
    assert!(!pointer.is_gesture_recognition_skipped());
    assert_eq!(pointer.capture_source(), CaptureSource::Platform);

    pointer.capture(None);
    pointer.dispose();
    assert!(pointer.captured().is_none());
}

// --- mouse device ----------------------------------------------------------------

/// `root(200x200) -> panel(200x200) -> (canvas(0,0,100,200),
/// border(100,0,100,200) -> decorator(0,0,100,200))`.
struct PointerTree {
    root: TestRoot,
    panel: Ref<TestControl>,
    canvas: Ref<TestControl>,
    border: Ref<TestControl>,
    decorator: Ref<TestControl>,
}

const OVER_CANVAS: Point = Point::new(50.0, 50.0);
const OVER_DECORATOR: Point = Point::new(150.0, 50.0);

fn pointer_tree() -> PointerTree {
    let decorator = TestControl::new("decorator").at(0.0, 0.0, 100.0, 200.0);
    let border = TestControl::new("border").at(100.0, 0.0, 100.0, 200.0).with_children(&[&decorator]);
    let canvas = TestControl::new("canvas").at(0.0, 0.0, 100.0, 200.0);
    let panel = TestControl::new("panel").at(0.0, 0.0, 200.0, 200.0).with_children(&[&canvas, &border]);
    let root = TestRoot::with_child(&panel);
    PointerTree { root, panel, canvas, border, decorator }
}

#[test]
fn initial_buttons_are_not_set_without_corresponding_mouse_down() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let button = Rc::new(Cell::new(MouseButton::None));

    let recorded = button.clone();
    tree.root.root.pointer_released(move |_, e| recorded.set(e.initial_press_mouse_button()));

    let host = &tree.root.host;

    host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 0);
    assert_eq!(button.get(), MouseButton::None);

    host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 0);
    host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 0);
    assert_eq!(button.get(), MouseButton::Left);

    host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 0);
    assert_eq!(button.get(), MouseButton::None);
}

#[test]
fn capture_is_transferred_to_parent_when_control_removed() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let result: Rc<RefCell<Option<Rc<dyn IPointer>>>> = Rc::new(RefCell::new(None));

    // Synthesize an event to receive a pointer.
    let recorded = result.clone();
    tree.root.root.pointer_moved(move |_, a| *recorded.borrow_mut() = Some(a.pointer().clone()));

    tree.root.host.mouse_move(&device, OVER_CANVAS);

    let pointer = result.borrow().clone().expect("a pointer moved event was raised");
    let canvas: Ref<InputElement> = tree.canvas.clone().upcast();
    pointer.capture(Some(&canvas));
    assert_eq!(pointer.captured(), Some(canvas));

    tree.panel.remove_child(&tree.canvas);

    assert_eq!(pointer.captured(), Some(tree.panel.clone().upcast()));
}

#[test]
fn raw_press_is_routed_to_hit_tested_element_and_captures_it() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let events = log();

    for control in [&tree.root.root, &tree.panel, &tree.canvas, &tree.border, &tree.decorator] {
        let recorded = events.clone();
        control.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |s, e| {
                recorded.borrow_mut().push(format!(
                    "{} {:?} {} {}",
                    label_of(s),
                    e.route(),
                    e.click_count(),
                    e.get_position(Some(s))
                ));
            },
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            false,
        );
    }

    let handled = tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 10);

    assert!(!handled);
    assert_eq!(
        *events.borrow(),
        [
            "root RoutingStrategies(TUNNEL) 1 150, 50",
            "panel RoutingStrategies(TUNNEL) 1 150, 50",
            "border RoutingStrategies(TUNNEL) 1 50, 50",
            "decorator RoutingStrategies(TUNNEL) 1 50, 50",
            "decorator RoutingStrategies(BUBBLE) 1 50, 50",
            "border RoutingStrategies(BUBBLE) 1 50, 50",
            "panel RoutingStrategies(BUBBLE) 1 150, 50",
            "root RoutingStrategies(BUBBLE) 1 150, 50",
        ]
    );
    assert_eq!(device.pointer().captured(), Some(tree.decorator.clone().upcast()));
    assert_eq!(device.pointer().capture_source(), CaptureSource::Implicit);

    // Releasing over another element still goes to the captured one, then
    // the implicit capture is released.
    let released = log();
    let recorded = released.clone();
    tree.root.root.pointer_released(move |_, e| {
        let source = e.source().and_then(|s| s.cast::<Interactive>());
        recorded.borrow_mut().push(source.map_or("?", |i| label_of(&i)).to_string());
    });
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 20);

    assert_eq!(*released.borrow(), ["decorator"]);
    assert!(device.pointer().captured().is_none());
}

#[test]
fn handled_press_marks_raw_event_handled() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    tree.canvas.pointer_pressed(|_, e| e.set_handled(true));

    assert!(tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 0));
    assert!(!tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 1));
}

#[test]
fn click_count_follows_double_tap_time_and_size() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let counts = Rc::new(RefCell::new(Vec::new()));

    let recorded = counts.clone();
    tree.root.root.pointer_pressed(move |_, e| recorded.borrow_mut().push(e.click_count()));

    let host = &tree.root.host;
    let click = |position: Point, timestamp: u64| {
        host.pointer(&device, RawPointerEventType::LeftButtonDown, position, timestamp);
        host.pointer(&device, RawPointerEventType::LeftButtonUp, position, timestamp);
    };

    click(OVER_CANVAS, 1000);
    click(OVER_CANVAS, 1100);
    click(OVER_CANVAS, 1200);
    // Too late: starts again.
    click(OVER_CANVAS, 2000);
    // Too far away: starts again.
    click(Point::new(60.0, 50.0), 2100);

    assert_eq!(*counts.borrow(), [1, 2, 3, 1, 1]);
}

#[test]
fn click_count_is_zero_without_platform_settings() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    *tree.root.host.platform_settings.borrow_mut() = None;
    let counts = Rc::new(RefCell::new(Vec::new()));

    let recorded = counts.clone();
    tree.root.root.pointer_pressed(move |_, e| recorded.borrow_mut().push(e.click_count()));

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 0);

    assert_eq!(*counts.borrow(), [0]);
}

#[test]
fn wheel_is_routed_to_element_under_pointer() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let deltas = Rc::new(RefCell::new(Vec::new()));

    let recorded = deltas.clone();
    tree.border.pointer_wheel_changed(move |_, e| recorded.borrow_mut().push((e.delta(), e.timestamp())));

    let args = Rc::new(RawMouseWheelEventArgs::new(
        device.clone(),
        42,
        tree.root.host.as_input_root(),
        OVER_DECORATOR,
        Vector::new(0.0, 1.0),
        RawInputModifiers::NONE,
    ));
    tree.root.host.input(args);

    assert_eq!(*deltas.borrow(), [(Vector::new(0.0, 1.0), 42)]);
}

#[test]
fn mouse_pointer_should_set_focus_on_pointer_pressed() {
    let _keyboard = real_focus();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    tree.canvas.set_focusable(true);

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 0);

    assert!(tree.canvas.is_focused());
    assert_eq!(
        tree.root.focus_manager().get_focused_element(),
        Some(tree.canvas.clone().upcast())
    );
}

#[test]
fn pointer_press_focuses_nearest_focusable_ancestor() {
    let _keyboard = real_focus();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    tree.border.set_focusable(true);

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);

    assert!(tree.border.is_focused());
    assert!(!tree.decorator.is_focused());
}

#[test]
fn control_should_not_gain_focus_on_mouse_release() {
    let _keyboard = real_focus();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    tree.canvas.set_focusable(true);
    tree.border.set_focusable(true);

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_DECORATOR, 0);
    assert!(tree.border.is_focused());

    // Release the capture so that the release goes to the canvas.
    device.pointer().capture(None);
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 1);

    assert!(tree.border.is_focused());
    assert!(!tree.canvas.is_focused());
}

#[test]
fn get_position_should_respect_control_render_transform() {
    let device = MouseDevice::new();
    let border = TestControl::new("border").at(0.0, 0.0, 200.0, 200.0);
    border.set_render_transform(Some(Rc::new(crate::media::immutable::ImmutableTransform::new(
        Matrix::create_translation(10.0, 0.0),
    ))));
    border.set_render_transform_origin(RelativePoint::TOP_LEFT);
    let root = TestRoot::with_child(&border);
    let result = Rc::new(Cell::new(None));

    let (recorded, relative_to) = (result.clone(), border.clone());
    root.root.pointer_moved(move |_, a| recorded.set(Some(a.get_position(Some(&relative_to)))));

    root.host.mouse_move(&device, Point::new(11.0, 11.0));

    assert_eq!(result.get(), Some(Point::new(1.0, 11.0)));

    // Hit testing follows the transform: the border now starts at x = 10.
    assert_eq!(root.root.input_hit_test(Point::new(5.0, 5.0)), Some(root.root.clone().upcast()));
    assert_eq!(root.root.input_hit_test(Point::new(15.0, 5.0)), Some(border.clone().upcast()));
}

#[test]
fn render_transform_is_applied_around_its_origin() {
    let border = TestControl::new("border").at(50.0, 50.0, 100.0, 100.0);
    border.set_render_transform(Some(Rc::new(crate::media::immutable::ImmutableTransform::new(
        Matrix::create_scale(2.0, 2.0),
    ))));
    let root = TestRoot::with_child(&border);

    // The default origin is the centre of the control: scaled by two, the
    // 100x100 control covers (0,0)-(200,200) of its parent.
    assert_eq!(border.translate_point(Point::new(0.0, 0.0), &root.root), Some(Point::new(0.0, 0.0)));
    assert_eq!(border.translate_point(Point::new(100.0, 100.0), &root.root), Some(Point::new(200.0, 200.0)));
    assert_eq!(root.root.translate_point(Point::new(100.0, 100.0), &border), Some(Point::new(50.0, 50.0)));
    assert_eq!(root.root.input_hit_test(Point::new(10.0, 10.0)), Some(border.clone().upcast()));
    assert_eq!(
        border.get_transformed_bounds().map(|b| b.transform),
        Some(Matrix::create_scale(2.0, 2.0))
    );
}

#[test]
fn get_position_should_support_cross_tree_requests() {
    let device = MouseDevice::new();
    let top_level_offset = PixelPoint::new(5, 0);

    let tree1 = pointer_tree();
    tree1.root.host.screen_offset.set(top_level_offset);
    let tree2 = pointer_tree();
    tree2.root.host.screen_offset.set(top_level_offset + top_level_offset);

    let positions = Rc::new(RefCell::new(Vec::new()));
    let recorded = positions.clone();
    let element_b = tree2.canvas.clone();
    let detached = TestControl::new("detached");
    tree1.canvas.pointer_moved(move |_, e| {
        recorded.borrow_mut().push(e.get_position(Some(&element_b)));
        recorded.borrow_mut().push(e.get_position(None));
        recorded.borrow_mut().push(e.get_position(Some(&detached)));
    });

    tree1.root.host.mouse_move(&device, Point::new(10.0, 10.0));

    assert_eq!(*positions.borrow(), [Point::new(5.0, 10.0), Point::new(10.0, 10.0), Point::default()]);
}

// --- pointer over ----------------------------------------------------------------

fn pointer_over_states(tree: &PointerTree) -> [bool; 4] {
    [
        tree.decorator.is_pointer_over(),
        tree.border.is_pointer_over(),
        tree.canvas.is_pointer_over(),
        tree.root.root.is_pointer_over(),
    ]
}

#[test]
fn mouse_move_should_update_is_pointer_over() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    tree.root.host.mouse_move(&device, OVER_DECORATOR);

    assert_eq!(pointer_over_states(&tree), [true, true, false, true]);
    assert!(tree.decorator.has_class(":pointerover"));
    assert_eq!(tree.root.host.pointer_over_element(), Some(tree.decorator.clone().upcast()));
    assert_eq!(tree.root.host.cursor_element(), Some(tree.decorator.clone().upcast()));

    tree.root.host.mouse_move(&device, OVER_CANVAS);

    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);
    assert!(!tree.decorator.has_class(":pointerover"));
}

#[test]
fn hit_test_should_ignore_non_captured_elements() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    device.pointer().capture(Some(&tree.decorator.clone().upcast()));

    // Move the pointer over the canvas: the captured decorator should lose
    // the pointer over state.
    tree.root.host.mouse_move(&device, OVER_CANVAS);

    assert_eq!(pointer_over_states(&tree), [false, false, false, false]);

    // Move back the pointer over the decorator: raise events normally for
    // it since it's captured.
    tree.root.host.mouse_move(&device, OVER_DECORATOR);

    assert_eq!(pointer_over_states(&tree), [true, true, false, true]);
}

#[test]
fn is_pointer_over_should_be_updated_when_child_sets_handled_true() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    tree.root.host.mouse_move(&device, OVER_CANVAS);

    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);

    // Ensure that e.Handled is reset between controls.
    tree.root.root.pointer_moved(|_, e| e.set_handled(true));
    tree.decorator.pointer_entered(|_, e| e.set_handled(true));

    tree.root.host.mouse_move(&device, OVER_DECORATOR);

    assert_eq!(pointer_over_states(&tree), [true, true, false, true]);
}

fn add_entered_exited_handlers(events: &Rc<RefCell<Vec<String>>>, controls: &[&Ref<TestControl>]) {
    for control in controls {
        for event in [
            InputElement::pointer_entered_event(),
            InputElement::pointer_exited_event(),
            InputElement::pointer_moved_event(),
        ] {
            let recorded = events.clone();
            control.add_handler(event, move |s, e| {
                recorded.borrow_mut().push(format!(
                    "{} {} {}",
                    label_of(s),
                    e.routed_event().unwrap().name(),
                    e.get_position(None)
                ));
            });
        }
    }
}

#[test]
fn pointer_enter_move_leave_should_be_followed() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let result = log();

    add_entered_exited_handlers(&result, &[&tree.canvas, &tree.decorator]);

    // Enter decorator.
    tree.root.host.mouse_move(&device, OVER_DECORATOR);
    // Leave decorator.
    tree.root.host.mouse_move(&device, OVER_CANVAS);

    assert_eq!(
        *result.borrow(),
        [
            "decorator PointerEntered 150, 50",
            "decorator PointerMoved 150, 50",
            "decorator PointerExited 50, 50",
            "canvas PointerEntered 50, 50",
            "canvas PointerMoved 50, 50",
        ]
    );
}

#[test]
fn pointer_entered_exited_should_be_raised_in_correct_order() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let result = log();

    tree.root.host.mouse_move(&device, OVER_CANVAS);

    for control in [&tree.root.root, &tree.canvas, &tree.border, &tree.decorator] {
        for event in [InputElement::pointer_entered_event(), InputElement::pointer_exited_event()] {
            let recorded = result.clone();
            control.add_handler(event, move |s, e| {
                recorded.borrow_mut().push(format!("{} {}", label_of(s), e.routed_event().unwrap().name()));
            });
        }
    }

    tree.root.host.mouse_move(&device, OVER_DECORATOR);

    assert_eq!(
        *result.borrow(),
        ["canvas PointerExited", "decorator PointerEntered", "border PointerEntered"]
    );
}

#[test]
fn leave_window_should_reset_pointer_over() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let result = log();

    add_entered_exited_handlers(&result, &[&tree.root.root, &tree.panel, &tree.canvas]);

    tree.root.host.mouse_move(&device, OVER_CANVAS);
    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);

    tree.root.host.pointer(&device, RawPointerEventType::LeaveWindow, Point::new(-1.0, -1.0), 0);

    assert_eq!(pointer_over_states(&tree), [false, false, false, false]);
    assert!(tree.root.host.pointer_over_element().is_none());
    assert_eq!(
        *result.borrow(),
        [
            "canvas PointerEntered 50, 50",
            "panel PointerEntered 50, 50",
            "root PointerEntered 50, 50",
            "canvas PointerMoved 50, 50",
            "panel PointerMoved 50, 50",
            "root PointerMoved 50, 50",
            "canvas PointerExited 50, 50",
            "panel PointerExited 50, 50",
            "root PointerExited 50, 50",
        ]
    );
}

#[test]
fn disabled_element_should_set_pointer_over_on_visual_parent() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    tree.decorator.set_is_enabled(false);

    tree.root.host.mouse_move(&device, OVER_DECORATOR);

    assert_eq!(pointer_over_states(&tree), [false, true, false, true]);
}

#[test]
fn scene_invalidation_should_affect_pointer_over() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let pointer_over = tree.root.host.pointer_over.borrow().clone().unwrap();

    tree.root.host.mouse_move(&device, OVER_CANVAS);
    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);
    assert_eq!(pointer_over.last_position(), Some(PixelPoint::new(50, 50)));

    // The canvas is replaced by something else under a resting pointer.
    tree.canvas.set_is_visible(false);
    pointer_over.scene_invalidated(Rect::new(0.0, 0.0, 10000.0, 10000.0));

    assert_eq!(pointer_over_states(&tree), [false, false, false, true]);
    assert!(tree.panel.is_pointer_over());

    // A dirty rect that does not contain the pointer changes nothing.
    tree.canvas.set_is_visible(true);
    pointer_over.scene_invalidated(Rect::new(150.0, 150.0, 10.0, 10.0));
    assert!(!tree.canvas.is_pointer_over());
}

#[test]
fn capture_change_updates_pointer_over_through_input_root() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    tree.root.host.mouse_move(&device, OVER_CANVAS);
    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);

    // Capturing another element invalidates the pointer-over state: the
    // pointer rests over the canvas, which is not the capturing element.
    device.pointer().capture(Some(&tree.decorator.clone().upcast()));
    assert_eq!(pointer_over_states(&tree), [false, false, false, false]);
    assert_eq!(tree.root.host.cursor_element(), Some(tree.decorator.clone().upcast()));

    device.pointer().capture(None);
    assert_eq!(pointer_over_states(&tree), [false, false, true, true]);
}

#[test]
fn completing_input_clears_pointer_over() {
    let device = MouseDevice::new();
    let tree = pointer_tree();

    tree.root.host.mouse_move(&device, OVER_CANVAS);
    assert!(tree.canvas.is_pointer_over());

    tree.root.host.input_manager.dispose();

    assert_eq!(pointer_over_states(&tree), [false, false, false, false]);
}

// --- input manager ---------------------------------------------------------------

#[test]
fn input_manager_notifies_in_order() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let order = log();
    let manager = &tree.root.host.input_manager;

    struct Recorder(&'static str, Rc<RefCell<Vec<String>>>);

    impl crate::reactive::IObserver<Rc<dyn IRawInputEventArgs>> for Recorder {
        fn on_next(&self, value: Rc<dyn IRawInputEventArgs>) {
            self.1.borrow_mut().push(format!("{} {}", self.0, value.timestamp()));
        }
    }

    manager.pre_process().subscribe(Rc::new(Recorder("pre", order.clone())));
    manager.process().subscribe(Rc::new(Recorder("process", order.clone())));
    let post = manager.post_process().subscribe(Rc::new(Recorder("post", order.clone())));

    let recorded = order.clone();
    tree.root.root.pointer_moved(move |_, _| recorded.borrow_mut().push("device".to_string()));

    tree.root.host.pointer(&device, RawPointerEventType::Move, OVER_CANVAS, 7);
    post.dispose();
    tree.root.host.pointer(&device, RawPointerEventType::Move, OVER_CANVAS, 8);

    assert_eq!(*order.borrow(), ["pre 7", "device", "process 7", "post 7", "pre 8", "device", "process 8"]);
}

// --- gestures --------------------------------------------------------------------

/// Raises pointer events directly on elements, the way an input device
/// does, with full control over buttons and click counts.
struct MouseTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    pressed_buttons: Cell<RawInputModifiers>,
    pressed_button: Cell<MouseButton>,
}

impl MouseTestHelper {
    fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            next_stamp: Cell::new(1),
            pressed_buttons: Cell::new(RawInputModifiers::NONE),
            pressed_button: Cell::new(MouseButton::None),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn convert(mouse_button: MouseButton) -> RawInputModifiers {
        match mouse_button {
            MouseButton::Left => RawInputModifiers::LEFT_MOUSE_BUTTON,
            MouseButton::Right => RawInputModifiers::RIGHT_MOUSE_BUTTON,
            MouseButton::Middle => RawInputModifiers::MIDDLE_MOUSE_BUTTON,
            _ => RawInputModifiers::NONE,
        }
    }

    fn get_root(target: &Ref<TestControl>) -> Ref<Visual> {
        target.visual_root().unwrap_or_else(|| target.clone().upcast())
    }

    fn midpoint_relative_to_root(element: &Ref<TestControl>) -> Point {
        let root = Self::get_root(element);
        let bounds = element.bounds();
        element.translate_point(Point::new(bounds.width / 2.0, bounds.height / 2.0), &root).unwrap_or_default()
    }

    fn down(&self, target: &Ref<TestControl>, mouse_button: MouseButton, click_count: i32) {
        self.pressed_buttons.set(self.pressed_buttons.get() | Self::convert(mouse_button));
        let kind = match mouse_button {
            MouseButton::Left => PointerUpdateKind::LeftButtonPressed,
            MouseButton::Middle => PointerUpdateKind::MiddleButtonPressed,
            MouseButton::Right => PointerUpdateKind::RightButtonPressed,
            _ => PointerUpdateKind::Other,
        };
        let props = PointerPointProperties::new(self.pressed_buttons.get(), kind);

        self.pressed_button.set(mouse_button);
        self.pointer.capture(Some(&target.clone().upcast()));
        target.raise_event(&PointerPressedEventArgs::new(
            target,
            self.pointer.clone(),
            &Self::get_root(target),
            Self::midpoint_relative_to_root(target),
            self.timestamp(),
            props,
            KeyModifiers::NONE,
            click_count,
        ));
    }

    fn move_to(&self, target: &Ref<TestControl>, position: Point) {
        target.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            target,
            self.pointer.clone(),
            Some(&Self::get_root(target)),
            position,
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    fn up(&self, target: &Ref<TestControl>, mouse_button: MouseButton, position: Option<Point>) {
        let conv = Self::convert(mouse_button);
        self.pressed_buttons.set((self.pressed_buttons.get() | conv) ^ conv);
        let kind = match mouse_button {
            MouseButton::Left => PointerUpdateKind::LeftButtonReleased,
            MouseButton::Middle => PointerUpdateKind::MiddleButtonReleased,
            MouseButton::Right => PointerUpdateKind::RightButtonReleased,
            _ => PointerUpdateKind::Other,
        };
        let props = PointerPointProperties::new(self.pressed_buttons.get(), kind);

        target.raise_event(&PointerReleasedEventArgs::new(
            target,
            self.pointer.clone(),
            &Self::get_root(target),
            position.unwrap_or_else(|| Self::midpoint_relative_to_root(target)),
            self.timestamp(),
            props,
            KeyModifiers::NONE,
            self.pressed_button.get(),
        ));
        self.pointer.capture_lost(CaptureSource::Explicit);
    }

    fn click(&self, target: &Ref<TestControl>, button: MouseButton) {
        self.down(target, button, 1);
        let captured = self
            .pointer
            .captured()
            .and_then(|c| c.downcast::<TestControl>().ok())
            .unwrap_or_else(|| target.clone());
        self.up(&captured, button, None);
    }
}

struct GestureTree {
    root: TestRoot,
    border: Ref<TestControl>,
}

fn gesture_tree() -> GestureTree {
    let border = TestControl::new("border").at(0.0, 0.0, 100.0, 100.0);
    let root = TestRoot::with_child(&border);
    GestureTree { root, border }
}

fn add_gesture_handlers(tree: &GestureTree, result: &Rc<RefCell<Vec<String>>>, mark_handled: bool) {
    let push = |name: &'static str| {
        let result = result.clone();
        move || result.borrow_mut().push(name.to_string())
    };

    let dp = push("dp");
    tree.root.root.pointer_pressed(move |_, e| {
        dp();
        if mark_handled {
            e.set_handled(true);
        }
    });

    let dr = push("dr");
    tree.root.root.pointer_released(move |_, e| {
        dr();
        if mark_handled {
            e.set_handled(true);
        }
    });

    let bp = push("bp");
    tree.border.pointer_pressed(move |_, _| bp());
    let br = push("br");
    tree.border.pointer_released(move |_, _| br());

    let dt = push("dt");
    tree.root.root.tapped(move |_, _| dt());
    let ddt = push("ddt");
    tree.root.root.double_tapped(move |_, _| ddt());
    let bt = push("bt");
    tree.border.tapped(move |_, _| bt());
    let bdt = push("bdt");
    tree.border.double_tapped(move |_, _| bdt());
}

#[test]
fn tapped_should_follow_pointer_pressed_released() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let result = log();

    add_gesture_handlers(&tree, &result, false);

    mouse.click(&tree.border, MouseButton::Left);

    assert_eq!(*result.borrow(), ["bp", "dp", "br", "dr", "bt", "dt"]);
}

#[test]
fn tapped_should_be_raised_even_when_pressed_released_handled() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let result = log();

    add_gesture_handlers(&tree, &result, true);

    mouse.click(&tree.border, MouseButton::Left);

    assert_eq!(*result.borrow(), ["bp", "dp", "br", "dr", "bt", "dt"]);
}

fn raised_flag() -> (Rc<Cell<bool>>, impl Fn(&Interactive, &TappedEventArgs) + 'static) {
    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    (raised, move |_: &Interactive, _: &TappedEventArgs| flag.set(true))
}

#[test]
fn tapped_should_not_be_raised_for_middle_button() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.tapped(handler);

    mouse.click(&tree.border, MouseButton::Middle);

    assert!(!raised.get());
}

#[test]
fn tapped_should_not_be_raised_for_right_button() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.tapped(handler);

    mouse.click(&tree.border, MouseButton::Right);

    assert!(!raised.get());
}

#[test]
fn tapped_should_be_raised_from_captured_control() {
    let mouse = MouseTestHelper::new();
    let inner = TestControl::focusable("inner").at(0.0, 0.0, 100.0, 100.0);
    let border = TestControl::focusable("parent").at(0.0, 0.0, 100.0, 100.0).with_children(&[&inner]);
    let root = TestRoot::with_child(&border);
    let (raised, handler) = raised_flag();

    let capture_to: Ref<InputElement> = inner.clone().upcast();
    border.pointer_pressed(move |_, e| e.pointer().capture(Some(&capture_to)));

    mouse.click(&border, MouseButton::Left);

    root.root.tapped(handler);

    mouse.click(&border, MouseButton::Left);

    assert!(raised.get());
}

#[test]
fn right_tapped_should_be_raised_for_right_button() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.right_tapped(handler);

    mouse.click(&tree.border, MouseButton::Right);

    assert!(raised.get());
}

#[test]
fn double_tapped_should_follow_pointer_pressed_released_pressed() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let result = log();

    add_gesture_handlers(&tree, &result, false);

    mouse.click(&tree.border, MouseButton::Left);
    mouse.down(&tree.border, MouseButton::Left, 2);

    assert_eq!(*result.borrow(), ["bp", "dp", "br", "dr", "bt", "dt", "bp", "dp", "bdt", "ddt"]);
}

#[test]
fn double_tapped_should_be_raised_even_when_pressed_released_handled() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let result = log();

    add_gesture_handlers(&tree, &result, true);

    mouse.click(&tree.border, MouseButton::Left);
    mouse.down(&tree.border, MouseButton::Left, 2);

    assert_eq!(*result.borrow(), ["bp", "dp", "br", "dr", "bt", "dt", "bp", "dp", "bdt", "ddt"]);
}

#[test]
fn double_tapped_should_not_be_raised_for_middle_button() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.double_tapped(handler);

    mouse.click(&tree.border, MouseButton::Middle);
    mouse.down(&tree.border, MouseButton::Middle, 2);

    assert!(!raised.get());
}

#[test]
fn double_tapped_should_not_be_raised_for_right_button() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.double_tapped(handler);

    mouse.click(&tree.border, MouseButton::Right);
    mouse.down(&tree.border, MouseButton::Right, 2);

    assert!(!raised.get());
}

#[test]
fn tapped_should_not_be_raised_when_released_far_from_press() {
    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    let (raised, handler) = raised_flag();

    tree.root.root.tapped(handler);

    mouse.down(&tree.border, MouseButton::Left, 1);
    mouse.move_to(&tree.border, Point::new(80.0, 80.0));
    mouse.up(&tree.border, MouseButton::Left, Some(Point::new(80.0, 80.0)));

    assert!(!raised.get());
}

#[test]
fn tapped_is_synthesized_from_raw_mouse_input() {
    let device = MouseDevice::new();
    let tree = gesture_tree();
    let result = log();

    let recorded = result.clone();
    tree.border.tapped(move |s, e| {
        recorded.borrow_mut().push(format!("tapped {} {} {}", label_of(s), e.get_position(None), e.timestamp()));
    });
    let recorded = result.clone();
    tree.border.double_tapped(move |s, _| recorded.borrow_mut().push(format!("double {}", label_of(s))));

    let host = &tree.root.host;
    let position = Point::new(20.0, 30.0);
    host.pointer(&device, RawPointerEventType::LeftButtonDown, position, 100);
    host.pointer(&device, RawPointerEventType::LeftButtonUp, position, 150);
    host.pointer(&device, RawPointerEventType::LeftButtonDown, position, 200);
    host.pointer(&device, RawPointerEventType::LeftButtonUp, position, 250);

    assert_eq!(*result.borrow(), ["tapped border 20, 30 150", "double border"]);
}

fn force_fire(timer: &Rc<crate::threading::DispatcherTimer>) {
    timer.promote();
    timer.dispatcher().remove_timer(timer);
    timer.dispatcher().run_jobs(None);
}

#[test]
fn hold_should_be_raised_after_hold_duration() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = gesture_tree();
    InputElement::set_is_hold_with_mouse_enabled(&tree.border, true);
    let states = Rc::new(RefCell::new(Vec::new()));

    let recorded = states.clone();
    tree.root.root.holding(move |_, e| recorded.borrow_mut().push(e.holding_state()));
    let context = Rc::new(Cell::new(0));
    let counter = context.clone();
    tree.root.root.context_requested(move |_, e| {
        counter.set(counter.get() + 1);
        assert!(e.try_get_position(None).is_some());
    });

    mouse.down(&tree.border, MouseButton::Left, 1);
    assert!(states.borrow().is_empty());

    // Verify the timer duration, but execute it immediately.
    let timers = crate::threading::Dispatcher::snapshot_timers_for_unit_tests();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].interval(), std::time::Duration::from_millis(300));
    force_fire(&timers[0]);

    assert_eq!(*states.borrow(), [HoldingState::Started]);
    assert_eq!(context.get(), 1);

    mouse.up(&tree.border, MouseButton::Left, None);
    assert_eq!(*states.borrow(), [HoldingState::Started, HoldingState::Completed]);

    // A timer that fires after the pointer was released does nothing.
    mouse.down(&tree.border, MouseButton::Left, 1);
    mouse.up(&tree.border, MouseButton::Left, None);
    for timer in crate::threading::Dispatcher::snapshot_timers_for_unit_tests() {
        force_fire(&timer);
    }
    assert_eq!(states.borrow().len(), 2);
}

/// The tree of the hold tests of the reference suite: a border that raises
/// hold gestures for the mouse, in a root. The platform settings of the root
/// are the defaults (a hold after 300 milliseconds); the mouse helper presses
/// at the middle of the border, so the positions of the moves of the
/// reference tests (from a press at the origin) are offsets from there.
fn hold_tree() -> GestureTree {
    let tree = gesture_tree();
    InputElement::set_is_hold_with_mouse_enabled(&tree.border, true);
    tree
}

const HOLD_PRESS: Point = Point::new(50.0, 50.0);

/// The single timer of the dispatcher, with the interval of a hold verified.
fn single_hold_timer() -> Rc<crate::threading::DispatcherTimer> {
    let timers = crate::threading::Dispatcher::snapshot_timers_for_unit_tests();
    assert_eq!(timers.len(), 1);
    assert_eq!(timers[0].interval(), std::time::Duration::from_millis(300));
    timers[0].clone()
}

#[test]
fn hold_should_not_raised_when_pointer_released_before_timer() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = hold_tree();
    let raised = Rc::new(Cell::new(false));

    let flag = raised.clone();
    tree.root.root.holding(move |_, e| flag.set(e.holding_state() == HoldingState::Started));

    mouse.down(&tree.border, MouseButton::Left, 1);
    assert!(!raised.get());

    mouse.up(&tree.border, MouseButton::Left, None);
    assert!(!raised.get());

    // Verify timer duration, but execute it immediately.
    let timer = single_hold_timer();
    force_fire(&timer);

    assert!(!raised.get());
}

#[test]
fn hold_should_not_raised_when_pointer_is_moved_before_timer() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = hold_tree();
    let raised = Rc::new(Cell::new(false));

    let flag = raised.clone();
    tree.root.root.holding(move |_, e| flag.set(e.holding_state() == HoldingState::Completed));

    mouse.down(&tree.border, MouseButton::Left, 1);
    assert!(!raised.get());

    mouse.move_to(&tree.border, HOLD_PRESS + Vector::new(20.0, 20.0));
    assert!(!raised.get());

    // Verify timer duration, but execute it immediately.
    let timer = single_hold_timer();
    force_fire(&timer);

    assert!(!raised.get());
}

#[test]
fn hold_should_be_cancelled_when_second_contact_is_detected() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = hold_tree();
    let cancelled = Rc::new(Cell::new(false));

    let flag = cancelled.clone();
    tree.root.root.holding(move |_, e| flag.set(e.holding_state() == HoldingState::Canceled));

    mouse.down(&tree.border, MouseButton::Left, 1);
    assert!(!cancelled.get());

    let timer = single_hold_timer();
    force_fire(&timer);

    let second_mouse = MouseTestHelper::new();

    second_mouse.down(&tree.border, MouseButton::Left, 1);

    assert!(cancelled.get());
}

#[test]
fn hold_should_be_cancelled_when_pointer_moves_too_far() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = hold_tree();
    let cancelled = Rc::new(Cell::new(false));

    let flag = cancelled.clone();
    tree.root.root.holding(move |_, e| flag.set(e.holding_state() == HoldingState::Canceled));

    mouse.down(&tree.border, MouseButton::Left, 1);

    let timer = single_hold_timer();
    force_fire(&timer);

    mouse.move_to(&tree.border, HOLD_PRESS + Vector::new(3.0, 3.0));

    assert!(!cancelled.get());

    mouse.move_to(&tree.border, HOLD_PRESS + Vector::new(20.0, 20.0));

    assert!(cancelled.get());
}

#[test]
fn hold_should_not_be_raised_for_multiple_contacts() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();

    let mouse = MouseTestHelper::new();
    let tree = hold_tree();
    let raised = Rc::new(Cell::new(false));

    let flag = raised.clone();
    tree.root.root.holding(move |_, e| flag.set(e.holding_state() == HoldingState::Completed));

    let second_mouse = MouseTestHelper::new();

    mouse.down(&tree.border, MouseButton::Left, 1);

    // Verify timer duration, but execute it immediately.
    let timer = single_hold_timer();
    force_fire(&timer);

    second_mouse.down(&tree.border, MouseButton::Left, 1);

    assert!(!raised.get());
}

// --- hit testing -----------------------------------------------------------------

#[test]
fn input_hit_test_should_use_coordinates_relative_to_the_subtree_root() {
    let tree = pointer_tree();

    let result = tree.border.input_hit_test_with(Point::new(50.0, 50.0), false);

    assert_eq!(result, Some(tree.decorator.clone().upcast()));
}

#[test]
fn input_hit_test_respects_visibility_hit_test_visibility_and_enabled_state() {
    // The layout manager verifies that it is called on the UI thread.
    let _dispatcher = crate::threading::Dispatcher::unit_test_scope();
    let tree = pointer_tree();
    let root = &tree.root.root;

    assert_eq!(root.input_hit_test(OVER_DECORATOR), Some(tree.decorator.clone().upcast()));
    assert_eq!(
        root.get_input_elements_at(OVER_DECORATOR),
        [
            tree.decorator.clone().upcast::<InputElement>(),
            tree.border.clone().upcast(),
            tree.panel.clone().upcast(),
            root.clone().upcast()
        ]
    );

    tree.decorator.set_is_hit_test_visible(false);
    assert_eq!(root.input_hit_test(OVER_DECORATOR), Some(tree.border.clone().upcast()));
    tree.decorator.set_is_hit_test_visible(true);

    // A disabled element and its descendants are skipped unless asked for.
    tree.border.set_is_enabled(false);
    assert_eq!(root.input_hit_test(OVER_DECORATOR), Some(tree.panel.clone().upcast()));
    assert_eq!(root.input_hit_test_with(OVER_DECORATOR, false), Some(tree.decorator.clone().upcast()));
    tree.border.set_is_enabled(true);

    tree.border.set_is_visible(false);
    assert_eq!(root.input_hit_test(OVER_DECORATOR), Some(tree.panel.clone().upcast()));
    tree.border.set_is_visible(true);

    let panel = tree.panel.clone();
    assert_eq!(
        root.input_hit_test_filtered(OVER_DECORATOR, &move |v| !std::ptr::eq::<Visual>(v, Upcast::<Visual>::upcast(&*panel)), true),
        Some(root.clone().upcast())
    );

    assert!(root.input_hit_test(Point::new(500.0, 500.0)).is_none());
    assert!(TestControl::new("detached").input_hit_test(Point::default()).is_none());
}

#[test]
fn hit_test_respects_z_index_and_clip_to_bounds() {
    let lower = TestControl::new("lower").at(0.0, 0.0, 100.0, 100.0);
    let upper = TestControl::new("upper").at(0.0, 0.0, 100.0, 100.0);
    let overflow = TestControl::new("overflow").at(150.0, 0.0, 50.0, 50.0);
    let panel = TestControl::new("panel").at(0.0, 0.0, 100.0, 100.0).with_children(&[&lower, &upper, &overflow]);
    let root = TestRoot::with_child(&panel);
    let point = Point::new(10.0, 10.0);
    let hit = |p: Point| root.root.get_visual_at(p).map(|v| label_of(&v.cast::<Interactive>().unwrap()));

    // The last child is on top.
    assert_eq!(hit(point), Some("upper"));

    lower.set_z_index(1);
    assert_eq!(hit(point), Some("lower"));
    assert_eq!(
        root.root
            .get_visuals_at(point)
            .iter()
            .map(|v| label_of(&v.cast::<Interactive>().unwrap()))
            .collect::<Vec<_>>(),
        ["lower", "upper", "panel", "root"]
    );

    // A child outside of its parent's bounds is hit unless the parent clips.
    let outside = Point::new(160.0, 10.0);
    assert_eq!(hit(outside), Some("overflow"));

    panel.set_clip_to_bounds(true);
    assert_eq!(hit(outside), Some("root"));
}

// --- visual tree helpers -----------------------------------------------------------

#[test]
fn translate_point_and_transform_to_visual() {
    let tree = pointer_tree();
    let root = &tree.root.root;

    assert_eq!(tree.decorator.translate_point(Point::new(1.0, 2.0), root), Some(Point::new(101.0, 2.0)));
    assert_eq!(root.translate_point(Point::new(101.0, 2.0), &tree.decorator), Some(Point::new(1.0, 2.0)));
    assert_eq!(tree.decorator.translate_point(Point::new(1.0, 2.0), &tree.canvas), Some(Point::new(101.0, 2.0)));
    assert_eq!(
        tree.decorator.transform_to_visual(root),
        Some(Matrix::create_translation(100.0, 0.0))
    );
    assert!(tree.decorator.translate_point(Point::default(), &TestControl::new("other")).is_none());

    tree.root.host.screen_offset.set(PixelPoint::new(10, 20));
    assert_eq!(tree.decorator.point_to_screen(Point::new(1.0, 2.0)), PixelPoint::new(111, 22));
    assert_eq!(tree.decorator.point_to_client(PixelPoint::new(111, 22)), Point::new(1.0, 2.0));
}

#[test]
fn translate_point_respects_mirror_transform() {
    let tree = pointer_tree();

    tree.border.set_flow_direction(crate::media::FlowDirection::RightToLeft);
    assert!(tree.border.has_mirror_transform());
    assert!(!tree.decorator.has_mirror_transform());

    // The border is mirrored within its 100 wide bounds.
    assert_eq!(
        tree.decorator.translate_point(Point::new(10.0, 5.0), &tree.root.root),
        Some(Point::new(190.0, 5.0))
    );
    assert_eq!(
        tree.root.root.input_hit_test(Point::new(190.0, 5.0)),
        Some(tree.decorator.clone().upcast())
    );
}

#[test]
fn visual_tree_queries() {
    let tree = pointer_tree();
    let root = &tree.root.root;
    let labels = |visuals: Vec<Ref<Visual>>| {
        visuals.iter().map(|v| label_of(&v.cast::<Interactive>().unwrap())).collect::<Vec<_>>()
    };

    assert_eq!(labels(tree.decorator.get_visual_ancestors().collect()), ["border", "panel", "root"]);
    assert_eq!(
        labels(tree.decorator.get_self_and_visual_ancestors().collect()),
        ["decorator", "border", "panel", "root"]
    );
    assert_eq!(labels(root.get_visual_descendants().collect()), ["panel", "canvas", "border", "decorator"]);
    assert_eq!(
        labels(root.get_self_and_visual_descendants().collect()),
        ["root", "panel", "canvas", "border", "decorator"]
    );
    assert_eq!(labels(tree.panel.get_visual_children().to_vec()), ["canvas", "border"]);

    assert_eq!(tree.decorator.calculate_distance_from_root(), 3);
    assert_eq!(tree.decorator.calculate_distance_from_ancestor(Some(&tree.panel)), Some(2));
    assert_eq!(tree.decorator.calculate_distance_from_ancestor(Some(&tree.canvas)), None);
    assert_eq!(tree.decorator.calculate_distance_from_ancestor(None), None);

    assert_eq!(tree.decorator.find_common_visual_ancestor(&tree.canvas), Some(tree.panel.clone().upcast()));
    assert_eq!(tree.decorator.find_common_visual_ancestor(&tree.border), Some(tree.border.clone().upcast()));
    assert!(tree.decorator.find_common_visual_ancestor(&TestControl::new("other")).is_none());

    assert!(tree.panel.is_visual_ancestor_of(&tree.decorator));
    assert!(!tree.decorator.is_visual_ancestor_of(&tree.panel));
    assert!(!tree.panel.is_visual_ancestor_of(&tree.panel));

    assert_eq!(
        tree.decorator.find_ancestor_of_type::<TestControl>(false).map(|c| c.label),
        Some("border")
    );
    assert_eq!(
        tree.decorator.find_ancestor_of_type::<TestControl>(true).map(|c| c.label),
        Some("decorator")
    );
    assert_eq!(
        tree.decorator.find_ancestor_of_type_where::<TestControl>(false, |c| c.logical_root).map(|c| c.label),
        Some("root")
    );
    assert_eq!(root.find_descendant_of_type::<TestControl>(false).map(|c| c.label), Some("panel"));
    assert_eq!(
        root.find_descendant_of_type_where::<TestControl>(true, |c| c.label == "decorator").map(|c| c.label),
        Some("decorator")
    );
    assert!(tree.decorator.find_descendant_of_type::<TestControl>(false).is_none());

    assert_eq!(tree.decorator.get_visual_parent_of_type::<TestControl>().map(|c| c.label), Some("border"));
    assert!(tree.decorator.get_input_root().is_some());
    assert!(tree.decorator.get_platform_settings().is_some());
    assert!(TestControl::new("detached").get_input_root().is_none());
}

#[test]
fn transformed_bounds_include_clip() {
    let tree = pointer_tree();

    let bounds = tree.decorator.get_transformed_bounds().unwrap();
    assert_eq!(bounds.bounds, Rect::new(0.0, 0.0, 100.0, 200.0));
    assert_eq!(bounds.clip, Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(bounds.transform, Matrix::create_translation(100.0, 0.0));
    assert!(bounds.contains(Point::new(150.0, 10.0)));
    assert!(!bounds.contains(Point::new(50.0, 10.0)));

    tree.border.set_clip_to_bounds(true);
    assert_eq!(tree.decorator.get_transformed_bounds().unwrap().clip, Rect::new(100.0, 0.0, 100.0, 200.0));

    tree.border.set_is_visible(false);
    assert!(tree.decorator.get_transformed_bounds().is_none());
}

// --- raw input -------------------------------------------------------------------

#[test]
fn raw_args_hierarchy_supports_downcasting() {
    let device = MouseDevice::new();
    let root = TestRoot::new();
    let wheel: Rc<dyn IRawInputEventArgs> = Rc::new(RawMouseWheelEventArgs::new(
        device.clone(),
        5,
        root.host.as_input_root(),
        Point::new(1.0, 2.0),
        Vector::new(0.0, -1.0),
        RawInputModifiers::SHIFT,
    ));

    assert!(wheel.downcast_ref::<RawMouseWheelEventArgs>().is_some());
    let pointer = wheel.downcast_ref::<RawPointerEventArgs>().unwrap();
    assert_eq!(pointer.type_(), RawPointerEventType::Wheel);
    assert_eq!(pointer.position(), Point::new(1.0, 2.0));
    assert_eq!(pointer.point().pressure, 0.5);
    assert_eq!(pointer.point().contact_rect(), Rect::new(1.0, 2.0, 0.0, 0.0));
    assert!(wheel.downcast_ref::<RawInputEventArgs>().is_some());
    assert!(wheel.downcast_ref::<RawKeyEventArgs>().is_none());
    assert!(wheel.downcast_ref::<RawTouchEventArgs>().is_none());
    assert_eq!(wheel.timestamp(), 5);
    assert_eq!(pointer.input_modifiers().to_key_modifiers(), KeyModifiers::SHIFT);

    wheel.set_timestamp(6);
    pointer.set_position(Point::new(3.0, 4.0));
    assert_eq!(wheel.timestamp(), 6);
    assert_eq!(pointer.position(), Point::new(3.0, 4.0));
}

#[test]
fn pointer_point_properties_reflect_modifiers_and_update_kind() {
    let props = PointerPointProperties::new(
        RawInputModifiers::RIGHT_MOUSE_BUTTON | RawInputModifiers::PEN_ERASER,
        PointerUpdateKind::LeftButtonPressed,
    );
    assert!(props.is_left_button_pressed);
    assert!(props.is_right_button_pressed);
    assert!(props.is_eraser);
    assert!(!props.is_middle_button_pressed);
    assert_eq!(props.pressure, 0.5);

    // The platform may report the previous state: the kind wins.
    let released =
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonReleased);
    assert!(!released.is_left_button_pressed);

    assert_eq!(PointerUpdateKind::XButton2Released.get_mouse_button(), MouseButton::XButton2);
    assert_eq!(PointerUpdateKind::Other.get_mouse_button(), MouseButton::None);
    assert_eq!(RawPointerEventType::TouchBegin.to_update_kind(), PointerUpdateKind::LeftButtonPressed);
    assert_eq!(RawPointerEventType::Move.to_update_kind(), PointerUpdateKind::Other);
    assert_eq!(PointerPointProperties::default(), PointerPointProperties::NONE);
}

#[test]
fn key_modifiers_parse() {
    assert_eq!(KeyModifiers::parse("control"), Ok(KeyModifiers::CONTROL));
    assert_eq!(KeyModifiers::parse("Alt, Shift"), Ok(KeyModifiers::ALT | KeyModifiers::SHIFT));
    assert_eq!(KeyModifiers::parse("None"), Ok(KeyModifiers::NONE));
    assert_eq!(KeyModifiers::parse("8"), Ok(KeyModifiers::META));
    assert!(KeyModifiers::parse("Hyper").is_err());
    assert!(KeyModifiers::parse("64").is_err());
}

#[test]
fn swipe_and_scroll_args() {
    let swipe = SwipeGestureEventArgs::new(1, Vector::new(-10.0, 2.0), Vector::new(1.0, 1.0));
    assert_eq!(swipe.swipe_direction(), SwipeDirection::Right);
    assert_eq!(
        SwipeGestureEventArgs::new(1, Vector::new(1.0, 5.0), Vector::default()).swipe_direction(),
        SwipeDirection::Up
    );
    assert_eq!(swipe.routed_event(), Some(InputElement::swipe_gesture_event().as_routed_event()));

    let scroll = ScrollGestureEventArgs::new(ScrollGestureEventArgs::get_next_free_id(), Vector::new(1.0, 0.0));
    scroll.set_should_end_scroll_gesture(true);
    assert!(scroll.should_end_scroll_gesture());

    let pinch = PinchEventArgs::new(2.0, Point::new(1.0, 1.0));
    assert_eq!((pinch.scale(), pinch.angle(), pinch.angle_delta()), (2.0, 0.0, 0.0));

    let args = RoutedEventArgs::with_event(InputElement::context_canceled_event());
    assert_eq!(args.routed_event().unwrap().to_string(), "InputElement.ContextCanceled");
}

// --- tab navigation ----------------------------------------------------------------

fn button(label: &'static str) -> Ref<TestControl> {
    TestControl::focusable(label)
}

fn stack(children: &[&Ref<TestControl>]) -> Ref<TestControl> {
    TestControl::new("stack").with_children(children)
}

fn stack_with(mode: KeyboardNavigationMode, children: &[&Ref<TestControl>]) -> Ref<TestControl> {
    let result = stack(children);
    KeyboardNavigation::set_tab_navigation(&result, mode);
    result
}

fn element(control: &Ref<TestControl>) -> Ref<InputElement> {
    control.clone().upcast()
}

fn next_of(current: &Ref<TestControl>, direction: NavigationDirection) -> Option<&'static str> {
    KeyboardNavigationHandler::get_next(&element(current), direction)
        .map(|e| e.downcast_ref::<TestControl>().map_or("?", |c| c.label))
}

/// `top(Cycle) -> (stack(b1, b2, b3), stack(b4, b5, b6))`
struct TwoStacks {
    _top: Ref<TestControl>,
    first: Ref<TestControl>,
    second: Ref<TestControl>,
    b: [Ref<TestControl>; 6],
}

fn two_stacks(first_mode: Option<KeyboardNavigationMode>, second_mode: Option<KeyboardNavigationMode>) -> TwoStacks {
    let b = [button("b1"), button("b2"), button("b3"), button("b4"), button("b5"), button("b6")];
    let first = stack(&[&b[0], &b[1], &b[2]]);
    let second = stack(&[&b[3], &b[4], &b[5]]);
    if let Some(mode) = first_mode {
        KeyboardNavigation::set_tab_navigation(&first, mode);
    }
    if let Some(mode) = second_mode {
        KeyboardNavigation::set_tab_navigation(&second, mode);
    }
    let top = stack_with(KeyboardNavigationMode::Cycle, &[&first, &second]);
    TwoStacks { _top: top, first, second, b }
}

use NavigationDirection::{Next, Previous};

#[test]
fn next_continue_returns_next_control_in_container() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[1], Next), Some("b3"));
}

#[test]
fn next_continue_returns_first_control_in_next_sibling_container() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[2], Next), Some("b4"));
}

#[test]
fn next_skips_unfocusable_siblings() {
    let (b1, b2, current, next, b5, b6) =
        (button("b1"), button("b2"), button("b3"), button("b4"), button("b5"), button("b6"));
    let text_block = TestControl::new("text");
    let inner = stack(&[&current]);
    let first = stack(&[&b1, &b2, &inner, &text_block, &next]);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&first, &b5, &b6]);

    assert_eq!(next_of(&current, Next), Some("b4"));
}

#[test]
fn next_continue_doesnt_enter_panel_with_tab_navigation_none() {
    let t = two_stacks(None, None);
    // The second container holds its buttons in a nested stack and does not
    // take part in tab navigation.
    KeyboardNavigation::set_tab_navigation(&t.second, KeyboardNavigationMode::None);

    assert_eq!(next_of(&t.b[2], Next), Some("b1"));
}

#[test]
fn next_continue_returns_next_sibling() {
    let (b1, current, next) = (button("b1"), button("b2"), button("b3"));
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&b1, &current, &next]);

    assert_eq!(next_of(&current, Next), Some("b3"));
}

#[test]
fn next_skips_non_tab_stop_siblings() {
    let (b1, current, skipped, next) = (button("b1"), button("b2"), button("b3"), button("b4"));
    skipped.set_is_tab_stop(false);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&b1, &current, &skipped, &next]);

    assert_eq!(next_of(&current, Next), Some("b4"));
}

#[test]
fn next_continue_returns_first_control_in_next_uncle_container() {
    let (b1, b2, current, next, b5, b6) =
        (button("b1"), button("b2"), button("b3"), button("b4"), button("b5"), button("b6"));
    let inner = stack(&[&b1, &b2, &current]);
    let first = stack(&[&inner]);
    let second = stack(&[&next, &b5, &b6]);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&first, &second]);

    assert_eq!(next_of(&current, Next), Some("b4"));
}

#[test]
fn next_continue_returns_child_of_top_level() {
    let next = button("b1");
    let top = stack_with(KeyboardNavigationMode::Cycle, &[&next]);

    assert_eq!(next_of(&top, Next), Some("b1"));
}

#[test]
fn next_continue_wraps() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[5], Next), Some("b1"));
}

#[test]
fn next_cycle_returns_next_control_in_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::Cycle), None);
    assert_eq!(next_of(&t.b[1], Next), Some("b3"));
}

#[test]
fn next_cycle_wraps_to_first() {
    let t = two_stacks(Some(KeyboardNavigationMode::Cycle), None);
    assert_eq!(next_of(&t.b[2], Next), Some("b1"));
}

#[test]
fn next_contained_returns_next_control_in_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::Contained), None);
    assert_eq!(next_of(&t.b[1], Next), Some("b3"));
}

#[test]
fn next_contained_stops_at_end() {
    let t = two_stacks(Some(KeyboardNavigationMode::Contained), None);
    assert_eq!(next_of(&t.b[2], Next), None);
}

#[test]
fn next_once_moves_to_next_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::Once), None);
    assert_eq!(next_of(&t.b[1], Next), Some("b4"));
}

#[test]
fn next_once_moves_to_active_element() {
    let t = two_stacks(Some(KeyboardNavigationMode::Once), None);
    KeyboardNavigation::set_tab_once_active_element(&t.first, Some(element(&t.b[1])));

    assert_eq!(next_of(&t.b[5], Next), Some("b2"));
}

#[test]
fn next_none_moves_to_next_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::None), None);
    assert_eq!(next_of(&t.b[1], Next), Some("b4"));
}

#[test]
fn next_none_skips_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::None), None);
    assert_eq!(next_of(&t.b[5], Next), Some("b4"));
}

#[test]
fn previous_continue_returns_previous_control_in_container() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[2], Previous), Some("b2"));
}

#[test]
fn previous_continue_returns_last_control_in_previous_sibling_container() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[3], Previous), Some("b3"));
}

#[test]
fn previous_continue_returns_last_child_of_sibling() {
    let (b1, b2, next, current) = (button("b1"), button("b2"), button("b3"), button("b4"));
    let first = stack(&[&b1, &b2, &next]);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&first, &current]);

    assert_eq!(next_of(&current, Previous), Some("b3"));
}

#[test]
fn previous_continue_returns_last_control_in_previous_nephew_container() {
    let (b1, b2, next, current, b5, b6) =
        (button("b1"), button("b2"), button("b3"), button("b4"), button("b5"), button("b6"));
    let inner = stack(&[&b1, &b2, &next]);
    let first = stack(&[&inner]);
    let second = stack(&[&current, &b5, &b6]);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&first, &second]);

    assert_eq!(next_of(&current, Previous), Some("b3"));
}

#[test]
fn previous_continue_wraps() {
    let t = two_stacks(None, None);
    assert_eq!(next_of(&t.b[0], Previous), Some("b6"));
}

#[test]
fn previous_continue_returns_parent() {
    let current = button("b1");
    let top = TestControl::focusable("top").with_children(&[&current]);
    KeyboardNavigation::set_tab_navigation(&top, KeyboardNavigationMode::Continue);

    assert_eq!(next_of(&current, Previous), Some("top"));
}

#[test]
fn previous_cycle_returns_previous_control_in_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::Cycle), None);
    assert_eq!(next_of(&t.b[1], Previous), Some("b1"));
}

#[test]
fn previous_cycle_wraps_to_last() {
    let t = two_stacks(Some(KeyboardNavigationMode::Cycle), None);
    assert_eq!(next_of(&t.b[0], Previous), Some("b3"));
}

#[test]
fn previous_contained_returns_previous_control_in_container() {
    let t = two_stacks(Some(KeyboardNavigationMode::Contained), None);
    assert_eq!(next_of(&t.b[1], Previous), Some("b1"));
}

#[test]
fn previous_contained_stops_at_beginning() {
    let t = two_stacks(Some(KeyboardNavigationMode::Contained), None);
    assert_eq!(next_of(&t.b[0], Previous), None);
}

#[test]
fn previous_once_moves_to_previous_container() {
    let t = two_stacks(None, Some(KeyboardNavigationMode::Once));
    assert_eq!(next_of(&t.b[4], Previous), Some("b3"));
}

#[test]
fn previous_once_moves_to_active_element() {
    let t = two_stacks(Some(KeyboardNavigationMode::Once), None);
    KeyboardNavigation::set_tab_once_active_element(&t.first, Some(element(&t.b[1])));

    assert_eq!(next_of(&t.b[3], Previous), Some("b2"));
}

#[test]
fn previous_once_moves_to_first_element() {
    let t = two_stacks(Some(KeyboardNavigationMode::Once), None);
    assert_eq!(next_of(&t.b[3], Previous), Some("b1"));
}

#[test]
fn respects_tab_index_moving_forwards() {
    let t = two_stacks(None, None);
    for (button, index) in t.b.iter().zip([5, 2, 3, 6, 1, 4]) {
        button.set_tab_index(index);
    }

    let mut order = Vec::new();
    let mut current = t.b[4].clone();
    for _ in 0..6 {
        order.push(current.label);
        let next = KeyboardNavigationHandler::get_next(&element(&current), Next).unwrap();
        current = next.downcast::<TestControl>().ok().unwrap();
    }

    assert_eq!(order, ["b5", "b2", "b3", "b6", "b1", "b4"]);
    assert_eq!(current.label, "b5");
}

#[test]
fn respects_tab_index_moving_backwards() {
    let t = two_stacks(None, None);
    for (button, index) in t.b.iter().zip([5, 2, 3, 6, 1, 4]) {
        button.set_tab_index(index);
    }

    let mut order = Vec::new();
    let mut current = t.b[4].clone();
    for _ in 0..6 {
        order.push(current.label);
        let next = KeyboardNavigationHandler::get_next(&element(&current), Previous).unwrap();
        current = next.downcast::<TestControl>().ok().unwrap();
    }

    assert_eq!(order, ["b5", "b4", "b1", "b6", "b3", "b2"]);
}

#[test]
fn cannot_focus_child_of_disabled_control() {
    let (current, hidden, next) = (button("b1"), button("b2"), button("b3"));
    let disabled = stack(&[&hidden]);
    disabled.set_is_enabled(false);
    let _top = stack_with(KeyboardNavigationMode::Cycle, &[&current, &disabled, &next]);

    assert_eq!(next_of(&current, Next), Some("b3"));
    assert_eq!(next_of(&next, Previous), Some("b1"));
}

#[test]
#[should_panic(expected = "not supported")]
fn get_next_rejects_unsupported_directions() {
    let current = button("b1");
    let _top = stack(&[&current]);
    KeyboardNavigationHandler::get_next(&element(&current), NavigationDirection::PageDown);
}

// --- custom navigation -----------------------------------------------------------

fn custom_stack(children: &[&Ref<TestControl>], next: Option<&Ref<TestControl>>) -> Ref<TestControl> {
    let result = stack(children);
    *result.custom_navigation.borrow_mut() = Some((true, next.map(element)));
    result
}

#[test]
fn tab_should_custom_navigate_within_children() {
    let (current, b2, next) = (button("b1"), button("b2"), button("b3"));
    let _target = custom_stack(&[&current, &b2, &next], Some(&next));

    assert_eq!(next_of(&current, Next), Some("b3"));
}

#[test]
fn right_should_custom_navigate_within_children() {
    let (current, b2, next) = (button("b1"), button("b2"), button("b3"));
    let _target = custom_stack(&[&current, &b2, &next], Some(&next));

    assert_eq!(next_of(&current, NavigationDirection::Right), Some("b3"));
}

#[test]
fn tab_should_custom_navigate_from_outside() {
    let (b1, b2, next, current) = (button("b1"), button("b2"), button("b3"), button("outside"));
    let target = custom_stack(&[&b1, &b2, &next], Some(&next));
    let _root = stack(&[&current, &target]);

    assert_eq!(next_of(&current, Next), Some("b3"));
}

#[test]
fn tab_should_custom_navigate_from_outside_when_wrapping() {
    let (b1, b2, next, current) = (button("b1"), button("b2"), button("b3"), button("outside"));
    let target = custom_stack(&[&b1, &b2, &next], Some(&next));
    let _root = stack_with(KeyboardNavigationMode::Cycle, &[&target, &current]);

    assert_eq!(next_of(&current, Next), Some("b3"));
}

#[test]
fn shift_tab_should_custom_navigate_from_outside() {
    let (b1, b2, next, current) = (button("b1"), button("b2"), button("b3"), button("outside"));
    let target = custom_stack(&[&b1, &b2, &next], Some(&next));
    let _root = stack_with(KeyboardNavigationMode::Cycle, &[&current, &target]);

    assert_eq!(next_of(&current, Previous), Some("b3"));
}

#[test]
fn shift_tab_should_navigate_outside_when_null_returned_as_next() {
    let (b1, current, b3, next) = (button("b1"), button("b2"), button("b3"), button("outside"));
    let target = custom_stack(&[&b1, &current, &b3], None);
    let _root = stack_with(KeyboardNavigationMode::Cycle, &[&target, &next]);

    assert_eq!(next_of(&current, Previous), Some("outside"));
}

#[test]
fn tab_should_navigate_outside_when_null_returned_as_next() {
    let (b1, current, b3, next) = (button("b1"), button("b2"), button("b3"), button("outside"));
    let target = custom_stack(&[&b1, &current, &b3], None);
    let _root = stack_with(KeyboardNavigationMode::Cycle, &[&target, &next]);

    assert_eq!(next_of(&current, Next), Some("outside"));
}

// --- XY navigation -----------------------------------------------------------------

//  111
//         2
// 3
//
//   4
struct XYLayout {
    _root: TestRoot,
    buttons: [Ref<TestControl>; 4],
}

fn xy_layout() -> XYLayout {
    let buttons = [
        button("1").at(50.0, 0.0, 150.0, 150.0),
        button("2").at(400.0, 150.0, 50.0, 50.0),
        button("3").at(0.0, 200.0, 50.0, 50.0),
        button("4").at(100.0, 300.0, 50.0, 50.0),
    ];
    let canvas = TestControl::new("canvas")
        .at(150.0, 0.0, 500.0, 600.0)
        .with_children(&[&buttons[0], &buttons[1], &buttons[2], &buttons[3]]);
    let root = TestControl::create("root", true, true).at(0.0, 0.0, 800.0, 600.0);
    XYFocus::set_navigation_modes(&root, XYFocusNavigationModes::ENABLED);
    let host = TestHost::new(&root);
    root.add_child(&canvas);
    XYLayout { _root: TestRoot { root, host }, buttons }
}

fn xy_check(strategy: XYFocusNavigationStrategy, cases: &[(usize, NavigationDirection, i32)]) {
    use NavigationDirection::*;
    let _ = (Down, Up, Left, Right);

    for (from, direction, to) in cases {
        let layout = xy_layout();
        let from_button = &layout.buttons[from - 1];
        XYFocus::set_up_navigation_strategy(from_button, strategy);
        XYFocus::set_left_navigation_strategy(from_button, strategy);
        XYFocus::set_right_navigation_strategy(from_button, strategy);
        XYFocus::set_down_navigation_strategy(from_button, strategy);

        let result = KeyboardNavigationHandler::get_next(&element(from_button), *direction);
        let index = match result {
            None => -1,
            Some(result) => {
                layout.buttons.iter().position(|b| element(b) == result).map_or(0, |i| i as i32 + 1)
            }
        };

        assert_eq!(index, *to, "{strategy:?}: from {from} {direction:?}");
    }
}

#[test]
fn projection_focus_depending_on_direction() {
    use NavigationDirection::*;
    xy_check(
        XYFocusNavigationStrategy::Projection,
        &[
            (1, Down, 4),
            (1, Up, -1),
            (1, Left, -1),
            (1, Right, 2),
            (2, Left, 1),
            (2, Right, -1),
            (3, Down, 4),
            (3, Left, -1),
            (4, Down, -1),
            (4, Up, 1),
            (4, Left, 3),
            (4, Right, 2),
        ],
    );
}

#[test]
fn rectilinear_distance_focus_depending_on_direction() {
    use NavigationDirection::*;
    xy_check(
        XYFocusNavigationStrategy::RectilinearDistance,
        &[
            (1, Down, 3),
            (1, Up, -1),
            (1, Left, 3),
            (1, Right, 2),
            (2, Down, 3),
            (2, Up, 1),
            (2, Left, 1),
            (2, Right, -1),
            (3, Down, 4),
            (3, Up, 1),
            (3, Left, -1),
            (3, Right, 1),
            (4, Down, -1),
            (4, Up, 3),
            (4, Left, 3),
            (4, Right, 2),
        ],
    );
}

#[test]
fn navigation_direction_distance_focus_depending_on_direction() {
    use NavigationDirection::*;
    xy_check(
        XYFocusNavigationStrategy::NavigationDirectionDistance,
        &[
            (1, Down, 2),
            (1, Up, -1),
            (1, Left, 3),
            (1, Right, 2),
            (2, Down, 3),
            (2, Up, 1),
            (2, Left, 1),
            (2, Right, -1),
            (3, Down, 4),
            (3, Up, 2),
            (3, Left, -1),
            (3, Right, 1),
            (4, Down, -1),
            (4, Up, 3),
            (4, Left, 3),
            (4, Right, 2),
        ],
    );
}

#[test]
fn uses_xy_directional_overrides() {
    let (left, right, up, down, center) = (button("left"), button("right"), button("up"), button("down"), button("center"));
    XYFocus::set_left(&center, Some(element(&left)));
    XYFocus::set_right(&center, Some(element(&right)));
    XYFocus::set_up(&center, Some(element(&up)));
    XYFocus::set_down(&center, Some(element(&down)));
    let canvas = TestControl::new("canvas").at(0.0, 0.0, 200.0, 200.0).with_children(&[&left, &right, &up, &down, &center]);
    let root = TestRoot::with_child(&canvas);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::ENABLED);

    assert_eq!(next_of(&center, NavigationDirection::Left), Some("left"));
    assert_eq!(next_of(&center, NavigationDirection::Right), Some("right"));
    assert_eq!(next_of(&center, NavigationDirection::Up), Some("up"));
    assert_eq!(next_of(&center, NavigationDirection::Down), Some("down"));
}

#[test]
fn xy_directional_override_discarded_if_not_part_of_the_same_root() {
    let left = button("left");
    let center = button("center").at(0.0, 0.0, 50.0, 50.0);
    XYFocus::set_left(&center, Some(element(&left)));
    let root = TestRoot::with_child(&center);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::ENABLED);

    assert_eq!(next_of(&center, NavigationDirection::Left), None);
}

#[test]
fn parent_can_override_navigation_when_directional_is_set() {
    // [ [ EXPECTED, CURRENT ] CANDIDATE ]: normally focus would go from
    // current to candidate, but the nested panel overrides `Right`.
    let expected_override = button("expected").at(0.0, 0.0, 50.0, 50.0);
    let current = button("current").at(50.0, 0.0, 50.0, 50.0);
    let candidate = button("candidate").at(100.0, 0.0, 50.0, 50.0);
    let parent = TestControl::new("parent").at(0.0, 0.0, 100.0, 50.0).with_children(&[&expected_override, &current]);
    XYFocus::set_right(&parent, Some(element(&expected_override)));
    XYFocus::set_right_navigation_strategy(&parent, XYFocusNavigationStrategy::RectilinearDistance);
    let outer = TestControl::new("outer").at(0.0, 0.0, 200.0, 200.0).with_children(&[&parent, &candidate]);
    let root = TestRoot::with_child(&outer);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::ENABLED);

    assert_eq!(next_of(&current, NavigationDirection::Right), Some("expected"));
}

#[test]
fn cannot_focus_across_xy_focus_boundaries() {
    let current = button("current").at(0.0, 0.0, 50.0, 50.0);
    let inside = button("inside").at(0.0, 60.0, 50.0, 50.0);
    let outside = button("outside").at(0.0, 120.0, 50.0, 50.0);
    let enabled = TestControl::new("enabled").at(0.0, 0.0, 50.0, 110.0).with_children(&[&current, &inside]);
    XYFocus::set_navigation_modes(&enabled, XYFocusNavigationModes::ENABLED);
    let outer = TestControl::new("outer").at(0.0, 0.0, 200.0, 200.0).with_children(&[&enabled, &outside]);
    let root = TestRoot::with_child(&outer);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::DISABLED);

    assert_eq!(next_of(&current, NavigationDirection::Down), Some("inside"));
    assert_eq!(next_of(&inside, NavigationDirection::Down), None);
    assert_eq!(next_of(&outside, NavigationDirection::Up), None);
}

#[test]
fn xy_focus_skips_effectively_disabled_controls() {
    let current = button("current").at(0.0, 0.0, 50.0, 50.0);
    let disabled_child = button("disabled").at(0.0, 0.0, 50.0, 50.0);
    let disabled = TestControl::new("panel").at(0.0, 60.0, 50.0, 50.0).with_children(&[&disabled_child]);
    disabled.set_is_enabled(false);
    let next = button("next").at(0.0, 120.0, 50.0, 50.0);
    let outer = TestControl::new("outer").at(0.0, 0.0, 200.0, 200.0).with_children(&[&current, &disabled, &next]);
    let root = TestRoot::with_child(&outer);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::ENABLED);

    assert_eq!(next_of(&current, NavigationDirection::Down), Some("next"));
}

// --- keyboard navigation handler --------------------------------------------------

fn key_down(keyboard: &Rc<KeyboardDevice>, root: &TestRoot, key: Key, modifiers: RawInputModifiers) -> bool {
    let args = RawKeyEventArgs::new(
        keyboard.clone(),
        0,
        root.host.as_input_root(),
        RawKeyEventType::KeyDown,
        key,
        modifiers,
        PhysicalKey::None,
        None,
        KeyDeviceType::Keyboard,
    );
    keyboard.process_raw_event(&args);
    args.handled()
}

#[test]
fn tab_key_moves_focus_through_the_handler() {
    let keyboard = real_focus();
    let (b1, b2) = (button("b1"), button("b2"));
    let root = TestRoot::with_child(&stack(&[&b1, &b2]));
    let handler = KeyboardNavigationHandler::new();
    handler.set_owner(&element(&root.root));

    b1.focus();
    assert!(key_down(&keyboard, &root, Key::Tab, RawInputModifiers::NONE));
    assert!(b2.is_focused());
    assert!(b2.has_class(":focus-visible"));

    assert!(key_down(&keyboard, &root, Key::Tab, RawInputModifiers::SHIFT));
    assert!(b1.is_focused());
}

#[test]
fn arrow_key_should_focus_element() {
    let keyboard = real_focus();
    let current = button("current").at(0.0, 0.0, 50.0, 50.0);
    let below = button("below").at(0.0, 60.0, 50.0, 50.0);
    let outer = TestControl::new("outer").at(0.0, 0.0, 200.0, 200.0).with_children(&[&current, &below]);
    let root = TestRoot::with_child(&outer);
    XYFocus::set_navigation_modes(&root.root, XYFocusNavigationModes::ENABLED);
    let handler = KeyboardNavigationHandler::new();
    handler.set_owner(&element(&root.root));

    current.focus();
    assert!(key_down(&keyboard, &root, Key::Down, RawInputModifiers::NONE));
    assert!(below.is_focused());

    // Nothing further down: the key is not handled.
    assert!(!key_down(&keyboard, &root, Key::Down, RawInputModifiers::NONE));
    assert!(below.is_focused());
}

#[test]
#[should_panic(expected = "owner has already been set")]
fn keyboard_navigation_handler_owner_can_only_be_set_once() {
    let root = TestRoot::new();
    let handler = KeyboardNavigationHandler::new();
    handler.set_owner(&element(&root.root));
    handler.set_owner(&element(&root.root));
}

// --- focus manager navigation ----------------------------------------------------

// Not from upstream; upstream's navigation tests are in `input_element_focus_tests.rs`
// of `ferroui-controls`.

/// A root whose content root is set, for the navigation entry points.
fn focus_tree_root() -> TestRoot {
    let root = TestRoot::with_child(&stack(&[&button("1")]));
    root.focus_manager().set_content_root(Some(element(&root.root)));
    root
}

#[test]
#[should_panic(expected = "directions are supported")]
fn try_move_focus_rejects_unsupported_directions() {
    let root = focus_tree_root();
    root.focus_manager().try_move_focus(NavigationDirection::First, None);
}

#[test]
fn focus_scope_hops_to_the_host_of_a_hosted_visual_tree_root() {
    // Covered through `as_hosted_visual_tree_root`: an element that is not a
    // hosted root keeps the default.
    let control = TestControl::new("control");
    assert!(control.as_hosted_visual_tree_root().is_none());
    assert!(control.as_scrollable().is_none());
    assert!(control.as_custom_keyboard_navigation().is_none());
}

#[test]
fn navigation_direction_helpers() {
    assert!(Next.is_tab() && Previous.is_tab() && !NavigationDirection::Left.is_tab());
    assert!(NavigationDirection::First.is_directional() && NavigationDirection::PageDown.is_directional());
    assert!(!Next.is_directional());
    assert_eq!(Key::Tab.to_navigation_direction(KeyModifiers::NONE), Some(Next));
    assert_eq!(Key::Tab.to_navigation_direction(KeyModifiers::SHIFT), Some(Previous));
    assert_eq!(Key::Home.to_navigation_direction(KeyModifiers::NONE), Some(NavigationDirection::First));
    assert_eq!(Key::A.to_navigation_direction(KeyModifiers::NONE), None);
}

// --- key bindings ------------------------------------------------------------------

struct TestCommand {
    executed: Cell<i32>,
    can_execute: Cell<bool>,
    on_execute: RefCell<Option<Box<dyn Fn()>>>,
}

impl TestCommand {
    fn new() -> Rc<Self> {
        Rc::new(Self { executed: Cell::new(0), can_execute: Cell::new(true), on_execute: RefCell::new(None) })
    }
}

impl ICommand for TestCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        self.can_execute.get()
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        self.executed.set(self.executed.get() + 1);
        if let Some(action) = &*self.on_execute.borrow() {
            action();
        }
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn crate::reactive::IDisposable> {
        crate::reactive::Disposable::empty()
    }
}

fn key_binding(command: &Rc<TestCommand>, gesture: KeyGesture) -> Ref<KeyBinding> {
    let binding = KeyBinding::new();
    binding.set_command(Some(command.clone()));
    binding.set_gesture(Some(gesture));
    binding
}

#[test]
fn can_change_key_bindings_in_keybinding_event_handler() {
    let target = KeyboardDevice::new();
    let button = button("button");
    let root = TestRoot::with_child(&button);
    let command = TestCommand::new();

    let cleared = button.clone();
    *command.on_execute.borrow_mut() = Some(Box::new(move || cleared.key_bindings().clear()));
    button.key_bindings().add(key_binding(&command, KeyGesture::new(Key::O, KeyModifiers::CONTROL)));

    target.set_focused_element(Some(&element(&button)), NavigationMethod::Pointer, KeyModifiers::NONE);

    let handled = key_down(&target, &root, Key::O, RawInputModifiers::CONTROL);

    assert_eq!(command.executed.get(), 1);
    assert!(handled);
    assert!(button.key_bindings().is_empty());
}

#[test]
fn key_bindings_of_ancestors_are_consulted_and_respect_can_execute() {
    let target = KeyboardDevice::new();
    let button = button("button");
    let root = TestRoot::with_child(&button);
    let command = TestCommand::new();
    let key_downs = Rc::new(Cell::new(0));

    root.root.key_bindings().add(key_binding(&command, KeyGesture::from_key(Key::F5)));
    let counter = key_downs.clone();
    root.root.add_handler_with(
        InputElement::key_down_event(),
        move |_, _| counter.set(counter.get() + 1),
        RoutingStrategies::BUBBLE,
        true,
    );
    target.set_focused_element(Some(&element(&button)), NavigationMethod::Pointer, KeyModifiers::NONE);

    // Not matching: nothing happens.
    assert!(!key_down(&target, &root, Key::F6, RawInputModifiers::NONE));
    assert_eq!(command.executed.get(), 0);

    assert!(key_down(&target, &root, Key::F5, RawInputModifiers::NONE));
    assert_eq!(command.executed.get(), 1);
    // The key down event is still raised, already handled.
    assert_eq!(key_downs.get(), 2);

    command.can_execute.set(false);
    assert!(!key_down(&target, &root, Key::F5, RawInputModifiers::NONE));
    assert_eq!(command.executed.get(), 1);
}

// --- gesture recognizers -----------------------------------------------------------

#[repr(C)]
struct TestGestureRecognizer {
    base: gesture_recognizers::GestureRecognizer,
    log: RefCell<Vec<&'static str>>,
    capture_on_move: Cell<bool>,
}

ferro_class!(TestGestureRecognizer: GestureRecognizer);
ferro_impl_classes!(TestGestureRecognizer: FerroObjectImpl, StyledElementImpl);

use gesture_recognizers::{GestureRecognizer, GestureRecognizerImpl};

impl GestureRecognizerImpl for TestGestureRecognizer {
    fn pointer_pressed(this: &Self, _e: &PointerPressedEventArgs) {
        this.log.borrow_mut().push("pressed");
    }

    fn pointer_released(this: &Self, _e: &PointerReleasedEventArgs) {
        this.log.borrow_mut().push("released");
    }

    fn pointer_moved(this: &Self, e: &PointerEventArgs) {
        this.log.borrow_mut().push("moved");
        if this.capture_on_move.get() {
            this.capture(e.pointer());
        }
    }

    fn pointer_capture_lost(this: &Self, _pointer: &Rc<dyn IPointer>) {
        this.log.borrow_mut().push("lost");
    }
}

impl TestGestureRecognizer {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: GestureRecognizer::construct(),
            log: RefCell::new(Vec::new()),
            capture_on_move: Cell::new(false),
        })
    }
}

fn recording_pointer() -> (Rc<Pointer>, Rc<RefCell<Vec<Option<&'static str>>>>) {
    let captures = Rc::new(RefCell::new(Vec::new()));
    let recorded = captures.clone();
    let pointer = Pointer::with_platform_capture(Pointer::get_next_free_id(), PointerType::Mouse, true, move |e| {
        recorded.borrow_mut().push(e.map(|e| e.downcast_ref::<TestControl>().map_or("?", |c| c.label)))
    });
    (pointer, captures)
}

#[test]
fn gesture_recognizer_capture_should_keep_platform_capture_on_same_target() {
    let (pointer, platform_captures) = recording_pointer();
    let target = TestControl::new("target");
    let recognizer = TestGestureRecognizer::new();
    target.gesture_recognizers().add(&recognizer);

    pointer.capture_with_source(Some(&element(&target)), CaptureSource::Implicit);
    recognizer.capture(&(pointer.clone() as Rc<dyn IPointer>));

    assert!(pointer.captured().is_none());
    assert_eq!(pointer.captured_gesture_recognizer(), Some(recognizer.clone().upcast()));
    assert_eq!(*platform_captures.borrow(), [Some("target")]);
}

#[test]
fn gesture_recognizer_capture_should_move_platform_capture_to_target() {
    let (pointer, platform_captures) = recording_pointer();
    let initial_capture = TestControl::new("initial");
    let target = TestControl::new("target").with_children(&[&initial_capture]);
    let recognizer = TestGestureRecognizer::new();
    target.gesture_recognizers().add(&recognizer);

    pointer.capture_with_source(Some(&element(&initial_capture)), CaptureSource::Implicit);
    recognizer.capture(&(pointer.clone() as Rc<dyn IPointer>));

    assert!(pointer.captured().is_none());
    assert_eq!(pointer.captured_gesture_recognizer(), Some(recognizer.clone().upcast()));
    assert_eq!(*platform_captures.borrow(), [Some("initial"), Some("target")]);
}

#[test]
fn gesture_recognizers_receive_pointer_events_and_capture() {
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let recognizer = TestGestureRecognizer::new();
    tree.canvas.gesture_recognizers().add(&recognizer);

    assert_eq!(tree.canvas.gesture_recognizers().count(), 1);
    assert_eq!(recognizer.target(), Some(element(&tree.canvas)));
    assert_eq!(recognizer.parent(), Some(tree.canvas.clone().upcast()));

    let moves = Rc::new(Cell::new(0));
    let counter = moves.clone();
    tree.canvas.pointer_moved(move |_, _| counter.set(counter.get() + 1));

    let host = &tree.root.host;
    host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 1);
    host.pointer(&device, RawPointerEventType::Move, OVER_CANVAS, 2);
    assert_eq!(*recognizer.log.borrow(), ["pressed", "moved"]);
    assert_eq!(moves.get(), 1);

    // Once the recognizer captures the pointer, moves and the release go to
    // it directly and no longer to the element.
    recognizer.capture_on_move.set(true);
    host.pointer(&device, RawPointerEventType::Move, OVER_CANVAS, 3);
    assert_eq!(moves.get(), 2);
    assert!(device.pointer().captured_gesture_recognizer().is_some());
    assert!(device.pointer().is_gesture_recognition_skipped());

    host.pointer(&device, RawPointerEventType::Move, OVER_DECORATOR, 4);
    assert_eq!(moves.get(), 2);
    host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_DECORATOR, 5);

    assert_eq!(*recognizer.log.borrow(), ["pressed", "moved", "moved", "moved", "released", "lost"]);
    assert!(device.pointer().captured_gesture_recognizer().is_none());
    assert!(!device.pointer().is_gesture_recognition_skipped());

    assert!(tree.canvas.gesture_recognizers().remove(&recognizer));
    assert!(recognizer.target().is_none());
    assert!(recognizer.parent().is_none());
    assert!(!tree.canvas.gesture_recognizers().remove(&recognizer));
}

// --- touch and pen -----------------------------------------------------------------

fn touch(
    host: &TestHost,
    device: &Rc<TouchDevice>,
    type_: RawPointerEventType,
    position: Point,
    timestamp: u64,
    id: i64,
) {
    host.input(Rc::new(RawTouchEventArgs::new(
        device.clone(),
        timestamp,
        host.as_input_root(),
        type_,
        position,
        RawInputModifiers::NONE,
        id,
    )));
}

fn tap_once(host: &TestHost, device: &Rc<TouchDevice>, timestamp: u64, id: i64) {
    touch(host, device, RawPointerEventType::TouchBegin, Point::new(10.0, 10.0), timestamp, id);
    touch(host, device, RawPointerEventType::TouchEnd, Point::new(10.0, 10.0), timestamp, id);
}

fn tap_counters(root: &TestRoot) -> (Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let tapped = Rc::new(Cell::new(0));
    let double_tapped = Rc::new(Cell::new(0));
    let counter = tapped.clone();
    root.root.tapped(move |_, _| counter.set(counter.get() + 1));
    let counter = double_tapped.clone();
    root.root.double_tapped(move |_, _| counter.set(counter.get() + 1));
    (tapped, double_tapped)
}

#[test]
fn tapped_event_is_fired_with_touch() {
    let root = TestRoot::new();
    let touch_device = TouchDevice::new();
    let (tapped, _) = tap_counters(&root);

    tap_once(&root.host, &touch_device, 0, 0);

    assert_eq!(tapped.get(), 1);
}

#[test]
fn double_tapped_event_is_fired_with_touch() {
    let root = TestRoot::new();
    let touch_device = TouchDevice::new();
    let (tapped, double_tapped) = tap_counters(&root);

    tap_once(&root.host, &touch_device, 0, 0);
    tap_once(&root.host, &touch_device, 0, 1);

    assert_eq!(tapped.get(), 1);
    assert_eq!(double_tapped.get(), 1);
}

#[test]
fn pointer_pressed_counts_clicks_correctly() {
    for click_count in 1..=5 {
        let root = TestRoot::new();
        let touch_device = TouchDevice::new();
        let clicks = Rc::new(RefCell::new(Vec::new()));
        let recorded = clicks.clone();
        root.root.pointer_pressed(move |_, e| recorded.borrow_mut().push(e.click_count()));

        for i in 0..click_count {
            tap_once(&root.host, &touch_device, 0, i);
        }

        assert_eq!(clicks.borrow().len() as i64, click_count);
        assert_eq!(*clicks.borrow().last().unwrap() as i64, click_count);
    }
}

#[test]
fn double_tapped_not_fired_when_click_too_late() {
    let root = TestRoot::new();
    let touch_device = TouchDevice::new();
    let (tapped, double_tapped) = tap_counters(&root);

    tap_once(&root.host, &touch_device, 0, 0);
    tap_once(&root.host, &touch_device, 501, 1);

    assert_eq!(tapped.get(), 2);
    assert_eq!(double_tapped.get(), 0);
}

#[test]
fn double_tapped_not_fired_when_second_click_is_from_different_touch_contact() {
    let root = TestRoot::new();
    let touch_device = TouchDevice::new();
    let (tapped, double_tapped) = tap_counters(&root);
    let position = Point::new(10.0, 10.0);

    touch(&root.host, &touch_device, RawPointerEventType::TouchBegin, position, 0, 0);
    touch(&root.host, &touch_device, RawPointerEventType::TouchBegin, position, 0, 1);
    touch(&root.host, &touch_device, RawPointerEventType::TouchEnd, position, 0, 0);
    touch(&root.host, &touch_device, RawPointerEventType::TouchEnd, position, 0, 1);

    assert_eq!(tapped.get(), 2);
    assert_eq!(double_tapped.get(), 0);
}

#[test]
fn touch_move_should_not_set_is_pointer_over_and_cancel_disposes_pointer() {
    let tree = pointer_tree();
    let touch_device = TouchDevice::new();
    let moves = Rc::new(Cell::new(0));
    let counter = moves.clone();
    tree.canvas.pointer_moved(move |_, e| {
        assert_eq!(e.pointer().type_(), PointerType::Touch);
        counter.set(counter.get() + 1)
    });

    touch(&tree.root.host, &touch_device, RawPointerEventType::TouchBegin, OVER_CANVAS, 0, 7);
    touch(&tree.root.host, &touch_device, RawPointerEventType::TouchUpdate, OVER_DECORATOR, 1, 7);

    // The touch is implicitly captured by the element it started on.
    assert_eq!(moves.get(), 1);
    assert_eq!(pointer_over_states(&tree), [false, false, false, false]);

    let lost = Rc::new(Cell::new(0));
    let counter = lost.clone();
    tree.canvas.pointer_capture_lost(move |_, _| counter.set(counter.get() + 1));
    touch(&tree.root.host, &touch_device, RawPointerEventType::TouchCancel, OVER_DECORATOR, 2, 7);
    assert_eq!(lost.get(), 1);

    touch_device.dispose();
    touch(&tree.root.host, &touch_device, RawPointerEventType::TouchBegin, OVER_CANVAS, 3, 8);
    assert_eq!(moves.get(), 1);
}

#[test]
fn pen_input_raises_pointer_events_with_pen_state() {
    let tree = pointer_tree();
    let pen = PenDevice::new(true);
    let events = log();

    for event in ["pressed", "moved", "released"] {
        let recorded = events.clone();
        let handler = move |e: &PointerEventArgs| {
            recorded.borrow_mut().push(format!(
                "{event} {:?} {} {}",
                e.pointer().type_(),
                e.properties().pressure,
                e.properties().is_left_button_pressed
            ));
        };
        match event {
            "pressed" => {
                tree.canvas.pointer_pressed(move |_, e| handler(e));
            }
            "moved" => {
                tree.canvas.pointer_moved(move |_, e| handler(e));
            }
            _ => {
                tree.canvas.pointer_released(move |_, e| handler(e));
            }
        }
    }

    let send = |type_: RawPointerEventType, modifiers: RawInputModifiers, position: Point| {
        let mut point = RawPointerPoint::new();
        point.position = position;
        point.pressure = 0.75;
        let args = Rc::new(RawPointerEventArgs::with_point(
            pen.clone(),
            0,
            tree.root.host.as_input_root(),
            type_,
            point,
            modifiers,
        ));
        args.set_raw_pointer_id(3);
        tree.root.host.input(args.clone());
        args
    };

    send(RawPointerEventType::LeftButtonDown, RawInputModifiers::LEFT_MOUSE_BUTTON, OVER_CANVAS);
    let move_args = send(RawPointerEventType::Move, RawInputModifiers::LEFT_MOUSE_BUTTON, OVER_DECORATOR);
    assert!(pen.try_get_pointer(&move_args).is_some());
    send(RawPointerEventType::LeftButtonUp, RawInputModifiers::NONE, OVER_DECORATOR);

    assert_eq!(*events.borrow(), ["pressed Pen 0.75 true", "moved Pen 0.75 true", "released Pen 0.75 false"]);
    // The pointer is released on pen up.
    assert!(pen.try_get_pointer(&move_args).is_none());
}

#[test]
fn platform_hotkey_configuration_defaults() {
    let config = super::platform::PlatformHotkeyConfiguration::default();
    assert_eq!(config.copy, [KeyGesture::new(Key::C, KeyModifiers::CONTROL), KeyGesture::new(Key::Insert, KeyModifiers::CONTROL)]);
    assert_eq!(config.redo[1], KeyGesture::new(Key::Z, KeyModifiers::CONTROL | KeyModifiers::SHIFT));

    let mac = super::platform::PlatformHotkeyConfiguration::with_modifiers(KeyModifiers::META, KeyModifiers::SHIFT, KeyModifiers::ALT);
    assert_eq!(mac.select_all, [KeyGesture::new(Key::A, KeyModifiers::META)]);
    assert_eq!(mac.whole_word_text_action_modifiers, KeyModifiers::ALT);
}

// --- access keys -------------------------------------------------------------------

/// Makes focusable test controls answer the access key pressed query the
/// way a button does: they are the target of their own access key.
fn access_key_targets() {
    thread_local! {
        static DONE: Cell<bool> = const { Cell::new(false) };
    }
    if DONE.replace(true) {
        return;
    }
    AccessKeyHandler::access_key_pressed_event().add_class_handler::<TestControl>(|sender, e| {
        if e.handled() || e.target().is_some() || !sender.focusable() {
            return;
        }
        e.set_target(Some(sender.to_ref().upcast()));
        e.set_handled(true);
    });
}

fn raise_key(target: &Ref<TestControl>, down: bool, key: Key, key_symbol: Option<&str>, modifiers: KeyModifiers) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(if down { InputElement::key_down_event() } else { InputElement::key_up_event() }));
    args.key = key;
    args.key_symbol = key_symbol.map(str::to_string);
    args.key_modifiers = modifiers;
    target.raise_event(&args);
}

fn key_events(root: &TestRoot) -> Rc<RefCell<Vec<String>>> {
    let events = log();
    let recorded = events.clone();
    root.root.key_down(move |_, e| recorded.borrow_mut().push(format!("KeyDown {}", e.key)));
    let recorded = events.clone();
    root.root.key_up(move |_, e| recorded.borrow_mut().push(format!("KeyUp {}", e.key)));
    events
}

fn alt_a_sequence(root: &Ref<TestControl>) {
    raise_key(root, true, Key::LeftAlt, None, KeyModifiers::NONE);
    raise_key(root, true, Key::A, Some("a"), KeyModifiers::ALT);
    raise_key(root, false, Key::A, Some("a"), KeyModifiers::ALT);
    raise_key(root, false, Key::LeftAlt, None, KeyModifiers::NONE);
}

struct FakeMenu {
    is_open: Cell<bool>,
    times_open_called: Cell<i32>,
    times_close_called: Cell<i32>,
    closed: crate::utilities::HandlerList<dyn Fn(&RoutedEventArgs)>,
}

impl FakeMenu {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            is_open: Cell::new(false),
            times_open_called: Cell::new(0),
            times_close_called: Cell::new(0),
            closed: crate::utilities::HandlerList::new(),
        })
    }
}

impl IMainMenu for FakeMenu {
    fn is_open(&self) -> bool {
        self.is_open.get()
    }

    fn close(&self) {
        self.is_open.set(false);
        self.times_close_called.set(self.times_close_called.get() + 1);
        for (_, handler) in self.closed.snapshot().iter() {
            handler(&RoutedEventArgs::new());
        }
    }

    fn open(&self) {
        self.is_open.set(true);
        self.times_open_called.set(self.times_open_called.get() + 1);
    }

    fn closed(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn crate::reactive::IDisposable> {
        self.closed.add(handler);
        crate::reactive::Disposable::empty()
    }
}

#[test]
fn should_raise_key_events_for_unregistered_access_key() {
    for with_menu in [false, true] {
        let root = TestRoot::new();
        let target = AccessKeyHandler::new();
        target.set_owner(&element(&root.root));
        if with_menu {
            target.set_main_menu(Some(FakeMenu::new()));
        }
        let events = key_events(&root);

        alt_a_sequence(&root.root);

        assert_eq!(*events.borrow(), ["KeyDown LeftAlt", "KeyDown A", "KeyUp A", "KeyUp LeftAlt"]);
    }
}

#[test]
fn should_raise_key_events_for_alt_key_with_main_menu() {
    let root = TestRoot::new();
    let target = AccessKeyHandler::new();
    let menu = FakeMenu::new();
    target.set_owner(&element(&root.root));
    target.set_main_menu(Some(menu.clone()));
    let events = key_events(&root);

    for _ in 0..2 {
        raise_key(&root.root, true, Key::LeftAlt, None, KeyModifiers::NONE);
        raise_key(&root.root, false, Key::LeftAlt, None, KeyModifiers::NONE);
    }

    assert_eq!(*events.borrow(), ["KeyDown LeftAlt", "KeyUp LeftAlt", "KeyDown LeftAlt", "KeyUp LeftAlt"]);
    // The first Alt opened the menu, the second one closed it.
    assert_eq!(menu.times_open_called.get(), 1);
    assert_eq!(menu.times_close_called.get(), 1);
    assert!(!root.root.get_value(AccessKeyHandler::show_access_key_property()));
}

#[test]
fn should_raise_key_events_for_registered_access_key() {
    access_key_targets();
    let button = button("button");
    let root = TestRoot::with_child(&button);
    let target = AccessKeyHandler::new();
    target.set_owner(&element(&root.root));
    target.register("A", &element(&button));
    let events = key_events(&root);

    alt_a_sequence(&root.root);

    // The handler marks the Alt+A key down as handled once it matches a
    // registered access key, so a plain key down subscriber doesn't see it.
    assert_eq!(*events.borrow(), ["KeyDown LeftAlt", "KeyUp A", "KeyUp LeftAlt"]);
}

fn access_key_raised(
    registered: &str,
    key: Key,
    key_symbol: &str,
    root_enabled: bool,
    focus: Option<&'static str>,
    raise_on_button: bool,
) -> i32 {
    access_key_targets();
    let keyboard = real_focus();
    let button = button("button");
    let root = TestRoot::with_child(&button);
    root.root.set_is_enabled(root_enabled);
    let target = AccessKeyHandler::new();
    let raised = Rc::new(Cell::new(0));

    match focus {
        Some("button") => {
            keyboard.set_focused_element(Some(&element(&button)), NavigationMethod::Unspecified, KeyModifiers::NONE)
        }
        Some(_) => {
            keyboard.set_focused_element(Some(&element(&root.root)), NavigationMethod::Unspecified, KeyModifiers::NONE)
        }
        None => assert!(keyboard.focused_element().is_none()),
    }

    target.set_owner(&element(&root.root));
    target.register(registered, &element(&button));

    let counter = raised.clone();
    button.add_handler(AccessKeyHandler::access_key_event(), move |_, e| {
        assert!(!e.is_multiple());
        counter.set(counter.get() + 1)
    });

    let raise_on = if raise_on_button { &button } else { &root.root };
    raise_key(raise_on, true, Key::LeftAlt, None, KeyModifiers::NONE);
    assert_eq!(raised.get(), 0);
    raise_key(raise_on, true, key, Some(key_symbol), KeyModifiers::ALT);
    let after_down = raised.get();
    raise_key(raise_on, false, key, Some(key_symbol), KeyModifiers::ALT);
    raise_key(raise_on, false, Key::LeftAlt, None, KeyModifiers::NONE);
    assert_eq!(raised.get(), after_down);

    after_down
}

#[test]
fn should_raise_access_key_for_registered_access_key_matching_key_symbol() {
    for (registered, key, key_symbol) in
        [("A", Key::A, "a"), ("A", Key::Q, "a"), ("\u{e9}", Key::D2, "\u{e9}"), ("2", Key::D2, "2")]
    {
        assert_eq!(access_key_raised(registered, key, key_symbol, true, Some("button"), false), 1, "{registered}");
    }
}

#[test]
fn should_not_raise_access_key_for_registered_access_key_not_matching_key_symbol() {
    assert_eq!(access_key_raised("A", Key::A, "q", true, Some("button"), false), 0);
}

#[test]
fn should_raise_access_key_for_registered_access_key_when_effectively_enabled() {
    assert_eq!(access_key_raised("A", Key::A, "a", false, Some("button"), false), 0);
    assert_eq!(access_key_raised("A", Key::A, "a", true, Some("button"), false), 1);
}

#[test]
fn should_raise_access_key_whatever_is_focused() {
    assert_eq!(access_key_raised("A", Key::A, "a", true, None, false), 1);
    assert_eq!(access_key_raised("A", Key::A, "a", true, Some("root"), false), 1);
    assert_eq!(access_key_raised("A", Key::A, "a", true, Some("button"), true), 1);
}

#[test]
fn access_key_focuses_the_target_by_default() {
    access_key_targets();
    let _keyboard = real_focus();
    let button = button("button");
    let root = TestRoot::with_child(&button);
    let target = AccessKeyHandler::new();
    target.set_owner(&element(&root.root));
    target.register("f", &element(&button));

    assert!(target.process_key(Some("F"), None));
    assert!(button.is_focused());
    assert!(button.has_class(":focus-visible"));
    assert!(!target.process_key(Some("x"), None));
    assert!(!target.process_key(None, None));

    target.unregister(&button);
    assert_eq!(target.registration_count(), 0);
    assert!(!target.process_key(Some("F"), None));
}

#[test]
fn should_open_main_menu_on_alt_key_up() {
    for focus_menu in [true, false] {
        let keyboard = real_focus();
        let target = AccessKeyHandler::new();
        let menu = FakeMenu::new();
        let menu_element = button("menu");
        let root = TestRoot::with_child(&menu_element);

        if focus_menu {
            keyboard.set_focused_element(Some(&element(&menu_element)), NavigationMethod::Unspecified, KeyModifiers::NONE);
        }

        target.set_owner(&element(&root.root));
        target.set_main_menu(Some(menu.clone()));

        raise_key(&root.root, true, Key::LeftAlt, None, KeyModifiers::NONE);
        assert_eq!(menu.times_open_called.get(), 0);
        assert!(root.root.get_value(AccessKeyHandler::show_access_key_property()));
        assert!(menu_element.get_value(AccessKeyHandler::show_access_key_property()));

        raise_key(&root.root, false, Key::LeftAlt, None, KeyModifiers::NONE);
        assert_eq!(menu.times_open_called.get(), 1);
    }
}

#[test]
fn access_key_markers_are_hidden_on_pointer_press_and_other_keys_cancel_the_menu() {
    let mouse = MouseTestHelper::new();
    let target = AccessKeyHandler::new();
    let menu = FakeMenu::new();
    let child = button("child").at(0.0, 0.0, 10.0, 10.0);
    let root = TestRoot::with_child(&child);
    target.set_owner(&element(&root.root));
    target.set_main_menu(Some(menu.clone()));

    raise_key(&root.root, true, Key::LeftAlt, None, KeyModifiers::NONE);
    assert!(root.root.get_value(AccessKeyHandler::show_access_key_property()));

    mouse.down(&child, MouseButton::Left, 1);
    assert!(!root.root.get_value(AccessKeyHandler::show_access_key_property()));

    // Another key while Alt is down: releasing Alt does not open the menu.
    raise_key(&root.root, true, Key::Tab, None, KeyModifiers::ALT);
    raise_key(&root.root, false, Key::LeftAlt, None, KeyModifiers::NONE);
    assert_eq!(menu.times_open_called.get(), 0);
}

// --- concrete gesture recognizers --------------------------------------------------

use gesture_recognizers::{
    PinchGestureRecognizer, PullGestureRecognizer, ScrollGestureRecognizer, SwipeGestureRecognizer,
};

/// Raises touch pointer events directly on elements.
struct TouchTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    click_count: Cell<i32>,
}

impl TouchTestHelper {
    fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true),
            next_stamp: Cell::new(1),
            click_count: Cell::new(0),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn as_pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    fn down(&self, target: &Ref<TestControl>, position: Point) {
        self.pointer.capture(Some(&element(target)));
        self.click_count.set(self.click_count.get() + 1);
        target.raise_event(&PointerPressedEventArgs::new(
            target,
            self.as_pointer(),
            target,
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            self.click_count.get(),
        ));
    }

    fn move_to(&self, target: &Ref<TestControl>, position: Point) {
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            target,
            self.as_pointer(),
            Some(target),
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        );
        match self.pointer.captured_gesture_recognizer() {
            Some(recognizer) => recognizer.pointer_moved_internal(&e),
            None => target.raise_event(&e),
        }
    }

    fn up(&self, target: &Ref<TestControl>, position: Point) {
        let e = PointerReleasedEventArgs::new(
            target,
            self.as_pointer(),
            target,
            position,
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        match self.pointer.captured_gesture_recognizer() {
            Some(recognizer) => recognizer.pointer_released_internal(&e),
            None => target.raise_event(&e),
        }
        self.cancel();
    }

    fn cancel(&self) {
        self.pointer.capture_lost(CaptureSource::Platform);
    }
}

fn recognizer_target(recognizer: impl IntoRef<GestureRecognizer>) -> (Ref<TestControl>, TestRoot) {
    let border = TestControl::new("border").at(0.0, 0.0, 100.0, 100.0);
    border.gesture_recognizers().add(recognizer);
    let root = TestRoot::with_child(&border);
    (border, root)
}

fn swipe_recognizer(horizontal: bool, threshold: f64, mouse: bool) -> Ref<SwipeGestureRecognizer> {
    let recognizer = SwipeGestureRecognizer::new();
    recognizer.set_can_horizontally_swipe(horizontal);
    recognizer.set_threshold(threshold);
    recognizer.set_is_mouse_enabled(mouse);
    recognizer
}

#[test]
fn defaults_disable_both_axes() {
    let recognizer = SwipeGestureRecognizer::new();

    assert!(!recognizer.can_horizontally_swipe());
    assert!(!recognizer.can_vertically_swipe());
}

#[test]
fn does_not_raise_swipe_when_both_axes_are_disabled() {
    let recognizer = SwipeGestureRecognizer::new();
    assert!(!recognizer.can_horizontally_swipe());
    assert!(!recognizer.can_vertically_swipe());
    recognizer.set_threshold(1.0);

    let (border, root) = recognizer_target(&recognizer);
    let touch = TouchTestHelper::new();
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    root.root.swipe_gesture(move |_, _| counter.set(counter.get() + 1));
    let counter = raised.clone();
    root.root.swipe_gesture_ended(move |_, _| counter.set(counter.get() + 1));

    touch.down(&border, Point::new(50.0, 50.0));
    touch.move_to(&border, Point::new(20.0, 20.0));
    touch.up(&border, Point::new(20.0, 20.0));

    assert_eq!(raised.get(), 0);
}

#[test]
fn starts_only_after_threshold_is_exceeded() {
    let (border, root) = recognizer_target(swipe_recognizer(true, 50.0, false));
    let touch = TouchTestHelper::new();
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let recorded = deltas.clone();
    root.root.swipe_gesture(move |_, e| recorded.borrow_mut().push((e.delta(), e.swipe_direction())));

    touch.down(&border, Point::new(5.0, 5.0));
    touch.move_to(&border, Point::new(40.0, 5.0));
    assert!(deltas.borrow().is_empty());

    touch.move_to(&border, Point::new(80.0, 5.0));
    // The threshold is taken off the first delta.
    assert_eq!(*deltas.borrow(), [(Vector::new(-25.0, 0.0), SwipeDirection::Right)]);
    assert!(touch.pointer.captured_gesture_recognizer().is_some());
}

#[test]
fn ended_event_uses_same_id_and_last_velocity() {
    let (border, root) = recognizer_target(swipe_recognizer(true, 1.0, false));
    let touch = TouchTestHelper::new();
    let updates = Rc::new(RefCell::new(Vec::new()));
    let ended = Rc::new(RefCell::new(None));

    let recorded = updates.clone();
    root.root.swipe_gesture(move |_, e| recorded.borrow_mut().push((e.id(), e.velocity())));
    let recorded = ended.clone();
    root.root.swipe_gesture_ended(move |_, e| *recorded.borrow_mut() = Some((e.id(), e.velocity())));

    touch.down(&border, Point::new(50.0, 50.0));
    touch.move_to(&border, Point::new(40.0, 50.0));
    touch.move_to(&border, Point::new(30.0, 50.0));
    touch.up(&border, Point::new(30.0, 50.0));

    let updates = updates.borrow();
    assert!(updates.len() >= 2);
    assert!(updates.iter().all(|(id, _)| *id == updates[0].0));
    assert_eq!(*ended.borrow(), Some(*updates.last().unwrap()));
    // 10 pixels in one millisecond of platform time, smoothed by half.
    assert_eq!(updates.last().unwrap().1, Vector::new(5000.0, 0.0));
}

fn mouse_swipe_raised(enabled: bool) -> bool {
    let mouse = MouseTestHelper::new();
    let (border, root) = recognizer_target(swipe_recognizer(true, 1.0, enabled));
    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    root.root.swipe_gesture(move |_, _| flag.set(true));

    mouse.down(&border, MouseButton::Left, 1);
    mouse.move_to(&border, Point::new(30.0, 50.0));
    mouse.up(&border, MouseButton::Left, Some(Point::new(30.0, 50.0)));

    raised.get()
}

#[test]
fn mouse_swipe_is_raised_when_enabled() {
    assert!(mouse_swipe_raised(true));
}

// Not from the reference tests.
#[test]
fn mouse_swipe_is_not_raised_when_disabled() {
    assert!(!mouse_swipe_raised(false));
}

fn pinch_raised(cancel_first: bool, same_pointer: bool) -> bool {
    let (border, root) = recognizer_target(PinchGestureRecognizer::new());
    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    root.root.pinch(move |_, e| {
        assert!(e.scale() > 1.0);
        flag.set(true)
    });

    let first_touch = TouchTestHelper::new();
    let second_touch = TouchTestHelper::new();
    let second = if same_pointer { &first_touch } else { &second_touch };

    first_touch.down(&border, Point::new(5.0, 5.0));
    if cancel_first {
        first_touch.cancel();
    }
    second.down(&border, Point::new(10.0, 10.0));
    second.move_to(&border, Point::new(20.0, 20.0));

    raised.get()
}

#[test]
fn pinched_should_be_raised_for_two_pointers_moving() {
    assert!(pinch_raised(false, false));
}

#[test]
fn pinched_should_not_be_raised_for_same_pointer() {
    assert!(!pinch_raised(false, true));
}

#[test]
fn gestures_should_be_cancelled_when_pointer_capture_is_lost() {
    assert!(!pinch_raised(true, false));
}

#[test]
fn pinch_ended_is_raised_when_a_contact_is_released() {
    let (border, root) = recognizer_target(PinchGestureRecognizer::new());
    let ended = Rc::new(Cell::new(0));
    let counter = ended.clone();
    root.root.pinch_ended(move |_, _| counter.set(counter.get() + 1));

    let (first_touch, second_touch) = (TouchTestHelper::new(), TouchTestHelper::new());
    first_touch.down(&border, Point::new(5.0, 5.0));
    second_touch.down(&border, Point::new(10.0, 10.0));
    second_touch.up(&border, Point::new(10.0, 10.0));

    assert!(ended.get() >= 1);
}

#[test]
fn scrolling_should_start_after_start_distance_is_exceeded() {
    let recognizer = ScrollGestureRecognizer::new();
    recognizer.set_can_horizontally_scroll(true);
    recognizer.set_can_vertically_scroll(true);
    recognizer.set_scroll_start_distance(50);

    let (border, root) = recognizer_target(&recognizer);
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let ended = Rc::new(Cell::new(0));
    let recorded = deltas.clone();
    root.root.scroll_gesture(move |_, e| {
        recorded.borrow_mut().push(e.delta());
        e.set_handled(true);
    });
    let counter = ended.clone();
    root.root.scroll_gesture_ended(move |_, _| counter.set(counter.get() + 1));

    let first_touch = TouchTestHelper::new();
    first_touch.down(&border, Point::new(5.0, 5.0));
    first_touch.move_to(&border, Point::new(20.0, 20.0));
    assert!(deltas.borrow().is_empty());

    first_touch.move_to(&border, Point::new(70.0, 20.0));
    assert_eq!(*deltas.borrow(), [Vector::new(-15.0, 35.0)]);
    assert!(first_touch.pointer.captured_gesture_recognizer().is_some());

    first_touch.move_to(&border, Point::new(80.0, 20.0));
    assert_eq!(deltas.borrow().last(), Some(&Vector::new(-10.0, 0.0)));

    // Without inertia the gesture ends on release.
    first_touch.up(&border, Point::new(80.0, 20.0));
    assert_eq!(ended.get(), 1);
}

#[test]
fn scroll_recognizer_properties() {
    let recognizer = ScrollGestureRecognizer::new();
    assert_eq!(recognizer.scroll_start_distance(), 5);
    assert!(!recognizer.is_scroll_inertia_enabled());
    assert!(recognizer.offset().is_none());

    recognizer.set_direct_value(ScrollGestureRecognizer::is_scroll_inertia_enabled_property(), true);
    assert!(recognizer.is_scroll_inertia_enabled());
    recognizer.set_extent(Some(Size::new(10.0, 20.0)));
    assert_eq!(recognizer.get_direct_value(ScrollGestureRecognizer::extent_property()), Some(Size::new(10.0, 20.0)));
}

#[test]
fn scroll_inertia_runs_on_animation_frames() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let dispatcher = crate::threading::Dispatcher::current_dispatcher();

    let recognizer = ScrollGestureRecognizer::new();
    recognizer.set_can_vertically_scroll(true);
    recognizer.set_is_scroll_inertia_enabled(true);
    recognizer.set_scroll_start_distance(1);

    let (border, root) = recognizer_target(&recognizer);
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let events = log();
    // How many more scroll events are handled: an unhandled one ends the
    // gesture.
    let handled_budget = Rc::new(Cell::new(usize::MAX));
    let recorded = deltas.clone();
    let budget = handled_budget.clone();
    root.root.scroll_gesture(move |_, e| {
        recorded.borrow_mut().push(e.delta());
        if budget.get() > 0 {
            budget.set(budget.get() - 1);
            e.set_handled(true);
        }
    });
    let recorded = events.clone();
    root.root.scroll_gesture_inertia_starting(move |_, e| {
        recorded.borrow_mut().push(format!("inertia {}", e.inertia().y > 0.0))
    });
    let recorded = events.clone();
    root.root.scroll_gesture_ended(move |_, _| recorded.borrow_mut().push("ended".to_string()));

    let flick = |touch: &TouchTestHelper| {
        touch.down(&border, Point::new(50.0, 90.0));
        for y in [80.0, 70.0, 60.0, 50.0] {
            touch.move_to(&border, Point::new(50.0, y));
        }
        touch.up(&border, Point::new(50.0, 50.0));
    };

    let touch = TouchTestHelper::new();
    flick(&touch);

    // Releasing starts the inertia: it is driven by the animation frames of
    // the media context from here on.
    assert_eq!(*events.borrow(), ["inertia true"]);
    assert!(crate::media::MediaContext::instance().media_context_clock().has_subscriptions());
    let moves = deltas.borrow().len();

    // Every frame scrolls for as long as the events are handled; the
    // inertia events are delivered from the dispatcher.
    handled_budget.set(2);
    dispatcher.run_jobs(None);
    assert_eq!(deltas.borrow().len(), moves + 3);
    assert!(deltas.borrow()[moves..].iter().all(|delta| delta.x == 0.0 && delta.y >= 0.0));
    assert_eq!(*events.borrow(), ["inertia true", "ended"]);
    assert!(!crate::media::MediaContext::instance().media_context_clock().has_subscriptions());

    // A new press ends the inertia of the previous gesture.
    handled_budget.set(usize::MAX);
    events.borrow_mut().clear();
    flick(&touch);
    assert_eq!(*events.borrow(), ["inertia true"]);
    let moves = deltas.borrow().len();

    touch.down(&border, Point::new(50.0, 90.0));
    assert_eq!(*events.borrow(), ["inertia true", "ended"]);
    dispatcher.run_jobs(None);
    assert_eq!(deltas.borrow().len(), moves);
    assert!(!crate::media::MediaContext::instance().media_context_clock().has_subscriptions());
}

#[test]
fn pull_gesture_is_raised_from_the_edge() {
    let recognizer = PullGestureRecognizer::with_direction(PullDirection::TopToBottom);
    let (border, root) = recognizer_target(&recognizer);
    let events = log();
    let recorded = events.clone();
    root.root.pull_gesture(move |_, e| recorded.borrow_mut().push(format!("pull {} {:?}", e.delta().y, e.pull_direction())));
    let recorded = events.clone();
    root.root.pull_gesture_ended(move |_, _| recorded.borrow_mut().push("ended".to_string()));

    let touch = TouchTestHelper::new();
    touch.down(&border, Point::new(50.0, 10.0));
    touch.move_to(&border, Point::new(50.0, 40.0));
    touch.up(&border, Point::new(50.0, 40.0));

    assert_eq!(events.borrow()[0], "pull 30 TopToBottom");
    assert!(events.borrow().contains(&"ended".to_string()));

    // A press away from the edge does not start a pull.
    events.borrow_mut().clear();
    let touch = TouchTestHelper::new();
    touch.down(&border, Point::new(50.0, 80.0));
    touch.move_to(&border, Point::new(50.0, 95.0));
    assert!(events.borrow().is_empty());
}

// --- drag and drop --------------------------------------------------------------

struct DragTree {
    root: TestRoot,
    left: Ref<TestControl>,
    right: Ref<TestControl>,
    inner: Ref<TestControl>,
    events: Rc<RefCell<Vec<String>>>,
}

/// A root with two siblings side by side; `left` contains `inner`.
fn drag_tree() -> DragTree {
    let inner = TestControl::new("inner").at(10.0, 10.0, 30.0, 30.0);
    let left = TestControl::new("left").at(0.0, 0.0, 100.0, 200.0).with_children(&[&inner]);
    let right = TestControl::new("right").at(100.0, 0.0, 100.0, 200.0);
    let root = TestRoot::new();
    root.root.add_child(&left);
    root.root.add_child(&right);

    let events = log();
    for (name, event) in [
        ("enter", DragDrop::drag_enter_event()),
        ("over", DragDrop::drag_over_event()),
        ("leave", DragDrop::drag_leave_event()),
        ("drop", DragDrop::drop_event()),
    ] {
        let recorded = events.clone();
        root.root.add_handler(event, move |_, e: &DragEventArgs| {
            let source = e.source().and_then(|source| source.downcast::<TestControl>().ok()).map_or("?", |c| c.label);
            recorded.borrow_mut().push(format!("{name} {source}"));
        });
    }

    DragTree { root, left, right, inner, events }
}

impl DragTree {
    fn drag(&self, type_: RawDragEventType, location: Point, data: &Rc<DataTransfer>) -> DragDropEffects {
        self.drag_with(type_, location, data, DragDropEffects::COPY | DragDropEffects::MOVE, RawInputModifiers::NONE)
    }

    fn drag_with(
        &self,
        type_: RawDragEventType,
        location: Point,
        data: &Rc<DataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    ) -> DragDropEffects {
        let args = Rc::new(RawDragEvent::new(
            DragDropDevice::instance(),
            type_,
            self.root.host.as_input_root(),
            location,
            data.clone(),
            effects,
            modifiers,
        ));
        self.root.host.input(args.clone());
        args.effects()
    }

    fn take_events(&self) -> Vec<String> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}

fn text_data(text: &str) -> Rc<DataTransfer> {
    let data = DataTransfer::new();
    data.add(DataTransferItem::create_text(Some(text)));
    data
}

#[test]
fn drag_events_are_not_raised_without_allow_drop() {
    let tree = drag_tree();
    let data = text_data("text");

    assert!(!DragDrop::get_allow_drop(&tree.left));
    assert_eq!(tree.drag(RawDragEventType::DragEnter, Point::new(50.0, 100.0), &data), DragDropEffects::NONE);
    assert_eq!(tree.drag(RawDragEventType::DragOver, Point::new(60.0, 100.0), &data), DragDropEffects::NONE);
    assert_eq!(tree.drag(RawDragEventType::Drop, Point::new(60.0, 100.0), &data), DragDropEffects::NONE);
    tree.drag(RawDragEventType::DragLeave, Point::new(60.0, 100.0), &data);
    assert!(tree.take_events().is_empty());
}

#[test]
fn drag_events_are_raised_on_the_target_and_report_its_effects() {
    let tree = drag_tree();
    let data = text_data("dragged");
    DragDrop::set_allow_drop(&tree.left, true);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let left = tree.left.clone();
    let root = tree.root.root.clone();
    let token = DragDrop::add_drag_over_handler(&tree.left, move |_, e| {
        recorded.borrow_mut().push((
            e.drag_effects(),
            e.key_modifiers(),
            e.get_position(&left),
            e.get_position(&root),
            e.data_transfer().try_get_text(),
        ));
        e.set_drag_effects(DragDropEffects::MOVE);
    });

    // Without a handler changing them the allowed effects are returned.
    assert_eq!(
        tree.drag(RawDragEventType::DragEnter, Point::new(50.0, 100.0), &data),
        DragDropEffects::COPY | DragDropEffects::MOVE
    );
    assert_eq!(
        tree.drag_with(
            RawDragEventType::DragOver,
            Point::new(60.0, 110.0),
            &data,
            DragDropEffects::COPY | DragDropEffects::MOVE,
            RawInputModifiers::CONTROL | RawInputModifiers::LEFT_MOUSE_BUTTON,
        ),
        DragDropEffects::MOVE
    );
    assert_eq!(
        *seen.borrow(),
        [(
            DragDropEffects::COPY | DragDropEffects::MOVE,
            KeyModifiers::CONTROL,
            Point::new(60.0, 110.0),
            Point::new(60.0, 110.0),
            Some("dragged".to_string()),
        )]
    );
    assert_eq!(
        tree.drag(RawDragEventType::Drop, Point::new(60.0, 110.0), &data),
        DragDropEffects::COPY | DragDropEffects::MOVE
    );
    assert_eq!(tree.take_events(), ["enter left", "over left", "drop left"]);

    // The drop ended the operation: nothing is left to leave or drop on.
    tree.drag(RawDragEventType::DragLeave, Point::new(60.0, 110.0), &data);
    assert_eq!(tree.drag(RawDragEventType::Drop, Point::new(60.0, 110.0), &data), DragDropEffects::NONE);
    assert!(tree.take_events().is_empty());

    DragDrop::remove_drag_over_handler(&tree.left, token);
    tree.drag(RawDragEventType::DragEnter, Point::new(50.0, 100.0), &data);
    tree.drag(RawDragEventType::DragOver, Point::new(60.0, 110.0), &data);
    assert_eq!(seen.borrow().len(), 1);
}

#[test]
fn drag_over_another_target_leaves_the_old_one_and_enters_the_new_one() {
    let tree = drag_tree();
    let data = text_data("text");
    DragDrop::set_allow_drop(&tree.left, true);
    DragDrop::set_allow_drop(&tree.right, true);

    tree.drag(RawDragEventType::DragEnter, Point::new(50.0, 100.0), &data);
    tree.drag(RawDragEventType::DragOver, Point::new(150.0, 100.0), &data);
    assert_eq!(tree.take_events(), ["enter left", "leave left", "enter right"]);

    tree.drag(RawDragEventType::DragOver, Point::new(160.0, 100.0), &data);
    tree.drag(RawDragEventType::DragLeave, Point::new(160.0, 100.0), &data);
    assert_eq!(tree.take_events(), ["over right", "leave right"]);

    // Moving from a target to somewhere that does not allow drops.
    DragDrop::set_allow_drop(&tree.right, false);
    tree.drag(RawDragEventType::DragEnter, Point::new(50.0, 100.0), &data);
    assert_eq!(tree.drag(RawDragEventType::DragOver, Point::new(150.0, 100.0), &data), DragDropEffects::NONE);
    assert_eq!(tree.take_events(), ["enter left", "leave left"]);
    assert_eq!(tree.drag(RawDragEventType::Drop, Point::new(150.0, 100.0), &data), DragDropEffects::NONE);
    assert!(tree.take_events().is_empty());
}

#[test]
fn allow_drop_is_inherited_and_the_innermost_element_is_the_target() {
    let tree = drag_tree();
    let data = text_data("text");
    DragDrop::set_allow_drop(&tree.left, true);
    assert!(DragDrop::get_allow_drop(&tree.inner));
    assert!(!DragDrop::get_allow_drop(&tree.right));

    let positions = Rc::new(RefCell::new(Vec::new()));
    let recorded = positions.clone();
    let inner = tree.inner.clone();
    DragDrop::add_drag_enter_handler(&tree.inner, move |_, e| recorded.borrow_mut().push(e.get_position(&inner)));

    tree.drag(RawDragEventType::DragEnter, Point::new(15.0, 25.0), &data);
    assert_eq!(tree.take_events(), ["enter inner"]);
    assert_eq!(*positions.borrow(), [Point::new(5.0, 15.0)]);

    tree.drag(RawDragEventType::DragOver, Point::new(80.0, 100.0), &data);
    assert_eq!(tree.take_events(), ["leave inner", "enter left"]);

    // An element can opt out again.
    DragDrop::set_allow_drop(&tree.inner, false);
    tree.drag(RawDragEventType::DragOver, Point::new(15.0, 25.0), &data);
    assert_eq!(tree.take_events(), ["leave left"]);
}

#[test]
fn handled_raw_drag_events_are_not_processed() {
    let tree = drag_tree();
    let data = text_data("text");
    DragDrop::set_allow_drop(&tree.left, true);

    let args = RawDragEvent::new(
        DragDropDevice::instance(),
        RawDragEventType::DragEnter,
        tree.root.host.as_input_root(),
        Point::new(50.0, 100.0),
        data,
        DragDropEffects::COPY,
        RawInputModifiers::NONE,
    );
    args.set_handled(true);
    DragDropDevice::instance().process_raw_event(&args);

    assert_eq!(args.effects(), DragDropEffects::COPY);
    assert!(tree.take_events().is_empty());
}

#[test]
fn drag_events_update_the_last_pointer_position() {
    let tree = drag_tree();
    let data = text_data("text");
    tree.root.host.screen_offset.set(PixelPoint::new(1000, 2000));
    let pointer_over = tree.root.host.pointer_over.borrow().clone().unwrap();
    assert_eq!(pointer_over.last_position(), None);

    tree.drag(RawDragEventType::DragOver, Point::new(30.0, 40.0), &data);
    assert_eq!(pointer_over.last_position(), Some(PixelPoint::new(1030, 2040)));

    tree.drag(RawDragEventType::Drop, Point::new(130.0, 45.0), &data);
    assert_eq!(pointer_over.last_position(), Some(PixelPoint::new(1130, 2045)));
}

struct CountingDataTransfer {
    inner: Rc<DataTransfer>,
    disposed: Cell<u32>,
}

impl crate::reactive::IDisposable for CountingDataTransfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

impl IDataTransfer for CountingDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.inner.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        IDataTransfer::items(&*self.inner)
    }
}

struct TestDragSource {
    calls: RefCell<Vec<(DragDropEffects, Option<String>)>>,
}

impl super::platform::IPlatformDragSource for TestDragSource {
    fn do_drag_drop_async(
        &self,
        _trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        self.calls.borrow_mut().push((allowed_effects, data_transfer.try_get_text()));
        Box::pin(std::future::ready(DragDropEffects::LINK))
    }
}

#[test]
fn do_drag_drop_goes_through_the_platform_drag_source() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn ready(mut future: LocalBoxFuture<DragDropEffects>) -> DragDropEffects {
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the future is not ready"),
        }
    }

    let scope = FerroLocator::enter_scope();
    let device = MouseDevice::new();
    let tree = pointer_tree();
    let trigger: Rc<RefCell<Option<PointerPressedEventArgs>>> = Rc::new(RefCell::new(None));
    let recorded = trigger.clone();
    tree.root.root.pointer_pressed(move |_, e| *recorded.borrow_mut() = Some(e.clone()));
    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonDown, OVER_CANVAS, 1);
    let trigger = trigger.borrow_mut().take().expect("a pointer pressed event");

    // Without a drag source nothing is dragged and the data is disposed.
    let data = Rc::new(CountingDataTransfer { inner: text_data("text"), disposed: Cell::new(0) });
    assert_eq!(ready(DragDrop::do_drag_drop_async(&trigger, data.clone(), DragDropEffects::COPY)), DragDropEffects::NONE);
    assert_eq!(data.disposed.get(), 1);

    let source = Rc::new(TestDragSource { calls: RefCell::new(Vec::new()) });
    FerroLocator::current_mutable().bind::<dyn super::platform::IPlatformDragSource>().to_constant(source.clone());

    let data = Rc::new(CountingDataTransfer { inner: text_data("text"), disposed: Cell::new(0) });
    assert_eq!(
        ready(DragDrop::do_drag_drop_async(&trigger, data.clone(), DragDropEffects::COPY | DragDropEffects::LINK)),
        DragDropEffects::LINK
    );
    assert_eq!(*source.calls.borrow(), [(DragDropEffects::COPY | DragDropEffects::LINK, Some("text".to_string()))]);
    // Disposing is up to the drag source.
    assert_eq!(data.disposed.get(), 0);

    tree.root.host.pointer(&device, RawPointerEventType::LeftButtonUp, OVER_CANVAS, 2);
    scope.dispose();
}

// --- text input methods ---------------------------------------------------------

use text_input::{
    ContextMenuAction, ITextInputMethodImpl, TextInputContentType, TextInputMethodClient, TextInputMethodClientEvents,
    TextInputMethodClientRequeryRequestedEventArgs, TextInputOptions, TextInputReturnKeyType, TextSelection,
    TransformTrackingHelper,
};

struct TestInputMethod {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    options: RefCell<Option<TextInputOptions>>,
}

impl TestInputMethod {
    fn new(name: &'static str, log: &Rc<RefCell<Vec<String>>>) -> Rc<Self> {
        Rc::new(Self { name, log: log.clone(), client: RefCell::new(None), options: RefCell::new(None) })
    }

    fn record(&self, entry: String) {
        self.log.borrow_mut().push(format!("{} {}", self.name, entry));
    }
}

impl ITextInputMethodImpl for TestInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        self.record(match &client {
            Some(client) => format!("client {}", client.surrounding_text()),
            None => "client none".to_string(),
        });
        drop(self.client.replace(client));
    }

    fn set_cursor_rect(&self, rect: Rect) {
        self.record(format!("cursor {},{} {}x{}", rect.x, rect.y, rect.width, rect.height));
    }

    fn set_options(&self, options: &TextInputOptions) {
        self.record(format!("options {:?}", options.content_type));
        *self.options.borrow_mut() = Some(options.clone());
    }

    fn reset(&self) {
        self.record("reset".to_string());
    }
}

struct TestTextClient {
    events: TextInputMethodClientEvents,
    name: &'static str,
    view: RefCell<Ref<Visual>>,
    cursor: Cell<Rect>,
    selection: Cell<TextSelection>,
    preedit: RefCell<Vec<Option<String>>>,
}

impl TestTextClient {
    fn new(name: &'static str, view: &Ref<TestControl>) -> Rc<Self> {
        Rc::new(Self {
            events: TextInputMethodClientEvents::new(),
            name,
            view: RefCell::new(view.clone().upcast()),
            cursor: Cell::new(Rect::new(1.0, 2.0, 3.0, 4.0)),
            selection: Cell::new(TextSelection::default()),
            preedit: RefCell::new(Vec::new()),
        })
    }
}

impl TextInputMethodClient for TestTextClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn text_view_visual(&self) -> Ref<Visual> {
        self.view.borrow().clone()
    }
    fn supports_preedit(&self) -> bool {
        true
    }
    fn supports_surrounding_text(&self) -> bool {
        true
    }
    fn surrounding_text(&self) -> String {
        self.name.to_string()
    }
    fn cursor_rectangle(&self) -> Rect {
        self.cursor.get()
    }
    fn selection(&self) -> TextSelection {
        self.selection.get()
    }
    fn set_selection(&self, value: TextSelection) {
        self.selection.set(value);
        self.raise_selection_changed();
    }
    fn set_preedit_text(&self, preedit_text: Option<&str>) {
        self.preedit.borrow_mut().push(preedit_text.map(str::to_string));
    }
}

struct ImeTree {
    root: TestRoot,
    keyboard: Rc<KeyboardDevice>,
    /// A text box like element at (20, 30) offering `client`.
    editor: Ref<TestControl>,
    /// A focusable element without a client.
    button: Ref<TestControl>,
    client: Rc<TestTextClient>,
    im: Rc<TestInputMethod>,
    log: Rc<RefCell<Vec<String>>>,
}

fn offer_client(element: &Ref<TestControl>, client: &Rc<TestTextClient>) {
    let client = client.clone();
    element.text_input_method_client_requested(move |_, e| e.set_client(Some(client.clone())));
}

fn ime_tree() -> ImeTree {
    let keyboard = KeyboardDevice::new();
    let editor = TestControl::focusable("editor").at(20.0, 30.0, 100.0, 20.0);
    let button = TestControl::focusable("button").at(0.0, 100.0, 50.0, 20.0);
    let root = TestRoot::new();
    root.root.add_child(&editor);
    root.root.add_child(&button);

    let log = log();
    let im = TestInputMethod::new("im", &log);
    *root.host.input_method.borrow_mut() = Some(im.clone());

    let client = TestTextClient::new("editor", &editor);
    offer_client(&editor, &client);

    ImeTree { root, keyboard, editor, button, client, im, log }
}

impl ImeTree {
    fn focus(&self, element: Option<&Ref<TestControl>>) {
        let element = element.map(|element| element.clone().upcast::<InputElement>());
        self.keyboard.set_focused_element(element.as_ref(), NavigationMethod::Unspecified, KeyModifiers::NONE);
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

#[test]
fn focusing_an_element_with_a_client_connects_it_to_the_input_method() {
    let tree = ime_tree();

    tree.focus(Some(&tree.editor));

    // The cursor rectangle is reported as soon as the text view is tracked
    // and again once the client is set.
    assert_eq!(tree.take_log(), ["im options Normal", "im cursor 21,32 3x4", "im client editor", "im cursor 21,32 3x4"]);
    let current = tree.im.client.borrow().clone().expect("the client of the editor");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&current), Rc::as_ptr(&tree.client)));

    // Focusing it again changes nothing.
    tree.focus(Some(&tree.editor));
    assert!(tree.take_log().is_empty());
}

#[test]
fn focusing_an_element_without_a_client_disconnects_the_input_method() {
    let tree = ime_tree();

    tree.focus(Some(&tree.button));
    assert_eq!(tree.take_log(), Vec::<String>::new());

    tree.focus(Some(&tree.editor));
    tree.take_log();

    tree.focus(Some(&tree.button));
    assert_eq!(tree.take_log(), ["im reset", "im client none"]);

    tree.focus(Some(&tree.editor));
    tree.take_log();
    tree.focus(None);
    assert_eq!(tree.take_log(), ["im client none"]);
}

#[test]
fn the_client_is_not_looked_for_when_the_input_method_is_disabled() {
    let tree = ime_tree();
    assert!(InputMethod::get_is_input_method_enabled(&tree.editor));
    InputMethod::set_is_input_method_enabled(&tree.editor, false);

    tree.focus(Some(&tree.editor));
    assert!(tree.take_log().is_empty());

    // Enabling it on the focused element looks for the client.
    InputMethod::set_is_input_method_enabled(&tree.editor, true);
    assert_eq!(tree.take_log(), ["im options Normal", "im cursor 21,32 3x4", "im client editor", "im cursor 21,32 3x4"]);

    InputMethod::set_is_input_method_enabled(&tree.editor, false);
    assert_eq!(tree.take_log(), ["im reset", "im client none"]);

    // Changes on other elements are ignored.
    InputMethod::set_is_input_method_enabled(&tree.button, false);
    assert!(tree.take_log().is_empty());
}

#[test]
fn the_client_follows_its_cursor_rectangle_and_reset_requests() {
    let tree = ime_tree();
    tree.focus(Some(&tree.editor));
    tree.take_log();

    tree.client.cursor.set(Rect::new(5.0, 6.0, 1.0, 10.0));
    tree.client.raise_cursor_rectangle_changed();
    assert_eq!(tree.take_log(), ["im cursor 25,36 1x10"]);

    tree.client.request_reset();
    assert_eq!(tree.take_log(), ["im reset", "im options Normal", "im client editor", "im cursor 25,36 1x10"]);

    // Events of a client that is no longer the current one are ignored.
    tree.focus(Some(&tree.button));
    tree.take_log();
    tree.client.raise_cursor_rectangle_changed();
    tree.client.request_reset();
    assert!(tree.take_log().is_empty());
}

#[test]
fn requery_asks_the_focused_element_for_its_client_again() {
    let tree = ime_tree();
    tree.focus(Some(&tree.button));
    assert!(tree.take_log().is_empty());

    // The button starts offering a client and asks for a requery.
    let client = TestTextClient::new("button", &tree.button);
    offer_client(&tree.button, &client);
    tree.button.raise_event(&TextInputMethodClientRequeryRequestedEventArgs::with_event());
    assert_eq!(tree.take_log(), ["im options Normal", "im cursor 1,102 3x4", "im client button", "im cursor 1,102 3x4"]);

    // A requery that finds the same client changes nothing.
    tree.button.raise_event(&TextInputMethodClientRequeryRequestedEventArgs::with_event());
    assert!(tree.take_log().is_empty());

    // The handler is moved with the focus: once the focus is cleared a
    // requery does nothing.
    tree.focus(None);
    tree.take_log();
    tree.button.raise_event(&TextInputMethodClientRequeryRequestedEventArgs::with_event());
    assert!(tree.take_log().is_empty());
}

#[test]
fn the_input_method_gets_the_text_input_options_of_the_focused_element() {
    let tree = ime_tree();

    // The options are inherited.
    TextInputOptions::set_content_type(&tree.root.root, TextInputContentType::Email);
    TextInputOptions::set_multiline(&tree.root.root, true);
    TextInputOptions::set_return_key_type(&tree.editor, TextInputReturnKeyType::Search);
    TextInputOptions::set_show_suggestions(&tree.editor, Some(false));
    TextInputOptions::set_locale_hints(&tree.editor, Some(Rc::from(vec!["pl-PL".to_string()])));
    TextInputOptions::set_is_sensitive(&tree.editor, true);

    tree.focus(Some(&tree.editor));

    let options = tree.im.options.borrow().clone().expect("the options of the editor");
    assert_eq!(
        options,
        TextInputOptions {
            content_type: TextInputContentType::Email,
            return_key_type: TextInputReturnKeyType::Search,
            multiline: true,
            lowercase: false,
            uppercase: false,
            auto_capitalization: false,
            is_sensitive: true,
            show_suggestions: Some(false),
            locale_hints: Some(Rc::from(vec!["pl-PL".to_string()])),
        }
    );
    assert_eq!(TextInputOptions::from_styled_element(&tree.button).content_type, TextInputContentType::Email);
    assert_eq!(TextInputOptions::from_styled_element(&tree.button).return_key_type, TextInputReturnKeyType::Default);
    assert_eq!(TextInputOptions::default_options(), TextInputOptions::default());

    TextInputOptions::set_lowercase(&tree.button, true);
    TextInputOptions::set_uppercase(&tree.button, true);
    TextInputOptions::set_auto_capitalization(&tree.button, true);
    let options = TextInputOptions::from_styled_element(&tree.button);
    assert!(options.lowercase && options.uppercase && options.auto_capitalization && options.multiline);
    assert!(!options.is_sensitive);
    assert_eq!(options.show_suggestions, None);
    assert_eq!(options.locale_hints, None);
}

#[test]
fn the_input_method_of_the_new_root_takes_over_when_focus_moves_between_roots() {
    let tree = ime_tree();

    let other_editor = TestControl::focusable("other").at(0.0, 0.0, 10.0, 10.0);
    let other_root = TestRoot::with_child(&other_editor);
    let other_im = TestInputMethod::new("other-im", &tree.log);
    *other_root.host.input_method.borrow_mut() = Some(other_im.clone());
    let other_client = TestTextClient::new("other", &other_editor);
    offer_client(&other_editor, &other_client);

    tree.focus(Some(&tree.editor));
    tree.take_log();

    tree.focus(Some(&other_editor));
    assert_eq!(
        tree.take_log(),
        [
            "im client none",
            "other-im reset",
            "other-im options Normal",
            "other-im cursor 1,2 3x4",
            "other-im client other",
            "other-im cursor 1,2 3x4"
        ]
    );
}

#[test]
fn the_cursor_rectangle_follows_the_text_view_through_layout_changes() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let tree = ime_tree();
    tree.focus(Some(&tree.editor));
    tree.take_log();

    // Moving the text view is noticed after the render pass.
    tree.editor.set_bounds(Rect::new(40.0, 50.0, 100.0, 20.0));
    assert!(tree.take_log().is_empty());
    crate::threading::Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(tree.take_log(), ["im cursor 41,52 3x4"]);

    // A change that leaves the transform as it is reports nothing.
    tree.editor.set_bounds(Rect::new(40.0, 50.0, 120.0, 20.0));
    crate::threading::Dispatcher::current_dispatcher().run_jobs(None);
    assert!(tree.take_log().is_empty());
}

#[test]
fn text_input_method_client_defaults() {
    let view = TestControl::new("view");
    let client = TestTextClient::new("client", &view);

    // Without an override the cursor position is dropped.
    client.set_preedit_text_with_cursor(Some("pre"), Some(2));
    client.set_preedit_text_with_cursor(None, None);
    assert_eq!(*client.preedit.borrow(), [Some("pre".to_string()), None]);
    client.execute_context_menu_action(ContextMenuAction::Paste);

    let events = log();
    let subscriptions: Vec<Rc<dyn crate::reactive::IDisposable>> = {
        let record = |name: &'static str| -> Rc<dyn Fn()> {
            let events = events.clone();
            Rc::new(move || events.borrow_mut().push(name.to_string()))
        };
        vec![
            client.text_view_visual_changed(record("view")),
            client.cursor_rectangle_changed(record("cursor")),
            client.surrounding_text_changed(record("text")),
            client.selection_changed(record("selection")),
            client.reset_requested(record("reset")),
            client.input_pane_activation_requested(record("pane")),
        ]
    };

    client.raise_text_view_visual_changed();
    client.raise_cursor_rectangle_changed();
    client.raise_surrounding_text_changed();
    client.set_selection(TextSelection::new(1, 3));
    client.request_reset();
    client.raise_input_pane_activation_requested();
    assert_eq!(*events.borrow(), ["view", "cursor", "text", "selection", "reset", "pane"]);
    assert_eq!(client.selection(), TextSelection { start: 1, end: 3 });

    for subscription in subscriptions {
        subscription.dispose();
    }
    client.raise_cursor_rectangle_changed();
    client.request_reset();
    assert_eq!(events.borrow().len(), 6);
}

#[test]
fn transform_tracking_helper_reports_the_transform_to_the_root() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let child = TestControl::new("child").at(5.0, 6.0, 10.0, 10.0);
    let parent = TestControl::new("parent").at(10.0, 20.0, 100.0, 100.0).with_children(&[&child]);
    let root = TestRoot::new();

    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    let subscription = TransformTrackingHelper::track(&child.clone().upcast(), true, move |_, matrix| {
        recorded.borrow_mut().push(matrix.map(|m| (m.m31, m.m32)))
    });
    // Not attached: there is no transform to report yet.
    assert!(changes.borrow().is_empty());

    root.root.add_child(&parent);
    assert_eq!(*changes.borrow(), [Some((15.0, 26.0))]);

    // A change of the bounds of an ancestor is picked up after the render
    // pass, once.
    parent.set_bounds(Rect::new(30.0, 20.0, 100.0, 100.0));
    parent.set_bounds(Rect::new(40.0, 20.0, 100.0, 100.0));
    assert_eq!(changes.borrow().len(), 1);
    crate::threading::Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(*changes.borrow(), [Some((15.0, 26.0)), Some((45.0, 26.0))]);

    root.root.remove_child(&parent);
    assert_eq!(changes.borrow().last(), Some(&None));
    let count = changes.borrow().len();

    // Detached: the ancestors are no longer watched.
    parent.set_bounds(Rect::new(0.0, 0.0, 100.0, 100.0));
    crate::threading::Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(changes.borrow().len(), count);

    root.root.add_child(&parent);
    assert_eq!(changes.borrow().last(), Some(&Some((5.0, 6.0))));
    let count = changes.borrow().len();

    subscription.dispose();
    child.set_bounds(Rect::new(1.0, 1.0, 10.0, 10.0));
    crate::threading::Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(changes.borrow().len(), count);
}

// --- chrome hit testing, bitmap cursors, geometry hit testing -------------------

#[test]
fn input_roots_have_no_chrome_and_no_input_method_by_default() {
    let root = TestRoot::new();
    let input_root = root.host.as_input_root();

    assert_eq!(input_root.hit_test_chrome_element(Point::new(10.0, 10.0)), None);
    assert!(input_root.input_method().is_none());
    assert_eq!(WindowDecorationsElementRole::default(), WindowDecorationsElementRole::None);
    assert_eq!(WindowDecorationsElementRole::FullScreenButton as i32, 15);
}

struct TestBitmapImpl;

impl crate::platform::IBitmapImpl for TestBitmapImpl {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(16, 16)
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(
        &self,
        _stream: &mut dyn std::io::Write,
        _options: &crate::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Ok(())
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct RecordingCursorFactory(RefCell<Vec<(PixelSize, PixelPoint)>>);

impl ICursorFactory for RecordingCursorFactory {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(TestCursorImpl)
    }

    fn create_cursor(&self, cursor: &crate::media::imaging::Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        self.0.borrow_mut().push((cursor.pixel_size(), hot_spot));
        Rc::new(TestCursorImpl)
    }
}

#[test]
fn cursor_can_be_created_from_a_bitmap() {
    let scope = FerroLocator::enter_scope();
    let factory = Rc::new(RecordingCursorFactory(RefCell::new(Vec::new())));
    FerroLocator::current_mutable().bind::<dyn ICursorFactory>().to_constant(factory.clone());

    let bitmap = crate::media::imaging::Bitmap::from_impl(std::sync::Arc::new(TestBitmapImpl));
    let cursor = Cursor::from_bitmap(&bitmap, PixelPoint::new(3, 4));

    assert_eq!(cursor.to_string(), "BitmapCursor");
    assert_eq!(*factory.0.borrow(), [(PixelSize::new(16, 16), PixelPoint::new(3, 4))]);
    cursor.dispose();
    scope.dispose();
}

#[test]
fn geometry_hit_test_finds_the_elements_intersecting_a_region() {
    use crate::media::{Geometry, IntersectionResult, RectangleGeometry};
    use crate::rendering::testing::MockPlatformRenderInterface;

    let (scope, _render_interface) = MockPlatformRenderInterface::install();
    let inner = TestControl::new("inner").at(10.0, 10.0, 30.0, 30.0);
    let left = TestControl::new("left").at(0.0, 0.0, 100.0, 200.0).with_children(&[&inner]);
    let right = TestControl::new("right").at(100.0, 0.0, 100.0, 200.0);
    let root = TestRoot::new();
    root.root.add_child(&left);
    root.root.add_child(&right);

    let labels = |results: &[crate::media::GeometryHitTestResult]| -> Vec<&'static str> {
        results.iter().map(|x| x.visual_hit.downcast_ref::<TestControl>().map_or("?", |c| c.label)).collect()
    };
    let region = |x: f64, y: f64, width: f64, height: f64| -> Ref<Geometry> {
        RectangleGeometry::with_rect(Rect::new(x, y, width, height)).upcast()
    };

    // A region over the corner of `inner`, inside `left`.
    let hits = root.root.get_visuals_at_geometry(&region(30.0, 30.0, 40.0, 40.0));
    assert_eq!(labels(&hits), ["inner", "left", "root"]);
    assert_eq!(hits[0].intersection_result, IntersectionResult::Intersects);

    let first = root.root.get_visual_at_geometry(&region(30.0, 30.0, 40.0, 40.0)).expect("a hit");
    assert_eq!(labels(&[first]), ["inner"]);

    // A region across both siblings: the later sibling is on top.
    let hits = root.root.get_input_elements_at_geometry(&region(90.0, 150.0, 20.0, 10.0), true);
    assert_eq!(labels(&hits), ["right", "left", "root"]);
    assert!(hits.iter().all(|hit| hit.intersection_result != IntersectionResult::Empty));

    // The region is in the coordinates of the element the test starts at.
    let hits = left.get_visuals_at_geometry(&region(45.0, 45.0, 10.0, 10.0));
    assert_eq!(labels(&hits), ["left"]);
    let hits = left.get_visuals_at_geometry(&region(35.0, 35.0, 10.0, 10.0));
    assert_eq!(labels(&hits), ["inner", "left"]);

    // Outside of everything.
    assert!(root.root.get_visual_at_geometry(&region(500.0, 500.0, 10.0, 10.0)).is_none());
    assert!(root.root.get_visuals_at_geometry(&region(500.0, 500.0, 10.0, 10.0)).is_empty());

    // Filters exclude subtrees; input hit testing skips disabled and
    // hit-test-invisible elements.
    let hits = root.root.get_visuals_at_geometry_filtered(&region(30.0, 30.0, 40.0, 40.0), &|visual: &Visual| {
        visual.downcast_ref::<TestControl>().is_none_or(|c| c.label != "left")
    });
    assert_eq!(labels(&hits), ["root"]);

    inner.set_is_enabled(false);
    let hit = root.root.input_hit_test_geometry(&region(30.0, 30.0, 40.0, 40.0), true).expect("a hit");
    assert_eq!(labels(&[hit]), ["left"]);
    let hit = root.root.input_hit_test_geometry(&region(30.0, 30.0, 40.0, 40.0), false).expect("a hit");
    assert_eq!(labels(&[hit]), ["inner"]);
    let hit = root
        .root
        .input_hit_test_geometry_filtered(
            &region(30.0, 30.0, 40.0, 40.0),
            &|visual: &Visual| visual.downcast_ref::<TestControl>().is_none_or(|c| c.label != "inner"),
            false,
        )
        .expect("a hit");
    assert_eq!(hit.downcast_ref::<TestControl>().unwrap().label, "left");

    inner.set_is_visible(false);
    let hits = root.root.get_visuals_at_geometry(&region(30.0, 30.0, 40.0, 40.0));
    assert_eq!(labels(&hits), ["left", "root"]);

    // Clipping to bounds cuts off what lies outside of the parent.
    let overflowing = TestControl::new("overflowing").at(150.0, 0.0, 100.0, 50.0);
    right.add_child(&overflowing);
    let hits = root.root.get_visuals_at_geometry(&region(260.0, 10.0, 20.0, 20.0));
    assert_eq!(labels(&hits), ["overflowing"]);
    right.set_clip_to_bounds(true);
    assert!(root.root.get_visuals_at_geometry(&region(260.0, 10.0, 20.0, 20.0)).is_empty());

    scope.dispose();
}
