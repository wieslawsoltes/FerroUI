//! Port of `Platforms/iOS/AppDelegate.cs`: the iOS entry point of the SingleProjectSandbox
//! sample.
//!
//! ```text
//! scripts/ios/bundle.sh --package single-project-sandbox --bin single-project-sandbox-ios \
//!     --name "Single Project Sandbox" --bundle-id org.ferroui.singleprojectsandbox
//! ```
//!
//! The application delegate of the reference creates the builder of the application with the
//! iOS platform (the builder of `App` with `UseiOS()`); `Main` starts UIKit with it. The script
//! writes the property list the reference keeps as `Info.plist`; the launch screen
//! (`Resources/LaunchScreen.xib`) is an empty launch screen of the property list instead.
//!
//! Addition of the port, for the smoke runs: with `FERROUI_SMOKE_EXIT_MS=<n>` in the
//! environment (`SIMCTL_CHILD_FERROUI_SMOKE_EXIT_MS` of `simctl launch`) the process prints
//! `SANDBOX SHOWN` once the main view has a size and exits after `n` milliseconds.

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("single-project-sandbox-ios: the iOS entry point of the sample; it needs UIKit");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(host::AppDelegate))
}

#[cfg(target_os = "ios")]
mod host {
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use single_project_sandbox::App;
    use std::rc::Rc;
    use std::time::Duration;

    /// The application delegate of the reference: the delegate of the platform with `App` as
    /// its type argument.
    pub struct AppDelegate;

    impl FerroApplicationDelegate for AppDelegate {
        fn create_app_builder(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            App::build_ferro_app().use_ios_with_delegate(app_delegate)
        }

        fn customize_app_builder(&self, builder: AppBuilder) -> AppBuilder {
            builder.after_setup(|_| smoke_run())
        }
    }

    fn smoke_run() {
        let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) else {
            return;
        };
        let _shown = DispatcherTimer::run(
            || {
                let lifetime = Application::current().and_then(|application| application.application_lifetime());
                let view = lifetime.as_ref().and_then(|l| l.as_single_view_application_lifetime()).and_then(|l| l.main_view());
                match view {
                    Some(view) if view.bounds().width > 1.0 => {
                        println!("SANDBOX SHOWN {}x{}", view.bounds().width, view.bounds().height);
                        false
                    }
                    _ => true,
                }
            },
            Duration::from_millis(250),
            DispatcherPriority::NORMAL,
        )
        .detach();
        let _timer = DispatcherTimer::run_once(
            || {
                println!("SANDBOX FINISHED");
                std::process::exit(0);
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }
}
