//! The Android entry point of the frame benchmarks: a scene of `tests/FerroUI.FrameBenchmarks` as
//! the main view of an activity, for the window suite of the local performance gate in an
//! emulator (`scripts/perf/gate.py --only window --window-device android`).
//!
//! ```text
//! scripts/android/apk.sh package --package ferroui-frame-benchmarks-android --release
//! adb install -r <the APK the script names>
//! adb shell am start -n org.ferroui.ferroui.frame.benchmarks.android/org.ferroui.android.FerroMainActivity \
//!     --es FRAMES_SCENE wall
//! adb logcat -v raw -s ferroui-frames:I
//! ```
//!
//! An Android application has no environment of its own and no standard output that is read, so:
//!
//! - the scene is the string extra `FRAMES_SCENE` of the intent that starts the activity
//!   (default `text`);
//! - the lines of the benchmark (`PERF scene <name>`, `PERF first_frame`, `PERF second_frame`
//!   and `PERF fps N` once a second) go to the log of the system under the tag `ferroui-frames`,
//!   at the priority of an information; the warnings and errors of the framework go to the same
//!   tag.
//!
//! The application is the one of the desktop program (`ferroui_frame_benchmarks::App`).

#[cfg(target_os = "android")]
ferroui_android::android_application!(application::build);

#[cfg(target_os = "android")]
mod application {
    use ferroui_android::log::{self, LogPriority};
    use ferroui_android::{AndroidApplicationExtensions, FerroActivity};
    use ferroui_base::logging::LogEventLevel;
    use ferroui_controls::AppBuilder;
    use ferroui_frame_benchmarks::{report_to, scene_from, App};

    /// The tag of the lines of the benchmark in the log of the system.
    const TAG: &str = "ferroui-frames";

    /// The scene the intent of the main activity names. The activity is known once it is resumed,
    /// which is after its main view is made: the application asks again until then.
    fn scene() -> Option<String> {
        let activity = FerroActivity::current_main_activity()?;
        Some(activity.intent_string_extra("FRAMES_SCENE").unwrap_or_else(|| "text".to_string()))
    }

    pub fn build() -> AppBuilder {
        report_to(|line| log::write(LogPriority::Info, TAG, line));
        scene_from(scene);
        AppBuilder::configure::<App>().use_android().log_to_delegate(
            |line| log::write(LogPriority::Warn, TAG, line),
            LogEventLevel::Warning,
            &[],
        )
    }
}
