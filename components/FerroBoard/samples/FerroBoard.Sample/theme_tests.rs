use crate::{composition::SampleSession, tests::run};
use ferroboard_core::settings::ThemeMode;
use ferroui_base::{
    controls::ResourceKey,
    media::{Color, IBrush},
    metadata::from_markup_value,
    styling::ThemeVariant,
    threading::Dispatcher,
};
use ferroui_controls::{Application, Control, TextBlock};
use std::rc::Rc;
fn color(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush
        .expect("theme brush")
        .as_solid_color_brush()
        .expect("solid theme brush")
        .color()
}
#[test]
fn default_application_loads_winui_resources_and_control_themes() {
    run(|| {
        let app = Application::current().unwrap();
        assert_eq!(app.styles().count(), 1);
        assert_eq!(
            app.styles()
                .get(0)
                .as_object()
                .unwrap()
                .get_type()
                .full_name(),
            "FerroUI.Themes.WinUI.WinUITheme"
        );
        for (variant, foreground, background) in [
            (ThemeVariant::light(), "#E4000000", "#F3F3F3"),
            (ThemeVariant::dark(), "#FFFFFFFF", "#202020"),
        ] {
            let value = |key: &str| {
                from_markup_value::<Color>(
                    &app.try_get_resource(&ResourceKey::from(key), Some(&variant))
                        .unwrap(),
                )
                .unwrap()
            };
            assert_eq!(
                value("TextFillColorPrimary"),
                Color::parse(foreground).unwrap()
            );
            assert_eq!(
                value("SolidBackgroundFillColorBase"),
                Color::parse(background).unwrap()
            );
        }
    });
}
#[test]
fn sample_theme_mode_keeps_window_keyboard_and_settings_headers_in_sync_live() {
    run(|| {
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.bindings.model().set_settings_pane_open(true);
        session.window.show();
        let ui = crate::test_ui::ui(&session);
        let caption = |name| {
            let input = ui.find(name).unwrap().cast::<Control>().unwrap();
            input
                .visual_parent()
                .unwrap()
                .visual_children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
        };
        let captions = [
            caption("KeyHeightSlider"),
            caption("ThemeModeChoice"),
            caption("NumberRowToggle"),
        ];
        for (mode, variant, expected) in [
            (ThemeMode::Dark, ThemeVariant::dark(), "#FFFFFFFF"),
            (ThemeMode::Light, ThemeVariant::light(), "#E4000000"),
            (ThemeMode::Dark, ThemeVariant::dark(), "#FFFFFFFF"),
        ] {
            session.keyboard.settings().set_theme_mode(mode);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                session.window.requested_theme_variant(),
                Some(variant.clone())
            );
            assert_eq!(
                session.keyboard.effective_theme().is_dark(),
                mode == ThemeMode::Dark
            );
            for caption in &captions {
                assert_eq!(
                    color(caption.foreground()),
                    Color::parse(expected).unwrap(),
                    "caption {:?}; theme {:?}",
                    caption.text(),
                    caption.actual_theme_variant()
                );
            }
        }
        session
            .keyboard
            .settings()
            .set_theme_mode(ThemeMode::FollowSystem);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(session.window.requested_theme_variant(), None);
        session.window.close();
        session.release();
    });
}
#[test]
fn theme_overrides_are_scoped_to_each_sample_window() {
    run(|| {
        let first = SampleSession::new(crate::storage_tests::file_storage());
        let second = SampleSession::new(crate::storage_tests::file_storage());
        first.window.show();
        second.window.show();
        first.keyboard.settings().set_theme_mode(ThemeMode::Dark);
        second.keyboard.settings().set_theme_mode(ThemeMode::Light);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            first.window.requested_theme_variant(),
            Some(ThemeVariant::dark())
        );
        assert_eq!(
            second.window.requested_theme_variant(),
            Some(ThemeVariant::light())
        );
        first
            .keyboard
            .settings()
            .set_theme_mode(ThemeMode::FollowSystem);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(first.window.requested_theme_variant(), None);
        assert_eq!(
            second.window.requested_theme_variant(),
            Some(ThemeVariant::light())
        );
        first.window.close();
        second.window.close();
        first.release();
        second.release();
    });
}
