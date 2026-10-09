//! The pages of the fixture: each document of the sample by its path below the root of
//! the sample, with the module its compiled markup is written to. The build script and the
//! tests read the same list; the texts are the files of the sample, read where they are.

/// A page of the sample.
pub struct Page {
    /// The path of the document below `samples/ControlCatalog`, which is its path below
    /// the root URI of the assembly.
    pub path: &'static str,
    /// The module of the compiled markup of the document (`crate::<module>`).
    pub module: &'static str,
}

/// The pages: two pages without code of their own (nullable values and `{x:Null}`; named
/// elements and bindings between them); a page with a handler of its class; a page whose
/// items are a typed list of its view model, with compiled bindings, styles and templates; a
/// page with handlers, named elements and bitmaps named by the paths of assets.
///
/// Two pages that were tried are not here because rustc refuses their compiled markup
/// (docs/porting/xaml-compiler/HANDOVER.md, section 19): `Pages/CanvasPage.xaml`
/// (`Points="..."` of a polygon) and `Pages/SliderPage.xaml` (`Ticks="..."` of a slider).
pub const PAGES: &[Page] = &[
    Page { path: "Pages/CheckBoxPage.xaml", module: "compiled_check_box_page" },
    Page { path: "Pages/ProgressBarPage.xaml", module: "compiled_progress_bar_page" },
    Page { path: "Pages/ButtonSpinnerPage.xaml", module: "compiled_button_spinner_page" },
    Page { path: "Pages/WrapPanelPage.xaml", module: "compiled_wrap_panel_page" },
    Page { path: "Pages/ImagePage.xaml", module: "compiled_image_page" },
];

/// The directory of the sample, below the directory of this crate.
pub const SAMPLE: &str = "../../samples/ControlCatalog";
