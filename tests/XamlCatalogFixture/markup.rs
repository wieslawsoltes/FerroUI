//! What the classes of the sample name from the module `markup` of their crate
//! (`xaml_class!`, `user_control_class!`, `content_page_class!`, `XamlClass`, the loads of a
//! document by its path): the module of the sample, with one difference. Where the sample
//! populates a class from its embedded document with the run-time loader
//! ([`load_component`]), a class whose document the build of this crate compiled is
//! populated by its compiled markup ([`populate_compiled`]); a class whose document is not
//! compiled (`documents::REFUSED`, or a build without the feature `catalog`) is populated
//! by the run-time loader, as in the sample.

use crate::register_types::{register_types, ASSEMBLY};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, ObjectType, Ref, TypeInfo};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// A class of the sample with a document: the entry of the table of
/// compiled documents of the crate.
pub struct XamlClass {
    /// The rooted asset path of the document (`/Pages/BorderPage.xaml`).
    pub document: &'static str,
    /// The class.
    pub type_info: fn() -> &'static TypeInfo,
    /// Creates an instance with the constructor of the class (which loads
    /// the document), boxed as the handle of the class.
    pub create: fn() -> BoxedValue,
    /// Creates an instance without running the body of the constructor: the
    /// root instance of a load of the document on its own.
    pub create_uninitialized: fn() -> BoxedValue,
}

impl XamlClass {
    /// The class of the document with the rooted asset path `path`.
    pub fn find(path: &str) -> Option<&'static XamlClass> {
        crate::register_types::classes().find(|class| class.document.eq_ignore_ascii_case(path))
    }

    /// The URI of the document of the class.
    pub fn document_uri(&self) -> String {
        document_uri(self.document)
    }
}

/// Declares the document of a class: its entry in the table of the classes
/// with a document ([`XamlClass`]) and `initialize_component()`.
///
/// ```ignore
/// xaml_class!(BorderPage, "/Pages/BorderPage.xaml");
/// // A class without a parameterless constructor names how the table creates it:
/// xaml_class!(SectionPage, "/Pages/SectionPage.xaml", create: SectionPage::new(HomeSection::empty()));
/// ```
macro_rules! xaml_class {
    ($class:ident, $path:literal) => {
        $crate::markup::xaml_class!($class, $path, create: <$class>::new());
    };
    ($class:ident, $path:literal, create: $create:expr) => {
        impl $class {
            /// The rooted asset path of the document of the class.
            pub const DOCUMENT_PATH: &'static str = $path;

            /// The entry of the class in the table of the classes with a document.
            pub const XAML_CLASS: $crate::markup::XamlClass = $crate::markup::XamlClass {
                document: $path,
                type_info: || <$class>::TYPE,
                create: || ::std::rc::Rc::new($create) as ::ferroui_base::BoxedValue,
                create_uninitialized: || {
                    ::std::rc::Rc::new(::ferroui_base::instantiate(<$class>::construct())) as ::ferroui_base::BoxedValue
                },
            };

            /// `InitializeComponent()`: populates the instance from the
            /// document of the class.
            ///
            /// # Panics
            /// Panics if the document fails to load (an exception of the
            /// constructor in the managed original).
            #[allow(dead_code)]
            fn initialize_component(&self) {
                $crate::markup::load_component(&self.to_ref(), $path);
            }
        }
    };
}
pub(crate) use xaml_class;

/// Declares a class that derives directly from `UserControl`, without
/// overrides (the class declaration and the implementation traits of every
/// level below).
macro_rules! user_control_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: UserControl);
        ::ferroui_base::ferro_impl_classes!(
            $class: ::ferroui_base::FerroObjectImpl,
            ::ferroui_base::StyledElementImpl,
            ::ferroui_base::VisualImpl,
            ::ferroui_base::layout::LayoutableImpl,
            ::ferroui_base::interactivity::InteractiveImpl,
            ::ferroui_base::input::InputElementImpl,
            ::ferroui_controls::ControlImpl,
            ::ferroui_controls::primitives::TemplatedControlImpl,
            ::ferroui_controls::ContentControlImpl
        );
    };
}
pub(crate) use user_control_class;

/// Declares a class that derives directly from `ContentPage`, without
/// overrides.
macro_rules! content_page_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: ContentPage);
        ::ferroui_base::ferro_impl_classes!(
            $class: ::ferroui_base::FerroObjectImpl,
            ::ferroui_base::StyledElementImpl,
            ::ferroui_base::VisualImpl,
            ::ferroui_base::layout::LayoutableImpl,
            ::ferroui_base::interactivity::InteractiveImpl,
            ::ferroui_base::input::InputElementImpl,
            ::ferroui_controls::ControlImpl,
            ::ferroui_controls::primitives::TemplatedControlImpl,
            ::ferroui_controls::PageImpl
        );
    };
}
pub(crate) use content_page_class;

