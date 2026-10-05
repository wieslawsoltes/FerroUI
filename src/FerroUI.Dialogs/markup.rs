//! Loading the documents of the crate.
//!
//! TEMPORARY (until the XAML compiler exists): a class with compiled markup
//! is populated by generated code the compiler writes for its document, and
//! the constructor of the class calls it (`InitializeComponent()`). Until
//! then [`load_component`] populates the instance from the embedded
//! document with the run-time loader.

use crate::register_types::{register_types, ASSEMBLY};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// A load error as text, with the error of the compiler it carries.
pub(crate) fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}

/// Loads the embedded document with the rooted asset path `path` with the
/// run-time loader; `root_instance` is the instance the document populates.
pub(crate) fn try_load_document(path: &str, root_instance: Option<BoxedValue>) -> Result<BoxedValue, XamlLoadException> {
    register_types();
    let content = crate::assets::asset(path)
        .ok_or_else(|| XamlLoadException::with_message(format!("The resource {path} could not be found.")))?;
    let text = std::str::from_utf8(content)
        .map_err(|e| XamlLoadException::with_message(format!("{path} is not UTF-8: {e}")))?;

    let uri = Uri::absolute(&format!("ferres://{}{path}", ASSEMBLY.name))
        .map_err(|e| XamlLoadException::with_message(format!("Invalid document URI for {path}: {e}")))?;
    let mut document = RuntimeXamlLoaderDocument::with_base_uri(Some(uri), text);
    document.document = Some(path.trim_start_matches('/').to_string());
    document.root_instance = root_instance;

    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    // The upstream project is built with compiled bindings as the default of its documents.
    configuration.use_compiled_bindings_by_default = true;
    FerroRuntimeXamlLoader::load_document(document, Some(configuration))
}

/// What the generated `InitializeComponent()` of a class does: populates
/// `this` from the document of its class.
///
/// # Panics
/// Panics if the document fails to load (an exception of the constructor
/// in the managed original).
pub(crate) fn load_component<T: ObjectType>(this: &Ref<T>, path: &str) {
    let root: BoxedValue = Rc::new(this.clone());
    if let Err(error) = try_load_document(path, Some(root)) {
        panic!("{path}: {}", describe(&error));
    }
}
