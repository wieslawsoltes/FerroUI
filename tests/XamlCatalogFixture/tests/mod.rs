//! Not from upstream: the pages of the fixture, populated by the compiled markup the
//! build script wrote and rustc compiled into this crate, against the same classes
//! populated by the run-time loader from the same documents (the files of the sample).
//!
//! The two object trees are compared by a dump: every object with its class, the
//! registered properties that are set on it with their values and priorities, and its
//! logical children, recursively.

use std::path::Path;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions as _;
use ferroui_base::utilities::Uri;
use ferroui_base::{instantiate, BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, ObjectType, Ref, StyledElement};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::ItemsControl;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::{PAGES, SAMPLE};
use crate::markup::CompiledMarkup;
use crate::pages::{ButtonSpinnerPage, CanvasPage, CheckBoxPage, ImagePage, ProgressBarPage, SliderPage, WrapPanelPage};
use crate::view_models::WrapPanelPageViewModel;
use crate::ASSEMBLY;

/// The application of a test: the services of a styled window (an asset loader, the mock
/// render interface, which decodes no bitmap) and the run-time loader.
fn application() -> UnitTestApplicationScope {
    let application = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();
    application
}

/// The text of a document of the sample.
fn document_text(path: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE).join(path);
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()))
}

/// Populates `root` from the document `path` of the sample with the run-time loader, as
/// the sample loads it: the URI of the document, the assembly, compiled bindings as the
/// default.
fn load(path: &str, root: BoxedValue) -> Result<BoxedValue, XamlLoadException> {
    assert!(PAGES.iter().any(|page| page.path == path), "{path} is not a page of the fixture");
    let text = document_text(path);
    let mut document = RuntimeXamlLoaderDocument::new(&text);
    document.base_uri = Some(Uri::absolute(&format!("ferres://{}/{path}", ASSEMBLY.name)).expect("the URI of the document"));
    document.document = Some(path.to_string());
    document.root_instance = Some(root);
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    configuration.use_compiled_bindings_by_default = true;
    FerroRuntimeXamlLoader::load_document(document, Some(configuration))
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

/// An instance populated by the compiled markup of its class and one populated by the
/// run-time loader from the document `path`: both loads succeed and the trees are the
/// same. Returns the two instances and the dump.
fn populated<T: CompiledMarkup>(path: &str, construct: fn() -> T) -> (Ref<T>, Ref<T>, String) {
    let compiled = instantiate(construct());
    T::populate(&compiled).unwrap_or_else(|error| panic!("{path}: the compiled markup fails: {error}"));
    let loaded = instantiate(construct());
    load(path, Rc::new(loaded.clone())).unwrap_or_else(|error| panic!("{path}: the run-time loader fails: {error}"));
    let (from_code, from_loader) = (dump_of(&compiled), dump_of(&loaded));
    assert!(from_code == from_loader, "{path}: the trees differ ({})\n--- the run-time loader\n{from_loader}--- the compiled markup\n{from_code}", first_difference(&from_loader, &from_code));
    (compiled, loaded, from_code)
}

/// The number of lines of a dump that name an object of the class `name`.
fn objects_of(dump: &str, name: &str) -> usize {
    dump.lines().filter(|line| line.trim() == name).count()
}

/// Two pages without code of their own: the tree of the compiled markup is the tree the
/// run-time loader builds from the document of the sample, and it is the tree of the
/// constructor of the sample, which calls `initialize_component()`.
#[test]
fn pages_without_code_are_the_trees_of_the_run_time_loader() {
    let _application = application();
    let (_, _, check_boxes) = populated("Pages/CheckBoxPage.xaml", CheckBoxPage::construct);
    assert_eq!(objects_of(&check_boxes, "FerroUI.Controls.CheckBox"), 8, "{check_boxes}");
    assert!(check_boxes.contains("Page.Header = CheckBox (LocalValue)"), "{check_boxes}");
    assert_eq!(dump_of(&CheckBoxPage::new()), check_boxes);

    let (_, _, progress_bars) = populated("Pages/ProgressBarPage.xaml", ProgressBarPage::construct);
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
    let (compiled, loaded, dump) = populated("Pages/ButtonSpinnerPage.xaml", ButtonSpinnerPage::construct);
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
    let (compiled, loaded, dump) = populated("Pages/WrapPanelPage.xaml", WrapPanelPage::construct);
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
    let (compiled, loaded, dump) = populated("Pages/CanvasPage.xaml", CanvasPage::construct);
    assert!(objects_of(&dump, "FerroUI.Controls.Shapes.Polygon") >= 1, "{dump}");
    let points = |page: &Ref<CanvasPage>| -> Vec<Vec<ferroui_base::Point>> {
        page.get_logical_descendants().filter_map(|logical| logical.cast::<Polygon>()).filter_map(|polygon| polygon.points()).map(|points| points.list().to_vec()).collect()
    };
    assert!(points(&compiled).iter().any(|points| points.len() == 5), "{:?}", points(&compiled));
    assert_eq!(points(&compiled), points(&loaded));

    let (compiled, loaded, dump) = populated("Pages/SliderPage.xaml", SliderPage::construct);
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
/// assets (the text a type converter converts when the document is loaded). No service
/// of a test decodes a bitmap and the fixture embeds no asset, so the load fails in the
/// converter: with the same error from both back ends, which names the asset by its
/// path.
#[test]
fn image_page_fails_in_the_converter_as_the_run_time_loader_does() {
    let _application = application();
    let path = "Pages/ImagePage.xaml";
    let compiled = ImagePage::populate(&instantiate(ImagePage::construct())).expect_err("the compiled markup loads a bitmap");
    let loaded = load(path, Rc::new(instantiate(ImagePage::construct()))).expect_err("the run-time loader loads a bitmap");
    assert_eq!(compiled.message(), loaded.message());
    assert!(compiled.message().contains("/Assets/delicate-arch-896885_640.jpg"), "{}", compiled.message());
}
