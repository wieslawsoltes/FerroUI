//! Port of `Platforms/Android/Application.cs` and `Platforms/Android/MainActivity.cs`: the
//! Android entry point of the SingleProjectSandbox sample, compiled into the shared library of
//! the package for an Android target.
//!
//! ```text
//! scripts/android/apk.sh sandbox
//! adb shell am start -n org.ferroui.singleprojectsandbox/org.ferroui.android.FerroMainActivity \
//!     --es FERROUI_SMOKE_EXIT_MS 8000
//! adb logcat -s ferroui-sandbox
//! ```
//!
//! The application class of the reference derives from the application class of the backend
//! with the application of the sample as its type argument and adds nothing: its builder is
//! `AppBuilder.Configure<App>().UseAndroid()` (the default `CreateAppBuilder`), which the
//! library gives the backend here (`android_application!`). The main activity of the reference
//! adds nothing but its manifest attributes (the theme without an action bar, the configuration
//! changes it handles itself), which are the manifest `scripts/android/apk.sh` writes.
//!
//! Addition of the port, for the smoke runs: with the string extra `FERROUI_SMOKE_EXIT_MS=<n>`
//! of the intent, `SANDBOX SHOWN` is written to the log of the system under the tag
//! `ferroui-sandbox` once the main view has a size, and the activity is finished after `n`
//! milliseconds. The warnings and errors of the framework go to the log under the same tag.

ferroui_android::android_application!(build);

use ferroui_android::log::{self, LogPriority};
use ferroui_android::{AndroidApplicationExtensions, FerroActivity};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::AppBuilder;
use std::cell::Cell;
use std::time::Duration;

/// The tag of the lines of the sample in the log of the system.
const TAG: &str = "ferroui-sandbox";

fn note(text: &str) {
    log::write(LogPriority::Info, TAG, text);
}

/// The application builder of the sample.
fn build() -> AppBuilder {
    AppBuilder::configure::<crate::App>()
        .use_android()
        .log_to_delegate(|line| log::write(LogPriority::Warn, TAG, line), LogEventLevel::Warning, &[])
        .after_setup(|_| smoke_run())
}

fn smoke_run() {
    let started = Cell::new(false);
    let _timer = DispatcherTimer::run(
        move || {
            let Some(activity) = FerroActivity::current_main_activity() else { return true };
            let Some(top_level) = activity.view().and_then(|view| view.top_level()) else { return true };
            if top_level.client_size().width <= 1.0 || started.replace(true) {
                return !started.get();
            }
            note(&format!("SANDBOX SHOWN {}x{}", top_level.client_size().width, top_level.client_size().height));
            if let Some(ms) = activity.intent_string_extra("FERROUI_SMOKE_EXIT_MS").and_then(|value| value.trim().parse::<u64>().ok()) {
                let _finish = DispatcherTimer::run_once(
                    || {
                        if let Some(activity) = FerroActivity::current_main_activity() {
                            activity.finish();
                        }
                        note("SANDBOX FINISHED");
                    },
                    Duration::from_millis(ms),
                    DispatcherPriority::NORMAL,
                )
                .detach();
            }
            false
        },
        Duration::from_millis(250),
        DispatcherPriority::NORMAL,
    )
    .detach();
}