/// A load error as text, with the error of the compiler it carries.
pub fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}

/// The URI of the document with the rooted asset path `path`.
pub fn document_uri(path: &str) -> String {
    format!("ferres://{}{path}", ASSEMBLY.name)
}

fn document(xaml: &str, path: Option<&str>) -> Result<RuntimeXamlLoaderDocument, XamlLoadException> {
    let mut document = RuntimeXamlLoaderDocument::new(xaml);
    if let Some(path) = path {
        let uri = Uri::absolute(&document_uri(path))
            .map_err(|e| XamlLoadException::with_message(format!("Invalid document URI for {path}: {e}")))?;
        document.base_uri = Some(uri);
        document.document = Some(path.trim_start_matches('/').to_string());
    }
    Ok(document)
}

fn configuration() -> RuntimeXamlLoaderConfiguration {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    // The upstream sample is built with compiled bindings as the default of its documents.
    configuration.use_compiled_bindings_by_default = true;
    configuration
}

pub(crate) fn embedded_text(path: &str) -> Result<&'static str, XamlLoadException> {
    let content = crate::assets::asset(path)
        .ok_or_else(|| XamlLoadException::with_message(format!("The resource {path} could not be found.")))?;
    std::str::from_utf8(content).map_err(|e| XamlLoadException::with_message(format!("{path} is not UTF-8: {e}")))
}

/// Loads markup text as a document of the sample with the run-time loader;
/// `path` is the rooted asset path the text stands for and `root_instance`
/// the instance the document populates.
pub fn try_load_text(
    xaml: &str,
    path: Option<&str>,
    root_instance: Option<BoxedValue>,
) -> Result<BoxedValue, XamlLoadException> {
    register_types();
    let mut document = document(xaml, path)?;
    document.root_instance = root_instance;
    FerroRuntimeXamlLoader::load_document(document, Some(configuration()))
}

/// Loads the embedded document with the rooted asset path `path` with the
/// run-time loader; `root_instance` is the instance the document populates.
pub fn try_load_document(path: &str, root_instance: Option<BoxedValue>) -> Result<BoxedValue, XamlLoadException> {
    try_load_text(embedded_text(path)?, Some(path), root_instance)
}

/// Loads the embedded document `path` into `root_instance` together with
/// the embedded documents `included` it includes, as one group: the group
/// transformers then link the documents to each other as the compiler links
/// the documents of a project.
pub fn try_load_document_group(
    path: &str,
    root_instance: Option<BoxedValue>,
    included: &[&str],
) -> Result<BoxedValue, XamlLoadException> {
    let mut included_texts = Vec::new();
    for path in included {
        included_texts.push((*path, embedded_text(path)?));
    }
    try_load_text_group(embedded_text(path)?, path, root_instance, &included_texts)
}

/// [`try_load_document_group`] for markup texts that stand for embedded
/// documents: `xaml` for the document `path`, and `(path, text)` for each
/// included document.
pub fn try_load_text_group(
    xaml: &str,
    path: &str,
    root_instance: Option<BoxedValue>,
    included: &[(&str, &str)],
) -> Result<BoxedValue, XamlLoadException> {
    register_types();
    let mut root = document(xaml, Some(path))?;
    root.root_instance = root_instance;
    let mut documents = vec![root];
    for (path, text) in included {
        documents.push(document(text, Some(path))?);
    }
    let loaded = FerroRuntimeXamlLoader::load_group(documents, Some(configuration()))?;
    loaded
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| XamlLoadException::with_message(format!("The document {path} loaded no object.")))
}

/// What the generated `InitializeComponent()` of a class does: populates
/// `this` from the document of its class.
///
/// # Panics
/// Panics if the document fails to load.
pub fn load_component<T: ObjectType>(this: &Ref<T>, path: &str) {
    let root: BoxedValue = Rc::new(this.clone());
    let populated = match populate_compiled(path, &root) {
        Some(populated) => populated,
        None => try_load_document(path, Some(root)).map(|_| ()),
    };
    if let Err(error) = populated {
        panic!("{path}: {}", describe(&error));
    }
}

// `COMPILED`: the documents the build compiled, each with the function that populates an
// instance of its class from its compiled markup (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/compiled_classes.rs"));

/// Whether the build compiled the document with the rooted asset path `path`.
pub fn is_compiled(path: &str) -> bool {
    COMPILED.iter().any(|(document, _)| *document == path)
}

/// Populates `root`, an instance of the class of the document `path`, from the compiled
/// markup of the document; `None` when the build did not compile the document.
///
/// # Panics
/// Panics if `root` is not the handle of the class of the document.
pub fn populate_compiled(path: &str, root: &BoxedValue) -> Option<Result<(), XamlLoadException>> {
    register_types();
    COMPILED.iter().find(|(document, _)| *document == path).map(|(_, populate)| populate(root))
}
