extern crate self as ferroribbon_sample;
pub mod cad;
pub mod capture;
mod command_adapter;
pub mod demo_settings;
pub mod native_capture;
pub mod office_view_models;
pub mod pages;
mod ribbon_session;
pub mod settings_view_model;
pub mod shell_page;
pub mod shell_view_model;
pub mod view_model;
mod word_metadata;
pub mod word_view_model;
use ferroui_base::{
    Ref,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::Window;
use generated::compiled_markup;
use std::rc::Rc;
use view_model::SampleViewModel;
ferroui_base::ferro_markup_type!(class SampleViewModel {
 namespace:"FerroRibbon.Sample",this:Rc<SampleViewModel>,handles:[SampleViewModel,Rc<SampleViewModel>,Option<Rc<SampleViewModel>>],notify_property_changed:SampleViewModel,
 properties:[ActiveTool:String{get:|this:&Rc<SampleViewModel>|this.active_tool(),set:|this:&Rc<SampleViewModel>,v|this.set_active_tool(v)},FontColor:Option<ferroui_base::media::Color>{get:|this:&Rc<SampleViewModel>|this.font_color().map(ferroribbon::theming::to_color),set:|this:&Rc<SampleViewModel>,color:Option<ferroui_base::media::Color>|this.set_font_color(color.map(ferroribbon::theming::to_ribbon_color))},FontColorAction:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt_color(this.font_color_command())},DocumentForeground:Rc<dyn ferroui_base::media::IBrush>{get:|this:&Rc<SampleViewModel>|ferroui_base::media::SolidColorBrush::with_color(ferroribbon::theming::to_color(this.font_color().unwrap_or(ferroribbon_core::theming::RibbonColor::from_rgb(32,37,48)))).into()},TableAction:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.table_command())},ZoomPercent:f64{get:|this:&Rc<SampleViewModel>|this.zoom_percent(),set:|this:&Rc<SampleViewModel>,v|this.set_zoom_percent(v)},ZoomScale:f64{get:|this:&Rc<SampleViewModel>|this.zoom_scale()},ZoomAction:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.zoom_command())},OpacityPercent:f64{get:|this:&Rc<SampleViewModel>|this.opacity_percent(),set:|this:&Rc<SampleViewModel>,v|this.set_opacity_percent(v)},DocumentOpacity:f64{get:|this:&Rc<SampleViewModel>|this.document_opacity()},OpacityAction:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.opacity_command())},FontSize:f64{get:|this:&Rc<SampleViewModel>|this.font_size(),set:|this:&Rc<SampleViewModel>,v|this.set_font_size(v)},FontSizeAction:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.font_size_command())},FontFamily:String{get:|this:&Rc<SampleViewModel>|this.font_family(),set:|this:&Rc<SampleViewModel>,value|this.set_font_family(value)},Font:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.font_command())},SearchText:String{get:|this:&Rc<SampleViewModel>|this.search_text(),set:|this:&Rc<SampleViewModel>,v|this.set_search_text(v)},Find:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.find_command())},VisibilityMode:ferroribbon_core::RibbonVisibilityMode{get:|this:&Rc<SampleViewModel>|this.visibility_mode(),set:|this:&Rc<SampleViewModel>,v|this.set_visibility_mode(v)},Simplified:Option<bool>{get:|this:&Rc<SampleViewModel>|this.simplified(),set:|this:&Rc<SampleViewModel>,value|this.set_simplified(value)},DisplayMode:ferroribbon_core::RibbonDisplayMode{get:|this:&Rc<SampleViewModel>|this.display_mode(),set:|this:&Rc<SampleViewModel>,value|this.set_display_mode(value)},Bold:Option<bool>{get:|this:&Rc<SampleViewModel>|this.bold(),set:|this:&Rc<SampleViewModel>,value|this.set_bold(value)},ShowGuides:Option<bool>{get:|this:&Rc<SampleViewModel>|this.show_guides(),set:|this:&Rc<SampleViewModel>,value|this.set_show_guides(value)},Status:String{get:|this:&Rc<SampleViewModel>|this.status()},Document:String{get:|this:&Rc<SampleViewModel>|this.document(),set:|this:&Rc<SampleViewModel>,value|this.set_document(value)},Action:Rc<dyn ferroui_base::input::ICommand>{get:|this:&Rc<SampleViewModel>|command_adapter::adapt(this.command())}],
});
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub static ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "ferroribbon_sample",
        crate_name: "ferroribbon_sample",
        xmlns_definitions: &[],
        xmlns_prefixes: &[],
        metadata: &[],
    };
