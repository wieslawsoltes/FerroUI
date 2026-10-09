//! Shared helpers of the tests: the application scopes and the bodies of
//! the generated per-document tests.

use crate::markup::{describe, try_load_document, try_load_text, XamlClass};
use crate::register_types;
use crate::App;
use ferroui_base::controls::IResourceProvider;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::{
    CompositorTestServices, TestIconLoader, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{Application, Control, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

/// Starts a unit test application with the services of a styled window but
/// without a theme, and with the run-time loader registered.
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    let mut services = TestServices::styled_window();
    services.theme = None;
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    scope
}

/// Starts a unit test application with the services of a styled window
/// (with the Skia render interface and font manager and the HarfBuzz text
/// shaper in place of the mock ones, so that bitmaps decode and text is
/// laid out, and with the icon loader of tests, which loads the icon of the
/// tray icon of `App.xaml`), the Simple theme as the theme of the application and the resources the
/// application of the sample gives its pages (`CustomThemes.xaml`, which
/// `App.xaml` merges).
pub fn start_catalog_application() -> UnitTestApplicationScope {
    start_catalog_application_with(None)
}

/// [`start_catalog_application`] with another asset loader than the
/// standard one.
pub fn start_catalog_application_with(asset_loader: Option<Rc<dyn IAssetLoader>>) -> UnitTestApplicationScope {
    let scope = start_catalog_services(asset_loader);
    FerroRuntimeXamlLoader::register();
    merge_custom_themes();
    scope
}

/// Starts the unit test application of [`start_catalog_application_with`]
/// without registering the run-time loader and without the resources of the
/// application, as the application of the sample starts before it loads
/// `App.xaml`.
pub fn start_catalog_services(asset_loader: Option<Rc<dyn IAssetLoader>>) -> UnitTestApplicationScope {
    register_types();
    let mut services = catalog_services().with_theme(|| SimpleTheme::new().as_style());
    if let Some(asset_loader) = asset_loader {
        services = services.with_asset_loader(asset_loader);
    }
    UnitTestApplication::start(services)
}

/// The test services of the catalog: the services of a styled window with
/// the Skia render interface and font manager and the HarfBuzz text shaper
/// in place of the mock ones, a global clock that never ticks and the icon
/// loader of tests. The theme is the one of the styled window.
fn catalog_services() -> TestServices {
    catalog_services_with_clock(Rc::new(TestGlobalClock::default()))
}

/// [`catalog_services`] with `clock` as the global clock.
pub(super) fn catalog_services_with_clock(clock: Rc<TestGlobalClock>) -> TestServices {
    TestServices::styled_window()
        .with_render_interface(Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)))
        .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
        .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()))
        .with_global_clock(clock)
        .with_icon_loader(Rc::new(TestIconLoader))
}

/// Starts the application of the catalog ([`App`]) as the application of
/// the test, with the test services of the catalog in place of the services
/// of a platform: the application populates itself from `App.xaml` (which
/// merges `CustomThemes.xaml`) and applies the Fluent theme, as it does when
/// the application builder starts it. For what reads the application class,
/// as `{x:Static local:App.CurrentTheme}` does. The run-time loader is
/// registered for the tests that load a document with it: the application
/// itself registers it only when it loads its own documents at run time.
pub fn start_catalog_app() -> UnitTestApplicationScope {
    register_types();
    // The themes are the ones of the application.
    let mut services = catalog_services();
    services.theme = None;
    let scope = UnitTestApplication::start_with(services, || App::new().upcast());
    FerroRuntimeXamlLoader::register();
    scope
}

/// The application the generated tests of a document start; see
/// `test_applications.txt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestApplication {
    /// [`start_catalog_application`].
    UnitTest,
    /// [`start_catalog_app`].
    Catalog,
}

impl TestApplication {
    fn start(self) -> UnitTestApplicationScope {
        match self {
            TestApplication::UnitTest => start_catalog_application(),
            TestApplication::Catalog => start_catalog_app(),
        }
    }
}

/// Starts the application of [`start_catalog_application`] under the Fluent
/// theme (the theme the application of the sample starts with), with the
/// top-levels rendering through a compositing renderer over a test
/// compositor instead of the null renderer.
pub fn start_catalog_compositor_application() -> CompositorTestServices {
    start_catalog_compositor_application_with_clock(Rc::new(TestGlobalClock::default()))
}

