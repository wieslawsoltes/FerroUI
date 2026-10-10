//! The documents of a sample under the run-time loader, and the bodies of the generated
//! per-document tests (`sample-build`). Every function runs under the application the test
//! started.
//!
//! The comparison of a class populated by its compiled markup with the same class populated
//! by the run-time loader is by a dump: every object with its class, the registered
//! properties that are set on it with their values and priorities, and its logical children,
//! recursively (as `samples/ControlCatalog/tests/compiled_markup.rs`; docs/porting/xaml.md,
//! 9.5.19 and 9.5.22).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions as _;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, StyledElement};
use ferroui_controls::{Application, Control, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use sample_support::{describe, Sample};
use std::rc::Rc;

fn document(sample: &Sample, xaml: &str, path: Option<&str>) -> Result<RuntimeXamlLoaderDocument, XamlLoadException> {
    let mut document = RuntimeXamlLoaderDocument::new(xaml);
    if let Some(path) = path {
        let uri = Uri::absolute(&sample.document_uri(path))
            .map_err(|e| XamlLoadException::with_message(format!("Invalid document URI for {path}: {e}")))?;
        document.base_uri = Some(uri);
        document.document = Some(path.trim_start_matches('/').to_string());
    }
    Ok(document)
}

fn configuration(sample: &'static Sample) -> RuntimeXamlLoaderConfiguration {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(sample.assembly);
    // The upstream samples are built with compiled bindings as the default of their documents.
    configuration.use_compiled_bindings_by_default = true;
    configuration
}

fn embedded_text(sample: &Sample, path: &str) -> Result<&'static str, XamlLoadException> {
    let content = sample.asset(path).ok_or_else(|| XamlLoadException::with_message(format!("The resource {path} could not be found.")))?;
    std::str::from_utf8(content).map_err(|e| XamlLoadException::with_message(format!("{path} is not UTF-8: {e}")))
}

/// Loads markup text as a document of the sample with the run-time loader; `path` is the
/// rooted asset path the text stands for and `root_instance` the instance the document
/// populates.
pub fn try_load_text(
    sample: &'static Sample,
    xaml: &str,
    path: Option<&str>,
    root_instance: Option<BoxedValue>,
) -> Result<BoxedValue, XamlLoadException> {
    (sample.register_types)();
    FerroRuntimeXamlLoader::register();
    let mut document = document(sample, xaml, path)?;
    document.root_instance = root_instance;
    FerroRuntimeXamlLoader::load_document(document, Some(configuration(sample)))
}

/// Loads markup text; a failure panics with the described error.
#[track_caller]
pub fn load_text(sample: &'static Sample, xaml: &str) -> BoxedValue {
    match try_load_text(sample, xaml, None, None) {
        Ok(value) => value,
        Err(error) => panic!("the document failed to load: {}", describe(&error)),
    }
}

/// Loads the embedded document with the rooted asset path `path` with the run-time loader;
/// `root_instance` is the instance the document populates.
pub fn try_load_document(sample: &'static Sample, path: &str, root_instance: Option<BoxedValue>) -> Result<BoxedValue, XamlLoadException> {
    try_load_text(sample, embedded_text(sample, path)?, Some(path), root_instance)
}

/// The body of the generated test `document_<name>`: the document with the rooted asset path
/// `path` loads through the run-time loader, into a new instance of its class when it names
/// one.
pub fn document_loads(sample: &'static Sample, path: &str) {
    (sample.register_types)();
    let root = match sample.documents.iter().find(|(document, _)| *document == path) {
        Some((_, Some(class_name))) => {
            let class = sample.find_class(path).unwrap_or_else(|| panic!("the class {class_name} of {path} is not declared (see excluded.txt)"));
            Some((class.create_uninitialized)())
        }
        _ => None,
    };
    if let Err(error) = try_load_document(sample, path, root) {
        panic!("{path}: {}", describe(&error));
    }
}

/// The body of the generated test `class_<name>`: the class of the document constructs
/// (which loads the document) and can be shown: a control as the content of a window, a
/// window on its own, styles as the styles of a window.
pub fn class_constructs(sample: &'static Sample, path: &str) {
    (sample.register_types)();
    let class = sample.find_class(path).unwrap_or_else(|| panic!("the class of {path} is not declared"));
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

/// The class of an object and the registered properties that are set on it, with their
/// values and priorities.
fn dump_object(object: &Ref<FerroObject>, indent: usize, output: &mut String) {
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
    if let Some(name) = object.cast::<StyledElement>().and_then(|styled| styled.name()) {
        output.push_str(&format!("{pad}  named '{name}'\n"));
    }
}

/// The dump of an object and of its logical children.
fn dump(object: &Ref<FerroObject>, indent: usize, output: &mut String) {
    dump_object(object, indent, output);
    if let Some(styled) = object.cast::<StyledElement>() {
        for child in styled.logical_children().to_vec() {
            dump(&child.upcast::<FerroObject>(), indent + 1, output);
        }
    }
}

/// The dump of the instance of a class held untyped.
pub fn dump_of_value(root: &BoxedValue) -> String {
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

/// The body of the generated test `compiled_<name>`: an instance of the class populated by
/// the compiled markup of the document and one populated by the run-time loader from the
/// document are the same tree. Both instances are created without the body of the
/// constructor of the class, as the root of a load of the document on its own, and each tree
/// is dumped before the other instance exists.
pub fn compare(sample: &'static Sample, path: &str) {
    (sample.register_types)();
    let class = sample.find_class(path).unwrap_or_else(|| panic!("{path}: no class of the sample has the document"));
    let from_code = {
        let compiled = (class.create_uninitialized)();
        let result = sample.populate_compiled(path, &compiled).unwrap_or_else(|| panic!("{path} is not compiled by the build"));
        result.map(|()| dump_of_value(&compiled))
    };
    let from_loader = {
        let loaded = (class.create_uninitialized)();
        try_load_document(sample, path, Some(loaded.clone())).map(|_| dump_of_value(&loaded))
    };
    match (from_code, from_loader) {
        (Ok(from_code), Ok(from_loader)) => {
            assert!(
                from_code == from_loader,
                "{path}: the trees differ ({})\n--- the run-time loader\n{from_loader}--- the compiled markup\n{from_code}",
                first_difference(&from_loader, &from_code)
            );
        }
        (Err(from_code), Err(from_loader)) => {
            panic!("{path}: neither back end loads the document: the compiled markup: {}; the run-time loader: {}", describe(&from_code), describe(&from_loader))
        }
        (Ok(_), Err(error)) => panic!("{path}: the compiled markup loads and the run-time loader fails: {}", describe(&error)),
        (Err(error), Ok(_)) => panic!("{path}: the run-time loader loads and the compiled markup fails: {}", describe(&error)),
    }
}
