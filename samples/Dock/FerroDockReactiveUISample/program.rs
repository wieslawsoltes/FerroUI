// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original sample desktop entry point and source builder configuration.
use ferrodock_reactiveui_sample::App;
use ferrodock_settings::AppBuilderExtensions as DockAppBuilderExtensions;
use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_reactiveui_controls::AppBuilderExtensions as ReactiveUiAppBuilderExtensions;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    build_ferro_app().start_with_classic_desktop_lifetime(&args);
}
/// Configures the original sample's native platform, font, ReactiveUI and dock settings.
pub fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .with_inter_font()
        .use_reactive_ui(|_| {})
        .show_dockable_preview_on_drag(true)
        .set_drag_preview_opacity(0.6)
        .log_to_trace(LogEventLevel::Warning, &[])
}
