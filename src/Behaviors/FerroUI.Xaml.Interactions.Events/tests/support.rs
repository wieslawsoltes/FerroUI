//! The test application, the host of the triggers and behaviors (a border in a panel in a window),
//! an action that records its executions, and the args of the input events.

use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::input::{
    DataTransfer, DragEventArgs, FocusChangedEventArgs, IPointer, InputElement, KeyEventArgs, KeyModifiers, MouseButton,
    Pointer, PointerCaptureLostEventArgs, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, PointerWheelEventArgs, RawInputModifiers,
    ScrollGestureEndedEventArgs, ScrollGestureEventArgs, TappedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, ObjectType, Point, Ref, StyledElementImpl,
    Vector, Visual,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Panel, Window};
use ferroui_xaml_interactivity::{ActionCollection, Interaction, StyledElementAction, StyledElementActionImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Starts the application of a test.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// `Dispatcher.UIThread.RunJobs()`.
#[allow(dead_code)]
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// The object as the root class (the items of the behavior and action collections).
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// Whether running `f` panics (the invalid operations of the reference are panics here).
pub(crate) fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// A border (the associated object) in a panel in a window.
pub(crate) struct Host {
    pub(crate) window: Ref<Window>,
    pub(crate) panel: Ref<Panel>,
    pub(crate) border: Ref<Border>,
    pointer: Rc<dyn IPointer>,
}

impl Host {
    /// Creates the host; the window is not shown yet.
    pub(crate) fn new() -> Self {
        let border = Border::new();
        let panel = Panel::new();
        panel.children().add(&border);
        let window = Window::new();
        let content: Ref<Control> = panel.clone().upcast();
        window.set_content(Some(Control::boxed(content)));
        let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        Self { window, panel, border, pointer }
    }

    /// Adds a behavior or a trigger to the border (`<Interaction.Behaviors>` of markup).
    pub(crate) fn add_behavior(&self, behavior: Ref<FerroObject>) {
        Interaction::get_behaviors(&self.border).add(behavior);
    }

    /// Removes the border from the visual tree.
    pub(crate) fn detach_border(&self) {
        self.panel.children().remove(&self.border);
    }

    /// The root of the tree that observes the pointer events.
    pub(crate) fn root(&self) -> &Visual {
        &self.window
    }

    /// The pointer of the pointer events.
    pub(crate) fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    /// The properties of a pointer that moves with no button pressed.
    pub(crate) fn properties() -> PointerPointProperties {
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other)
    }

    /// Args of a pointer event raised on the border.
    pub(crate) fn pointer_event_args(&self, routed_event: &RoutedEvent<PointerEventArgs>) -> PointerEventArgs {
        PointerEventArgs::new(
            Some(routed_event),
            &self.border,
            self.pointer(),
            Some(self.root()),
            Point::default(),
            1,
            Self::properties(),
            KeyModifiers::NONE,
        )
    }

    /// The pointer event a gesture is made of (the released pointer of a tap).
    pub(crate) fn last_pointer_event_args(&self) -> PointerEventArgs {
        self.pointer_event_args(InputElement::pointer_moved_event())
    }

    /// Args of a tap gesture (`Tapped`, `DoubleTapped`, `RightTapped`) raised on the border.
    pub(crate) fn tapped_event_args(&self, routed_event: &RoutedEvent<TappedEventArgs>) -> TappedEventArgs {
        TappedEventArgs::new(Some(routed_event), &self.last_pointer_event_args())
    }

    /// Args of a drag-and-drop event over the border.
    pub(crate) fn drag_event_args(&self, routed_event: &RoutedEvent<DragEventArgs>) -> DragEventArgs {
        let target: Ref<Interactive> = self.border.clone().upcast();
        DragEventArgs::new(Some(routed_event), DataTransfer::new(), target, Point::default(), KeyModifiers::NONE)
    }

    /// Args of a focus change of the border.
    pub(crate) fn focus_changed_event_args(&self, routed_event: &RoutedEvent<FocusChangedEventArgs>) -> FocusChangedEventArgs {
        FocusChangedEventArgs::new(routed_event)
    }

    /// Args of a key event.
    pub(crate) fn key_event_args(&self, routed_event: &RoutedEvent<KeyEventArgs>) -> KeyEventArgs {
        let args = KeyEventArgs::new();
        args.set_routed_event(Some(routed_event));
        args
    }

    /// Args of a text input.
    pub(crate) fn text_input_event_args(&self) -> TextInputEventArgs {
        let args = TextInputEventArgs::new();
        args.set_routed_event(Some(InputElement::text_input_event()));
        args
    }

    /// Args of the request of a text input method client.
    pub(crate) fn text_input_method_client_requested_event_args(&self) -> TextInputMethodClientRequestedEventArgs {
        let args = TextInputMethodClientRequestedEventArgs::new();
        args.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
        args
    }

    /// Args of a pointer press on the border.
    pub(crate) fn pointer_pressed_event_args(&self) -> PointerPressedEventArgs {
        PointerPressedEventArgs::new(
            &self.border,
            self.pointer(),
            self.root(),
            Point::default(),
            1,
            Self::properties(),
            KeyModifiers::NONE,
            1,
        )
    }

    /// Args of a pointer release on the border.
    pub(crate) fn pointer_released_event_args(&self) -> PointerReleasedEventArgs {
        PointerReleasedEventArgs::new(
            &self.border,
            self.pointer(),
            self.root(),
            Point::default(),
            1,
            Self::properties(),
            KeyModifiers::NONE,
            MouseButton::Left,
        )
    }

    /// Args of a pointer wheel change over the border.
    pub(crate) fn pointer_wheel_event_args(&self) -> PointerWheelEventArgs {
        PointerWheelEventArgs::new(
            &self.border,
            self.pointer(),
            self.root(),
            Point::default(),
            1,
            Self::properties(),
            KeyModifiers::NONE,
            Vector::new(0.0, 1.0),
        )
    }

    /// Args of the loss of the capture of the pointer by the border.
    pub(crate) fn pointer_capture_lost_event_args(&self) -> PointerCaptureLostEventArgs {
        PointerCaptureLostEventArgs::new(&self.border, self.pointer())
    }

    /// Args of a scroll gesture.
    pub(crate) fn scroll_gesture_event_args(&self) -> ScrollGestureEventArgs {
        ScrollGestureEventArgs::new(ScrollGestureEventArgs::get_next_free_id(), Vector::new(0.0, 10.0))
    }

    /// Args of the end of a scroll gesture.
    pub(crate) fn scroll_gesture_ended_event_args(&self) -> ScrollGestureEndedEventArgs {
        ScrollGestureEndedEventArgs::new(ScrollGestureEventArgs::get_next_free_id())
    }

    /// Raises `args` on the border and returns a shared handle to them.
    pub(crate) fn raise<T: IRoutedEventArgs>(&self, args: T) -> Rc<dyn IRoutedEventArgs> {
        self.border.raise_event(&args);
        args.share()
    }
}

/// An action that records its executions (the `StubAction` of the tests of the interactivity
/// crate).
#[repr(C)]
pub(crate) struct RecordingAction {
    base: StyledElementAction,
    sender: RefCell<Option<BoxedValue>>,
    parameter: RefCell<Option<BoxedValue>>,
    execute_count: Cell<i32>,
}

ferro_class!(RecordingAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(RecordingAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for RecordingAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execute_count.set(this.execute_count.get() + 1);
        *this.sender.borrow_mut() = sender.cloned();
        *this.parameter.borrow_mut() = parameter.cloned();
        None
    }
}

impl RecordingAction {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElementAction::construct(),
            sender: RefCell::new(None),
            parameter: RefCell::new(None),
            execute_count: Cell::new(0),
        })
    }

    /// Adds a new recording action to `actions`.
    pub(crate) fn add_to(actions: Option<ActionCollection>) -> Ref<Self> {
        let action = Self::new();
        actions.expect("a trigger starts with a collection of actions").add(object(&action));
        action
    }

    pub(crate) fn sender(&self) -> Option<BoxedValue> {
        self.sender.borrow().clone()
    }

    pub(crate) fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    pub(crate) fn execute_count(&self) -> i32 {
        self.execute_count.get()
    }
}