/// [`start_catalog_compositor_application`] with `clock` as the global
/// clock: the test ticks it ([`TestGlobalClock::pulse`]), and the animations
/// and page transitions of the application run to their end.
pub fn start_catalog_compositor_application_with_clock(clock: Rc<TestGlobalClock>) -> CompositorTestServices {
    register_types();
    let services = catalog_services_with_clock(clock)
        .with_input_manager(Rc::new(ferroui_base::input::InputManager::new()))
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style());
    let services = CompositorTestServices::start(services);
    FerroRuntimeXamlLoader::register();
    merge_custom_themes();
    services
}

/// Merges the resources the application of the sample gives its pages
/// (`CustomThemes.xaml`, which `App.xaml` merges) into the resources of the
/// application.
fn merge_custom_themes() {
    let application = Application::current().expect("the unit test application");
    let custom_themes = match try_load_document("/CustomThemes.xaml", None) {
        Ok(value) => value,
        Err(error) => panic!("/CustomThemes.xaml: {}", describe(&error)),
    };
    let provider = from_markup_value::<Rc<dyn IResourceProvider>>(&Some(custom_themes))
        .expect("CustomThemes.xaml is a resource provider");
    application.resources().merged_dictionaries().add(provider);
}

/// Loads markup text; a failure panics with the described error.
#[track_caller]
pub fn load_text(xaml: &str) -> BoxedValue {
    match try_load_text(xaml, None, None) {
        Ok(value) => value,
        Err(error) => panic!("the document failed to load: {}", describe(&error)),
    }
}


/// The body of the generated test `document_<name>`: the document with the
/// rooted asset path `path` loads through the run-time loader, into a new
/// instance of its class when it names one, under `application`.
pub fn document_loads(path: &str, application: TestApplication) {
    let _app = application.start();
    let root = match crate::assets::documents().iter().find(|(document, _)| *document == path) {
        Some((_, Some(class_name))) => {
            let class = XamlClass::find(path)
                .unwrap_or_else(|| panic!("the class {class_name} of {path} is not declared (see excluded.txt)"));
            Some((class.create_uninitialized)())
        }
        _ => None,
    };
    if let Err(error) = try_load_document(path, root) {
        panic!("{path}: {}", describe(&error));
    }
}

/// The body of the generated test `class_<name>`: the class of the document
/// constructs (which loads the document) and can be shown: a control as the
/// content of a window, a window on its own, styles as the styles of a
/// window, under `application`.
pub fn class_constructs(path: &str, application: TestApplication) {
    let _app = application.start();
    let class = XamlClass::find(path).unwrap_or_else(|| panic!("the class of {path} is not declared"));
    let value = Some((class.create)());

    if let Some(window) = from_markup_value::<Ref<Window>>(&value) {
        window.show();
        window.close();
    } else if let Some(control) = from_markup_value::<Ref<Control>>(&value) {
        let window = Window::new();
        window.set_width(1100.0);
        window.set_height(800.0);
        window.set_content(Some(Control::boxed(&control)));
        window.show();
        assert!(control.is_attached_to_visual_tree(), "{path}: the control is not in the tree of the window");
        window.close();
    } else if let Some(styles) = from_markup_value::<Ref<Styles>>(&value) {
        let window = Window::new();
        let style: Rc<dyn IStyle> = ferroui_base::styling::styles_as_style(&styles);
        window.styles().add(style);
        window.show();
        window.close();
    } else if from_markup_value::<Ref<Application>>(&value).is_none() {
        panic!("{path}: the class is neither a control, a window, styles nor an application");
    }
}

/// The global clock of the tests: animations a page starts subscribe to
/// it; it ticks when the test tells it to ([`pulse`](Self::pulse)), which
/// most tests never do.
#[derive(Default)]
pub struct TestGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: std::cell::Cell<Option<ferroui_base::animation::PlayState>>,
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::TimeSpan> for TestGlobalClock {
    fn subscribe(
        &self,
        observer: Rc<dyn ferroui_base::reactive::IObserver<ferroui_base::animation::TimeSpan>>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl TestGlobalClock {
    /// Ticks the clock: its subscribers are told that the time is `time`.
    pub fn pulse(&self, time: ferroui_base::animation::TimeSpan) {
        use ferroui_base::reactive::IObserver;
        self.subject.on_next(time);
    }
}

impl ferroui_base::animation::IClock for TestGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get().unwrap_or(ferroui_base::animation::PlayState::Run)
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(Some(value))
    }
}

impl ferroui_base::animation::IGlobalClock for TestGlobalClock {}
