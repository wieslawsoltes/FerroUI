use ferroui_controls::{AppBuilder, Application};
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_skia::SkiaApplicationExtensions;

pub(crate) fn start_if_requested() {
    RemoteDesignerEntryPoint::run_if_requested(|| {
        AppBuilder::configure::<Application>()
            .use_skia()
            .use_harfbuzz()
            .after_setup(|builder| {
                FerroRuntimeXamlLoader::register();
                ferroui_themes_winui::register_types();
                if let Some(app) = builder.instance() {
                    app.styles()
                        .add(ferroui_themes_winui::WinUITheme::new().as_style());
                }
            })
    });
}
