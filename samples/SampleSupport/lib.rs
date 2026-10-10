//! sample-support
//!
//! What the small samples share (not a port: the role of the generated `InitializeComponent()`
//! of the managed originals and of the tables the catalog keeps in its `markup.rs`): the
//! table of the classes of a sample that have a document ([`XamlClass`], declared with
//! [`xaml_class!`]), what a sample states about its documents and assets ([`Sample`]), the
//! load of a class from the compiled markup of its document ([`Sample::load_component`];
//! docs/porting/xaml.md, 9.5.22) and the smoke option of the desktop entry points
//! ([`close_main_window_after`]).
//!
//! A sample declares one [`Sample`] as `crate::SAMPLE`; the macros name it.

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::platform::register_assets;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{BoxedValue, ObjectType, Ref, TypeInfo};
use ferroui_controls::Application;
use ferroui_markup_xaml::XamlLoadException;
use std::rc::Rc;
use std::time::Duration;

/// A class of a sample with a document.
pub struct XamlClass {
    /// The rooted asset path of the document (`/Pages/BorderPage.xaml`).
    pub document: &'static str,
    /// The class.
    pub type_info: fn() -> &'static TypeInfo,
    /// Creates an instance with the constructor of the class (which loads the document),
    /// boxed as the handle of the class.
    pub create: fn() -> BoxedValue,
    /// Creates an instance without running the body of the constructor: the root instance
    /// of a load of the document on its own.
    pub create_uninitialized: fn() -> BoxedValue,
}

/// The function that populates an instance of a class from the compiled markup of its
/// document.
pub type Populate = fn(&BoxedValue) -> Result<(), XamlLoadException>;

/// What a sample states about its documents and assets.
pub struct Sample {
    /// What the crate states about itself for markup; its name is the authority of the URIs
    /// of the documents and assets.
    pub assembly: &'static MarkupAssembly,
    /// Registers the types, the assets and the compiled markup of the crate.
    pub register_types: fn(),
    /// The classes with a document, per namespace.
    pub classes: &'static [&'static [&'static XamlClass]],
    /// The documents the build compiled, by their rooted asset paths (`COMPILED` of
    /// `$OUT_DIR/compiled_classes.rs`).
    pub compiled: &'static [(&'static str, Populate)],
    /// The assets of the assembly (`ASSETS` of `$OUT_DIR/assets.rs`).
    pub assets: &'static [(&'static str, &'static [u8])],
    /// The compiled documents as assets, in a test build (`DOCUMENT_ASSETS`); empty otherwise.
    pub document_assets: &'static [(&'static str, &'static [u8])],
    /// The documents with the classes they name (`DOCUMENTS`).
    pub documents: &'static [(&'static str, Option<&'static str>)],
}

impl Sample {
    /// The classes of the sample that have a document.
    pub fn xaml_classes(&self) -> impl Iterator<Item = &'static XamlClass> {
        self.classes.iter().flat_map(|classes| classes.iter().copied())
    }

    /// The class of the document with the rooted asset path `path`.
    pub fn find_class(&self, path: &str) -> Option<&'static XamlClass> {
        self.xaml_classes().find(|class| class.document.eq_ignore_ascii_case(path))
    }

    /// The URI of the document with the rooted asset path `path`.
    pub fn document_uri(&self, path: &str) -> String {
        format!("ferres://{}{path}", self.assembly.name)
    }

    /// The content of the embedded asset or document with the rooted path `path`.
    pub fn asset(&self, path: &str) -> Option<&'static [u8]> {
        self.assets.iter().chain(self.document_assets).find(|(asset_path, _)| *asset_path == path).map(|(_, content)| *content)
    }

    /// Registers the embedded assets with the asset loader under the name of the assembly.
    pub fn register_assets(&self) {
        register_assets(self.assembly.name, self.assets);
        register_assets(self.assembly.name, self.document_assets);
    }

    /// Whether the build compiled the document of a class with the rooted asset path `path`.
    pub fn is_compiled(&self, path: &str) -> bool {
        self.compiled.iter().any(|(document, _)| *document == path)
    }

    /// Populates `root`, an instance of the class of the document `path`, from the compiled
    /// markup of the document; `None` when the build did not compile the document.
    ///
    /// # Panics
    /// Panics if `root` is not the handle of the class of the document.
    pub fn populate_compiled(&self, path: &str, root: &BoxedValue) -> Option<Result<(), XamlLoadException>> {
        (self.register_types)();
        self.compiled.iter().find(|(document, _)| *document == path).map(|(_, populate)| populate(root))
    }

    /// What the generated `InitializeComponent()` of a class does: populates `this` from the
    /// compiled markup of the document of its class.
    ///
    /// # Panics
    /// Panics if the document fails to load (an exception of the constructor in the managed
    /// original).
    pub fn load_component<T: ObjectType>(&self, this: &Ref<T>, path: &str) {
        let root: BoxedValue = Rc::new(this.clone());
        let result = self
            .populate_compiled(path, &root)
            .unwrap_or_else(|| Err(XamlLoadException::with_message(format!("The document {path} is not compiled by the build of the sample."))));
        if let Err(error) = result {
            panic!("{path}: {}", describe(&error));
        }
    }
}

