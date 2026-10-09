//! Not from upstream: the classes of the sample, populated by the compiled markup the
//! build script wrote and rustc compiled into this crate, against the same classes
//! populated by the run-time loader from the same documents (the files of the sample).
//!
//! The two object trees are compared by a dump: every object with its class, the
//! registered properties that are set on it with their values and priorities, and its
//! logical children, recursively.
//!
//! `documents` has one test per compiled document with a class (written by the build
//! script): [`compare`]. The other tests look closer at the pages a build without the
//! feature `catalog` compiles.

use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions as _;
use ferroui_base::{instantiate, BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, ObjectType, Ref, StyledElement};
use ferroui_base::controls::IResourceProvider;
use ferroui_base::metadata::from_markup_value;
use ferroui_controls::testing::{TestIconLoader, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, ItemsControl};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_themes_simple::SimpleTheme;

use crate::fixture_documents::{NOT_LOADED, PAGES};
use crate::markup::{describe, is_compiled, populate_compiled, try_load_document, XamlClass};
use crate::pages::{ButtonSpinnerPage, CanvasPage, CheckBoxPage, ImagePage, ProgressBarPage, SliderPage, WrapPanelPage};
use crate::view_models::WrapPanelPageViewModel;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/compiled_document_tests.rs"));
}

/// The global clock of the tests: animations a page starts subscribe to it; it never ticks.
#[derive(Default)]
struct TestGlobalClock {
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

impl ferroui_base::animation::IClock for TestGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get().unwrap_or(ferroui_base::animation::PlayState::Run)
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(Some(value))
    }
}

impl ferroui_base::animation::IGlobalClock for TestGlobalClock {}

/// The test services of the catalog, as the tests of the sample have them
/// (`samples/ControlCatalog/tests/support.rs`): the services of a styled window with the
/// Skia render interface and font manager and the HarfBuzz text shaper in place of the
/// mock ones (bitmaps decode, text is laid out), a global clock that never ticks and the
/// icon loader of tests.
fn catalog_services() -> TestServices {
    TestServices::styled_window()
        .with_render_interface(Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)))
        .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
        .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()))
        .with_global_clock(Rc::new(TestGlobalClock::default()))
        .with_icon_loader(Rc::new(TestIconLoader))
}

/// The application the test of a document starts, as the tests of the sample choose it
/// (`test_applications.txt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TestApplication {
    /// A unit test application with the test services of the catalog, the Simple theme as
    /// the theme of the application, the run-time loader registered, and the resources
    /// the application of the sample gives its pages (`CustomThemes.xaml`, which
    /// `App.xaml` merges).
    UnitTest,
    /// The application of the catalog (`App`) with the test services of the catalog: it
    /// loads `App.xaml` and applies the Fluent theme. For what reads the application class.
    Catalog,
}

impl TestApplication {
    fn start(self) -> UnitTestApplicationScope {
        crate::register_types();
        match self {
            TestApplication::UnitTest => {
                let scope = UnitTestApplication::start(catalog_services().with_theme(|| SimpleTheme::new().as_style()));
                FerroRuntimeXamlLoader::register();
                // The run-time loader loads `CustomThemes.xaml`, as in the tests of the sample,
                // also when the build compiled the document
                // (`the_resources_of_the_application_are_compiled` reads its compiled markup).
                // The first load of the run-time loader in a process also makes the
                // registrations of its type system (`Option<Vec<T>>` as the nullable form of a
                // list of values, among them), which the dump of a tree reads values with:
                // with it here, both trees of a test are dumped in the same state.
                let custom_themes = try_load_document("/CustomThemes.xaml", None).unwrap_or_else(|error| panic!("/CustomThemes.xaml: {}", describe(&error)));
                let provider = from_markup_value::<Rc<dyn IResourceProvider>>(&Some(custom_themes)).expect("CustomThemes.xaml is a resource provider");
                Application::current().expect("the unit test application").resources().merged_dictionaries().add(provider);
                scope
            }
            TestApplication::Catalog => {
                let mut services = catalog_services();
                services.theme = None;
                UnitTestApplication::start_with(services, || crate::App::new().upcast())
            }
        }
    }
}

