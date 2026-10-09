//! Which documents of the sample the build of the fixture compiles, and what is known of
//! the ones it does not compile or the tests do not compare. The build script and the tests
//! read the same lists; the texts are the files of the sample, read where they are. A
//! document is named by its path below `samples/ControlCatalog`, which is its path below
//! the root URI of the assembly.

/// The directory of the sample, below the directory of this crate.
pub const SAMPLE: &str = "../../samples/ControlCatalog";

/// The pages a build without the feature `catalog` compiles: two pages without code of
/// their own (nullable values and `{x:Null}`; named elements and bindings between them); a
/// page with a handler of its class; a page whose items are a typed list of its view model,
/// with compiled bindings, styles and templates; a page with handlers, named elements and
/// bitmaps named by the paths of assets; two pages with a list written as text for a named
/// collection whose Rust type does not dereference to the list it derives from
/// (`Points="..."` of a polygon, `Ticks="..."` of a slider).
pub const PAGES: &[&str] = &[
    "Pages/CheckBoxPage.xaml",
    "Pages/ProgressBarPage.xaml",
    "Pages/ButtonSpinnerPage.xaml",
    "Pages/WrapPanelPage.xaml",
    "Pages/ImagePage.xaml",
    "Pages/CanvasPage.xaml",
    "Pages/SliderPage.xaml",
];

/// The documents the compiler refuses, each with the reason of its first error
/// (docs/porting/xaml-compiler/HANDOVER.md has the table). A build with the feature
/// `catalog` compiles every other document of the sample and fails if one of them is
/// refused; the measure of the catalog
/// (`ferroui-markup-xaml-tests`, `emitter::catalog_measure`) is the test that a document
/// listed here is still refused.
pub const REFUSED: &[(&str, &str)] = &[
    ("App.xaml", "the include of the theme documents of the colour picker, which is not compiled by its build"),
    ("MainView.xaml", "a container query: no emitter for XamlIlWidthQuery"),
    ("Pages/ContainerQueryPage.xaml", "a container query: no emitter for XamlIlWidthQuery"),
    ("Pages/ContextFlyoutPage.xaml", "a compiled binding path with a method as a command"),
    ("Pages/LabelsPage.xaml", "a compiled binding path with a method as a command"),
    ("Pages/OpenGl/OpenGlLeasePage.xaml", "the class of the document is not ported"),
    ("Pages/PipsPager/PipsPagerCustomButtonThemesPage.xaml", "PreviousButtonTheme: not a plain property setter"),
    ("Pages/TransitioningContentControlPage.xaml", "a compiled binding path with a method as a command"),
];

/// The compiled documents that neither back end loads in the services of a test, each
/// with the reason: the test of such a document asserts that the compiled markup and the
/// run-time loader fail with the same error, and fails if the document loads.
pub const NOT_LOADED: &[(&str, &str)] = &[];

/// The compiled documents whose comparison is not run, each with the reason (the test of
/// such a document is ignored with it): a document whose load panics in the services of a
/// test, with either back end.
pub const NOT_RUN: &[(&str, &str)] = &[];
