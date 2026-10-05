//! The calls of the run-time XAML loader the suites make, as thin wrappers
//! over [`FerroRuntimeXamlLoader`].
//!
//! The `try_*` forms return the load error (for tests that assert a
//! failure); the plain forms panic with the described error, as an
//! exception that escapes a test does.
//!
//! A failed load is a [`XamlLoadException`] whose inner error is the error
//! of the compiler ([`XamlError`]: the XML, parse, transform and load
//! exceptions of the managed original); [`xaml_error`] returns it.

use std::any::type_name;

use ferroui_base::metadata::{from_markup_value, MarkupAssembly};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use xamlx::exceptions::XamlError;

/// The result of a load.
pub type XamlResult<T> = Result<T, XamlLoadException>;

/// The error of the compiler a failed load carries, when it failed in the
/// compiler (and not, for example, in the cast of the loaded object).
pub fn xaml_error(error: &XamlLoadException) -> Option<&XamlError> {
    error.inner_exception().and_then(|inner| inner.downcast_ref::<XamlError>())
}

/// The assembly of the test project: `typeof(TestClass).Assembly`.
pub fn test_assembly() -> &'static MarkupAssembly {
    crate::register_types();
    &crate::ASSEMBLY
}

/// A load error as text: message, and the type, message, position and
/// inner errors of the error of the compiler.
pub fn describe(error: &XamlLoadException) -> String {
    fn describe_xaml(error: &XamlError) -> String {
        let mut text = format!("{}: {}", error.type_name(), error.message());
        if let (Some(line), Some(position)) = (error.line_number(), error.line_position()) {
            text.push_str(&format!(" (line {line}, position {position})"));
        }
        if let Some(inner) = error.inner_exception() {
            text.push_str(&format!("\n ---> {}", describe_xaml(inner)));
        }
        text
    }

    match (xaml_error(error), error.inner_exception()) {
        (Some(inner), _) => format!("{}\n ---> {}", error.message(), describe_xaml(inner)),
        (None, Some(inner)) => format!("{}\n ---> {inner}", error.message()),
        (None, None) => error.message().to_string(),
    }
}

/// Unwraps the result of a load; a failure panics with the described error.
#[track_caller]
pub fn expect_loaded<T>(result: XamlResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("the document failed to load: {}", describe(&error)),
    }
}

/// The cast `(T)value` of a loaded object to the handle type `T`
/// (`Ref<Border>`, `Rc<Style>`, ...).
#[track_caller]
pub fn cast<T: Clone + 'static>(value: &BoxedValue) -> T {
    try_cast(value).unwrap_or_else(|| {
        panic!("Unable to cast object of type '{}' to type '{}'.", value.type_name(), type_name::<T>())
    })
}

/// The cast `value as T` of a loaded object.
pub fn try_cast<T: Clone + 'static>(value: &BoxedValue) -> Option<T> {
    from_markup_value::<T>(&Some(value.clone()))
}

/// A URI from text (absolute or relative).
#[track_caller]
pub fn uri(text: &str) -> Uri {
    Uri::new(text, UriKind::RelativeOrAbsolute).unwrap_or_else(|_| panic!("invalid URI '{text}'"))
}

/// `FerroRuntimeXamlLoader.Load(xaml, localAssembly, rootInstance, uri, designMode)`.
pub fn try_load_with(
    xaml: &str,
    local_assembly: Option<&'static MarkupAssembly>,
    root_instance: Option<BoxedValue>,
    uri: Option<Uri>,
    design_mode: bool,
) -> XamlResult<BoxedValue> {
    crate::register_types();
    FerroRuntimeXamlLoader::load(xaml, local_assembly, root_instance, uri, design_mode)
}

/// `FerroRuntimeXamlLoader.Load(xaml)`.
pub fn try_load(xaml: &str) -> XamlResult<BoxedValue> {
    try_load_with(xaml, None, None, None, false)
}

/// `FerroRuntimeXamlLoader.Load(xaml)`.
#[track_caller]
pub fn load(xaml: &str) -> BoxedValue {
    expect_loaded(try_load(xaml))
}

/// `(T)FerroRuntimeXamlLoader.Load(xaml)`.
#[track_caller]
pub fn load_as<T: Clone + 'static>(xaml: &str) -> T {
    cast(&load(xaml))
}

/// `FerroRuntimeXamlLoader.Load(xaml, typeof(TestClass).Assembly)`.
pub fn try_load_local(xaml: &str) -> XamlResult<BoxedValue> {
    try_load_with(xaml, Some(test_assembly()), None, None, false)
}

/// `FerroRuntimeXamlLoader.Load(xaml, typeof(TestClass).Assembly)`.
#[track_caller]
pub fn load_local(xaml: &str) -> BoxedValue {
    expect_loaded(try_load_local(xaml))
}

/// `(T)FerroRuntimeXamlLoader.Load(xaml, typeof(TestClass).Assembly)`.
#[track_caller]
pub fn load_local_as<T: Clone + 'static>(xaml: &str) -> T {
    cast(&load_local(xaml))
}

