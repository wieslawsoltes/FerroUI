//! Opt-in real Skia/HarfBuzz rendering of the actual compiled sample.
//! Offscreen frames are not native OS window, activation or accessibility evidence.
#[path = "bitmap_icon_render_tests.rs"]
mod bitmap_icon_tests;
#[path = "cad/menu_render_tests.rs"]
mod cad_menu_tests;
#[cfg(not(target_arch = "wasm32"))]
#[path = "http_image_render_tests.rs"]
mod http_image_tests;
#[path = "symbol_render_tests.rs"]
mod symbol_tests;
use super::*;
use ferroribbon::controls::{Ribbon, RibbonSlider, RibbonSpinner};
use ferroui_base::{
    media::{FontFamily, FontManagerOptions, imaging::PngBitmapEncoderOptions},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{AppBuilder, Application, TextBox, TopLevel};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};
use std::path::{Path, PathBuf};
use view_model::{InMemoryDocument, SampleViewModel};

fn build() -> AppBuilder {
    build_with_popups(true)
}
fn native_popup_build() -> AppBuilder {
    build_with_popups(false)
}
fn build_with_popups(overlay_popups: bool) -> AppBuilder {
    // Configure the same desktop Skia and HarfBuzz services, then replace only
    // the windowing subsystem BEFORE setup. No native desktop is initialized.
    let builder = AppBuilder::configure::<Application>()
        .with(Rc::new(FontManagerOptions {
            font_family_mappings: Some(std::collections::HashMap::from([
                (
                    "Segoe Fluent Icons".to_owned(),
                    FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ),
                ),
                (
                    "Segoe MDL2 Assets".to_owned(),
                    FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ),
                ),
            ])),
            ..Default::default()
        }))
        .use_platform_detect()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            overlay_popups,
            ..Default::default()
        });
    assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
    assert_eq!(
        builder.windowing_subsystem_name().as_deref(),
        Some("Headless")
    );
    assert_eq!(
        builder.text_shaping_subsystem_name().as_deref(),
        Some("HarfBuzz")
    );
    builder
}
fn capture(w: &TopLevel, directory: &Path, name: &str) {
    Dispatcher::ui_thread().run_jobs(None);
    // A snapshot requests a synchronous paint, as the framework render tests do.
    w.renderer()
        .paint(ferroui_base::Rect::from_size(w.client_size()));
    let frame = w
        .get_last_rendered_frame()
        .expect("actual rendered sample frame");
    eprintln!(
        "{name}: window={:?}, client={:?}",
        w.bounds(),
        w.client_size()
    );
    for text in w
        .get_visual_descendants()
        .filter_map(|c| c.cast::<ferroui_controls::TextBlock>())
    {
        if text.text().as_deref() == Some("Font size: 24") {
            eprintln!(
                "{name}: status={:?}, origin={:?}",
                text.bounds(),
                text.translate_point(ferroui_base::Point::default(), w)
            );
        }
    }
    let path = directory.join(name);
    frame
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .expect("save actual sample frame");
}

