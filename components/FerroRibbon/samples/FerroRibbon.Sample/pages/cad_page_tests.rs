use super::CadPage;
use ferroui_base::{
    FerroObject,
    controls::NameScopeExtensions,
    input::{InputElement, Key, KeyEventArgs},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{
    AppBuilder, Application, Canvas, Control, Grid, TextBlock, TextBox, UserControl, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};

pub(super) fn build() -> AppBuilder {
    use ferroui_desktop::AppBuilderDesktopExtensions;
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..Default::default()
        })
}
#[test]
fn whole_cad_page_native_constructor_compiled_runtime_and_source_structure() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                let direct = CadPage::new();
                let compiled = ferroui_markup_xaml::FerroXamlLoader::load(
                    &ferroui_base::utilities::UriRef::parse_absolute(
                        "ferres://ferroribbon_sample/CadPage.xaml",
                    )
                    .unwrap()
                    .into(),
                    None,
                )
                .unwrap();
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register_class_document(
                    CadPage::TYPE,
                    "ferres://ferroribbon_sample/CadPage.xaml",
                );
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("../CadPage.xaml"),
                    Some(&crate::ASSEMBLY),
                )
                .unwrap();
                let compiled = ferroui_base::data::core::ValueTypes::as_object(&*compiled)
                    .unwrap()
                    .cast::<CadPage>()
                    .unwrap();
                let runtime = ferroui_base::data::core::ValueTypes::as_object(&*runtime)
                    .unwrap()
                    .cast::<CadPage>()
                    .unwrap();
                let mapping: serde_json::Value =
                    serde_json::from_str(include_str!("../../../docs/cad-page-markup.json"))
                        .unwrap();
                for page in [direct, compiled, runtime] {
                    assert!(page.is::<UserControl>());
                    let ribbon = page.cad_ribbon();
                    assert_eq!(ribbon.tabs().count(), 14);
                    assert_eq!(
                        ribbon
                            .tabs()
                            .iter()
                            .flatten()
                            .map(|t| t.groups().count())
                            .sum::<usize>(),
                        71
                    );
                    for name in mapping["source_names"].as_array().unwrap() {
                        assert!(
                            NameScopeExtensions::find::<FerroObject>(&page, name.as_str().unwrap())
                                .is_some(),
                            "Original named source object {}",
                            name
                        );
                    }
                    assert!(
                        page.draw_panel()
                            .ptr_eq(&ribbon.tabs().get(0).unwrap().groups().get(0).unwrap())
                    );
                    assert!(page.layer_drop_down().ptr_eq(&page.layer_drop_down()));
                    assert_eq!(page.application_menu().items().count(), 10);
                    assert!(page.save_as_menu_item().items().count() > 0);
                    verify_native_actions(&page, &mapping["chrome"]);
                }
                let page = CadPage::new();
                let weak_page = page.downgrade();
                let model = page.view_model();
                let weak_model = std::rc::Rc::downgrade(&model);
                let retained_command = model.contextual_action();
                let retained_ribbon = page.cad_ribbon();
                drop(model);
                drop(page);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(
                    weak_page.upgrade().is_none(),
                    "Native sessions and commands must not retain the page"
                );
                assert!(
                    weak_model.upgrade().is_none(),
                    "Retaining the ribbon or a command must not retain the view model"
                );
                retained_command.execute(Some(
                    &crate::cad::cad_view_model::CadContextAction::ShowTextEditor,
                ));
                drop(retained_ribbon);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