pub fn register_types() {
    ferroribbon::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::shell_page",
        "FerroRibbon.Sample",
    )]);
    ferroui_base::TypeInfo::register_all(&[shell_page::ShellPage::TYPE]);
    cad::cad_models::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::cad::cad_drawing_canvas",
        "FerroRibbon.Sample.Cad",
    )]);
    ferroui_base::TypeInfo::register_all(&[cad::CadDrawingCanvas::TYPE]);
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::pages::cad_page",
        "FerroRibbon.Sample.Pages",
    )]);
    ferroui_base::TypeInfo::register_all(&[pages::CadPage::TYPE]);
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::pages::settings_page",
        "FerroRibbon.Sample.Pages",
    )]);
    ferroui_base::TypeInfo::register_all(&[pages::SettingsPage::TYPE]);
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::pages::word_page",
        "FerroRibbon.Sample.Pages",
    )]);
    ferroui_base::TypeInfo::register_all(&[pages::WordPage::TYPE]);
    MarkupType::register_all(&[word_view_model::WordViewModel::MARKUP]);
    ferroui_base::data::core::ValueTypes::register_reference::<word_view_model::WordViewModel>();
    ferroui_base::TypeInfo::register_namespaces(&[
        (
            "ferroribbon_sample::pages::powerpoint_page",
            "FerroRibbon.Sample.Pages",
        ),
        (
            "ferroribbon_sample::pages::tools_page",
            "FerroRibbon.Sample.Pages",
        ),
    ]);
    ferroui_base::TypeInfo::register_all(&[pages::PowerPointPage::TYPE, pages::ToolsPage::TYPE]);
    MarkupType::register_all(&[
        office_view_models::PowerPointViewModel::MARKUP,
        office_view_models::ToolsViewModel::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_reference::<
        office_view_models::PowerPointViewModel,
    >();
    ferroui_base::data::core::ValueTypes::register_reference::<office_view_models::ToolsViewModel>(
    );
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_sample::pages::excel_page",
        "FerroRibbon.Sample.Pages",
    )]);
    ferroui_base::TypeInfo::register_all(&[pages::ExcelPage::TYPE]);
    MarkupType::register_all(&[excel_view_model::ExcelViewModel::MARKUP]);
    ferroui_base::data::core::ValueTypes::register_reference::<excel_view_model::ExcelViewModel>();
    ferroui_themes_fluent::register_types();
    ferroui_themes_simple::register_types();
    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
    MarkupType::register_all(&[
        SampleViewModel::MARKUP,
        cad::CadIcons::MARKUP,
        cad::CadArt::MARKUP,
        cad::CadIcon::MARKUP,
        cad::cad_view_model::CadViewModel::MARKUP,
        settings_view_model::SettingsViewModel::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_reference::<SampleViewModel>();
    ferroui_base::data::core::ValueTypes::register_reference::<cad::cad_view_model::CadViewModel>();
    ferroui_base::data::core::ValueTypes::register_reference::<
        settings_view_model::SettingsViewModel,
    >();
    compiled_markup::register();
}
ferroui_base::ferro_thread_local! {static RESOURCES:std::cell::RefCell<Vec<(ferroui_base::WeakRef<ferroui_controls::Application>,ferroui_base::WeakRef<ferroui_base::controls::ResourceDictionary>)>>=const{std::cell::RefCell::new(Vec::new())};}
fn install_application_resources() {
    let Some(app) = ferroui_controls::Application::current() else {
        return;
    };
    let installed = RESOURCES.with_borrow_mut(|cache| {
        cache.retain(|(a, d)| a.upgrade().is_some() && d.upgrade().is_some());
        cache
            .iter()
            .any(|(a, _)| a.upgrade().as_ref() == Some(&app))
    });
    if installed {
        return;
    }
    let value =
        generated::compiled_xaml::try_load(None, "ferres://ferroribbon_sample/AppResources.xaml")
            .expect("application resources")
            .expect("application dictionary");
    let dictionary = ferroui_base::data::core::ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ferroui_base::controls::ResourceDictionary>()
        .unwrap();
    #[cfg(windows)]
    {
        let pin_resources = generated::compiled_windows_pin_resources::try_load(
            None,
            "ferres://ferroribbon_sample/windows/PinResources.xaml",
        )
        .expect("Windows pin resources")
        .expect("Windows pin dictionary");
        let pin_resources = ferroui_base::data::core::ValueTypes::as_object(&*pin_resources)
            .unwrap()
            .cast::<ferroui_base::controls::ResourceDictionary>()
            .unwrap();
        dictionary.add_merged_dictionary(pin_resources);
    }
    app.resources()
        .merged_dictionaries()
        .insert(0, dictionary.clone().into());
    RESOURCES.with_borrow_mut(|cache| cache.push((app.downgrade(), dictionary.downgrade())));
}
pub fn create_window(model: Rc<SampleViewModel>) -> Ref<Window> {
    install_application_resources();
    let value = generated::compiled_xaml::try_load(None, "ferres://ferroribbon_sample/Main.xaml")
        .expect("compiled sample")
        .expect("sample document");
    let w = value
        .downcast_ref::<Ref<Window>>()
        .expect("Window root")
        .clone();
    w.set_data_context(Some(model.clone()));
    let ribbon =
        ferroui_base::controls::NameScopeExtensions::find::<ferroribbon::controls::Ribbon>(
            &w, "Ribbon",
        )
        .expect("sample ribbon");
    model.set_ribbon_session(Rc::new(ribbon_session::NativeRibbonSession::new(&ribbon)));
    w
}
#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "desktop"))]
mod render_tests;

#[cfg(all(test, feature = "desktop"))]
#[path = "../../tests/fixtures/menu_image_uri_cases.rs"]
mod menu_image_uri_cases;

#[cfg(test)]
mod cad_markup_tests;

#[cfg(test)]
mod cad_model_tests;

/// Interactive original canvas milestone.
pub fn create_cad_canvas_window() -> Ref<Window> {
    install_application_resources();
    let value = generated::compiled_xaml::try_load(
        None,
        "ferres://ferroribbon_sample/CadCanvasWindow.xaml",
    )
    .expect("compiled CAD canvas window")
    .expect("CAD canvas document");
    ferroui_base::data::core::ValueTypes::as_object(&*value)
        .expect("native CAD window object")
        .cast::<Window>()
        .expect("CAD Window root")
}

/// Complete source markup; application actions are being integrated separately.
pub fn create_cad_window() -> Ref<Window> {
    install_application_resources();
    ferroribbon::theming::RibbonTheme::apply_with_style(
        Some(ferroribbon_core::theming::RibbonThemePalette::cad()),
        ferroribbon_core::theming::RibbonChromeStyle::Neutral,
        ferroribbon_core::theming::RibbonThemeStyle::Cad,
    )
    .expect("original CAD surface before creating controls");
    let window = Window::new();
    window.set_title(Some("FerroRibbon — CAD".into()));
    window.set_width(1440.);
    window.set_height(960.);
    window.set_content(Some(
        ferroui_controls::Control::boxed(pages::CadPage::new()),
    ));
    window
}

pub fn create_settings_window() -> Ref<Window> {
    install_application_resources();
    let window = Window::new();
    window.set_title(Some("FerroRibbon — Settings".into()));
    window.set_width(1200.);
    window.set_height(900.);
    window.set_content(Some(ferroui_controls::Control::boxed(
        pages::SettingsPage::new(),
    )));
    window
}

/// Original source Word page.
pub fn create_word_window() -> Ref<Window> {
    install_application_resources();
    let window = Window::new();
    window.set_title(Some("FerroRibbon — Word".into()));
    window.set_width(1440.);
    window.set_height(1000.);
    window.set_content(Some(ferroui_controls::Control::boxed(
        pages::WordPage::new(),
    )));
    window
}
#[cfg(test)]
mod word_view_model_tests;

pub fn create_powerpoint_window() -> Ref<Window> {
    install_application_resources();
    let window = Window::new();
    window.set_title(Some("FerroRibbon — PowerPoint".into()));
    window.set_width(1440.);
    window.set_height(1000.);
    window.set_content(Some(ferroui_controls::Control::boxed(
        pages::PowerPointPage::new(),
    )));
    window
}
pub fn create_tools_window() -> Ref<Window> {
    install_application_resources();
    let window = Window::new();
    window.set_title(Some("FerroRibbon — Tools".into()));
    window.set_width(1440.);
    window.set_height(1000.);
    window.set_content(Some(ferroui_controls::Control::boxed(
        pages::ToolsPage::new(),
    )));
    window
}

mod excel_ribbon_model;
pub mod excel_view_model;

#[cfg(all(test, feature = "desktop"))]
mod excel_model_native_tests;
#[cfg(test)]
mod excel_model_tests;
pub mod excel_sheet;

pub fn create_excel_window() -> Ref<Window> {
    install_application_resources();
    let w = Window::new();
    w.set_title(Some("FerroRibbon — Excel".into()));
    w.set_width(1440.);
    w.set_height(1000.);
    w.set_content(Some(ferroui_controls::Control::boxed(
        pages::ExcelPage::new(),
    )));
    w
}

pub fn initialize_gallery(app: &ferroui_controls::Application) {
    ferroribbon::localization::initialize_ui_culture_from_platform()
        .expect("native preferred application language");
    register_types();
    app.resources().set(
        "SymbolThemeFontFamily",
        Some(Rc::new(ferroui_base::media::FontFamily::new(
            "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
        ))),
    );
    app.set_name(Some("FerroRibbon Sample".into()));
    // WinUI uses the system UI font on the window and on detached popup roots.
    #[cfg(windows)]
    app.resources().set(
        "ContentControlThemeFontFamily",
        Some(Rc::new(
            ferroui_base::media::FontFamily::new("Segoe UI Variable, Segoe UI, $Default")
                .with_automatic_optical_sizing(),
        )),
    );
    app.styles()
        .add(ferroui_themes_simple::SimpleTheme::new().upcast::<ferroui_base::styling::Styles>());
    ferroribbon::theming::RibbonTheme::ensure_resources();
    ferroui_base::styling::ThemeVariant::set_high_contrast_variant(Some(
        ferroribbon::theming::RibbonThemeResources::high_contrast_variant(),
    ));
}

pub fn gallery_font_options() -> ferroui_base::media::FontManagerOptions {
    ferroui_base::media::FontManagerOptions {
        font_family_mappings: Some(std::collections::HashMap::from([
            (
                "Segoe Fluent Icons".to_owned(),
                ferroui_base::media::FontFamily::new(
                    "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                ),
            ),
            (
                "Segoe MDL2 Assets".to_owned(),
                ferroui_base::media::FontFamily::new(
                    "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                ),
            ),
        ])),
        ..Default::default()
    }
}

pub fn create_shell_view() -> Ref<shell_page::ShellPage> {
    install_application_resources();
    shell_page::ShellPage::new()
}

pub fn create_shell_window() -> Ref<Window> {
    let shell = create_shell_view();
    let window = Window::new();
    window.set_title(Some("FerroRibbon Gallery".into()));
    window.set_width(1440.);
    window.set_height(1000.);
    window.set_content(Some(ferroui_controls::Control::boxed(shell)));
    window
}

#[cfg(all(test, feature = "desktop"))]
mod shell_page_tests;

// The real-text runtime check compiles the same test fixture, not a second port.
#[cfg(test)]
pub(crate) use ferroribbon::{compat, compiled_shared, controls};
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../src/FerroRibbon/runtime_sample.rs"]
mod source_runtime_sample;
