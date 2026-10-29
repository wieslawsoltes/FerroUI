//! Not from the reference: the page-cycling smoke run of the desktop entry point.
//!
//! With `FERROUI_SMOKE_PAGES` the gallery selects its pages one after the other, each for
//! `FERROUI_SMOKE_PAGE_MS` milliseconds (150 without it), prints each page with its state, then
//! closes the main window, which ends the main loop. The value is `all` (every page), `ported`
//! (the ported pages) or a list of headers or view names separated by commas. A page whose
//! creation panics is reported, shown as an error, and makes the run fail ([`failed`]).

use crate::pages::{PageInfo, PAGES};
use crate::views::{MainView, MainWindow};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_thread_local, Ref};
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

ferro_thread_local! {
    /// The pages whose creation failed, with the message of the failure.
    static FAILURES: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
}

/// Records that the creation of `page` failed with `message`.
pub fn record_failure(page: &'static PageInfo, message: String) {
    eprintln!("Page {} failed: {message}", page.header);
    FAILURES.with(|failures| failures.borrow_mut().push((page.header, message)));
}

/// Whether the creation of a page failed during the run.
pub fn failed() -> bool {
    FAILURES.with(|failures| !failures.borrow().is_empty())
}

/// The main view of the main window, once the window exists.
fn main_view() -> Option<Ref<MainView>> {
    let lifetime = Application::current()?.application_lifetime()?;
    let window = lifetime
        .as_classic_desktop_style_application_lifetime()?
        .main_window()?;
    window.cast::<MainWindow>()?.main_view()
}

/// Closes the main window of the application.
fn close_main_window() {
    let lifetime =
        Application::current().and_then(|application| application.application_lifetime());
    let main_window = lifetime
        .as_ref()
        .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        .and_then(|l| l.main_window());
    if let Some(main_window) = main_window {
        main_window.close();
    }
}

/// The pages `FERROUI_SMOKE_PAGES` names: `all`, `ported`, or headers and view names separated by
/// commas (a name no page has is reported and passed over).
fn pages_of(spec: &str) -> Vec<&'static PageInfo> {
    match spec.trim() {
        "" | "all" => PAGES.iter().collect(),
        "ported" => PAGES.iter().filter(|page| page.is_ported()).collect(),
        names => names
            .split(',')
            .filter_map(|name| {
                let page = PageInfo::find(name);
                if page.is_none() {
                    println!("No page has the name {:?}", name.trim());
                }
                page
            })
            .collect(),
    }
}

/// Starts the smoke run when `FERROUI_SMOKE_PAGES` is set (called once the application is set
/// up). Returns whether it started.
pub fn start() -> bool {
    let Ok(spec) = std::env::var("FERROUI_SMOKE_PAGES") else {
        return false;
    };
    let pages = pages_of(&spec);
    let interval = std::env::var("FERROUI_SMOKE_PAGE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(150);
    println!("Will show {} pages, each for {interval} ms", pages.len());

    let next = Cell::new(0usize);
    let timer: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let stop = timer.clone();
    let subscription = DispatcherTimer::run(
        move || {
            let Some(main_view) = main_view() else {
                return true;
            };
            let index = next.get();
            if let Some(page) = pages.get(index) {
                let state = if page.is_ported() {
                    "ported"
                } else {
                    "placeholder"
                };
                println!(
                    "Page {}/{}: {} ({}, {state})",
                    index + 1,
                    pages.len(),
                    page.header,
                    page.view_name()
                );
                if !main_view.select_page(page) {
                    println!("The page {} has no tab", page.header);
                }
                next.set(index + 1);
                return true;
            }

            let failures = FAILURES.with(|failures| failures.borrow().len());
            println!(
                "Showed {} pages ({} ported), {failures} failed",
                pages.len(),
                pages.iter().filter(|page| page.is_ported()).count()
            );
            close_main_window();
            stop.borrow_mut().take();
            false
        },
        Duration::from_millis(interval),
        DispatcherPriority::NORMAL,
    );
    *timer.borrow_mut() = Some(subscription);
    true
}