/// A load error as text, with the error of the compiler it carries.
pub fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}

/// Declares the document of a class of a sample: its entry in the table of the classes with
/// a document ([`XamlClass`]) and `initialize_component()`. The sample declares its
/// [`Sample`] as `crate::SAMPLE`.
///
/// ```ignore
/// xaml_class!(BrushesPage, "/Pages/BrushesPage.xaml");
/// // A class without a parameterless constructor names how the table creates it:
/// xaml_class!(TestItemView, "/TestItemView.xaml", create: TestItemView::with(..));
/// // A class whose constructor prepares the instance for its document (a resource the
/// // document refers to) names the instance a load of the document on its own populates:
/// xaml_class!(MainWindow, "/MainWindow.xaml", create: MainWindow::new(), uninitialized: MainWindow::before_document());
/// ```
#[macro_export]
macro_rules! xaml_class {
    ($class:ident, $path:literal) => {
        $crate::xaml_class!($class, $path, create: <$class>::new());
    };
    ($class:ident, $path:literal, create: $create:expr) => {
        $crate::xaml_class!($class, $path, create: $create, uninitialized: ::ferroui_base::instantiate(<$class>::construct()));
    };
    ($class:ident, $path:literal, create: $create:expr, uninitialized: $uninitialized:expr) => {
        impl $class {
            /// The rooted asset path of the document of the class.
            pub const DOCUMENT_PATH: &'static str = $path;

            /// The entry of the class in the table of the classes with a document.
            pub const XAML_CLASS: $crate::XamlClass = $crate::XamlClass {
                document: $path,
                type_info: || <$class>::TYPE,
                create: || ::std::rc::Rc::new($create) as ::ferroui_base::BoxedValue,
                create_uninitialized: || ::std::rc::Rc::new($uninitialized) as ::ferroui_base::BoxedValue,
            };

            /// `InitializeComponent()`: populates the instance from the document of the
            /// class.
            ///
            /// # Panics
            /// Panics if the document fails to load (an exception of the constructor in the
            /// managed original).
            #[allow(dead_code)]
            fn initialize_component(&self) {
                crate::SAMPLE.load_component(&self.to_ref(), $path);
            }
        }
    };
}

/// The smoke run of a desktop entry point (not a port): with the environment variable
/// `FERROUI_SMOKE_EXIT_MS=<n>` the main window of the application is closed after `n`
/// milliseconds, which ends the main loop. Called once the application is set up
/// (`AppBuilder::after_setup`).
pub fn smoke_run() {
    if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) {
        println!("Will close the main window after {ms} ms");
        close_main_window_after(Duration::from_millis(ms));
    }
}

/// Closes the main window of the application after `interval`.
pub fn close_main_window_after(interval: Duration) {
    // The timer stops itself after its only tick.
    let _timer = DispatcherTimer::run_once(
        || {
            println!("Timer fired: closing the main window");
            let lifetime = Application::current().and_then(|application| application.application_lifetime());
            let main_window =
                lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|l| l.main_window());
            if let Some(main_window) = main_window {
                main_window.close();
            }
        },
        interval,
        DispatcherPriority::NORMAL,
    );
}