/// `FerroRuntimeXamlLoader.Load(xaml, localAssembly, rootInstance)`: the
/// document populates `root_instance` (a boxed handle).
pub fn try_load_with_root(
    xaml: &str,
    local_assembly: Option<&'static MarkupAssembly>,
    root_instance: BoxedValue,
) -> XamlResult<BoxedValue> {
    try_load_with(xaml, local_assembly, Some(root_instance), None, false)
}

/// `FerroRuntimeXamlLoader.Load(xaml, localAssembly, rootInstance)`.
#[track_caller]
pub fn load_with_root(xaml: &str, local_assembly: Option<&'static MarkupAssembly>, root_instance: BoxedValue) -> BoxedValue {
    expect_loaded(try_load_with_root(xaml, local_assembly, root_instance))
}

/// `FerroRuntimeXamlLoader.Load(xaml, designMode: true)`.
pub fn try_load_design_mode(xaml: &str) -> XamlResult<BoxedValue> {
    try_load_with(xaml, None, None, None, true)
}

/// `FerroRuntimeXamlLoader.Parse(xaml, localAssembly)`.
pub fn try_parse_untyped(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> XamlResult<BoxedValue> {
    crate::register_types();
    FerroRuntimeXamlLoader::parse(xaml, local_assembly)
}

/// `FerroRuntimeXamlLoader.Parse<T>(xaml, localAssembly)`.
pub fn try_parse_with<T: Clone + 'static>(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> XamlResult<T> {
    crate::register_types();
    FerroRuntimeXamlLoader::parse_as::<T>(xaml, local_assembly)
}

/// `FerroRuntimeXamlLoader.Parse<T>(xaml)`.
pub fn try_parse<T: Clone + 'static>(xaml: &str) -> XamlResult<T> {
    try_parse_with(xaml, None)
}

/// `FerroRuntimeXamlLoader.Parse<T>(xaml)`.
#[track_caller]
pub fn parse<T: Clone + 'static>(xaml: &str) -> T {
    expect_loaded(try_parse(xaml))
}

/// `FerroRuntimeXamlLoader.Parse<T>(xaml, typeof(TestClass).Assembly)`.
pub fn try_parse_local<T: Clone + 'static>(xaml: &str) -> XamlResult<T> {
    try_parse_with(xaml, Some(test_assembly()))
}

/// `FerroRuntimeXamlLoader.Parse<T>(xaml, typeof(TestClass).Assembly)`.
#[track_caller]
pub fn parse_local<T: Clone + 'static>(xaml: &str) -> T {
    expect_loaded(try_parse_local(xaml))
}

/// `new RuntimeXamlLoaderDocument(new Uri(baseUri), xaml)`.
#[track_caller]
pub fn document(base_uri: &str, xaml: &str) -> RuntimeXamlLoaderDocument {
    RuntimeXamlLoaderDocument::with_base_uri(Some(uri(base_uri)), xaml)
}

/// `new RuntimeXamlLoaderDocument(xaml)`.
pub fn document_without_uri(xaml: &str) -> RuntimeXamlLoaderDocument {
    RuntimeXamlLoaderDocument::new(xaml)
}

/// `new RuntimeXamlLoaderConfiguration { LocalAssembly = typeof(TestClass).Assembly }`.
pub fn local_configuration() -> RuntimeXamlLoaderConfiguration {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(test_assembly());
    configuration
}

/// `FerroRuntimeXamlLoader.Load(document, configuration)`.
pub fn try_load_document(
    document: RuntimeXamlLoaderDocument,
    configuration: Option<RuntimeXamlLoaderConfiguration>,
) -> XamlResult<BoxedValue> {
    crate::register_types();
    FerroRuntimeXamlLoader::load_document(document, configuration)
}

/// `FerroRuntimeXamlLoader.Load(document, configuration)`.
#[track_caller]
pub fn load_document(
    document: RuntimeXamlLoaderDocument,
    configuration: Option<RuntimeXamlLoaderConfiguration>,
) -> BoxedValue {
    expect_loaded(try_load_document(document, configuration))
}

/// `FerroRuntimeXamlLoader.LoadGroup(documents, configuration)`: the loaded
/// object of each document, in order.
pub fn try_load_group(
    documents: Vec<RuntimeXamlLoaderDocument>,
    configuration: Option<RuntimeXamlLoaderConfiguration>,
) -> XamlResult<Vec<Option<BoxedValue>>> {
    crate::register_types();
    FerroRuntimeXamlLoader::load_group(documents, configuration)
}

/// `FerroRuntimeXamlLoader.LoadGroup(documents, configuration)`.
#[track_caller]
pub fn load_group(
    documents: Vec<RuntimeXamlLoaderDocument>,
    configuration: Option<RuntimeXamlLoaderConfiguration>,
) -> Vec<Option<BoxedValue>> {
    expect_loaded(try_load_group(documents, configuration))
}

/// The loaded object at `index` of a group, cast to `T`.
#[track_caller]
pub fn group_item<T: Clone + 'static>(group: &[Option<BoxedValue>], index: usize) -> T {
    cast(group[index].as_ref().unwrap_or_else(|| panic!("document {index} of the group loaded no object")))
}