/// The application of a test that looks at one page: [`TestApplication::UnitTest`].
fn application() -> UnitTestApplicationScope {
    TestApplication::UnitTest.start()
}

/// Populates `root` from the document `path` of the sample with the run-time loader, as
/// the sample loads it: the URI of the document, the assembly, compiled bindings as the
/// default.
fn load(path: &str, root: BoxedValue) -> Result<(), XamlLoadException> {
    try_load_document(path, Some(root)).map(|_| ())
}

/// Populates `root` from the compiled markup of the document `path`.
fn populate(path: &str, root: &BoxedValue) -> Result<(), XamlLoadException> {
    populate_compiled(path, root).unwrap_or_else(|| panic!("{path} is not compiled by the build"))
}

/// A value in display form, in its untyped form (the contents of a nullable, `null` for
/// none): an object of the object model as its class, anything else as the untyped value
/// conversions print it.
fn display(value: &BoxedValue) -> String {
    match ferroui_markup_xaml::xaml_il::runtime::compiled::to_untyped(value.clone()) {
        None => "null".to_string(),
        Some(value) => match ValueTypes::as_object(&*value) {
            Some(object) => format!("<{}>", object.get_type().full_name()),
            None => ValueTypes::to_display_string(Some(&value)),
        },
    }
}

/// The dump of an object and of its logical children.
fn dump(object: &Ref<FerroObject>, indent: usize, output: &mut String) {
    let pad = "  ".repeat(indent);
    let class = object.get_type();
    output.push_str(&format!("{pad}{}\n", class.full_name()));
    let registry = FerroPropertyRegistry::instance();
    let mut properties: Vec<&'static FerroProperty> = Vec::new();
    for property in registry.get_registered(class).iter().chain(registry.get_registered_attached(class).iter()) {
        if !property.is_direct() && !properties.iter().any(|known| std::ptr::eq(*known, *property)) {
            properties.push(*property);
        }
    }
    let mut lines: Vec<String> = Vec::new();
    for property in properties {
        if object.is_set(property) {
            let priority = object.get_diagnostic(property).priority();
            let value = display(&object.get_value_untyped(property));
            lines.push(format!("{pad}  {}.{} = {value} ({priority:?})\n", property.owner_type().name(), property.name()));
        }
    }
    lines.sort();
    lines.iter().for_each(|line| output.push_str(line));
    if let Some(styled) = object.cast::<StyledElement>() {
        if let Some(name) = styled.name() {
            output.push_str(&format!("{pad}  named '{name}'\n"));
        }
        for child in styled.logical_children().to_vec() {
            dump(&child.upcast::<FerroObject>(), indent + 1, output);
        }
    }
}

fn dump_of<T: ObjectType>(root: &Ref<T>) -> String {
    let mut output = String::new();
    dump(&root.clone().upcast::<FerroObject>(), 0, &mut output);
    output
}

/// The dump of the instance of a class held untyped.
fn dump_of_value(root: &BoxedValue) -> String {
    let object = ValueTypes::as_object(&**root).expect("the instance of a class of the object model");
    let mut output = String::new();
    dump(&object, 0, &mut output);
    output
}

/// The first line two dumps differ in.
fn first_difference(loaded: &str, compiled: &str) -> String {
    let (mut left, mut right) = (loaded.lines(), compiled.lines());
    let mut line = 1;
    loop {
        match (left.next(), right.next()) {
            (None, None) => return "no difference".to_string(),
            (a, b) if a == b => line += 1,
            (a, b) => return format!("line {line}: the run-time loader `{}`, the compiled markup `{}`", a.unwrap_or("<end>"), b.unwrap_or("<end>")),
        }
    }
}

