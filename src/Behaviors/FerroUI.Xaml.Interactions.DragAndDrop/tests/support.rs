//! The test application (`TestAppBuilder.cs`, `App.xaml` and `HeadlessWindowExtensions.cs` of the
//! reference test project).
//!
//! The reference runs its tests on the headless platform with a window whose theme is the Fluent
//! theme, and simulates the mouse through the input of the headless window; here they run in the
//! unit test application of the framework, with the services of a styled window, and the mouse
//! raises the pointer events on the element directly (as the mouse helper of the controls tests
//! of the framework does).

use ferroui_base::input::{
    IPointer, KeyModifiers, MouseButton, Pointer, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use crate::drag_drop_context_store::{DragDropContextStore, RemoveContextOnDrop};
use crate::{ContextDropBehaviorBase, DropHandlerBase, IDragHandler};
use ferroui_base::input::{
    DataTransfer, DataTransferItem, DragEventArgs, ICommand, IDataTransfer, InputElement, Key, KeyEventArgs,
    LocalBoxFuture,
};
use ferroui_base::interactivity::{Interactive, RoutedEvent};
use ferroui_base::platform::storage::{IStorageFolder, IStorageItem, StorageItemProperties};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{HandlerList, Uri};
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Point, Ref, Upcast, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Window};
use ferroui_xaml_interactivity::Interaction;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Starts the application of a test (the headless test attribute of the reference).
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// [`start_app`] with the Fluent theme, the theme of the reference's test application.
pub(crate) fn start_app_with_fluent_theme() -> UnitTestApplicationScope {
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    UnitTestApplication::start(
        fluent_services(),
    )
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// `new Window { Content = content, Width = width, Height = height }`, shown and laid out.
pub(crate) fn show_window(content: impl ferroui_base::IntoRef<Control>, width: f64, height: f64) -> Ref<Window> {
    let window = Window::new();
    window.set_width(width);
    window.set_height(height);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    run_jobs();
    capture_rendered_frame(&window);
    window
}

/// `window.CaptureRenderedFrame()`: what the tests need of it, a layout pass and the jobs it
/// queued.
pub(crate) fn capture_rendered_frame(window: &Window) {
    window.update_layout();
    run_jobs();
}

/// The object as the root class.
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// A mouse that raises the pointer events of a press, a move and a release on an element (the
/// `MouseDown`, `MouseMove` and `MouseUp` of `HeadlessWindowExtensions.cs`): the position is
/// relative to the element, as the reference gives it.
pub(crate) struct Mouse {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    pressed_buttons: Cell<RawInputModifiers>,
    pressed_button: Cell<MouseButton>,
}

impl Mouse {
    pub(crate) fn new() -> Self {
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

    fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    fn root_of(target: &Control) -> Ref<Visual> {
        target.visual_root().unwrap_or_else(|| target.to_ref().upcast())
    }

    fn root_position(target: &Control, root: &Visual, position: Point) -> Point {
        target.translate_point(position, root).unwrap_or(position)
    }

    /// Presses the left button over `target` at `position`.
    pub(crate) fn down(&self, target: &Control, position: Point) {
        self.down_with(target, position, KeyModifiers::NONE);
    }

    /// Presses the left button over `target` at `position` with the keyboard modifiers.
    pub(crate) fn down_with(&self, target: &Control, position: Point, modifiers: KeyModifiers) {
        self.pressed_buttons.set(self.pressed_buttons.get() | RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.pressed_button.set(MouseButton::Left);
        let properties = PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::LeftButtonPressed);
        let root = Self::root_of(target);
        target.raise_event(&PointerPressedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            &root,
            Self::root_position(target, &root, position),
            self.timestamp(),
            properties,
            modifiers,
            1,
        ));
    }

    /// Moves the pointer over `target` to `position`.
    pub(crate) fn move_to(&self, target: &Control, position: Point) {
        let root = Self::root_of(target);
        target.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            target.to_ref(),
            self.pointer(),
            Some(&*root),
            Self::root_position(target, &root, position),
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    /// Releases the left button over `target` at `position`.
    pub(crate) fn up(&self, target: &Control, position: Point) {
        let left = RawInputModifiers::LEFT_MOUSE_BUTTON;
        self.pressed_buttons.set((self.pressed_buttons.get() | left) ^ left);
        let properties =
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::LeftButtonReleased);
        let root = Self::root_of(target);
        target.raise_event(&PointerReleasedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            &root,
            Self::root_position(target, &root, position),
            self.timestamp(),
            properties,
            KeyModifiers::NONE,
            self.pressed_button.get(),
        ));
    }
}

