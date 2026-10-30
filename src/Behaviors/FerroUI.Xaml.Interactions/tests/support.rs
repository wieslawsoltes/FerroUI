//! The test application (`TestAppBuilder.cs`, the application document and
//! `HeadlessWindowExtensions.cs` of the reference).
//!
//! The reference runs its tests on the headless platform with the Fluent theme and simulates the
//! input of a keyboard and a mouse on the window. Here the tests run in the unit test application
//! of the framework, with the services of a styled window (whose test theme gives windows a
//! template that presents their content); the windows of the tests are documents loaded with the
//! run-time loader; a key press is raised on the element that has the focus in the reference, and
//! a click of the mouse is a press and a release of the pointer raised on the element.

use crate::ASSEMBLY;
use ferroui_base::controls::NameScope;
use ferroui_base::input::{
    CaptureSource, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Point, Ref, StyledElement, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
use std::rc::Rc;

/// Starts the application of a test (the headless test attribute of the reference).
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    let scope = UnitTestApplication::start(TestServices::styled_window());
    FerroRuntimeXamlLoader::register();
    scope
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// Loads a document of the tests with the run-time loader and returns its root object.
pub(crate) fn load<T: ObjectType>(name: &str, xaml: &str) -> Ref<T> {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("{name} failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("{name} failed to load: {}", error.message()),
        });
    from_markup_value::<Ref<T>>(&Some(loaded)).unwrap_or_else(|| panic!("{name} is not a {}", T::TYPE.name()))
}

/// A named element of the name scope of `root` (the fields the compiler of the reference generates
/// for the named elements of a window).
pub(crate) fn find<T: ObjectType>(root: &StyledElement, name: &str) -> Ref<T> {
    NameScope::get_name_scope(root)
        .and_then(|scope| scope.find(name))
        .and_then(|object| object.cast::<T>())
        .unwrap_or_else(|| panic!("the named element {name}"))
}

/// The object as an untyped value (an `object` argument of the reference).
pub(crate) fn boxed<T: ObjectType>(value: &Ref<T>) -> BoxedValue {
    let object: Ref<FerroObject> = value.clone().upcast();
    Rc::new(object)
}

/// A text as an untyped value.
pub(crate) fn text(value: &str) -> BoxedValue {
    Rc::new(value.to_string())
}

/// The boolean an untyped result holds (`(bool)result`).
pub(crate) fn as_bool(value: Option<BoxedValue>) -> bool {
    value.and_then(|value| value.downcast_ref::<bool>().copied()).expect("a boolean result")
}

/// Whether running `f` panics (`Assert.Throws<..>` of the reference: the exceptions of the
/// reference are panics here).
pub(crate) fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// Presses a key on `target`, the element that has the focus in the reference
/// (`target.Focus(); window.KeyPressQwerty(key, RawInputModifiers.None)`): the key down event is
/// raised on it, then the key up event.
pub(crate) fn key_press(target: &Interactive, key: Key) {
    let mut down = KeyEventArgs::new();
    down.key = key;
    down.key_modifiers = KeyModifiers::NONE;
    down.set_routed_event(Some(InputElement::key_down_event()));
    target.raise_event(&down);

    let mut up = KeyEventArgs::new();
    up.key = key;
    up.key_modifiers = KeyModifiers::NONE;
    up.set_routed_event(Some(InputElement::key_up_event()));
    target.raise_event(&up);
}

/// Clicks `target` with the keyboard (Enter), as the tests of the reference click their buttons
/// after focusing them; a check box toggles.
pub(crate) fn click(target: &Interactive) {
    key_press(target, Key::Enter);
}

/// The mouse of the tests (`MouseDown` and `MouseUp` of the headless window of the reference): a
/// press and a release of the left button raised on an element, which the gestures of the
/// framework turn into taps.
pub(crate) struct MouseTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
}

impl MouseTestHelper {
    pub(crate) fn new() -> Self {
        Self { pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true), next_stamp: Cell::new(1) }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn root_of(target: &Interactive) -> Ref<Visual> {
        target.visual_root().unwrap_or_else(|| target.to_ref().upcast())
    }

    fn position_of(target: &Interactive) -> Point {
        let root = Self::root_of(target);
        let bounds = target.bounds();
        target.translate_point(Point::new(bounds.width / 2.0, bounds.height / 2.0), &root).unwrap_or_default()
    }

    /// Presses the left button on `target`.
    pub(crate) fn down(&self, target: &Interactive, click_count: i32) {
        let root = Self::root_of(target);
        let element: Option<Ref<InputElement>> = target.to_ref().cast::<InputElement>();
        self.pointer.capture_with_source(element.as_ref(), CaptureSource::Explicit);
        target.raise_event(&PointerPressedEventArgs::new(
            target.to_ref(),
            self.pointer.clone(),
            &root,
            Self::position_of(target),
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            click_count,
        ));
    }

    /// Releases the left button on `target`.
    pub(crate) fn up(&self, target: &Interactive) {
        let root = Self::root_of(target);
        target.raise_event(&PointerReleasedEventArgs::new(
            target.to_ref(),
            self.pointer.clone(),
            &root,
            Self::position_of(target),
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
        self.pointer.capture_with_source(None, CaptureSource::Explicit);
    }

    /// A click (`window.Click(target)`).
    pub(crate) fn click(&self, target: &Interactive) {
        self.down(target, 1);
        self.up(target);
    }

    /// Two clicks in a row (`window.Click(target); window.Click(target)`): the second press is
    /// the second click.
    pub(crate) fn double_click(&self, target: &Interactive) {
        self.down(target, 1);
        self.up(target);
        self.down(target, 2);
        self.up(target);
    }
}

/// `new Window { Content = content }`.
pub(crate) fn window_with(content: Option<Ref<Control>>) -> Ref<Window> {
    let window = Window::new();
    if let Some(content) = content {
        window.set_content(Some(Control::boxed(content)));
    }
    window
}