/// The test of a compiled document with a class (`documents`): an instance of the class
/// populated by the compiled markup of the document and one populated by the run-time
/// loader from the document are the same tree. Both instances are created without the
/// body of the constructor of the class, as the root of a load of the document on its own.
///
/// Each tree is dumped before the other instance exists: two instances of a page are not
/// independent of each other (the radio buttons of a group that are in no visual tree are
/// one group, so the second page unchecks a button of the first).
///
/// A document `NOT_LOADED` lists is one neither back end loads in the services of a test:
/// both fail, with the same error.
pub(super) fn compare(path: &str, application: TestApplication) {
    let _application = application.start();
    let class = XamlClass::find(path).unwrap_or_else(|| panic!("{path}: no class of the sample has the document"));
    let from_code = {
        let compiled = (class.create_uninitialized)();
        populate(path, &compiled).map(|()| dump_of_value(&compiled))
    };
    let from_loader = {
        let loaded = (class.create_uninitialized)();
        load(path, loaded.clone()).map(|()| dump_of_value(&loaded))
    };
    let not_loaded = NOT_LOADED.iter().find(|(document, _)| path.strip_prefix('/') == Some(*document));
    match (from_code, from_loader, not_loaded) {
        (Ok(from_code), Ok(from_loader), None) => {
            assert!(from_code == from_loader, "{path}: the trees differ ({})\n--- the run-time loader\n{from_loader}--- the compiled markup\n{from_code}", first_difference(&from_loader, &from_code));
        }
        (Ok(_), Ok(_), Some((_, reason))) => panic!("{path}: both back ends load the document, which is listed as not loaded ({reason})"),
        (Err(from_code), Err(from_loader), listed) => {
            assert_eq!(from_code.message(), from_loader.message(), "{path}: the back ends fail with different errors");
            assert!(listed.is_some(), "{path}: neither back end loads the document, and it is not listed as not loaded: {}", from_loader.message());
        }
        (Ok(_), Err(error), _) => panic!("{path}: the compiled markup loads and the run-time loader fails: {error}"),
        (Err(error), Ok(_), _) => panic!("{path}: the run-time loader loads and the compiled markup fails: {error}"),
    }
}

/// An instance populated by the compiled markup of its class and one populated by the
/// run-time loader from the document `path`: both loads succeed and the trees are the
/// same. Returns the two instances and the dump.
fn populated<T: ObjectType>(path: &str, construct: fn() -> T) -> (Ref<T>, Ref<T>, String) {
    let compiled = instantiate(construct());
    populate(path, &(Rc::new(compiled.clone()) as BoxedValue)).unwrap_or_else(|error| panic!("{path}: the compiled markup fails: {error}"));
    let from_code = dump_of(&compiled);
    let loaded = instantiate(construct());
    load(path, Rc::new(loaded.clone())).unwrap_or_else(|error| panic!("{path}: the run-time loader fails: {error}"));
    let from_loader = dump_of(&loaded);
    assert!(from_code == from_loader, "{path}: the trees differ ({})\n--- the run-time loader\n{from_loader}--- the compiled markup\n{from_code}", first_difference(&from_loader, &from_code));
    (compiled, loaded, from_code)
}

/// The number of lines of a dump that name an object of the class `name`.
fn objects_of(dump: &str, name: &str) -> usize {
    dump.lines().filter(|line| line.trim() == name).count()
}

/// The document without a class (`CustomThemes.xaml`, the resources the application of the
/// sample gives its pages) is compiled by a build with the feature `catalog`: its compiled
/// markup, from the loader table of the crate, holds the resources the run-time loader
/// builds from the document, by their number.
#[cfg(feature = "catalog")]
#[test]
fn the_resources_of_the_application_are_compiled() {
    use ferroui_base::controls::IResourceDictionary;
    let _application = application();
    let count = |value: BoxedValue| from_markup_value::<Rc<dyn IResourceDictionary>>(&Some(value)).expect("CustomThemes.xaml is a resource dictionary").count();
    let compiled = crate::compiled_markup::try_load(None, &crate::markup::document_uri("/CustomThemes.xaml")).expect("the compiled markup loads");
    let compiled = count(compiled.expect("CustomThemes.xaml is compiled"));
    let loaded = count(try_load_document("/CustomThemes.xaml", None).unwrap_or_else(|error| panic!("{}", describe(&error))));
    assert!(compiled > 0);
    assert_eq!(compiled, loaded);
}