fn verify_native_actions(page: &ferroui_base::Ref<CadPage>, chrome: &serde_json::Value) {
    use crate::cad::{CadDrawingCanvas, CadLayer};
    use ferroribbon::controls::{RibbonButton, RibbonComboBox};
    let window = Window::new();
    window.set_width(1440.);
    window.set_height(960.);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let vm = page.view_model();
    let input = page.named::<TextBox>("CommandInput");
    let history = page.named::<TextBlock>("CommandHistory");
    assert_eq!(history.text().as_deref(), Some(vm.history_text().as_str()));
    let history_color = || {
        history
            .foreground()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
    };
    assert_eq!(
        history_color(),
        ferroui_base::media::Color::from_rgb(0xAE, 0xB8, 0xC4)
    );
    verify_chrome_brushes(page, chrome, "dark");
    let fallback = page.chrome_resource_brush("MissingCadChromeBrush");
    assert_eq!(
        fallback.as_solid_color_brush().unwrap().color(),
        ferroui_base::media::Colors::GRAY
    );
    assert!(!fallback.equals(&*page.chrome_resource_brush("MissingCadChromeBrush")));
    // Source chooses RibbonTheme's brush even if the native page shadows the key.
    page.resources().set(
        "RibbonCommandBarBackgroundBrush",
        Some(std::rc::Rc::new(
            ferroui_base::media::SolidColorBrush::from_uint32(0xffff00ff),
        )),
    );
    let first_dark_history_brush = history.foreground().unwrap();
    page.set_light_theme(true);
    assert_eq!(
        history_color(),
        ferroui_base::media::Color::from_rgb(0x4E, 0x58, 0x66)
    );
    verify_chrome_brushes(page, chrome, "light");
    page.set_light_theme(false);
    assert_eq!(
        history_color(),
        ferroui_base::media::Color::from_rgb(0xAE, 0xB8, 0xC4)
    );
    assert!(
        !first_dark_history_brush.equals(&*history.foreground().unwrap()),
        "Returning to Dark should assign a fresh history brush"
    );
    let cube = page.named::<TextBlock>("CubeText").foreground().unwrap();
    let north = page.named::<TextBlock>("CubeN").foreground().unwrap();
    assert!(
        !cube.equals(&*north),
        "Source foreach creates separate brushes"
    );
    verify_chrome_brushes(page, chrome, "dark");
    page.resources()
        .set("RibbonCommandBarBackgroundBrush", None);
    assert_eq!(
        page.layer_drop_down().text().as_deref(),
        Some(CadLayer::samples()[1].name())
    );
    assert_eq!(
        page.layer_drop_down()
            .get_item_text(Some(CadLayer::samples()[2].clone()))
            .as_deref(),
        Some(CadLayer::samples()[2].name())
    );
    let send = |key| {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        input.raise_event(&event);
        event.handled()
    };
    input.set_text(Some(" l "));
    assert_eq!(vm.input().as_deref(), Some(" l "));
    let before = vm.history().len();
    assert!(send(Key::Enter));
    assert_eq!(
        vm.history().len(),
        before + 1,
        "Initial trailing prompt is replaced, then two LINE messages are appended"
    );
    assert_eq!(
        vm.history().last().map(String::as_str),
        Some("Specify first point:")
    );
    assert_eq!(history.text().as_deref(), Some(vm.history_text().as_str()));
    assert_eq!(input.text().as_deref(), Some(""));
    assert!(!send(Key::Space));
    assert!(send(Key::Escape));
    assert_eq!(vm.history().last().map(String::as_str), Some("*Cancel*"));
    let line = page
        .cad_ribbon()
        .find_item(Some("line".into()))
        .unwrap()
        .cast::<RibbonButton>()
        .unwrap();
    line.set_is_enabled(false);
    let before = vm.history();
    vm.run_action().execute(Some(&"l".to_owned()));
    assert_eq!(
        vm.history(),
        before,
        "Disabled alias is found; source does not fall through to search"
    );
    line.set_is_enabled(true);
    page.run_command("Layer Properties");
    assert_eq!(
        vm.history().last().map(String::as_str),
        Some("Layer Properties Manager opened."),
        "Non-alias search executes the actual native search result"
    );
    let workspace = page.named::<RibbonComboBox>("WorkspaceCombo");
    workspace.commit_text(Some("3D Modeling".into()));
    assert_eq!(vm.workspace().as_deref(), Some("3D Modeling"));
    assert!(
        page.cad_ribbon()
            .find_tab("solid")
            .unwrap()
            .is_tab_visible()
    );
    workspace.commit_text(Some("Drafting & Annotation".into()));
    assert!(
        !page
            .cad_ribbon()
            .find_tab("solid")
            .unwrap()
            .is_tab_visible()
    );
    assert_eq!(page.cad_ribbon().selected_tab_id().as_deref(), Some("home"));
    vm.show_text_editor();
    assert!(page.named::<Canvas>("MTextLayer").is_visible());
    assert!(
        page.cad_ribbon()
            .find_contextual_group("textEditor")
            .unwrap()
            .is_visible()
    );
    assert!(
        !page
            .cad_ribbon()
            .find_contextual_group("hatchCreation")
            .unwrap()
            .is_visible()
    );
    let canvas = page.named::<CadDrawingCanvas>("Canvas");
    let top_left = canvas.model_to_screen(3700., 4450.);
    let editor = page.named::<Grid>("MTextEditor");
    assert_eq!(Canvas::get_left(&editor), top_left.x);
    assert_eq!(Canvas::get_top(&editor), top_left.y - 12.);
    vm.close_contextual("Text Editor");
    assert!(!page.named::<Canvas>("MTextLayer").is_visible());
    assert_eq!(page.cad_ribbon().selected_tab_id().as_deref(), Some("home"));
    window.set_content(None);
    window.close();
}

fn verify_chrome_brushes(page: &CadPage, chrome: &serde_json::Value, theme: &str) {
    use ferroribbon::{compat::FontIcon, theming::RibbonTheme};
    use ferroui_base::media::{Color, IBrush};
    use ferroui_controls::{
        Border,
        shapes::{Ellipse, Rectangle},
    };
    use std::{collections::HashMap, rc::Rc};

    let mut assigned: HashMap<&str, Rc<dyn IBrush>> = HashMap::new();
    for assignment in chrome["assignments"].as_array().unwrap() {
        let name = assignment["element"].as_str().unwrap();
        let property = assignment["property"].as_str().unwrap();
        let key = assignment["brush"].as_str().unwrap();
        let brush = match (name, property) {
            ("WcsChevron", "Foreground") => page.named::<FontIcon>(name).foreground(),
            (_, "Foreground") => page.named::<TextBlock>(name).foreground(),
            (_, "Background") => page.named::<Border>(name).background(),
            (_, "BorderBrush") => page.named::<Border>(name).border_brush(),
            ("CubeRing", "Stroke") => page.named::<Ellipse>(name).stroke(),
            ("MTextFrame", "Stroke") => page.named::<Rectangle>(name).stroke(),
            ("MTextCaret", "Fill") => page.named::<Rectangle>(name).fill(),
            _ => panic!("Unsupported CAD brush assignment {name}.{property}"),
        }
        .unwrap_or_else(|| panic!("Missing CAD brush on {name}.{property}"));

        if let Some(hex) = chrome["colors"][key][theme].as_str() {
            let expected = Color::from_uint32(u32::from_str_radix(&hex[1..], 16).unwrap());
            assert_eq!(
                brush.as_solid_color_brush().unwrap().color(),
                expected,
                "{name}.{property}"
            );
        } else {
            let expected: Rc<dyn IBrush> = RibbonTheme::get_brush(Some(page), Some(key))
                .unwrap()
                .unwrap()
                .into();
            assert!(brush.equals(&*expected), "{name}.{property} / {key}");
        }
        if let Some(previous) = assigned.insert(key, brush.clone()) {
            assert!(brush.equals(&*previous), "{key} must share one brush");
        }
    }
}
