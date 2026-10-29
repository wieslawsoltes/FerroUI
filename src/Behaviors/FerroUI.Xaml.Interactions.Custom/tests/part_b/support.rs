//! The test application of the tests of part B (`TestAppBuilder.cs`, the application document and
//! `HeadlessWindowExtensions.cs` of the reference test project).
//!
//! The reference runs its tests on the headless platform with the Fluent theme and simulates the
//! input of a keyboard and a mouse on the window. Here the tests run in the unit test application
//! of the framework, with the services of a styled window, the Fluent theme and a global clock
//! that never pulses; the windows of the tests are documents loaded with the run-time loader; a
//! key press is raised on the element directly, and a click of the mouse is a press and a release
//! of the pointer raised on the element (as the mouse helper of the controls tests of the
//! framework does).

use crate::ASSEMBLY;
use ferroui_base::controls::NameScope;
use ferroui_base::input::{
    CaptureSource, IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Point, Ref, StyledElement, Upcast, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_xaml_interactivity::Interaction;
use std::cell::Cell;
use std::rc::Rc;

/// Starts the application of a test (the headless test attribute of the reference), with the
/// Fluent theme of the reference's test application.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    let scope = UnitTestApplication::start(fluent_services());
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

/// Adds `behavior` to the behaviors of `object` (`Interaction.GetBehaviors(object).Add(behavior)`).
pub(crate) fn add_behavior<T: ObjectType + Upcast<FerroObject>>(object: &FerroObject, behavior: &Ref<T>) {
    Interaction::get_behaviors(object).add(behavior.clone().upcast());
}

/// `new Window { Content = content, Width = width, Height = height }`, shown and laid out
/// (`window.Show(); window.CaptureRenderedFrame()`).
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

/// `new Window { Content = content }`, shown (`window.Show()`).
pub(crate) fn show_window_with(content: impl ferroui_base::IntoRef<Control>) -> Ref<Window> {
    let window = Window::new();
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

/// Raises the key down event of `key` with `modifiers` on `target` (a `KeyEventArgs` raised on the
/// element, or `KeyPressQwerty` of the headless window of the reference on the focused element).
/// Returns the args, whose handled flag the handlers set.
pub(crate) fn key_down(target: &Interactive, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = modifiers;
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.set_source(target.to_ref());
    target.raise_event(&e);
    e
}

/// Raises the key up event of `key` with `modifiers` on `target`. Returns the args.
pub(crate) fn key_up(target: &Interactive, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = modifiers;
    e.set_routed_event(Some(InputElement::key_up_event()));
    e.set_source(target.to_ref());
    target.raise_event(&e);
    e
}

/// A key press on `target`: the key down event, then the key up event.
pub(crate) fn key_press(target: &Interactive, key: Key, modifiers: KeyModifiers) {
    key_down(target, key, modifiers);
    key_up(target, key, modifiers);
}

/// The mouse of the tests (`Click`, `MouseDown` and `MouseUp` of `HeadlessWindowExtensions.cs`): a
/// press and a release of a button raised on an element, which the gestures of the framework turn
/// into taps. A position is relative to the element; without one it is the centre of the element.
pub(crate) struct MouseTestHelper {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    click_count: Cell<i32>,
}

impl MouseTestHelper {
    pub(crate) fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            next_stamp: Cell::new(1),
            click_count: Cell::new(0),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn root_of(target: &Interactive) -> Ref<Visual> {
        target.visual_root().unwrap_or_else(|| target.to_ref().upcast())
    }

    fn centre_of(target: &Interactive) -> Point {
        let bounds = target.bounds();
        Point::new(bounds.width / 2.0, bounds.height / 2.0)
    }

    fn root_position(target: &Interactive, root: &Visual, position: Point) -> Point {
        target.translate_point(position, root).unwrap_or(position)
    }

    /// Presses `button` on `target` at `position` (relative to `target`) with `modifiers`. Each
    /// press after a release counts as the next click, so two clicks in a row are a double click.
    pub(crate) fn down_at(&self, target: &Interactive, position: Point, button: MouseButton, modifiers: KeyModifiers) {
        let root = Self::root_of(target);
        let element: Option<Ref<InputElement>> = target.to_ref().cast::<InputElement>();
        self.pointer.capture_with_source(element.as_ref(), CaptureSource::Explicit);
        let (raw, kind) = match button {
            MouseButton::Right => (RawInputModifiers::RIGHT_MOUSE_BUTTON, PointerUpdateKind::RightButtonPressed),
            MouseButton::Middle => (RawInputModifiers::MIDDLE_MOUSE_BUTTON, PointerUpdateKind::MiddleButtonPressed),
            _ => (RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        };
        self.click_count.set(self.click_count.get() + 1);
        target.raise_event(&PointerPressedEventArgs::new(
            target.to_ref(),
            self.pointer.clone(),
            &root,
            Self::root_position(target, &root, position),
            self.timestamp(),
            PointerPointProperties::new(raw, kind),
            modifiers,
            self.click_count.get(),
        ));
    }

    /// The element the pointer events go to: the element that captured the pointer, else the
    /// element under the pointer (the mouse device of the headless platform of the reference).
    fn receiver_of(&self, target: &Interactive) -> Ref<Interactive> {
        match self.pointer.captured() {
            Some(captured) => captured.upcast(),
            None => target.to_ref(),
        }
    }

    /// Moves the pointer over `target` to `position` (relative to `target`) with the buttons
    /// `buttons` pressed (`MouseMove` of the reference); the event goes to the element that
    /// captured the pointer, if any.
    pub(crate) fn move_at(&self, target: &Interactive, position: Point, buttons: RawInputModifiers) {
        let root = Self::root_of(target);
        let receiver = self.receiver_of(target);
        receiver.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            receiver.clone(),
            self.pointer.clone(),
            Some(&*root),
            Self::root_position(target, &root, position),
            self.timestamp(),
            PointerPointProperties::new(buttons, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    /// Releases `button` over `target` at `position` (relative to `target`) with `modifiers`; the
    /// event goes to the element that captured the pointer, if any.
    pub(crate) fn up_at(&self, target: &Interactive, position: Point, button: MouseButton, modifiers: KeyModifiers) {
        let root = Self::root_of(target);
        let receiver = self.receiver_of(target);
        let kind = match button {
            MouseButton::Right => PointerUpdateKind::RightButtonReleased,
            MouseButton::Middle => PointerUpdateKind::MiddleButtonReleased,
            _ => PointerUpdateKind::LeftButtonReleased,
        };
        receiver.raise_event(&PointerReleasedEventArgs::new(
            receiver.clone(),
            self.pointer.clone(),
            &root,
            Self::root_position(target, &root, position),
            self.timestamp(),
            PointerPointProperties::new(RawInputModifiers::NONE, kind),
            modifiers,
            button,
        ));
        self.pointer.capture_with_source(None, CaptureSource::Explicit);
    }

    /// Presses the left button on the centre of `target`.
    pub(crate) fn down(&self, target: &Interactive) {
        self.down_at(target, Self::centre_of(target), MouseButton::Left, KeyModifiers::NONE);
    }

    /// Releases the left button on the centre of `target`.
    pub(crate) fn up(&self, target: &Interactive) {
        self.up_at(target, Self::centre_of(target), MouseButton::Left, KeyModifiers::NONE);
    }

    /// A click of the left button on the centre of `target` (`window.Click(target)`).
    pub(crate) fn click(&self, target: &Interactive) {
        self.click_with(target, KeyModifiers::NONE);
    }

    /// A click of the left button on the centre of `target` with `modifiers`
    /// (`window.Click(target, modifiers: ..)`).
    pub(crate) fn click_with(&self, target: &Interactive, modifiers: KeyModifiers) {
        let centre = Self::centre_of(target);
        self.down_at(target, centre, MouseButton::Left, modifiers);
        self.up_at(target, centre, MouseButton::Left, modifiers);
    }

    /// Starts counting clicks again (a pause longer than the double click time).
    pub(crate) fn reset_click_count(&self) {
        self.click_count.set(0);
    }
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

/// The services of the reference's test application: the Fluent theme and a global clock, with the
/// keyboard focus and navigation of the headless platform of the reference.
fn fluent_services() -> TestServices {
    let clock = Rc::new(MockGlobalClock {
        subject: ferroui_base::reactive::LightweightSubject::new(),
        play_state: Cell::new(ferroui_base::animation::PlayState::Run),
    });
    let mut services = TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style());
    let focus = TestServices::focusable_window();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    services
}