/// Presses `key` on `target` (`KeyPressQwerty` of the headless window of the reference: the key
/// down event, raised on the element directly).
pub(crate) fn key_down(target: &Control, key: Key) {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.set_source(target.to_ref());
    target.raise_event(&e);
}

/// Raises the drag event `routed_event` with `data` on `target`, at `position` in the coordinates
/// of `target` (what the drag and drop device of the framework raises over an element). Returns
/// the event args, whose effects the handlers set.
pub(crate) fn raise_drag_event(
    target: &Control,
    routed_event: &RoutedEvent<DragEventArgs>,
    data: Rc<dyn IDataTransfer>,
    position: Point,
) -> DragEventArgs {
    let interactive: Ref<Interactive> = target.to_ref().upcast();
    let e = DragEventArgs::new(Some(routed_event), data, interactive, position, KeyModifiers::NONE);
    e.set_source(target.to_ref());
    target.raise_event(&e);
    e
}

/// The drag data of a drag of `context` from a context drag behavior: the key of the context in
/// the context store (the reference stores the key as the drag starts).
pub(crate) fn context_drag_data(context: &BoxedValue) -> (Rc<dyn IDataTransfer>, RemoveContextOnDrop) {
    let key = DragDropContextStore::add(Some(context));
    let data = DataTransfer::new();
    data.add(DataTransferItem::create(&ContextDropBehaviorBase::context_data_transfer_format(), key.clone()));
    (data, RemoveContextOnDrop(key))
}

/// Drag data with a text.
pub(crate) fn text_drag_data(text: &str) -> Rc<dyn IDataTransfer> {
    let data = DataTransfer::new();
    data.add(DataTransferItem::create_text(Some(text)));
    data
}

/// A text as an untyped value.
pub(crate) fn text(value: &str) -> BoxedValue {
    Rc::new(value.to_string())
}

/// Whether an untyped value holds the text `text`.
pub(crate) fn is_text(value: Option<&BoxedValue>, text: &str) -> bool {
    value.and_then(|value| value.downcast_ref::<String>()).is_some_and(|value| value == text)
}

/// Adds `behavior` to the behaviors of `object`.
pub(crate) fn add_behavior<T: ObjectType + Upcast<FerroObject>>(object: &FerroObject, behavior: &Ref<T>) {
    Interaction::get_behaviors(object).add(behavior.clone().upcast());
}

/// A border of a fixed size.
pub(crate) fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

/// A command that counts the subscriptions to its notification and the calls of `can_execute`,
/// and records the parameter of its last execution (`Core/ObservableCommand.cs` of the
/// reference test project, with the record of the executions).
pub(crate) struct ObservableCommand {
    this: Weak<ObservableCommand>,
    can_execute_changed: HandlerList<dyn Fn()>,
    pub(crate) can_execute_result: Cell<bool>,
    pub(crate) can_execute_callback: RefCell<Option<Box<dyn Fn(Option<&BoxedValue>) -> bool>>>,
    subscription_count: Cell<i32>,
    last_can_execute_parameter: RefCell<Option<BoxedValue>>,
    can_execute_call_count: Cell<i32>,
    execute_count: Cell<i32>,
    last_execute_parameter: RefCell<Option<BoxedValue>>,
}

impl ObservableCommand {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            can_execute_changed: HandlerList::new(),
            can_execute_result: Cell::new(true),
            can_execute_callback: RefCell::new(None),
            subscription_count: Cell::new(0),
            last_can_execute_parameter: RefCell::new(None),
            can_execute_call_count: Cell::new(0),
            execute_count: Cell::new(0),
            last_execute_parameter: RefCell::new(None),
        })
    }

    pub(crate) fn with_result(can_execute_result: bool) -> Rc<Self> {
        let command = Self::new();
        command.can_execute_result.set(can_execute_result);
        command
    }

    pub(crate) fn with_callback(callback: impl Fn(Option<&BoxedValue>) -> bool + 'static) -> Rc<Self> {
        let command = Self::new();
        *command.can_execute_callback.borrow_mut() = Some(Box::new(callback));
        command
    }

    pub(crate) fn subscription_count(&self) -> i32 {
        self.subscription_count.get()
    }

    pub(crate) fn last_can_execute_parameter(&self) -> Option<BoxedValue> {
        self.last_can_execute_parameter.borrow().clone()
    }

    pub(crate) fn can_execute_call_count(&self) -> i32 {
        self.can_execute_call_count.get()
    }

    pub(crate) fn execute_count(&self) -> i32 {
        self.execute_count.get()
    }

    pub(crate) fn last_execute_parameter(&self) -> Option<BoxedValue> {
        self.last_execute_parameter.borrow().clone()
    }

    pub(crate) fn raise_can_execute_changed(&self) {
        for (_, handler) in self.can_execute_changed.snapshot().iter() {
            handler();
        }
    }

    /// The command as the contract.
    pub(crate) fn as_command(self: &Rc<Self>) -> Option<Rc<dyn ICommand>> {
        Some(self.clone())
    }
}

