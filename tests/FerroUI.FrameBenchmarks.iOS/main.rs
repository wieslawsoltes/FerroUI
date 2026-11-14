//! The iOS entry point of the frame benchmarks: a scene of `tests/FerroUI.FrameBenchmarks` as the
//! main view of an application, for the window suite of the local performance gate in a
//! simulator (`scripts/perf/gate.py --only window --window-device ios`).
//!
//! ```text
//! cargo build -p ferroui-frame-benchmarks-ios --target aarch64-apple-ios-sim --release
//! scripts/ios/bundle.sh --package ferroui-frame-benchmarks-ios --bin frame-benchmarks-ios \
//!     --profile release --name FrameBenchmarks --bundle-id org.ferroui.framebenchmarks --no-build
//! xcrun simctl install booted <the bundle the script names>
//! SIMCTL_CHILD_FRAMES_SCENE=wall SIMCTL_CHILD_PERF_REPORT=1 \
//!     xcrun simctl launch --console-pty --terminate-running-process booted org.ferroui.framebenchmarks
//! ```
//!
//! The scene and the report come from the environment, as in the desktop program (`simctl launch`
//! passes `SIMCTL_CHILD_<NAME>` to the application as `<NAME>`): `FRAMES_SCENE` names the scene
//! (default `text`), and the application prints `PERF scene <name>`, `PERF first_frame`,
//! `PERF second_frame` and, with `PERF_REPORT=1`, `PERF fps N` once a second.
//!
//! The application is the one of the desktop program (`ferroui_frame_benchmarks::App`).

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("frame-benchmarks-ios: this is the iOS entry point of the frame benchmarks; it needs UIKit");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(host::AppDelegate))
}

#[cfg(target_os = "ios")]
mod host {
    use ferroui_base::logging::LogEventLevel;
    use ferroui_controls::AppBuilder;
    use ferroui_frame_benchmarks::App;
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use std::rc::Rc;

    /// The application delegate of the benchmark.
    pub struct AppDelegate;

    impl FerroApplicationDelegate for AppDelegate {
        fn create_app_builder(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            AppBuilder::configure::<App>().use_ios_with_delegate(app_delegate)
        }

        fn customize_app_builder(&self, builder: AppBuilder) -> AppBuilder {
            builder.log_to_trace(LogEventLevel::Warning, &[])
        }
    }
}