#[test]
#[ignore = "Explicit separate native popup-root rendering"]
fn original_cad_page_skia_separate_popup_screen_tips() {
    use ferroribbon::controls::RibbonScreenTipService;
    use ferroui_base::{animation::SignedDuration, data::core::ValueTypes};
    use pages::CadPage;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(native_popup_build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let window = create_cad_window();
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let page = ValueTypes::as_object(&*window.content().unwrap())
                    .unwrap()
                    .cast::<CadPage>()
                    .unwrap();
                let save_tip = |id: &str, name: &str| {
                    let item = page.cad_ribbon().find_item(Some(id.into())).unwrap();
                    assert!(item.is_attached_to_visual_tree());
                    assert!(
                        item.translate_point(ferroui_base::Point::default(), &window)
                            .is_none()
                    );
                    assert!(page.show_tool_tip(Some(id)).unwrap());
                    let popup = page.captured_tip_popup().unwrap();
                    assert!(popup.is_open() && !popup.is_using_overlay_layer());
                    let child = popup.child().unwrap();
                    let root = TopLevel::get_top_level(Some(&child)).unwrap();
                    assert!(!root.ptr_eq(&window));
                    assert!(root.client_size().width > 0. && root.client_size().height > 0.);
                    capture(&root, &directory, name);
                    popup.set_is_open(false);
                };
                page.draw_panel().open_slide_out();
                page.draw_panel().set_is_slide_out_pinned(true);
                Dispatcher::ui_thread().run_jobs(None);
                save_tip("multiplePoints", "cad-native-popup-slideout-tip.png");
                page.draw_panel().close_slide_out();
                page.float_layers_panel();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(page.layers_panel().is_floating());
                save_tip("layerProperties", "cad-native-popup-floating-tip.png");
                page.layers_panel().return_to_ribbon();
                window.set_width(260.);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    page.draw_panel().state(),
                    ferroribbon_core::RibbonGroupState::Collapsed
                );
                page.draw_panel().open_popup();
                Dispatcher::ui_thread().run_jobs(None);
                let delay = RibbonScreenTipService::extended_delay();
                RibbonScreenTipService::set_extended_delay(SignedDuration::ZERO);
                for (light, name) in [
                    (false, "cad-native-popup-collapsed-dark-tip.png"),
                    (true, "cad-native-popup-collapsed-light-tip.png"),
                ] {
                    page.set_light_theme(light);
                    Dispatcher::ui_thread().run_jobs(None);
                    save_tip("line", name);
                }
                RibbonScreenTipService::set_extended_delay(delay);
                page.draw_panel().close_popup();
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit native CAD ScreenTip captures"]
fn original_cad_page_skia_screen_tips() {
    use ferroribbon::controls::{RibbonButton, RibbonScreenTip, RibbonScreenTipService};
    use ferroui_base::{animation::SignedDuration, data::core::ValueTypes};
    use ferroui_controls::ToolTip;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let window = create_cad_window();
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let page = ValueTypes::as_object(&*window.content().unwrap())
                    .unwrap()
                    .cast::<pages::CadPage>()
                    .unwrap();
                assert!(page.show_tool_tip(Some("multiplePoints")).unwrap());
                capture(&window, &directory, "cad-tip-source-description.png");
                assert!(page.show_tool_tip(Some("line")).unwrap());
                capture(&window, &directory, "cad-tip-source-line.png");
                let button = page
                    .cad_ribbon()
                    .find_item(Some("line".into()))
                    .unwrap()
                    .cast::<RibbonButton>()
                    .unwrap();
                let original = ValueTypes::as_object(&*button.screen_tip().unwrap())
                    .unwrap()
                    .cast::<RibbonScreenTip>()
                    .unwrap();
                let extended_image = original
                    .extended_image()
                    .expect("Original LineHelp illustration");
                let delay = RibbonScreenTipService::extended_delay();
                RibbonScreenTipService::set_extended_delay(SignedDuration::ZERO);
                for (light, name) in [
                    (false, "cad-tip-extended-dark.png"),
                    (true, "cad-tip-extended-light.png"),
                ] {
                    page.set_light_theme(light);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(page.show_tool_tip(Some("line")).unwrap());
                    let popup = page.captured_tip_popup().unwrap();
                    let tip = popup.child().unwrap().cast::<ToolTip>().unwrap();
                    Dispatcher::ui_thread().run_jobs(None);
                    let content = ValueTypes::as_object(&*tip.content().unwrap())
                        .unwrap()
                        .cast::<RibbonScreenTip>()
                        .unwrap();
                    assert!(content.is_extended());
                    assert!(!content.ptr_eq(&original));
                    assert!(Rc::ptr_eq(
                        &content.extended_image().unwrap(),
                        &extended_image
                    ));
                    capture(&window, &directory, name);
                }
                RibbonScreenTipService::set_extended_delay(delay);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit full CAD page Skia/HarfBuzz capture"]
fn original_cad_page_skia_commands_workspaces_and_contexts() {
    use ferroui_base::controls::NameScopeExtensions;
    use pages::CadPage;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let window = create_cad_window();
                window.show();
                let page =
                    ferroui_base::data::core::ValueTypes::as_object(&*window.content().unwrap())
                        .unwrap()
                        .cast::<CadPage>()
                        .unwrap();
                assert_eq!(page.cad_ribbon().tabs().count(), 14);
                capture(&window, &directory, "cad-page-drafting.png");
                let circle = page
                    .cad_ribbon()
                    .find_item(Some("circle".into()))
                    .unwrap()
                    .cast::<ferroribbon::controls::RibbonSplitButton>()
                    .unwrap();
                let circle_menu = circle
                    .flyout()
                    .unwrap()
                    .cast::<ferroui_controls::MenuFlyout>()
                    .unwrap();
                let choice = ferroui_base::data::core::ValueTypes::as_object(
                    &*circle_menu.items().get_at(0).unwrap(),
                )
                .unwrap()
                .cast::<ferroribbon::compat::MenuFlyoutItem>()
                .unwrap();
                let tag = ferroui_base::metadata::from_markup_value::<
                    Rc<ferroribbon_core::model::RibbonIcon>,
                >(&choice.tag())
                .unwrap();
                let dark_image =
                    ferroui_base::data::core::ValueTypes::as_object(&*choice.icon().unwrap())
                        .unwrap()
                        .cast::<ferroribbon::compat::ImageIcon>()
                        .unwrap();
                assert_eq!(
                    dark_image.source().unwrap().size(),
                    ferroui_base::Size::new(40., 40.)
                );
                circle.open_drop_down();
                Dispatcher::ui_thread().run_jobs(None);
                let popup = TopLevel::get_top_level(Some(&choice)).unwrap();
                assert_eq!(
                    choice.actual_theme_variant(),
                    Some(ferroui_base::styling::ThemeVariant::dark()),
                    "Menu content follows the page's explicit Dark theme inside the default window"
                );
                capture(&popup, &directory, "cad-page-circle-popup-dark.png");
                circle.close_drop_down();
                let canvas =
                    NameScopeExtensions::find::<cad::CadDrawingCanvas>(&page, "Canvas").unwrap();
                assert!(canvas.bounds().width > 500. && canvas.bounds().height > 200.);
                page.run_command("l");
                assert_eq!(
                    page.view_model().history().last().map(String::as_str),
                    Some("Specify first point:")
                );
                capture(&window, &directory, "cad-page-line.png");
                page.show_text_editor();
                assert!(page.view_model().text_visible());
                capture(&window, &directory, "cad-page-text-editor.png");
                page.view_model().close_contextual("Text Editor");
                page.show_hatch_creation();
                capture(&window, &directory, "cad-page-hatch.png");
                page.view_model().close_contextual("Hatch Creation");
                page.apply_workspace(Some("3D Modeling".into()));
                capture(&window, &directory, "cad-page-3d.png");
                window.set_width(720.);
                capture(&window, &directory, "cad-page-constrained.png");
                window.set_width(1440.);
                page.set_light_theme(true);
                capture(&window, &directory, "cad-page-light.png");
                let light_image =
                    ferroui_base::data::core::ValueTypes::as_object(&*choice.icon().unwrap())
                        .unwrap()
                        .cast::<ferroribbon::compat::ImageIcon>()
                        .unwrap();
                assert!(!Rc::ptr_eq(
                    &dark_image.source().unwrap(),
                    &light_image.source().unwrap()
                ));
                assert!(
                    Rc::ptr_eq(
                        &ferroui_base::metadata::from_markup_value::<
                            Rc<ferroribbon_core::model::RibbonIcon>,
                        >(&choice.tag())
                        .unwrap(),
                        &tag
                    ),
                    "Theme conversion keeps the same vector Tag"
                );
                circle.open_drop_down();
                Dispatcher::ui_thread().run_jobs(None);
                let popup = TopLevel::get_top_level(Some(&choice)).unwrap();
                assert_eq!(
                    choice.actual_theme_variant(),
                    Some(ferroui_base::styling::ThemeVariant::light())
                );
                capture(&popup, &directory, "cad-page-circle-popup-light.png");
                circle.close_drop_down();
                window.set_content(None);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit native CAD scene/control pointer captures"]
fn original_cad_page_skia_scene_controls_and_application_menu() {
    use ferroribbon::controls::RibbonToggleButton;
    use ferroui_base::{
        Point,
        input::{MouseButton, RawInputModifiers},
    };
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let window = create_cad_window();
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let page =
                    ferroui_base::data::core::ValueTypes::as_object(&*window.content().unwrap())
                        .unwrap()
                        .cast::<pages::CadPage>()
                        .unwrap();
                let canvas = ferroui_base::controls::NameScopeExtensions::find::<
                    cad::CadDrawingCanvas,
                >(&page, "Canvas")
                .unwrap();
                let click = |name: &str| {
                    Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    let control = ferroui_base::controls::NameScopeExtensions::find::<
                        ferroui_controls::Control,
                    >(&page, name)
                    .unwrap();
                    assert!(
                        control.bounds().width > 0. && control.bounds().height > 0.,
                        "Actual visible control {name}"
                    );
                    let point = control
                        .translate_point(
                            Point::new(control.bounds().width * 0.5, control.bounds().height * 0.5),
                            &window,
                        )
                        .unwrap();
                    window.mouse_move(point, RawInputModifiers::NONE);
                    window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
                    window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
                    Dispatcher::ui_thread().run_jobs(None);
                };
                capture(&window, &directory, "cad-actions-loaded.png");
                click("CadLayoutTab1");
                assert_eq!(
                    page.view_model().selected_layout_tab(),
                    cad::cad_view_model::CadDocumentTab::Layout1
                );
                assert_eq!(
                    page.named::<RibbonToggleButton>("ModelToggle")
                        .label()
                        .as_deref(),
                    Some("PAPER")
                );
                capture(&window, &directory, "cad-tabs-layout1.png");
                click("CadFileTab2");
                click("CadLayoutTab0");
                assert_eq!(
                    page.view_model().selected_file_tab(),
                    cad::cad_view_model::CadDocumentTab::FloorPlan
                );
                assert_eq!(
                    page.view_model().selected_layout_tab(),
                    cad::cad_view_model::CadDocumentTab::Model
                );
                capture(&window, &directory, "cad-tabs-model-and-floor-plan.png");
                assert!(page.cad_ribbon().select_tab_id("view"));
                for name in [
                    "UcsToggle",
                    "ViewCubeToggle",
                    "NavBarToggle",
                    "CommandLineToggle",
                ] {
                    click(name);
                }
                for name in ["UcsIcon", "ViewCube", "NavBar", "CommandLine"] {
                    assert!(
                        !ferroui_base::controls::NameScopeExtensions::find::<
                            ferroui_controls::Control,
                        >(&page, name)
                        .unwrap()
                        .is_visible()
                    );
                }
                click("GridToggle");
                assert!(!canvas.is_grid_visible());
                click("LineweightToggle");
                assert!(canvas.show_lineweights());
                capture(&window, &directory, "cad-actions-hidden-widgets.png");
                click("PanelTitlesToggle");
                assert!(!page.cad_ribbon().show_group_captions());
                capture(&window, &directory, "cad-actions-no-panel-titles.png");
                page.float_layers_panel();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(page.layers_panel().is_floating());
                capture(&window, &directory, "cad-actions-floating-layers.png");
                click("ReturnPanelsButton");
                assert!(!page.layers_panel().is_floating());
                click("ModelToggle");
                assert_eq!(
                    ferroui_base::controls::NameScopeExtensions::find::<RibbonToggleButton>(
                        &page,
                        "ModelToggle"
                    )
                    .unwrap()
                    .label()
                    .as_deref(),
                    Some("PAPER")
                );
                click("CleanScreenToggle");
                assert_eq!(
                    page.cad_ribbon().visibility_mode(),
                    ferroribbon_core::RibbonVisibilityMode::FullScreen
                );
                capture(&window, &directory, "cad-actions-clean-screen.png");
                click("CleanScreenToggle");
                assert_eq!(
                    page.cad_ribbon().visibility_mode(),
                    ferroribbon_core::RibbonVisibilityMode::AlwaysShow
                );
                page.show_application_menu();
                Dispatcher::ui_thread().run_jobs(None);
                let flyout = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroui_controls::Flyout,
                >(&page, "AppMenuFlyout")
                .unwrap();
                assert!(flyout.is_open());
                let presenter = flyout
                    .popup()
                    .child()
                    .unwrap()
                    .cast::<ferroui_controls::FlyoutPresenter>()
                    .unwrap();
                assert_eq!(presenter.padding(), ferroui_base::Thickness::uniform(0.));
                assert_eq!(
                    presenter.corner_radius(),
                    ferroui_base::CornerRadius::uniform(2.)
                );
                capture(&window, &directory, "cad-actions-application-menu.png");
                flyout.hide();
                window.set_content(None);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit CAD Skia/HarfBuzz capture"]
fn original_cad_canvas_skia_pixels_and_pointer_input() {
    use crate::cad::CadDrawingCanvas;
    use ferroui_base::{
        PixelRect, Point, Vector,
        controls::NameScopeExtensions,
        input::{MouseButton, RawInputModifiers},
        platform::PixelFormat,
        styling::ThemeVariant,
    };

    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY")
            .expect("set FERRORIBBON_CAPTURE_DIRECTORY to an absolute directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let window = create_cad_canvas_window();
                window.show();
                window.update_layout();
                let canvas = NameScopeExtensions::find::<CadDrawingCanvas>(&window, "Canvas")
                    .expect("actual compiled CAD canvas");
                let pixel = |x: f64, y: f64| {
                    let at = canvas
                        .translate_point(canvas.model_to_screen(x, y), &window)
                        .unwrap();
                    let frame = window.get_last_rendered_frame().unwrap();
                    let size = frame.pixel_size();
                    let client = window.client_size();
                    let px = (at.x * f64::from(size.width) / client.width).floor() as i32;
                    let py = (at.y * f64::from(size.height) / client.height).floor() as i32;
                    let mut bytes = [0; 4];
                    frame.copy_pixels(PixelRect::new(px, py, 1, 1), &mut bytes, 4);
                    match frame.format().unwrap() {
                        PixelFormat::RGBA8888 => {}
                        PixelFormat::BGRA8888 => bytes.swap(0, 2),
                        format => panic!("Unexpected CAD frame format {format:?}"),
                    }
                    bytes
                };
                capture(&window, &directory, "cad-dark-fit.png");
                assert_eq!(
                    pixel(500., 125.),
                    [0xC9, 0xCF, 0xD6, 255],
                    "original filled wall renders"
                );
                assert_eq!(
                    pixel(8600., 2500.),
                    [0x2E, 0x7F, 0xE0, 255],
                    "original selection grip renders"
                );

                let anchor = Point::new(321.25, 210.5);
                let scale = canvas.view_scale();
                window.mouse_wheel(anchor, Vector::new(0., 1.), RawInputModifiers::NONE);
                assert_eq!(canvas.view_scale(), scale * 1.15);
                window.mouse_move(anchor, RawInputModifiers::NONE);
                capture(&window, &directory, "cad-wheel-crosshair.png");
                let origin = canvas.model_to_screen(0., 0.);
                window.mouse_down(anchor, MouseButton::Middle, RawInputModifiers::NONE);
                window.mouse_move(
                    Point::new(anchor.x + 40., anchor.y + 20.),
                    RawInputModifiers::NONE,
                );
                window.mouse_up(
                    Point::new(anchor.x + 40., anchor.y + 20.),
                    MouseButton::Middle,
                    RawInputModifiers::NONE,
                );
                let panned = canvas.model_to_screen(0., 0.);
                assert!((panned.x - origin.x - 40.).abs() < 1e-10);
                assert!((panned.y - origin.y - 20.).abs() < 1e-10);
                capture(&window, &directory, "cad-panned.png");

                canvas.set_crosshair_cursor(Point::new(f64::NAN, f64::NAN));
                canvas.zoom_extents();
                canvas.set_is_grid_visible(false);
                canvas.hidden_layers().borrow_mut().insert("A-FURN".into());
                canvas.redraw();
                capture(&window, &directory, "cad-hidden-furniture.png");
                assert_eq!(pixel(8600., 2500.), [0x21, 0x28, 0x30, 255]);
                canvas.hidden_layers().borrow_mut().insert("A-WALL".into());
                canvas.redraw();
                capture(&window, &directory, "cad-hidden-walls.png");
                assert_eq!(pixel(500., 125.), [0x21, 0x28, 0x30, 255]);

                canvas.hidden_layers().borrow_mut().clear();
                window.set_requested_theme_variant(Some(ThemeVariant::light()));
                canvas.redraw();
                capture(&window, &directory, "cad-light.png");
                assert_eq!(pixel(500., 125.), [0x3A, 0x41, 0x4A, 255]);
                assert_eq!(pixel(8600., 2500.), [0x2E, 0x7F, 0xE0, 255]);
                canvas.set_show_lineweights(true);
                canvas.redraw();
                capture(&window, &directory, "cad-light-lineweights.png");
                window.set_width(650.5);
                Dispatcher::ui_thread().run_jobs(None);
                window.set_height(340.25);
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                canvas.zoom_extents();
                capture(&window, &directory, "cad-constrained-fit.png");
                assert_eq!(canvas.bounds().size(), window.client_size());
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit offscreen Skia capture; set FERRORIBBON_CAPTURE_DIRECTORY."]
fn actual_sample_skia_frames_and_interactive_font_size() {
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY")
            .expect("set FERRORIBBON_CAPTURE_DIRECTORY to an absolute directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    session
        .dispatch(
            move || {
                register_types();
                let app = Application::current().unwrap();
                app.resources().set(
                    "SymbolThemeFontFamily",
                    Some(Rc::new(FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ))),
                );
                app.styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
                let w = create_window(vm.clone());
                w.set_width(1400.);
                w.set_height(850.);
                w.show();
                capture(&w, &directory, "home.png");
                let ribbon = w
                    .get_visual_descendants()
                    .find_map(|c| c.cast::<Ribbon>())
                    .unwrap();
                assert_eq!(ribbon.selected_tab_id().as_deref(), Some("home"));
                let group = ribbon.tabs().get(0).unwrap().groups().get(0).unwrap();
                group.open_slide_out();
                Dispatcher::ui_thread().run_jobs(None);
                let extra = group.slide_out_items().get(0).unwrap();
                let popup_root = TopLevel::get_top_level(Some(&extra))
                    .expect("expanded sample command attached");
                popup_root.update_layout();
                assert!(extra.bounds().width > 0. && extra.bounds().height > 0.);
                capture(&popup_root, &directory, "expanded-clipboard.png");
                group.close_slide_out();
                let spinner = ribbon
                    .find_item(Some("font-size".into()))
                    .unwrap()
                    .cast::<RibbonSpinner>()
                    .unwrap();
                let input = spinner
                    .get_visual_descendants()
                    .find_map(|c| c.cast::<TextBox>())
                    .unwrap();
                input.set_text(Some("24 pt"));
                spinner.commit_text();
                assert_eq!(vm.font_size(), 24.);
                assert_eq!(vm.status(), "Font size: 24");
                capture(&w, &directory, "font-size-committed.png");
                w.set_width(620.);
                capture(&w, &directory, "constrained-classic.png");
                vm.set_simplified(Some(true));
                capture(&w, &directory, "constrained-simplified.png");
                assert_eq!(
                    ribbon.display_mode(),
                    ferroribbon_core::RibbonDisplayMode::Simplified
                );
                vm.set_simplified(Some(false));
                w.set_width(1400.);
                ribbon.set_selected_tab_id(Some("view".into()));
                w.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                let slider = ribbon
                    .find_item(Some("opacity".into()))
                    .unwrap()
                    .cast::<RibbonSlider>()
                    .unwrap();
                let part = slider
                    .get_visual_descendants()
                    .find_map(|c| c.cast::<ferroribbon::compat::RibbonInputSlider>())
                    .unwrap();
                let track = part.track().unwrap();
                let thumb = track.thumb().unwrap();
                capture(&w, &directory, "opacity-before.png");
                let point = track
                    .translate_point(
                        ferroui_base::Point::new(
                            thumb.bounds().width / 2.
                                + 0.4 * (track.bounds().width - thumb.bounds().width),
                            track.bounds().height / 2.,
                        ),
                        &w,
                    )
                    .unwrap();
                w.mouse_down(
                    point,
                    ferroui_base::input::MouseButton::Left,
                    ferroui_base::input::RawInputModifiers::NONE,
                );
                w.mouse_up(
                    point,
                    ferroui_base::input::MouseButton::Left,
                    ferroui_base::input::RawInputModifiers::NONE,
                );
                assert_eq!(vm.opacity_percent(), 40.);
                assert_eq!(vm.document_opacity(), 0.4);
                assert_eq!(vm.status(), "Opacity: 40%");
                capture(&w, &directory, "opacity-40.png");
                vm.set_opacity_percent(100.);
                ribbon.set_selected_tab_id(Some("home".into()));
                w.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                let picker = ribbon
                    .find_item(Some("font-color".into()))
                    .unwrap()
                    .cast::<ferroribbon::controls::RibbonColorPicker>()
                    .unwrap();
                picker.open_drop_down();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(picker.is_drop_down_open());
                capture(&w, &directory, "font-color-open.png");
                let palette_root = TopLevel::get_top_level(Some(&picker.palette())).unwrap();
                capture(&palette_root, &directory, "font-color-palette.png");
                let more = picker
                    .palette()
                    .get_visual_descendants()
                    .filter_map(|c| c.cast::<ferroui_controls::Button>())
                    .filter(|b| b.tag().is_none())
                    .last()
                    .unwrap();
                more.on_click();
                capture(&w, &directory, "more-colors-dialog.png");
                let hex = w
                    .get_visual_descendants()
                    .filter_map(|c| c.cast::<ferroui_controls::TextBox>())
                    .find(|c| {
                        ferroui_controls::automation::AutomationProperties::get_name(c).as_deref()
                            == Some("Hex")
                    })
                    .unwrap();
                let mut f10 = ferroui_base::input::KeyEventArgs::default();
                f10.key = ferroui_base::input::Key::F10;
                f10.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
                hex.raise_event(&f10);
                assert!(
                    !ribbon.is_key_tip_mode(),
                    "dialog events do not route through source XamlRoot.Content"
                );
                hex.set_text(Some("#336699"));
                Dispatcher::ui_thread().run_jobs(None);
                capture(&w, &directory, "more-colors-hex.png");
                let ok = w
                    .get_visual_descendants()
                    .filter_map(|c| c.cast::<ferroui_controls::Button>())
                    .find(|b| {
                        b.content()
                            .as_ref()
                            .and_then(|v| v.downcast_ref::<String>())
                            .is_some_and(|s| s == "OK")
                    })
                    .unwrap();
                ok.on_click();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    vm.font_color(),
                    Some(ferroribbon_core::theming::RibbonColor::from_rgb(
                        51, 102, 153
                    ))
                );
                capture(&w, &directory, "font-color-applied.png");
                ribbon.set_selected_tab_id(Some("insert".into()));
                w.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                let gallery = ribbon
                    .find_item(Some("styles-gallery".into()))
                    .unwrap()
                    .cast::<ferroribbon::controls::RibbonGallery>()
                    .unwrap();
                assert_eq!(gallery.items().count(), 5);
                capture(&w, &directory, "gallery-inline.png");
                gallery.open_drop_down();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(gallery.is_drop_down_open());
                capture(&w, &directory, "gallery-expanded.png");
                gallery.pick(gallery.effective_items().get(1));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    vm.status(),
                    "Heading 1 style applied to the sample document."
                );
                assert!(!gallery.is_drop_down_open());
                let toolbar = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroribbon::controls::RibbonToolBar,
                >(&w, "DocumentToolbar")
                .unwrap();
                assert!(toolbar.overflow_items().count() > 0);
                vm.set_active_tool("draw".into());
                Dispatcher::ui_thread().run_jobs(None);
                let draw = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroribbon::controls::RibbonToolBar,
                >(&w, "DrawOptions")
                .unwrap();
                assert!(draw.is_visible());
                capture(&w, &directory, "toolbar-draw-context.png");
                ribbon.set_selected_tab_id(Some("home".into()));
                w.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                let search = w
                    .get_visual_descendants()
                    .find_map(|control| control.cast::<ferroribbon::controls::RibbonSearchBox>())
                    .unwrap();
                search.focus_search();
                let query = search
                    .get_visual_descendants()
                    .find_map(|control| control.cast::<TextBox>())
                    .unwrap();
                query.set_text(Some("Save"));
                Dispatcher::ui_thread().run_jobs(None);
                assert!(search.is_results_open());
                capture(&w, &directory, "command-search-save.png");
                search.close();

                vm.set_zoom_percent(150.);
                let zoom = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroribbon::controls::RibbonZoomControl,
                >(&w, "DocumentZoomControl")
                .unwrap();
                let plus = zoom
                    .get_visual_descendants()
                    .find_map(|control| {
                        control
                            .cast::<ferroui_controls::Button>()
                            .filter(|button| button.name().as_deref() == Some("PART_ZoomIn"))
                    })
                    .unwrap();
                plus.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
                    ferroui_controls::Button::click_event(),
                ));
                assert_eq!(vm.zoom_percent(), 160.);
                assert_eq!(vm.status(), "Zoom: 160%");
                capture(&w, &directory, "document-zoom-160.png");

                w.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
    session.dispose();
}

#[test]
#[ignore = "Real Skia bitmap decoder/resource test; default Headless uses a bitmap stub"]
fn native_bitmap_uri_loading_uses_real_skia_assets_files_and_errors() {
    use ferroribbon::primitives::{
        RibbonDeferredImage, RibbonIconPresenter, RibbonImageFuture, RibbonImageKind,
        RibbonImageLoadError, RibbonImageProvider,
    };
    use ferroribbon_core::RibbonString;
    use ferroui_base::{
        FerroLocator, PixelRect, PixelSize, Size,
        media::{ImageDrawing, imaging::Bitmap},
        platform::{
            AssetLoader, PixelFormat, register_assets, register_manifest_resources,
            storage::file_io::StorageProviderHelpers,
        },
        threading::Dispatcher,
        utilities::UriRef,
    };
    use ferroui_controls::{Border, Image};

    struct FallbackProvider;
    impl RibbonImageProvider for FallbackProvider {
        fn open(&self, _: &UriRef, _: RibbonImageKind) -> Option<RibbonImageFuture> {
            None
        }
    }

    const PNG: &[u8] = include_bytes!("../../tests/fixtures/images/bitmap-3x2.png");
    const RGBA: [u8; 24] = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255, 0, 255,
        255, 255,
    ];
    fn assert_pixels(bitmap: &Bitmap) {
        assert_eq!(bitmap.pixel_size(), PixelSize::new(3, 2));
        let mut bytes = [0; 24];
        bitmap.copy_pixels(PixelRect::new(0, 0, 3, 2), &mut bytes, 12);
        match bitmap.format().unwrap() {
            PixelFormat::RGBA8888 => assert_eq!(bytes, RGBA),
            PixelFormat::BGRA8888 => {
                for pixel in bytes.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
                assert_eq!(bytes, RGBA);
            }
            format => panic!("Unexpected decoded PNG format {format:?}"),
        }
    }
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                register_assets(
                    "ferroribbon_image_tests",
                    &[
                        ("Assets/cut.png", PNG),
                        ("Assets/blue space.png", PNG),
                        ("Assets/corrupt.png", b"not an encoded image"),
                        // Menu helper always uses bitmap decoding, regardless of suffix.
                        ("Assets/cut.svg", PNG),
                    ],
                );
                register_manifest_resources("ferroribbon_image_tests", &[("Assets.cut.png", PNG)]);
                AssetLoader::invalidate_assembly_cache("ferroribbon_image_tests");
                let app = Application::current().unwrap();
                app.resources().set(
                    RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE,
                    Some(Rc::new("ferres://ferroribbon_image_tests/".to_owned())),
                );
                for input in [
                    "Assets/cut.png",
                    "\\Assets\\cut.png",
                    "resm:Assets.cut.png?assembly=ferroribbon_image_tests",
                    "ms-appx:///Assets/cut.png",
                    "ms-appx://ferroribbon_image_tests/Assets/cut.png",
                    "ferres://ferroribbon_image_tests/Assets/cut.png",
                    "Assets/blue%20space.png",
                    "Assets/blue space.png",
                    "Assets/../Assets/cut.png?ignored=.svg#fragment",
                ] {
                    assert_pixels(
                        &RibbonIconPresenter::try_load_bitmap_uri(&input.into()).unwrap(),
                    );
                }
                #[cfg(windows)]
                assert_pixels(
                    &RibbonIconPresenter::try_load_bitmap_uri(&"/Assets/cut.png".into()).unwrap(),
                );
                // Exact original runtime sequence: an image URI, then the same free
                // element rehosted after an icon-size change. Real pixels prove decoding.
                let presenter = RibbonIconPresenter::new();
                presenter.set_icon(Some(Rc::new("Assets/cut.png".to_owned())));
                assert!(presenter.has_icon());
                let old = presenter.children().get(0).cast::<Image>().unwrap();
                let decoded = old.source().unwrap();
                assert_pixels(decoded.as_any().downcast_ref::<Bitmap>().unwrap());
                assert_eq!(old.width(), 16.);
                presenter.set_icon_size(24.);
                let new = presenter.children().get(0).cast::<Image>().unwrap();
                assert!(!old.ptr_eq(&new));
                assert!(!Rc::ptr_eq(&decoded, &new.source().unwrap()));
                assert_pixels(
                    new.source()
                        .unwrap()
                        .as_any()
                        .downcast_ref::<Bitmap>()
                        .unwrap(),
                );
                let weak = Rc::downgrade(&decoded);
                drop(decoded);
                drop(old);
                assert!(
                    weak.upgrade().is_none(),
                    "Discarded images must not be rooted by a loader cache"
                );
                // Match the original runtime image -> free element -> 16-to-24 rebuild.
                presenter.set_icon_size(16.);
                let shared = Border::new();
                shared.set_width(10.);
                shared.set_height(10.);
                presenter.set_icon(Some(ferroui_controls::Control::boxed(shared.clone())));
                presenter.set_icon_size(24.);
                let host = presenter
                    .children()
                    .get(0)
                    .cast::<ferroui_controls::Viewbox>()
                    .unwrap();
                assert!(host.child().unwrap().ptr_eq(&shared));
                assert!(shared.parent().unwrap().ptr_eq(&host));
                presenter.set_icon(None);
                assert!(shared.parent().is_none());
                for input in ["Assets/missing.png", "Assets/corrupt.png"] {
                    assert!(matches!(
                        RibbonIconPresenter::try_load_bitmap_uri(&input.into()),
                        Err(RibbonImageLoadError::Load(_))
                    ));
                    presenter.set_icon(Some(Rc::new(input.to_owned())));
                    assert!(presenter.has_icon());
                    let source = presenter
                        .children()
                        .get(0)
                        .cast::<Image>()
                        .unwrap()
                        .source()
                        .unwrap();
                    let failed = source
                        .as_object()
                        .unwrap()
                        .downcast_ref::<RibbonDeferredImage>()
                        .unwrap();
                    assert!(matches!(
                        failed.load_result().unwrap().unwrap_err().as_ref(),
                        RibbonImageLoadError::Load(_)
                    ));
                    assert_eq!(source.size(), Size::default());
                }
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"ferres://ferroribbon_image_tests/a.%73vg?x=.png".into()
                    ),
                    Err(RibbonImageLoadError::SvgNotImplemented)
                ));
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"https://example.invalid/a.png".into()
                    ),
                    Err(RibbonImageLoadError::UnsupportedScheme(_))
                ));
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(&RibbonString::from_utf16(vec![
                        0xd800
                    ])),
                    Err(RibbonImageLoadError::UnpairedUtf16)
                ));
                let path = std::env::temp_dir()
                    .join(format!("ferroribbon bitmap URI {}.png", std::process::id()));
                assert!(!path.exists());
                std::fs::write(&path, PNG).unwrap();
                let file_uri =
                    StorageProviderHelpers::uri_from_file_path(path.to_str().unwrap(), false);
                let loaded = RibbonIconPresenter::try_load_bitmap_uri(&file_uri.as_str().into());
                let bare = RibbonIconPresenter::try_load_bitmap_uri(&path.to_str().unwrap().into());
                std::fs::remove_file(&path).unwrap();
                assert_pixels(&loaded.unwrap());
                assert_pixels(&bare.unwrap());
                let data_root = std::env::temp_dir()
                    .join(format!("ferroribbon-appdata-{}", std::process::id()));
                std::fs::create_dir(&data_root).unwrap();
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"ms-appdata:///local/icon.png".into()
                    ),
                    Err(RibbonImageLoadError::MissingApplicationDataDirectory(_))
                ));
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"ms-appdata://FerroRibbon.Image-Test/local/a%23b.png".into()
                    ),
                    Err(RibbonImageLoadError::InvalidApplicationDataPath)
                ));
                app.resources().set(
                    RibbonIconPresenter::IMAGE_APPLICATION_DATA_PACKAGE_RESOURCE,
                    Some(Rc::new("FerroRibbon.Image-Test".to_owned())),
                );
                for (folder, key, file, encoded) in [
                    (
                        "local",
                        RibbonIconPresenter::IMAGE_LOCAL_DATA_DIRECTORY_RESOURCE,
                        "a#b.png",
                        "a%23b.png",
                    ),
                    (
                        "roaming",
                        RibbonIconPresenter::IMAGE_ROAMING_DATA_DIRECTORY_RESOURCE,
                        "roaming.png",
                        "roaming.png",
                    ),
                    (
                        "temp",
                        RibbonIconPresenter::IMAGE_TEMP_DATA_DIRECTORY_RESOURCE,
                        "temp image.png",
                        "temp%20image.png",
                    ),
                ] {
                    let directory = data_root.join(folder);
                    std::fs::create_dir(&directory).unwrap();
                    std::fs::write(directory.join(file), PNG).unwrap();
                    let resource: ferroui_base::BoxedValue = if folder == "roaming" {
                        Rc::new(directory.to_str().unwrap().to_owned())
                    } else {
                        Rc::new(directory.clone())
                    };
                    app.resources().set(key, Some(resource));
                    let unqualified = format!("ms-appdata:///{folder}/{encoded}?ignored=.svg");
                    let input = format!(
                        "MS-APPDATA://ferroribbon.image-test/{folder}/{encoded}?ignored=.svg#icon"
                    );
                    for uri in [&unqualified, &input] {
                        assert_pixels(
                            &RibbonIconPresenter::try_load_bitmap_uri(&uri.as_str().into())
                                .unwrap(),
                        );
                    }
                    let menu_icon = ferroribbon::controls::RibbonItemHelper::create_menu_icon(
                        Some(Rc::new(input)),
                    )
                    .unwrap()
                    .unwrap()
                    .cast::<ferroribbon::compat::ImageIcon>()
                    .unwrap();
                    assert_pixels(
                        menu_icon
                            .source()
                            .unwrap()
                            .as_any()
                            .downcast_ref::<Bitmap>()
                            .unwrap(),
                    );
                }
                for rejected in [
                    "ms-appdata://another.package/local/a%23b.png",
                    "ms-appdata://user@FerroRibbon.Image-Test/local/a%23b.png",
                    "ms-appdata://@FerroRibbon.Image-Test/local/a%23b.png",
                    "ms-appdata://FerroRibbon.Image-Test:80/local/a%23b.png",
                    "ms-appdata://FerroRibbon.Image-Test:/local/a%23b.png",
                    "ms-appdata://FerroRibbon.Image-Test/local/%2e%2e%2fescape.png",
                ] {
                    assert!(
                        matches!(
                            RibbonIconPresenter::try_load_bitmap_uri(&rejected.into()),
                            Err(RibbonImageLoadError::InvalidApplicationDataPath)
                        ),
                        "{rejected}"
                    );
                }
                presenter.set_icon(Some(Rc::new(
                    "ms-appdata://FerroRibbon.Image-Test/local/a%23b.png".to_owned(),
                )));
                let data_image = presenter.children().get(0).cast::<Image>().unwrap();
                assert_pixels(
                    data_image
                        .source()
                        .unwrap()
                        .as_any()
                        .downcast_ref::<Bitmap>()
                        .unwrap(),
                );
                assert_pixels(
                    &RibbonIconPresenter::try_load_bitmap_uri(
                        &"ms-appdata://FerroRibbon.Image-Test/local/../roaming/roaming.png".into(),
                    )
                    .unwrap(),
                );
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"ms-appdata:///local/%2e%2e%2fescape.png".into()
                    ),
                    Err(RibbonImageLoadError::InvalidApplicationDataPath)
                ));
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(
                        &"ms-appdata:///local/missing.png".into()
                    ),
                    Err(RibbonImageLoadError::Load(_))
                ));
                for (folder, file) in [
                    ("local", "a#b.png"),
                    ("roaming", "roaming.png"),
                    ("temp", "temp image.png"),
                ] {
                    std::fs::remove_file(data_root.join(folder).join(file)).unwrap();
                    std::fs::remove_dir(data_root.join(folder)).unwrap();
                }
                std::fs::remove_dir(data_root).unwrap();
                app.resources().set(
                    RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE,
                    Some(Rc::new("relative/".to_owned())),
                );
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(&"Assets/cut.png".into()),
                    Err(RibbonImageLoadError::InvalidApplicationBaseUri)
                ));
                app.resources().set(
                    RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE,
                    Some(Rc::new(
                        ferroui_base::utilities::Uri::new(
                            "relative/",
                            ferroui_base::utilities::UriKind::Relative,
                        )
                        .unwrap(),
                    )),
                );
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(&"Assets/cut.png".into()),
                    Err(RibbonImageLoadError::InvalidApplicationBaseUri)
                ));
                app.resources().set(
                    RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE,
                    Some(Rc::new(
                        ferroui_base::utilities::Uri::absolute("ferres://ferroribbon_image_tests/")
                            .unwrap(),
                    )),
                );
                assert_pixels(
                    &RibbonIconPresenter::try_load_bitmap_uri(&"Assets/cut.png".into()).unwrap(),
                );
                crate::menu_image_uri_cases::assert_source_resource_routes(Some(|image| {
                    assert_pixels(image.as_any().downcast_ref::<Bitmap>().unwrap());
                }));
                // The original CAD menu scenario requires ImageIcon, not a
                // stand-in glyph. Real pixels prove both the loader and native image path.
                for input in [
                    "ms-appx:///Assets/cut.png",
                    "ms-appx://ferroribbon_image_tests/Assets/cut.png",
                    "Assets/cut.svg",
                ] {
                    let icon = ferroribbon::controls::RibbonItemHelper::create_menu_icon(Some(
                        Rc::new(input.to_owned()),
                    ))
                    .unwrap()
                    .unwrap()
                    .cast::<ferroribbon::compat::ImageIcon>()
                    .unwrap();
                    let source = icon.source().unwrap();
                    assert_pixels(source.as_any().downcast_ref::<Bitmap>().unwrap());
                    let window = ferroui_controls::Window::new();
                    window.set_content(Some(ferroui_controls::Control::boxed(icon.clone())));
                    window.show();
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    let root = icon
                        .visual_children()
                        .get(0)
                        .cast::<ferroui_controls::Grid>()
                        .unwrap();
                    let image = root.children().get(0).cast::<Image>().unwrap();
                    assert!(Rc::ptr_eq(&image.source().unwrap(), &source));
                    assert_pixels(
                        image
                            .source()
                            .unwrap()
                            .as_any()
                            .downcast_ref::<Bitmap>()
                            .unwrap(),
                    );
                    window.set_content(None);
                    window.close();
                }
                {
                    let _scope = FerroLocator::enter_scope();
                    FerroLocator::current_mutable()
                        .bind::<dyn RibbonImageProvider>()
                        .to_constant(Rc::new(FallbackProvider) as Rc<dyn RibbonImageProvider>);
                    presenter.set_icon(Some(Rc::new("Assets/cut.png".to_owned())));
                    let image = presenter.children().get(0).cast::<Image>().unwrap();
                    let source = image.source().unwrap();
                    let deferred = source
                        .as_object()
                        .unwrap()
                        .downcast_ref::<RibbonDeferredImage>()
                        .unwrap();
                    assert!(deferred.load_result().is_none());
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(deferred.load_result().unwrap().is_ok());
                    assert!(Rc::ptr_eq(&image.source().unwrap(), &source));
                    let drawing = deferred.drawing().unwrap().cast::<ImageDrawing>().unwrap();
                    let bitmap = drawing.image_source().unwrap();
                    assert_pixels(bitmap.as_any().downcast_ref::<Bitmap>().unwrap());
                }
                app.resources()
                    .remove(&RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE.into());
                assert!(matches!(
                    RibbonIconPresenter::try_load_bitmap_uri(&"Assets/cut.png".into()),
                    Err(RibbonImageLoadError::MissingApplicationBaseUri)
                ));
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit source SettingsPage Skia/HarfBuzz rendering; OS/source pixel parity separate"]
fn original_settings_page_skia_live_controls_and_log() {
    use ferroui_controls::ComboBox;
    use pages::{CadPage, SettingsPage};
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                install_application_resources();
                let settings = demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let page = SettingsPage::new_with_settings(settings.clone());
                let window = Window::new();
                window.set_width(1200.);
                window.set_height(1000.);
                window.set_content(Some(ferroui_controls::Control::boxed(page.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let cad = CadPage::new_with_settings(settings.clone());
                cad.run_command("line");
                page.view_model()
                    .word_action(settings_view_model::SettingsWordAction::ShowState)
                    .execute(None);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(
                    page.view_model()
                        .log_text()
                        .unwrap()
                        .contains("CAD · Command: _LINE")
                );
                page.named::<ComboBox>("Theme").set_selected_index(2);
                Dispatcher::ui_thread().run_jobs(None);
                capture(&window, &directory, "settings-page-dark.png");
                page.named::<ComboBox>("Theme").set_selected_index(1);
                Dispatcher::ui_thread().run_jobs(None);
                capture(&window, &directory, "settings-page-light.png");
                window.set_width(800.);
                Dispatcher::ui_thread().run_jobs(None);
                capture(&window, &directory, "settings-page-constrained.png");
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit original WordPage Skia/HarfBuzz rendering; OS/source pixel parity separate"]
fn original_word_page_skia_formatting_contexts_and_adaptive_layout() {
    use ferroribbon::controls::{
        RibbonDropDownButton, RibbonGallery, RibbonGridPicker, RibbonZoomControl,
    };
    use ferroribbon_core::model::RibbonGridSize;
    use pages::WordPage;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                install_application_resources();
                let settings = demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let page = WordPage::new_with_settings(settings);
                let w = Window::new();
                w.set_width(1440.);
                w.set_height(1100.);
                w.set_content(Some(ferroui_controls::Control::boxed(page.clone())));
                w.show();
                Dispatcher::ui_thread().run_jobs(None);
                capture(&w, &directory, "word-home.png");
                w.set_width(900.);
                Dispatcher::ui_thread().run_jobs(None);
                capture(&w, &directory, "word-home-constrained.png");
                w.set_width(1440.);
                page.named::<RibbonGallery>("Styles")
                    .pick(Some(Rc::new("Title".to_owned())));
                page.view_model()
                    .action()
                    .execute(Some(&Some(word_view_model::WordAction::ToggleDark)));
                Dispatcher::ui_thread().run_jobs(None);
                capture(&w, &directory, "word-dark-style.png");
                page.ribbon().select_tab_id("insert");
                Dispatcher::ui_thread().run_jobs(None);
                let equation = page
                    .ribbon()
                    .get_visual_descendants()
                    .filter_map(|visual| visual.cast::<ferroui_controls::TextBlock>())
                    .find(|label| label.text().as_deref() == Some("Equation"))
                    .expect("visual label for the Insert Equation command");
                assert_eq!(
                    equation.font_family(),
                    FontFamily::new("Segoe UI Variable Small, Segoe UI, $Default")
                        .with_automatic_optical_sizing(),
                );
                let table = page.named::<RibbonDropDownButton>("InsertTable");
                let item = ferroui_base::data::core::ValueTypes::try_cast(
                    &ferroui_controls::Control::boxed(table.upcast::<ferroui_controls::Control>()),
                    ferroui_base::data::core::ValueType::of::<
                        Rc<dyn ferroribbon::controls::IRibbonItem>,
                    >(),
                )
                .expect("native table item interface");
                item.downcast_ref::<Rc<dyn ferroribbon::controls::IRibbonItem>>()
                    .unwrap()
                    .invoke();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(!w.opened_popups().is_empty());
                capture(&w, &directory, "word-table-picker.png");
                page.named::<RibbonGridPicker>("TablePicker")
                    .pick(RibbonGridSize::new(2, 3));
                page.named::<RibbonZoomControl>("Zoom")
                    .set_numeric_value(125.);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(
                    page.named::<ferroui_controls::Border>("TableSample")
                        .is_visible()
                );
                capture(&w, &directory, "word-table-context.png");
                w.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit original Office pages Skia/HarfBuzz frames; OS/source pixel parity separate"]
fn original_powerpoint_and_tools_pages_skia_interaction_states() {
    use ferroribbon::{
        compat::MenuFlyoutItem,
        controls::{RibbonGallery, RibbonGalleryItem, RibbonSplitButton, RibbonToggleButton},
    };
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::{Control, MenuFlyout};
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                install_application_resources();
                let settings = demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let pp = pages::PowerPointPage::new_with_settings(settings.clone());
                let w = Window::new();
                w.set_width(1440.);
                w.set_height(1000.);
                w.set_content(Some(Control::boxed(pp.clone())));
                w.show();
                capture(&w, &directory, "powerpoint-home.png");
                w.set_width(700.);
                capture(&w, &directory, "powerpoint-constrained.png");
                w.set_width(1440.);
                pp.ribbon().select_tab_id("design");
                let themes = pp.named::<RibbonGallery>("Themes");
                themes.open_drop_down();
                assert!(!w.opened_popups().is_empty());
                capture(&w, &directory, "powerpoint-themes-popup.png");
                let facet = themes
                    .effective_items()
                    .iter()
                    .flatten()
                    .filter_map(|v| ValueTypes::as_object(&*v))
                    .filter_map(|o| o.cast::<RibbonGalleryItem>())
                    .find(|t| t.label().as_deref() == Some("Facet"))
                    .unwrap();
                themes.pick(Some(Rc::new(facet)));
                assert_eq!(
                    pp.named::<ferroui_controls::TextBlock>("Status")
                        .text()
                        .as_deref(),
                    Some("pp.themes (RibbonSpace.Controls.RibbonGalleryItem)")
                );
                capture(&w, &directory, "powerpoint-facet.png");
                w.set_content(None);
                w.close();

                let tools = pages::ToolsPage::new_with_settings(settings.clone());
                let w = Window::new();
                w.set_width(1440.);
                w.set_height(1000.);
                w.set_content(Some(Control::boxed(tools.clone())));
                w.show();
                capture(&w, &directory, "tools-brush.png");
                let buttons = tools
                    .tools()
                    .items()
                    .iter()
                    .flatten()
                    .filter_map(|c| c.cast::<RibbonToggleButton>())
                    .collect::<Vec<_>>();
                buttons[0].set_is_checked(Some(true));
                capture(&w, &directory, "tools-move.png");
                buttons[5].set_is_checked(Some(true));
                capture(&w, &directory, "tools-text.png");
                let shape = tools
                    .tools()
                    .items()
                    .iter()
                    .flatten()
                    .find_map(|c| c.cast::<RibbonSplitButton>())
                    .unwrap();
                shape.open_drop_down();
                assert!(shape.is_drop_down_open());
                capture(&w, &directory, "tools-shapes-popup.png");
                let menu = shape.flyout().unwrap().cast::<MenuFlyout>().unwrap();
                let ellipse = menu
                    .items()
                    .get_at(1)
                    .and_then(|v| ValueTypes::as_object(&*v))
                    .unwrap()
                    .cast::<MenuFlyoutItem>()
                    .unwrap();
                ellipse.invoke_menu_core();
                shape.close_drop_down();
                assert!(shape.last_choice().unwrap().ptr_eq(&ellipse));
                let before = settings.log().borrow().len();
                let item = ValueTypes::try_cast(
                    &Control::boxed(shape.clone().upcast::<Control>()),
                    ferroui_base::data::core::ValueType::of::<
                        Rc<dyn ferroribbon::controls::IRibbonItem>,
                    >(),
                )
                .unwrap();
                item.downcast_ref::<Rc<dyn ferroribbon::controls::IRibbonItem>>()
                    .unwrap()
                    .invoke();
                assert_eq!(settings.log().borrow().len(), before + 1);
                capture(&w, &directory, "tools-ellipse-last-choice.png");
                w.set_content(None);
                w.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
#[test]
#[ignore = "Explicit original ExcelPage Skia/HarfBuzz frames; OS/source pixels separate"]
fn original_excel_page_skia_model_sheet_menu_and_context() {
    use ferroribbon::{
        controls::{IRibbonItem, RibbonSplitButton},
        theming::RibbonTheme,
    };
    use ferroribbon_core::theming::RibbonThemePalette;
    use ferroui_base::data::core::{ValueType, ValueTypes};
    use ferroui_controls::Control;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                install_application_resources();
                let settings = demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let page = pages::ExcelPage::new_with_settings(settings);
                let w = Window::new();
                w.set_width(1440.);
                w.set_height(1000.);
                w.set_content(Some(Control::boxed(page.clone())));
                RibbonTheme::apply_palette(Some(RibbonThemePalette::excel())).unwrap();
                w.show();
                capture(&w, &directory, "excel-home.png");
                assert!(
                    page.ribbon().get_visual_descendants().any(|v| {
                        v.cast::<ferroui_controls::TextBlock>()
                            .is_some_and(|t| t.text().as_deref() == Some("Normal"))
                    }),
                    "the real model gallery must render its Core string content"
                );
                page.sheet().select((2, 2));
                let bold = page.ribbon().find_item(Some("bold".into())).unwrap();
                let item = ValueTypes::try_cast(
                    &Control::boxed(bold),
                    ValueType::of::<Rc<dyn IRibbonItem>>(),
                )
                .unwrap();
                item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke();
                assert_eq!(page.cell_text((2, 2)).unwrap().font_size(), 11.);
                capture(&w, &directory, "excel-bold-cell.png");
                let paste = page
                    .ribbon()
                    .find_item(Some("paste".into()))
                    .unwrap()
                    .cast::<RibbonSplitButton>()
                    .unwrap();
                paste.open_drop_down();
                assert!(paste.is_drop_down_open());
                capture(&w, &directory, "excel-paste-menu.png");
                paste.close_drop_down();
                page.view_model().toggle_chart_command().execute(None);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    page.ribbon().selected_tab().unwrap().id().as_deref(),
                    Some("chartDesign")
                );
                capture(&w, &directory, "excel-chart-context.png");
                page.ribbon().select_tab_id("home");
                w.set_width(800.);
                capture(&w, &directory, "excel-constrained.png");
                w.set_content(None);
                w.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit original six-page shell Skia/HarfBuzz integration; OS parity separate"]
fn original_shell_six_pages_cache_palette_and_style_invalidation() {
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                install_application_resources();
                let settings = demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let page = shell_page::ShellPage::new_with_settings(settings.clone());
                let w = Window::new();
                w.set_width(1600.);
                w.set_height(1040.);
                w.set_content(Some(ferroui_controls::Control::boxed(page.clone())));
                w.show();
                capture(&w, &directory, "shell-word.png");
                let original_word = page.current_content().unwrap().unwrap();
                for key in ["excel", "powerpoint", "tools", "cad", "settings"] {
                    let toggle = page
                        .rail()
                        .items()
                        .iter()
                        .flatten()
                        .filter_map(|item| item.cast::<ferroribbon::controls::RibbonToggleButton>())
                        .find(|t| {
                            t.tag()
                                .and_then(|v| v.downcast_ref::<String>().cloned())
                                .as_deref()
                                == Some(key)
                        })
                        .unwrap();
                    excel_model_native_tests::invoke(toggle.upcast());
                    capture(&w, &directory, &format!("shell-{key}.png"));
                    assert_eq!(page.current_page().as_deref(), Some(key));
                }
                assert_eq!(page.page_host().children().count(), 6);
                let view = page
                    .current_content()
                    .unwrap()
                    .unwrap()
                    .cast::<pages::SettingsPage>()
                    .unwrap();
                view.named::<ferroui_controls::ComboBox>("Surface")
                    .set_selected_index(1);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(page.page_host().children().count(), 2);
                page.show("word").unwrap();
                assert!(
                    !page
                        .current_content()
                        .unwrap()
                        .unwrap()
                        .ptr_eq(&original_word)
                );
                capture(&w, &directory, "shell-word-cad-style.png");
                w.set_width(900.);
                capture(&w, &directory, "shell-constrained.png");
                w.close();
                w.set_content(None);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit source-width scenario with native Skia/HarfBuzz text services"]
fn group_by_group_reduction_keeps_important_groups_large() {
    use ferroribbon_core::{RibbonGroupState, layout::RibbonReductionStrategy};
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let m = crate::source_runtime_sample::Mounted::new(640.);
                m.ribbon
                    .set_reduction_strategy(RibbonReductionStrategy::GroupByGroup);
                m.settle();
                assert_eq!(
                    crate::source_runtime_sample::find_group(&m.ribbon, "styles").state(),
                    RibbonGroupState::Collapsed
                );
                assert_eq!(
                    crate::source_runtime_sample::find_group(&m.ribbon, "clipboard").state(),
                    RibbonGroupState::Large
                );
                m.ribbon
                    .set_reduction_strategy(RibbonReductionStrategy::Stepwise);
                m.settle();
                assert!(
                    crate::source_runtime_sample::find_group(&m.ribbon, "clipboard").state()
                        != RibbonGroupState::Large
                        || crate::source_runtime_sample::find_group(&m.ribbon, "styles").state()
                            != RibbonGroupState::Collapsed
                );
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.expect("source CAD group-by-group scenario with actual text services");
}

#[test]
#[ignore = "Explicit real-text separate KeyTip popup rendering; native OS composition is separate"]
fn word_key_tips_render_in_their_separate_popup_root() {
    use ferroui_controls::{Control, TextBlock};
    use serde_json::json;
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(native_popup_build);
    let result = session.dispatch(move || {
        register_types();
        Application::current().unwrap().styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        let window = create_shell_window();
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let shell = Control::from_boxed(&window.content().unwrap()).unwrap()
            .cast::<shell_page::ShellPage>().unwrap();
        let page = shell.current_content().unwrap().unwrap().cast::<pages::WordPage>().unwrap();
        let ribbon = page.ribbon();
        ribbon.show_key_tips();
        Dispatcher::ui_thread().run_jobs(None);
        let badges: Vec<_> = ribbon.active_key_tips().iter().flatten().collect();
        assert!(!badges.is_empty());
        let root = TopLevel::get_top_level(Some(&badges[0])).expect("badge popup root");
        assert!(!root.ptr_eq(&window), "default native-capable popup is a separate top-level");
        let details: Vec<_> = badges.iter().map(|badge| {
            let texts: Vec<_> = badge.get_visual_descendants()
                .filter_map(|v| v.cast::<TextBlock>()).map(|t| t.text()).collect();
            json!({"text":badge.text(), "desired":[badge.desired_size().width,badge.desired_size().height],
                "bounds":[badge.bounds().x,badge.bounds().y,badge.bounds().width,badge.bounds().height],
                "visible":badge.is_visible(), "attached":badge.is_attached_to_visual_tree(),
                "texts":texts})
        }).collect();
        std::fs::write(directory.join("keytip-popup-layout.json"), serde_json::to_vec_pretty(&json!({
            "root":[root.client_size().width,root.client_size().height],"badges":details
        })).unwrap()).unwrap();
        capture(&window, &directory, "keytip-main-window.png");
        capture(&root, &directory, "keytip-popup-root.png");
        for badge in badges {
            assert!(badge.is_visible() && badge.is_attached_to_visual_tree());
            assert!(badge.bounds().width > 0. && badge.bounds().height > 0., "badge {:?}",badge.text());
            assert!(badge.get_visual_descendants().filter_map(|v|v.cast::<TextBlock>())
                .any(|t| t.text() == badge.text()), "templated text {:?}",badge.text());
        }
        assert!(ribbon.process_key_tip_input(b'N' as u16));
        // Native input queues normal callbacks before the new tab's layout.
        Dispatcher::ui_thread().run_jobs(Some(ferroui_base::threading::DispatcherPriority::NORMAL));
        window.get_layout_manager().unwrap().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
        let nested = ribbon.active_key_tips().iter().flatten().collect::<Vec<_>>();
        std::fs::write(directory.join("keytip-nested-layout.json"), serde_json::to_vec_pretty(&json!({
            "selected":ribbon.selected_tab_id(),"tips":ribbon.current_key_tips().count(),
            "badges":nested.len(),"target_widths":ribbon.current_key_tips().iter().map(|e|e.target.map(|t|t.bounds().width)).collect::<Vec<_>>()
        })).unwrap()).unwrap();
        assert!(!nested.is_empty(), "newly selected Insert scope must display its command badges");
        assert_eq!(nested.len(), 25, "all command targets in the full-width Insert fixture");
        let nested_root = TopLevel::get_top_level(Some(&nested[0])).unwrap();
        capture(&nested_root, &directory, "keytip-insert-popup-root.png");
        for badge in nested {
            assert!(badge.bounds().width > 0. && badge.bounds().height > 0.);
        }
        ribbon.cancel_key_tips();
        window.close();
    }, CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}
