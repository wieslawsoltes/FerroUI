//! Port of `Application.cs` and `MainActivity.cs` of the Android host of
//! the ControlCatalog sample: the Android entry point of the catalog.
//!
//! ```text
//! scripts/android/apk.sh catalog
//! scripts/android/emu-catalog.sh
//! ```
//!
//! The application class of the reference derives from the application
//! class of the backend and customizes the application builder; here the
//! library gives the backend the function that makes the builder
//! (`android_application!`). The main activity of the reference derives
//! from the main activity of the backend and adds nothing but its manifest
//! attributes, which are the manifest `scripts/android/apk.sh` writes: the
//! activity is the main activity class of the Java layer of the backend.
//!
//! Not ported yet: the native control demo (`EmbedSample.Android.cs`, with
//! the native control host of the backend, stage 2 of
//! `docs/porting/android-platform.md`), the activity for the `OpenUri`
//! activation (`DataSchemeActivity`, with the intents of stage 2), and the
//! splash screen resources.
//!
//! Additions of the port, for the smoke runs
//! (`scripts/android/emu-catalog.sh`), with the names of the options of the
//! desktop host. They are string extras of the intent that starts the
//! activity (`am start --es NAME VALUE`), because an Android application has
//! no environment of its own:
//!
//! - `FERROUI_SMOKE_PAGES=<n>`: the pages of the catalog are selected one
//!   after the other, each for `n` milliseconds. Half way through the time
//!   of a page the line `PAGE-SHOWN <index> <header>` is written to the log
//!   of the system under the tag `ferroui-catalog`, and `CATALOG DONE` after
//!   the last page.
//! - `FERROUI_SMOKE_PAGE_LIST=<header>;<header>;...`: only these pages, in
//!   this order.
//! - `FERROUI_SMOKE_EXIT_MS=<n>`: the activity is finished `n` milliseconds
//!   after the pages were shown (after the start, without pages).
//!
//! The file `ferroui.properties` of the files directory of the application,
//! when it exists, selects the rendering mode with the line
//! `rendering=software` (the platform is initialized before an activity and
//! its intent exist).
//!
//! The warnings and errors of the framework go to the log of the system
//! under the same tag.

#[cfg(target_os = "android")]
ferroui_android::android_application!(application::build);

#[cfg(target_os = "android")]
mod application {
    use control_catalog::models::PageItem;
    use control_catalog::view_models::MainWindowViewModel;
    use control_catalog::{App, MainView};
    use ferroui_android::interop::java::{call_object, string_of};
    use ferroui_android::log::{self, LogPriority};
    use ferroui_android::{
        AndroidApplicationExtensions, AndroidPlatformOptions, AndroidRenderingMode, FerroActivity,
        FerroAndroidApplication,
    };
    use ferroui_base::logging::LogEventLevel;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::AppBuilder;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    /// The tag of the lines of the catalog in the log of the system.
    const TAG: &str = "ferroui-catalog";

    fn note(text: impl AsRef<str>) {
        log::write(LogPriority::Info, TAG, text.as_ref());
    }

