//! Compiled and runtime markup share native CAD lookup, object identity and presenters.
use super::*;
use ferroribbon::primitives::RibbonIconPresenter;
use ferroribbon_core::model::RibbonIcon;
use ferroui_base::{data::core::ValueTypes, threading::CancellationToken};
use ferroui_controls::{AppBuilder, Application, StackPanel, TextBlock, Viewbox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
#[test]
fn compiled_and_runtime_cad_extension_and_static_resources_use_native_presenters() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                register_types();
                assert!(MarkupType::find("FerroRibbon.Sample.Cad", "CadIcon").is_some());
                let compiled = generated::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_sample/CadIcons.xaml",
                )
                .unwrap()
                .unwrap();
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("CadIcons.xaml"),
                    Some(&ASSEMBLY),
                )
                .unwrap();
                for (mode, value) in [("compiled", compiled), ("runtime", runtime)] {
                    let host = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    assert_eq!(host.children().count(), 5);
                    for (index, name) in [(0, "Line"), (1, "LineHelp")] {
                        let presenter = host
                            .children()
                            .get(index)
                            .cast::<RibbonIconPresenter>()
                            .unwrap();
                        let value = presenter.icon().unwrap();
                        let icon = ferroui_base::metadata::from_markup_value::<Rc<RibbonIcon>>(
                            &Some(value),
                        )
                        .expect(mode);
                        assert!(Rc::ptr_eq(&icon, &cad::CadIcon::get(name).unwrap()));
                        assert!(presenter.has_icon());
                    }
                    let unknown = host
                        .children()
                        .get(2)
                        .cast::<RibbonIconPresenter>()
                        .unwrap();
                    assert_eq!(
                        unknown
                            .children()
                            .get(0)
                            .cast::<TextBlock>()
                            .unwrap()
                            .text()
                            .as_deref(),
                        Some("?")
                    );
                    for index in [3, 4] {
                        let presenter = host
                            .children()
                            .get(index)
                            .cast::<RibbonIconPresenter>()
                            .unwrap();
                        assert!(presenter.has_icon());
                        assert!(
                            presenter
                                .children()
                                .get(0)
                                .cast::<Viewbox>()
                                .unwrap()
                                .child()
                                .unwrap()
                                .is::<ferroui_controls::Canvas>()
                        );
                    }
                }
                // The real compiled application uses the same cached icons as the
                // standalone markup consumer and retains its MVVM command routing.
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let vm = SampleViewModel::new(Rc::new(view_model::InMemoryDocument));
                vm.set_active_tool("draw".to_owned());
                let window = create_window(vm.clone());
                window.show();
                window.update_layout();
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                let toolbar = ferroui_base::controls::NameScopeExtensions::find::<
                    ferroribbon::controls::RibbonToolBar,
                >(&window, "DrawOptions")
                .unwrap();
                assert!(toolbar.is_visible());
                for (index, name) in [(0, "Line"), (1, "HatchSolid")] {
                    let button = toolbar
                        .items()
                        .get(index)
                        .unwrap()
                        .cast::<ferroribbon::controls::RibbonButton>()
                        .unwrap();
                    let icon =
                        ferroui_base::metadata::from_markup_value::<Rc<RibbonIcon>>(&button.icon())
                            .unwrap();
                    assert!(Rc::ptr_eq(&icon, &cad::CadIcon::get(name).unwrap()));
                }
                toolbar
                    .items()
                    .get(0)
                    .unwrap()
                    .cast::<ferroribbon::controls::RibbonButton>()
                    .unwrap()
                    .perform_click();
                assert_eq!(vm.status(), "Pen applied to the sample document.");
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[cfg(windows)]
#[test]
fn windows_slide_out_pin_uses_the_local_winui_toggle_template() {
    use ferroribbon::controls::{RibbonButton, RibbonGroup};
    use ferroui_base::{
        CornerRadius, FerroLocator,
        media::Color,
        platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues},
        styling::ThemeVariant,
        threading::Dispatcher,
    };
    use ferroui_controls::{Control, TopLevel, primitives::ToggleButton};
    use ferroui_themes_winui::accents::SystemAccentColors;

    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                let _locator = FerroLocator::enter_scope();
                let settings = Rc::new(DefaultPlatformSettings::new());
                let color_values = |r, g, b| {
                    PlatformColorValues::new().with_accent_color1(Color::from_rgb(r, g, b))
                };
                let colors = color_values(162, 36, 70);
                settings.set_color_values(colors);
                FerroLocator::current_mutable()
                    .bind::<dyn IPlatformSettings>()
                    .to_constant(settings.clone());
                register_types();
                let app = Application::current().unwrap();
                app.styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                app.set_requested_theme_variant(Some(ThemeVariant::dark()));
                install_application_resources();

                let group = RibbonGroup::new();
                let item = RibbonButton::new();
                group.slide_out_items().add(Some(item.clone().upcast()));
                let window = Window::new();
                window.set_content(Some(Control::boxed(group.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                group.open_slide_out();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(group.is_slide_out_open());

                let root = TopLevel::get_top_level(Some(&item)).expect("slide-out visual root");
                root.update_layout();
                let pin = root
                    .get_visual_descendants()
                    .find_map(|visual| visual.cast::<ToggleButton>())
                    .expect("slide-out pin");
                let presenter = pin.presenter().expect("WinUI pin presenter");
                presenter.set_transitions(None);
                let brush_color = |brush: Rc<dyn ferroui_base::media::IBrush>| {
                    brush.as_solid_color_brush().unwrap().color()
                };
                let background = || brush_color(presenter.background().unwrap());
                assert_eq!(background().a, 0);
                group.set_is_slide_out_pinned(true);
                assert_eq!(pin.is_checked(), Some(true));
                let (_, shades) = SystemAccentColors::accent_colors(Some(&colors));
                assert_eq!(background(), shades.light2);
                assert_eq!(
                    brush_color(presenter.foreground().unwrap()),
                    Color::from_rgb(0, 0, 0)
                );
                assert_eq!(presenter.corner_radius(), CornerRadius::uniform(4.));

                let colors = color_values(35, 139, 70);
                settings.set_color_values(colors);
                let (_, shades) = SystemAccentColors::accent_colors(Some(&colors));
                assert_eq!(background(), shades.light2);

                group.set_is_slide_out_pinned(false);
                assert_eq!(background().a, 0);
                group.close_slide_out();
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
