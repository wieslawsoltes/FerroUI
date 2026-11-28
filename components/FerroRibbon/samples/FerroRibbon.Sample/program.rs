#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use ferroribbon_sample::view_model::{InMemoryDocument, SampleViewModel};
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_impl_classes, instantiate};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::{cell::Cell, rc::Rc};
#[repr(C)]
struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroribbon_sample::initialize_gallery(this);
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(desktop) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let window = if std::env::args().any(|arg| arg == "--cad") {
                ferroribbon_sample::create_cad_window()
            } else if std::env::args().any(|arg| arg == "--settings") {
                ferroribbon_sample::create_settings_window()
            } else if std::env::args().any(|arg| arg == "--powerpoint") {
                ferroribbon_sample::create_powerpoint_window()
            } else if std::env::args().any(|arg| arg == "--excel") {
                ferroribbon_sample::create_excel_window()
            } else if std::env::args().any(|arg| arg == "--tools") {
                ferroribbon_sample::create_tools_window()
            } else if std::env::args().any(|arg| arg == "--word") {
                ferroribbon_sample::create_word_window()
            } else if std::env::args().any(|arg| arg == "--cad-canvas") {
                let window = ferroribbon_sample::create_cad_canvas_window();
                window
                    .opened(|| println!("FerroRibbon original CAD canvas opened"))
                    .detach();
                window
            } else if std::env::args().any(|arg| arg == "--integration") {
                let model = SampleViewModel::new(Rc::new(InMemoryDocument));
                let window = ferroribbon_sample::create_window(model);
                let title_bar = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroribbon::controls::RibbonTitleBar,
                >(&window, "DocumentTitleBar")
                .expect("compiled document title bar");
                match title_bar.attach_to_window(&window) {
                    Ok(state) => println!("FerroRibbon native title bar: {state:?}"),
                    Err(error) => eprintln!("FerroRibbon title-bar extension unavailable: {error}"),
                }
                window
                    .opened(|| println!("FerroRibbon native ribbon sample opened"))
                    .detach();
                window
                    .closed(|| println!("FerroRibbon native ribbon sample closed"))
                    .detach();
                window
            } else {
                ferroribbon_sample::create_shell_window()
            };
            #[cfg(windows)]
            window.set_icon(Some(
                ferroui_controls::WindowIcon::from_stream(
                    &mut &include_bytes!("windows/icon.ico")[..],
                )
                .expect("embedded gallery icon"),
            ));
            if let Some(shell) = window
                .content()
                .as_ref()
                .and_then(ferroui_controls::Control::from_boxed)
                .and_then(|c| c.cast::<ferroribbon_sample::shell_page::ShellPage>())
            {
                // The desktop lifetime shows the main window after application setup.
                let pending_shell = Cell::new(Some(shell.downgrade()));
                let owner = window.downgrade();
                window
                    .opened(move || {
                        let Some(shell) = pending_shell.take().and_then(|shell| shell.upgrade())
                        else {
                            return;
                        };
                        if let Some(window) = owner.upgrade()
                            && let Err(error) =
                                ferroribbon_sample::native_capture::start(window, shell)
                        {
                            eprintln!("[capture] capture failed: {error}");
                        }
                    })
                    .detach();
            }
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--print-build-fingerprint"] {
        println!(
            "sample={}",
            option_env!("FERRORIBBON_BUILD_FINGERPRINT").unwrap_or("development")
        );
        println!(
            "library={}",
            ferroribbon::build_source_fingerprint().unwrap_or("development")
        );
        return;
    }
    AppBuilder::configure::<App>()
        .with(Rc::new(ferroribbon_sample::gallery_font_options()))
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon::theming::{RibbonTheme, RibbonThemeResources};
    use ferroui_base::{
        FerroLocator, LocatorExtensions,
        media::{Color, SolidColorBrush},
        platform::{
            ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings,
            PlatformColorValues, PlatformThemeVariant, SystemColors,
        },
        styling::ThemeVariant,
        threading::CancellationToken,
    };
    use ferroui_headless::{FerroHeadlessPlatformExtensions, HeadlessUnitTestSession};
    use std::sync::Arc;

    fn gallery_with_test_platform_settings() -> Arc<HeadlessUnitTestSession> {
        HeadlessUnitTestSession::start_new(|| {
            AppBuilder::configure::<App>()
                .use_headless(Default::default())
                .after_platform_services_setup(|_| {
                    let settings = Rc::new(DefaultPlatformSettings::new());
                    FerroLocator::current_mutable()
                        .bind::<dyn IPlatformSettings>()
                        .to_constant(settings.clone());
                    FerroLocator::current_mutable()
                        .bind::<DefaultPlatformSettings>()
                        .to_constant(settings);
                })
        })
    }

    #[test]
    fn application_initializes_the_ribbon_theme_before_creating_the_shell() {
        let session = HeadlessUnitTestSession::start_new(|| {
            AppBuilder::configure::<App>().use_headless(Default::default())
        });
        let result = session
            .dispatch(
                || {
                    let app = Application::current().expect("sample application");
                    assert_eq!(app.styles().count(), 1);
                    let ribbon_dictionaries = || {
                        app.resources()
                            .merged_dictionaries()
                            .snapshot()
                            .iter()
                            .filter_map(|provider| {
                                provider
                                    .as_object()?
                                    .downcast_ref::<RibbonThemeResources>()
                                    .map(RibbonThemeResources::to_ref)
                            })
                            .collect::<Vec<_>>()
                    };
                    let before = ribbon_dictionaries();
                    assert_eq!(before.len(), 1, "the application supplies the ribbon theme");

                    let _shell = ferroribbon_sample::create_shell_window();
                    let after = ribbon_dictionaries();
                    assert_eq!(after.len(), 1);
                    assert!(before[0].ptr_eq(&after[0]));
                    assert!(before[0].ptr_eq(&RibbonTheme::resources()));
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }

    #[test]
    fn gallery_tracks_system_contrast_and_restores_the_cad_theme_choice() {
        let session = gallery_with_test_platform_settings();
        let result = session
            .dispatch(
                || {
                    let settings = FerroLocator::current()
                        .get_service::<DefaultPlatformSettings>()
                        .expect("gallery test platform settings");
                    let app = Application::current().expect("sample application");
                    let normal = PlatformColorValues::new();
                    let dark_colors = SystemColors::high_contrast_black();
                    let contrast = normal
                        .with_contrast_preference(ColorContrastPreference::High)
                        .with_system_colors(Some(dark_colors));
                    let high = Some(RibbonThemeResources::high_contrast_variant());
                    settings.set_color_values(contrast);
                    assert_eq!(app.actual_theme_variant(), high);

                    let window = ferroribbon_sample::create_cad_window();
                    let page = ferroui_base::data::core::ValueTypes::as_object(
                        &*window.content().expect("CAD window content"),
                    )
                    .and_then(|control| control.cast::<ferroribbon_sample::pages::CadPage>())
                    .expect("CAD page");
                    assert_eq!(page.actual_theme_variant(), high);
                    let background = || {
                        RibbonTheme::get_brush(Some(&page), Some("RibbonCommandBarBackgroundBrush"))
                            .unwrap()
                            .and_then(|brush| brush.cast::<SolidColorBrush>())
                            .expect("CAD command bar surface brush")
                    };
                    let contrast_brush = background();
                    assert_eq!(contrast_brush.color(), dark_colors.button_face());

                    let light_colors = SystemColors::new(
                        Color::from_rgb(255, 250, 239),
                        Color::from_rgb(40, 35, 30),
                    )
                    .with_button_face(Color::from_rgb(238, 224, 202));
                    let light_contrast = contrast
                        .with_theme_variant(light_colors.theme_variant())
                        .with_system_colors(Some(light_colors));
                    settings.set_color_values(light_contrast);
                    assert_eq!(page.actual_theme_variant(), high);
                    assert!(background().ptr_eq(&contrast_brush));
                    assert_eq!(contrast_brush.color(), light_colors.button_face());

                    page.set_light_theme(true);
                    assert_eq!(page.actual_theme_variant(), high);

                    settings.set_color_values(normal);
                    assert_eq!(page.actual_theme_variant(), Some(ThemeVariant::light()));
                    assert!(!background().ptr_eq(&contrast_brush));
                    settings.set_color_values(contrast);
                    assert_eq!(page.actual_theme_variant(), high);

                    page.set_light_theme(false);
                    settings.set_color_values(normal);
                    assert_eq!(page.actual_theme_variant(), Some(ThemeVariant::dark()));
                    assert_eq!(app.actual_theme_variant(), Some(ThemeVariant::light()));
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }

    #[test]
    fn word_theme_toggle_respects_system_high_contrast() {
        let session = gallery_with_test_platform_settings();
        let result = session
            .dispatch(
                || {
                    use ferroribbon_sample::{
                        pages::WordPage, shell_page::ShellPage, word_view_model::WordAction,
                    };
                    use ferroui_controls::{Control, Window};

                    let settings = FerroLocator::current()
                        .get_service::<DefaultPlatformSettings>()
                        .expect("gallery test platform settings");
                    let window = ferroribbon_sample::create_shell_window();
                    let other_window = Window::new();
                    window.show();
                    other_window.show();
                    let shell = window
                        .content()
                        .as_ref()
                        .and_then(Control::from_boxed)
                        .and_then(|content| content.cast::<ShellPage>())
                        .expect("gallery window content");
                    shell.show("word").expect("open Word");
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    let page = shell
                        .current_content()
                        .expect("current gallery page")
                        .and_then(|content| content.cast::<WordPage>())
                        .expect("Word page");
                    let toggle = || {
                        page.view_model()
                            .action()
                            .execute(Some(&Some(WordAction::ToggleDark)));
                    };
                    let light = Some(ThemeVariant::light());
                    let dark = Some(ThemeVariant::dark());
                    let high = Some(RibbonThemeResources::high_contrast_variant());
                    let normal = PlatformColorValues::new();
                    let contrast = normal
                        .with_contrast_preference(ColorContrastPreference::High)
                        .with_system_colors(Some(SystemColors::high_contrast_black()));

                    settings.set_color_values(normal);
                    assert_eq!(page.actual_theme_variant(), light);
                    toggle();
                    assert_eq!(page.actual_theme_variant(), dark);
                    assert_eq!(other_window.actual_theme_variant(), light);

                    settings.set_color_values(contrast);
                    assert_eq!(page.actual_theme_variant(), high);
                    toggle();
                    assert_eq!(page.actual_theme_variant(), high);
                    assert_eq!(other_window.actual_theme_variant(), high);

                    settings.set_color_values(normal.with_theme_variant(PlatformThemeVariant::Dark));
                    assert_eq!(page.actual_theme_variant(), light);
                    assert_eq!(other_window.actual_theme_variant(), dark);
                    toggle();
                    settings.set_color_values(normal);
                    assert_eq!(page.actual_theme_variant(), dark);
                    assert_eq!(other_window.actual_theme_variant(), light);
                    window.close();
                    other_window.close();
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }
}
