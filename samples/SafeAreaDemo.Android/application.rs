//! Port of `Application.cs` and `MainActivity.cs` of `SafeAreaDemo.Android`: the Android entry
//! point of the SafeAreaDemo sample.
//!
//! ```text
//! scripts/android/apk.sh safearea
//! adb install -r <the APK the script names>
//! adb shell am start -n org.ferroui.safeareademo/org.ferroui.android.FerroMainActivity \
//!     --es FERROUI_SMOKE_STEPS_MS 3000 --es FERROUI_SMOKE_EXIT_MS 3000
//! adb logcat -s ferroui-safearea
//! ```
//!
//! The application class of the reference derives from the application class of the backend
//! with the application of the sample as its type argument and adds nothing: its application
//! builder is `AppBuilder.Configure<App>().UseAndroid()`, which the library gives the backend
//! here (`android_application!`). The main activity of the reference derives from the main
//! activity of the backend and adds nothing but its manifest attributes (the label, the theme
//! without an action bar, the configuration changes it handles itself: orientation, screen size
//! and UI mode), which are the manifest `scripts/android/apk.sh` writes.
//!
//! Not ported: the splash screen and the icon resources of the project (`Resources/drawable`,
//! `Resources/values*`): the manifest of the script names the theme of the system without an
//! action bar.
//!
//! Additions of the port, for the smoke runs. They are string extras of the intent that starts
//! the activity (`am start --es NAME VALUE`), because an Android application has no environment
//! of its own:
//!
//! - `FERROUI_SMOKE_STEPS_MS=<n>`: once the main view shows, the options of the view are changed
//!   one after the other, one every `n` milliseconds (`safe_area_demo::smoke`), and the state of
//!   the view model is written to the log of the system under the tag `ferroui-safearea` after
//!   each: `SAFEAREA SHOWN`, `SAFEAREA STEP <index> <name>: ...` and `SAFEAREA DONE`.
//! - `FERROUI_SMOKE_EXIT_MS=<n>`: the activity is finished `n` milliseconds after the steps
//!   (after the start, without steps).
//!
//! The warnings and errors of the framework go to the log of the system under the same tag.

#[cfg(target_os = "android")]
ferroui_android::android_application!(application::build);

#[cfg(target_os = "android")]
mod application {
    use ferroui_android::log::{self, LogPriority};
    use ferroui_android::{AndroidApplicationExtensions, FerroActivity};
    use ferroui_base::logging::LogEventLevel;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::AppBuilder;
    use safe_area_demo::view_models::MainViewModel;
    use safe_area_demo::views::MainView;
    use safe_area_demo::{smoke, App};
    use std::rc::Rc;
    use std::time::Duration;

    /// The tag of the lines of the sample in the log of the system.
    const TAG: &str = "ferroui-safearea";

    fn note(text: &str) {
        log::write(LogPriority::Info, TAG, text);
    }

    /// The application builder of the sample (`CreateAppBuilder` of the application class of
    /// the backend, which the application class of the reference does not override).
    pub fn build() -> AppBuilder {
        AppBuilder::configure::<App>()
            .use_android()
            .log_to_delegate(|line| log::write(LogPriority::Warn, TAG, line), LogEventLevel::Warning, &[])
            .after_setup(|_| smoke_run())
    }

    /// The view model of the main view of the main activity, once the view is loaded.
    fn main_view_model() -> Option<Rc<MainViewModel>> {
        let top_level = FerroActivity::current_main_activity()?.view()?.top_level()?;
        if top_level.client_size().width <= 1.0 {
            return None;
        }
        let view = top_level.get_visual_descendants().find_map(|visual| visual.cast::<MainView>())?;
        smoke::main_view_model(&view)
    }

    fn extra_milliseconds(activity: &FerroActivity, name: &str) -> Option<u64> {
        activity.intent_string_extra(name).and_then(|value| value.trim().parse::<u64>().ok())
    }

    fn finish_after(ms: u64) {
        note(&format!("Will finish the activity after {ms} ms"));
        let _timer = DispatcherTimer::run_once(
            || {
                if let Some(activity) = FerroActivity::current_main_activity() {
                    activity.finish();
                }
                note("SAFEAREA FINISHED");
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }

    /// The smoke run asked for with the extras of the intent; the activity and its intent exist
    /// once the main view does, so the run waits for it first.
    fn smoke_run() {
        let _timer = DispatcherTimer::run(
            || {
                let Some(activity) = FerroActivity::current_main_activity() else { return true };
                if main_view_model().is_none() {
                    return true;
                }
                let exit = extra_milliseconds(&activity, "FERROUI_SMOKE_EXIT_MS");
                match extra_milliseconds(&activity, "FERROUI_SMOKE_STEPS_MS") {
                    Some(step_ms) => smoke::run(
                        main_view_model,
                        Duration::from_millis(step_ms),
                        note,
                        |_, _| {},
                        move || {
                            if let Some(exit) = exit {
                                finish_after(exit);
                            }
                        },
                    ),
                    None => {
                        if let Some(view_model) = main_view_model() {
                            note(&format!("SAFEAREA SHOWN {}", smoke::describe(&view_model)));
                        }
                        if let Some(exit) = exit {
                            finish_after(exit);
                        }
                    }
                }
                false
            },
            Duration::from_millis(250),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }
}
