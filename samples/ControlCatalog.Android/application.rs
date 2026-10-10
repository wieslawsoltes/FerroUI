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
//! The native control demo of Android (`EmbedSampleAndroid`) is in
//! [`embed_sample_android`].
//!
//! Not ported yet: the activity for the `OpenUri` activation
//! (`DataSchemeActivity`: a second activity class and its manifest entry),
//! and the splash screen resources.
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
mod embed_sample_android;

#[cfg(target_os = "android")]
mod application {
    use crate::embed_sample_android::EmbedSampleAndroid;
    use control_catalog::models::PageItem;
    use control_catalog::pages::EmbedSample;
    use control_catalog::view_models::MainWindowViewModel;
    use control_catalog::{App, MainView};
    use ferroui_android::interop::java::{call_int, call_object, string_of, JavaValue};
    use ferroui_base::Thickness;
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

        EmbedSample::set_implementation(Some(Rc::new(EmbedSampleAndroid)));

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

    /// The children of the Java view of the main activity, in the order they
    /// are drawn in (a later one is above an earlier one; the surface the
    /// framework renders to is behind the window of the activity, so every
    /// other child is above it): the class, the rectangle in pixels and
    /// whether the child is visible.
    fn native_views() -> String {
        let Some(view) = FerroActivity::current_main_activity().and_then(|activity| activity.view()) else {
            return "no view".to_string();
        };
        let view = view.java_object();
        let count = call_int(view, "getChildCount", "()I", &[]);
        let children: Vec<String> = (0..count)
            .filter_map(|index| call_object(view, "getChildAt", "(I)Landroid/view/View;", &[JavaValue::Int(index)]))
            .map(|child| {
                let class = call_object(&child, "getClass", "()Ljava/lang/Class;", &[])
                    .and_then(|class| call_object(&class, "getSimpleName", "()Ljava/lang/String;", &[]))
                    .map(|name| string_of(&name))
                    .unwrap_or_default();
                let int = |name: &str| call_int(&child, name, "()I", &[]);
                format!(
                    "{class} at ({}, {}) {}x{} {}",
                    int("getLeft"),
                    int("getTop"),
                    int("getWidth"),
                    int("getHeight"),
                    if int("getVisibility") == 0 { "visible" } else { "not visible" }
                )
            })
            .collect();
        format!("{} child(ren) of a view {} px wide: {}", count, call_int(view, "getWidth", "()I", &[]), children.join("; "))
    }

    /// Changes the bounds of the main view by a point and back at the next
    /// change, so that what depends on the bounds of its ancestors is placed
    /// again.
    fn nudge_layout() {
        let main_view = FerroActivity::current_main_activity()
            .and_then(|activity| activity.view())
            .and_then(|view| view.top_level())
            .and_then(|top_level| top_level.get_visual_descendants().find_map(|visual| visual.cast::<MainView>()));
        if let Some(main_view) = main_view {
            let nudged = main_view.margin().bottom != 0.0;
            main_view.set_margin(Thickness::new(0.0, 0.0, 0.0, if nudged { 0.0 } else { 1.0 }));
        }
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
        // Whether the main view was laid out once more for the current page.
        let nudged = Cell::new(false);

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
                if !announced.get() {
                    let index = next.get() - 1;
                    // A native control host places its control when its bounds or those of
                    // an ancestor change, not when a render transform does: a page that
                    // slides in keeps its native controls where the page was when it was
                    // laid out, until the next such change (as in the reference). So that
                    // the pictures of the smoke run show the page as a person sees it after
                    // any change of layout, the main view is laid out once more when the
                    // transition is over, and the line of the page follows two ticks later.
                    if !nudged.replace(true) {
                        note(format!("Native views of {} before a layout: {}", pages[index].header(), native_views()));
                        nudge_layout();
                        countdown.set(1);
                        return true;
                    }
                    nudged.set(false);
                    announced.set(true);
                    note(format!("Native views of {} after a layout: {}", pages[index].header(), native_views()));
                    note(format!("PAGE-SHOWN {index} {}", pages[index].header()));
                    countdown.set(half.saturating_sub(2));
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