    /// Whether the properties file of the application asks for software
    /// rendering.
    fn software_rendering() -> bool {
        let context = FerroAndroidApplication::context();
        let path = call_object(&context, "getFilesDir", "()Ljava/io/File;", &[])
            .and_then(|directory| call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[]))
            .map(|path| string_of(&path));
        let text = path.and_then(|path| std::fs::read_to_string(format!("{path}/ferroui.properties")).ok());
        text.as_deref()
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.trim().split_once('='))
            .any(|(name, value)| name.trim() == "rendering" && value.trim() == "software")
    }

    /// The application builder of the catalog (`CreateAppBuilder` and
    /// `CustomizeAppBuilder` of the application class of the reference).
    pub fn build() -> AppBuilder {
        let mut options = AndroidPlatformOptions::default();
        if software_rendering() {
            options.rendering_mode = vec![AndroidRenderingMode::Software];
        }
        note(format!("ControlCatalog: rendering modes {:?}", options.rendering_mode));

        // The reference sets the native control demo of the embed page here
        // (`EmbedSampleAndroid`): with the native control host of the backend.
        AppBuilder::configure::<App>()
            .with(Rc::new(options))
            .use_android()
            .log_to_delegate(|line| log::write(LogPriority::Warn, TAG, line), LogEventLevel::Warning, &[])
            .after_setup(|_| smoke_run())
    }

    /// The view model of the main view, once the main activity shows it.
    fn main_view_model() -> Option<Rc<MainWindowViewModel>> {
        let top_level = FerroActivity::current_main_activity()?.view()?.top_level()?;
        if top_level.client_size().width <= 1.0 {
            return None;
        }
        let view = top_level.get_visual_descendants().find_map(|visual| visual.cast::<MainView>())?;
        from_markup_value::<Rc<MainWindowViewModel>>(&view.data_context())
    }

    /// The pages the drawer offers: the home page, the pages of the
    /// sections and the settings page.
    fn pages(view_model: &MainWindowViewModel) -> Vec<Rc<PageItem>> {
        let mut pages = vec![view_model.home_item()];
        for section in view_model.home_sections().iter() {
            pages.extend(section.items().unwrap_or_default().iter().cloned());
        }
        pages.push(view_model.settings_item());
        pages
    }

    fn extra_milliseconds(activity: &FerroActivity, name: &str) -> Option<u64> {
        activity.intent_string_extra(name).and_then(|value| value.trim().parse::<u64>().ok())
    }

    fn finish_after(ms: u64) {
        note(format!("Will finish the activity after {ms} ms"));
        let _timer = DispatcherTimer::run_once(
            || {
                note("Timer fired: finishing the activity");
                if let Some(activity) = FerroActivity::current_main_activity() {
                    activity.finish();
                }
                note("CATALOG FINISHED");
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }

    /// The smoke run asked for with the extras of the intent: waits for the
    /// main view, then shows the pages and finishes the activity as asked.
    fn smoke_run() {
        // What the run does next: wait for the view (None), or the pages
        // that are left with the time of each.
        let plan: RefCell<Option<(Vec<Rc<PageItem>>, u64, Option<u64>)>> = RefCell::new(None);
        let next = Cell::new(0usize);
        // Whether the line of the current page was written.
        let announced = Cell::new(true);
        let waited = Cell::new(0u32);
        // The ticks of the timer that are left of the current half of the time of a page.
        let countdown = Cell::new(0u64);

        let _timer = DispatcherTimer::run(
            move || {
                if plan.borrow().is_none() {
                    let Some(view_model) = main_view_model() else {
                        waited.set(waited.get() + 1);
                        if waited.get() % 40 == 0 {
                            note(format!("Waiting for the main view ({} s)", waited.get() / 4));
                        }
                        return true;
                    };
                    let Some(activity) = FerroActivity::current_main_activity() else { return true };
                    note("The main view is shown");

                    let exit = extra_milliseconds(&activity, "FERROUI_SMOKE_EXIT_MS");
                    let Some(page_ms) = extra_milliseconds(&activity, "FERROUI_SMOKE_PAGES") else {
                        if let Some(exit) = exit {
                            finish_after(exit);
                        }
                        return false;
                    };
                    let all = pages(&view_model);
                    let selected = match activity.intent_string_extra("FERROUI_SMOKE_PAGE_LIST") {
                        Some(list) => list
                            .split(';')
                            .map(str::trim)
                            .filter(|header| !header.is_empty())
                            .filter_map(|header| {
                                let page = all.iter().find(|page| page.header() == header).cloned();
                                if page.is_none() {
                                    note(format!("The catalog has no page '{header}'"));
                                }
                                page
                            })
                            .collect(),
                        None => all,
                    };
                    note(format!("Showing {} page(s), {page_ms} ms each", selected.len()));
                    *plan.borrow_mut() = Some((selected, page_ms, exit));
                    return true;
                }

                let plan = plan.borrow();
                let Some((pages, page_ms, exit)) = plan.as_ref() else { return true };
                if countdown.get() > 0 {
                    countdown.set(countdown.get() - 1);
                    return true;
                }
                // Half of the time of a page, in ticks of 250 ms; this tick is one of them.
                let half = (page_ms / 2).div_ceil(250).max(1) - 1;

                // The second half of the time of a page begins: it is drawn.
                if !announced.replace(true) {
                    let index = next.get() - 1;
                    note(format!("PAGE-SHOWN {index} {}", pages[index].header()));
                    countdown.set(half);
                    return true;
                }

                let index = next.get();
                if let (Some(item), Some(view_model)) = (pages.get(index), main_view_model()) {
                    note(format!("Selecting {}", item.header()));
                    view_model.navigate_to_item(item);
                    next.set(index + 1);
                    announced.set(false);
                    countdown.set(half);
                    return true;
                }

                note(format!("CATALOG DONE ({} page(s))", pages.len()));
                if let Some(exit) = *exit {
                    finish_after(exit);
                }
                false
            },
            Duration::from_millis(250),
            DispatcherPriority::NORMAL,
        );
    }
}
