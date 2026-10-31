//! The optional desktop entry point; uses the same compiled view as the renderer.

use ferroui_controls::Application;
use ferroui_desktop::AppBuilderDesktopExtensions;
use livecharts_demo::{app_builder, DemoView};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    app_builder().use_platform_detect().start(
        |app: &ferroui_base::Ref<Application>, _| {
            let view = DemoView::new().expect("the compiled gallery loads");
            app.run_window(view.create_window());
        },
        &args,
    );
}
