//! The page-cycling smoke run of the desktop entry point (not a port: the
//! role of the `--full-headless` run of the managed original, which selects
//! every page of the catalog).

use crate::models::PageItem;
use crate::view_models::MainWindowViewModel;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The view model of the main window (or of the main view) of the
/// application, once it exists.
fn main_view_model() -> Option<Rc<MainWindowViewModel>> {
    let lifetime = Application::current()?.application_lifetime()?;
    if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
        let main_window = desktop.main_window()?;
        return from_markup_value::<Rc<MainWindowViewModel>>(&main_window.data_context());
    }
    // With one view (the Linux framebuffer platform): the view is the
    // navigation host of the main view (`App::create_main_view_host`).
    let host = lifetime.as_single_view_application_lifetime()?.main_view()?;
    let page = host.cast::<ferroui_controls::PageNavigationHost>()?.page()?;
    from_markup_value::<Rc<MainWindowViewModel>>(&page.data_context())
}

/// The pages the drawer offers: the home page, the pages of the sections
/// and the settings page.
pub fn pages(view_model: &MainWindowViewModel) -> Vec<Rc<PageItem>> {
    let mut pages = vec![view_model.home_item()];
    for section in view_model.home_sections().iter() {
        pages.extend(section.items().unwrap_or_default().iter().cloned());
    }
    pages.push(view_model.settings_item());
    pages
}

/// Navigates to the page of the catalog whose header is `header` (compared without regard to
/// case), as a click on its entry does. Returns whether there is a main view model and a page with
/// that header; the navigation itself completes later. Used by the smoke runs of the pages of the
/// section "Composition", which go from one page to the next (`pages::smoke_show_section`).
pub fn select_page(header: &str) -> bool {
    let Some(view_model) = main_view_model() else { return false };
    match pages(&view_model).into_iter().find(|item| item.header().eq_ignore_ascii_case(header)) {
        Some(item) => {
            println!("Selecting {}", item.header());
            view_model.navigate_to_item(&item);
            true
        }
        None => {
            println!("No page has the header {header:?}");
            false
        }
    }
}

/// Navigates to the pages of the catalog whose headers are given, in that
/// order, each for `interval`. `shown` is called with the index and the
/// header of a page when its interval is over (the page has been laid out
/// and drawn by then), and `done` after the last page. A header no page
/// has is reported and passed over; headers compare without regard to
/// case.
///
/// This is what a run that takes a picture of some pages is built on
/// (`FERROUI_SMOKE_SCREENSHOTS` of the desktop entry point).
pub fn show_pages(headers: Vec<String>, interval: Duration, shown: impl Fn(usize, &str) + 'static, done: impl Fn() + 'static) {
    let next = Cell::new(0usize);
    let current: RefCell<Option<(usize, String)>> = RefCell::new(None);
    let timer: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let stop = timer.clone();
    let subscription = DispatcherTimer::run(
        move || {
            let Some(view_model) = main_view_model() else { return true };
            if let Some((index, header)) = current.borrow_mut().take() {
                shown(index, &header);
            }
            let pages = pages(&view_model);
            while let Some(wanted) = headers.get(next.get()) {
                let index = next.get();
                next.set(index + 1);
                match pages.iter().find(|item| item.header().eq_ignore_ascii_case(wanted)) {
                    Some(item) => {
                        println!("Selecting {}", item.header());
                        view_model.navigate_to_item(item);
                        *current.borrow_mut() = Some((index, item.header()));
                        return true;
                    }
                    None => println!("No page has the header {wanted:?}"),
                }
            }
            done();
            stop.borrow_mut().take();
            false
        },
        interval,
        DispatcherPriority::NORMAL,
    )
    .detach();
    *timer.borrow_mut() = Some(subscription);
}

/// Navigates to the pages of the catalog one after the other, each for
/// `interval`, and prints the header of each.
pub fn show_every_page(interval: Duration) {
    let next = Cell::new(0usize);
    let timer: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let stop = timer.clone();
    let subscription = DispatcherTimer::run(
        move || {
            let Some(view_model) = main_view_model() else { return true };
            let pages = pages(&view_model);
            let index = next.get();
            if let Some(item) = pages.get(index) {
                println!("Selecting {}", item.header());
                view_model.navigate_to_item(item);
                next.set(index + 1);
                return true;
            }
            println!("Selected every page ({})", pages.len());
            stop.borrow_mut().take();
            false
        },
        interval,
        DispatcherPriority::NORMAL,
    )
    .detach();
    *timer.borrow_mut() = Some(subscription);
}