impl ICommand for ObservableCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        self.can_execute_call_count.set(self.can_execute_call_count.get() + 1);
        *self.last_can_execute_parameter.borrow_mut() = parameter.cloned();
        match &*self.can_execute_callback.borrow() {
            Some(callback) => callback(parameter),
            None => self.can_execute_result.get(),
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        self.execute_count.set(self.execute_count.get() + 1);
        *self.last_execute_parameter.borrow_mut() = parameter.cloned();
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(handler);
        self.subscription_count.set(self.subscription_count.get() + 1);
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.can_execute_changed.remove(token);
                this.subscription_count.set(this.subscription_count.get() - 1);
            }
        })
    }
}

/// A drop handler that accepts every drag and records the contexts of the last drop
/// (`DragAndDrop/TestDropHandler.cs` of the reference test project).
#[derive(Default)]
pub(crate) struct TestDropHandler {
    pub(crate) last_source_context: RefCell<Option<BoxedValue>>,
    pub(crate) last_target_context: RefCell<Option<BoxedValue>>,
}

impl DropHandlerBase for TestDropHandler {
    fn validate(
        &self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        true
    }

    fn execute(
        &self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        *self.last_source_context.borrow_mut() = source_context.cloned();
        *self.last_target_context.borrow_mut() = target_context.cloned();
        true
    }
}

/// Not from the reference: a drag handler that records its calls.
#[derive(Default)]
pub(crate) struct TestDragHandler {
    pub(crate) before_called: Cell<bool>,
    pub(crate) after_called: Cell<bool>,
    pub(crate) last_context: RefCell<Option<BoxedValue>>,
}

impl IDragHandler for TestDragHandler {
    fn before_drag_drop(&self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, context: Option<&BoxedValue>) {
        self.before_called.set(true);
        *self.last_context.borrow_mut() = context.cloned();
    }

    fn after_drag_drop(&self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, _context: Option<&BoxedValue>) {
        self.after_called.set(true);
    }
}

/// Not from the reference: a file of the storage of a test.
pub(crate) struct TestFile {
    pub(crate) name: &'static str,
}

fn now<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

impl IDisposable for TestFile {
    fn dispose(&self) {}
}

impl IStorageItem for TestFile {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn path(&self) -> Uri {
        Uri::absolute(&format!("file:///files/{}", self.name)).expect("a file URI")
    }

    fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
        now(StorageItemProperties::new(Some(1), None, None))
    }

    fn can_bookmark(&self) -> bool {
        false
    }

    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
        now(None)
    }

    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        now(None)
    }

    fn delete_async(&self) -> LocalBoxFuture<std::io::Result<()>> {
        now(Ok(()))
    }

    fn move_async(
        &self,
        _destination: Rc<dyn IStorageFolder>,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageItem>>>> {
        now(Ok(None))
    }
}

/// Drag data with files.
pub(crate) fn files_drag_data(names: &[&'static str]) -> Rc<dyn IDataTransfer> {
    let data = DataTransfer::new();
    for name in names {
        data.add(DataTransferItem::create_file(Some(Rc::new(TestFile { name }))));
    }
    data
}

/// The global clock of the themed application: the transitions of the Fluent control themes need
/// one. It never pulses.
struct MockGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: Cell<ferroui_base::animation::PlayState>,
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::TimeSpan> for MockGlobalClock {
    fn subscribe(
        &self,
        observer: Rc<dyn ferroui_base::reactive::IObserver<ferroui_base::animation::TimeSpan>>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl ferroui_base::animation::IClock for MockGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(value)
    }
}

impl ferroui_base::animation::IGlobalClock for MockGlobalClock {}

/// The services of the reference's test application: the Fluent theme and a global clock.
fn fluent_services() -> TestServices {
    let clock = Rc::new(MockGlobalClock {
        subject: ferroui_base::reactive::LightweightSubject::new(),
        play_state: Cell::new(ferroui_base::animation::PlayState::Run),
    });
    TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style())
}
