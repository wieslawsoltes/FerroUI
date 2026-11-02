//! Port of `Main.cs` and `AppDelegate.cs` of `SafeAreaDemo.iOS`: the iOS entry point of the
//! SafeAreaDemo sample.
//!
//! ```text
//! scripts/ios/bundle.sh --package safe-area-demo-ios --bin safe-area-demo-ios \
//!     --name SafeAreaDemo --bundle-id org.ferroui.safeareademo
//! ```
//!
//! The script puts the executable into an application bundle and writes the property list the
//! reference keeps as `Info.plist` (the display name, the device families and the four
//! orientations of the reference; the identifier of the reference, `companyName.SafeAreaDemo`,
//! is a placeholder, and the bundle is named `org.ferroui.safeareademo` instead). The application
//! delegate of the reference derives from the delegate of the platform with the application of
//! the sample as its type argument and overrides nothing: its builder is
//! `AppBuilder.Configure<App>().UseiOS()`, which [`host::AppDelegate`] gives the platform.
//!
//! Additions of the port, for the smoke runs, from the environment (`simctl launch` passes
//! `SIMCTL_CHILD_<NAME>` to the application as `<NAME>`):
//!
//! - `FERROUI_SMOKE_STEPS_MS=<n>`: once the main view shows, the options of the view are changed
//!   one after the other, one every `n` milliseconds (`safe_area_demo::smoke`), and the state of
//!   the view model is printed after each: `SAFEAREA SHOWN`, `SAFEAREA STEP <index> <name>: ...`
//!   and `SAFEAREA DONE`.
//! - `FERROUI_SMOKE_EXIT_MS=<n>`: the process exits `n` milliseconds after the steps (after the
//!   start, without steps).
//!
//! Not ported: the launch screen (`Resources/LaunchScreen.xib`: the bundle states an empty launch
//! screen instead) and the entitlements file, which is empty.

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("safe-area-demo-ios: this is the iOS entry point of the SafeAreaDemo sample; it needs UIKit");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(host::AppDelegate))
}

#[cfg(target_os = "ios")]
mod host {
    use ferroui_base::logging::LogEventLevel;
    use ferroui_base::Ref;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use safe_area_demo::view_models::MainViewModel;
    use safe_area_demo::views::MainView;
    use safe_area_demo::{smoke, App};
    use std::rc::Rc;
    use std::time::Duration;

    /// The application delegate of the sample.
    pub struct AppDelegate;

    impl FerroApplicationDelegate for AppDelegate {
        fn create_app_builder(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            AppBuilder::configure::<App>().use_ios_with_delegate(app_delegate)
        }

        fn customize_app_builder(&self, builder: AppBuilder) -> AppBuilder {
            builder.after_setup(|_| smoke_run()).log_to_trace(LogEventLevel::Warning, &[])
        }
    }

    fn environment_milliseconds(name: &str) -> Option<u64> {
        std::env::var(name).ok().and_then(|value| value.parse::<u64>().ok())
    }

    /// The view model of the main view of the application, once the view is loaded.
    fn main_view_model() -> Option<Rc<MainViewModel>> {
        let lifetime = Application::current()?.application_lifetime()?;
        let main_view = lifetime.as_single_view_application_lifetime()?.main_view()?;
        let main_view: Ref<MainView> = main_view.cast::<MainView>()?;
        smoke::main_view_model(&main_view)
    }

    fn exit_after(ms: u64) {
        println!("Will exit after {ms} ms");
        let _timer = DispatcherTimer::run_once(
            || {
                println!("SAFEAREA FINISHED");
                std::process::exit(0);
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }

    fn note(text: &str) {
        println!("{text}");
    }

    /// The smoke run asked for with `FERROUI_SMOKE_STEPS_MS` and `FERROUI_SMOKE_EXIT_MS`.
    fn smoke_run() {
        let exit = environment_milliseconds("FERROUI_SMOKE_EXIT_MS");
        match environment_milliseconds("FERROUI_SMOKE_STEPS_MS") {
            Some(step_ms) => smoke::run(main_view_model, Duration::from_millis(step_ms), note, |_, _| {}, move || {
                if let Some(exit) = exit {
                    exit_after(exit);
                }
            }),
            None => {
                if let Some(exit) = exit {
                    exit_after(exit);
                }
            }
        }
    }
}