/// The pages of a build without the feature `catalog` are compiled by every build, and
/// each has its class in the table of the sample.
#[test]
fn the_pages_of_the_fixture_are_compiled() {
    crate::register_types();
    for page in PAGES {
        let path = format!("/{page}");
        assert!(is_compiled(&path), "{path} is not compiled");
        assert!(XamlClass::find(&path).is_some(), "{path} has no class");
    }
}

/// Two pages without code of their own: the tree of the compiled markup is the tree the
/// run-time loader builds from the document of the sample, and it is the tree of the
/// constructor of the sample, which calls `initialize_component()`.
#[test]
fn pages_without_code_are_the_trees_of_the_run_time_loader() {
    let _application = application();
    let (_, _, check_boxes) = populated("/Pages/CheckBoxPage.xaml", CheckBoxPage::construct);
    assert_eq!(objects_of(&check_boxes, "FerroUI.Controls.CheckBox"), 8, "{check_boxes}");
    assert!(check_boxes.contains("Page.Header = CheckBox (LocalValue)"), "{check_boxes}");
    assert_eq!(dump_of(&CheckBoxPage::new()), check_boxes);

    let (_, _, progress_bars) = populated("/Pages/ProgressBarPage.xaml", ProgressBarPage::construct);
    assert_eq!(objects_of(&progress_bars, "FerroUI.Controls.ProgressBar"), 5, "{progress_bars}");
    assert!(progress_bars.contains("named 'hprogress'"), "{progress_bars}");
    assert_eq!(dump_of(&ProgressBarPage::new()), progress_bars);
}

/// A page with a handler of its class (`Spin="OnSpin"`): the trees are the same, and a
/// spin of a spinner of the page populated by the compiled markup calls the method of the
/// class, which changes the text the spinner shows, as it does in the page the run-time
/// loader populated.
#[test]
fn button_spinner_page_calls_the_handler_of_its_class() {
    use ferroui_controls::{ButtonSpinner, SpinDirection, SpinEventArgs, Spinner, TextBlock};
    let _application = application();
    let (compiled, loaded, dump) = populated("/Pages/ButtonSpinnerPage.xaml", ButtonSpinnerPage::construct);
    assert!(objects_of(&dump, "FerroUI.Controls.ButtonSpinner") >= 1, "{dump}");
    let text_after_a_spin = |page: &Ref<ButtonSpinnerPage>| -> Option<String> {
        let spinner = page.get_logical_descendants().find_map(|logical| logical.cast::<ButtonSpinner>()).expect("a spinner of the page");
        let text_of = |spinner: &Ref<ButtonSpinner>| {
            spinner.content().and_then(|content| ValueTypes::as_object(&*content)).and_then(|content| content.cast::<TextBlock>()).and_then(|text| text.text())
        };
        let before = text_of(&spinner);
        spinner.raise_event(&SpinEventArgs::with_event(Some(&Spinner::spin_event()), SpinDirection::Increase));
        let after = text_of(&spinner);
        assert_ne!(before, after, "the handler of the class did not run");
        after
    };
    assert_eq!(text_after_a_spin(&compiled), text_after_a_spin(&loaded));
}

