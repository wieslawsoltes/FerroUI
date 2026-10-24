//! The test application (`TestAppBuilder.cs`, `App.xaml` and `HeadlessWindowExtensions.cs` of the
//! reference test project).
//!
//! The reference runs its tests on the headless platform with a window whose theme is the Fluent
//! theme, and simulates the mouse through the input of the headless window; here they run in the
//! unit test application of the framework, with the services of a styled window, and the mouse
//! raises the pointer events on the element directly (as the mouse helper of the controls tests
//! of the framework does).

use ferroui_base::controls::NameScope;
use ferroui_base::input::{
    IPointer, KeyModifiers, MouseButton, Pointer, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::input::InputElement;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroObject, ObjectType, Point, Ref, Visual};
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
    // The application of the reference's tests has the Fluent theme: items controls need their
    // templates to realize containers.
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

/// Loads a window of a test with the run-time loader (the `InitializeComponent` of the windows of
/// the reference).
pub(crate) fn load_window(name: &str, xaml: &str) -> Ref<Window> {
    FerroRuntimeXamlLoader::register();
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("{name} failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("{name} failed to load: {}", error.message()),
        });
    from_markup_value::<Ref<Window>>(&Some(loaded)).expect("a window")
}

/// The element named `name` in the name scope of `root` (the field `x:Name` generates in the
/// reference).
pub(crate) fn find<T: ObjectType>(root: &Window, name: &str) -> Ref<T> {
    NameScope::get_name_scope(root)
        .and_then(|scope| scope.find(name))
        .and_then(|object| object.cast::<T>())
        .unwrap_or_else(|| panic!("the element {name}"))
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
        self.down_over(target, target, position, modifiers);
    }

    /// Presses the left button at `position` relative to `relative_to`, raising the event on
    /// `receiver` (the element under the pointer that the headless window of the reference finds
    /// by hit testing).
    pub(crate) fn down_over(&self, receiver: &Control, relative_to: &Control, position: Point, modifiers: KeyModifiers) {
        self.pressed_buttons.set(self.pressed_buttons.get() | RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.pressed_button.set(MouseButton::Left);
        let properties = PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::LeftButtonPressed);
        let root = Self::root_of(relative_to);
        receiver.raise_event(&PointerPressedEventArgs::new(
            receiver.to_ref(),
            self.pointer(),
            &root,
            Self::root_position(relative_to, &root, position),
            self.timestamp(),
            properties,
            modifiers,
            1,
        ));
    }

    /// Moves the pointer over `target` to `position`.
    pub(crate) fn move_to(&self, target: &Control, position: Point) {
        self.move_over(target, target, position);
    }

    /// Moves the pointer to `position` relative to `relative_to`, raising the event on `receiver`
    /// (the element that captured the pointer, or the element under it).
    pub(crate) fn move_over(&self, receiver: &Control, relative_to: &Control, position: Point) {
        let root = Self::root_of(relative_to);
        receiver.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            receiver.to_ref(),
            self.pointer(),
            Some(&*root),
            Self::root_position(relative_to, &root, position),
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    /// Releases the left button over `target` at `position`.
    pub(crate) fn up(&self, target: &Control, position: Point) {
        self.up_over(target, target, position);
    }

    /// Releases the left button at `position` relative to `relative_to`, raising the event on
    /// `receiver` (the element that captured the pointer, or the element under it).
    pub(crate) fn up_over(&self, receiver: &Control, relative_to: &Control, position: Point) {
        let left = RawInputModifiers::LEFT_MOUSE_BUTTON;
        self.pressed_buttons.set((self.pressed_buttons.get() | left) ^ left);
        let properties =
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::LeftButtonReleased);
        let root = Self::root_of(relative_to);
        receiver.raise_event(&PointerReleasedEventArgs::new(
            receiver.to_ref(),
            self.pointer(),
            &root,
            Self::root_position(relative_to, &root, position),
            self.timestamp(),
            properties,
            KeyModifiers::NONE,
            self.pressed_button.get(),
        ));
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