/// A page with a view model: compiled bindings to its properties, an items control bound
/// to its typed list (declared with `ferroui_controls::ferro_markup_list!`), an item
/// template whose data type is inferred from the list, styles with selectors. The trees
/// are the same before the data context is set, and after it with the same view model.
#[test]
fn wrap_panel_page_is_the_tree_of_the_run_time_loader() {
    let _application = application();
    let (compiled, loaded, dump) = populated("/Pages/WrapPanelPage.xaml", WrapPanelPage::construct);
    assert_eq!(objects_of(&dump, "FerroUI.Controls.Slider"), 4, "{dump}");

    let view_model = WrapPanelPageViewModel::new();
    let items = view_model.items().items().count();
    assert!(items > 0);
    for page in [&compiled, &loaded] {
        page.set_data_context(Some(view_model.clone() as BoxedValue));
    }
    let (from_code, from_loader) = (dump_of(&compiled), dump_of(&loaded));
    assert!(from_code == from_loader, "the trees differ with the view model ({})", first_difference(&from_loader, &from_code));
    // The bindings of the compiled markup deliver: the items control has the items of the list.
    let counts = |page: &Ref<WrapPanelPage>| -> Vec<usize> {
        page.get_logical_descendants().filter_map(|logical| logical.cast::<ItemsControl>()).map(|control| control.item_count() as usize).collect()
    };
    assert!(counts(&compiled).contains(&items), "{:?}", counts(&compiled));
    assert_eq!(counts(&compiled), counts(&loaded));
}

/// Two pages with a list written as text for a named collection that holds its list and
/// does not dereference to it (`Points` of a polygon and of a polyline, `Ticks` of a
/// slider): the members of the list are called with the collection converted, as the
/// run-time loader converts it, and the lists have the elements of the text.
#[test]
fn lists_written_as_text_fill_collections_that_do_not_dereference_to_their_list() {
    use ferroui_controls::shapes::Polygon;
    use ferroui_controls::Slider;
    let _application = application();
    let (compiled, loaded, dump) = populated("/Pages/CanvasPage.xaml", CanvasPage::construct);
    assert!(objects_of(&dump, "FerroUI.Controls.Shapes.Polygon") >= 1, "{dump}");
    let points = |page: &Ref<CanvasPage>| -> Vec<Vec<ferroui_base::Point>> {
        page.get_logical_descendants().filter_map(|logical| logical.cast::<Polygon>()).filter_map(|polygon| polygon.points()).map(|points| points.list().to_vec()).collect()
    };
    assert!(points(&compiled).iter().any(|points| points.len() == 5), "{:?}", points(&compiled));
    assert_eq!(points(&compiled), points(&loaded));

    let (compiled, loaded, dump) = populated("/Pages/SliderPage.xaml", SliderPage::construct);
    assert!(objects_of(&dump, "FerroUI.Controls.Slider") >= 1, "{dump}");
    let ticks = |page: &Ref<SliderPage>| -> Vec<Vec<f64>> {
        page.get_logical_descendants()
            .filter_map(|logical| logical.cast::<Slider>())
            .filter_map(|slider| slider.ticks())
            .map(|ticks| ticks.list().to_vec())
            .collect()
    };
    assert!(ticks(&compiled).contains(&vec![0.0, 20.0, 25.0, 40.0, 75.0, 100.0]), "{:?}", ticks(&compiled));
    assert_eq!(ticks(&compiled), ticks(&loaded));
}

/// A page with handlers of its class, named elements and bitmaps named by the paths of
/// assets (the text a type converter converts when the document is loaded; one of them
/// the source of a cropped bitmap): the trees are the same, and the images of the page
/// populated by the compiled markup have the bitmaps of the page the run-time loader
/// populated, by their sizes.
#[test]
fn image_page_loads_its_bitmaps() {
    use ferroui_controls::Image;
    let _application = application();
    let (compiled, loaded, dump) = populated("/Pages/ImagePage.xaml", ImagePage::construct);
    assert!(objects_of(&dump, "FerroUI.Controls.Image") >= 2, "{dump}");
    let sizes = |page: &Ref<ImagePage>| -> Vec<Option<ferroui_base::Size>> {
        page.get_logical_descendants().filter_map(|logical| logical.cast::<Image>()).map(|image| image.source().map(|source| source.size())).collect()
    };
    assert!(sizes(&compiled).iter().any(|size| size.is_some_and(|size| size.width > 0.0 && size.height > 0.0)), "{:?}", sizes(&compiled));
    assert_eq!(sizes(&compiled), sizes(&loaded));
}
