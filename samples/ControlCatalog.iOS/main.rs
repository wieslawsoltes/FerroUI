//! Port of `Main.cs` and `AppDelegate.cs` of `ControlCatalog.iOS`: the iOS
//! entry point of the ControlCatalog sample.
//!
//! ```text
//! scripts/ios/sim-catalog.sh "iPhone 17 Pro"    # runs it in the simulator and takes screenshots of pages
//! ```
//!
//! The executable is put into an application bundle by
//! `scripts/ios/bundle.sh --package control-catalog-ios --bin control-catalog-ios`,
//! which writes the property list the reference keeps as `Info.plist`.
//!
//! As the desktop entry point, the host has a smoke run for looking at
//! the catalog without a person: with `FERROUI_SMOKE_PAGES=<n>` pages are
//! shown one after the other, `n` milliseconds each, and their headers
//! printed (`Selecting <header>`); `FERROUI_SMOKE_PAGE_NAMES=<a,b,c>`
//! names the pages to show, by header, in that order (all of them
//! without it); with `FERROUI_SMOKE_EXIT_MS=<n>` the process exits after
//! `n` milliseconds, and without it after the last page.
//!
//! Not ported yet: the native control samples (`EmbedSample.iOS.cs`: a
//! web view and a button as native controls), which wait for the native
//! control host of the platform (stage 2 of
//! `docs/porting/ios-platform.md`), and the launch screen
//! (`Resources/LaunchScreen.xib`: the bundle states an empty launch
//! screen instead).

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("control-catalog-ios: this is the iOS entry point of the catalog; it needs UIKit");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(host::AppDelegate))
}

#[cfg(target_os = "ios")]
mod host {
    use control_catalog::models::PageItem;
    use control_catalog::view_models::MainWindowViewModel;
    use control_catalog::App;
    use ferroui_base::logging::LogEventLevel;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::{AppBuilder, Application, PageNavigationHost};
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    /// The application delegate of the catalog: what the class of the
    /// reference overrides of the delegate of the platform.
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

    /// The view model of the main view of the application, once it exists.
    fn main_view_model() -> Option<Rc<MainWindowViewModel>> {
        let lifetime = Application::current()?.application_lifetime()?;
        let main_view = lifetime.as_single_view_application_lifetime()?.main_view()?;
        let page = main_view.cast::<PageNavigationHost>()?.page()?;
        from_markup_value::<Rc<MainWindowViewModel>>(&page.data_context())
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

    /// A header as it is compared: without spaces, in lower case.
    fn key(header: &str) -> String {
        header.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
    }

    /// The pages to show: the named ones in the order of the names, or
    /// all of them.
    fn selected_pages(all: Vec<Rc<PageItem>>) -> Vec<Rc<PageItem>> {
        let Ok(names) = std::env::var("FERROUI_SMOKE_PAGE_NAMES") else {
            return all;
        };
        let mut selected = Vec::new();
        for name in names.split(',').map(str::trim).filter(|name| !name.is_empty()) {
            match all.iter().find(|item| key(&item.header()) == key(name)) {
                Some(item) => selected.push(item.clone()),
                None => println!("No page with the header {name}"),
            }
        }
        selected
    }

    /// The smoke run asked for with `FERROUI_SMOKE_PAGES`,
    /// `FERROUI_SMOKE_PAGE_NAMES` and `FERROUI_SMOKE_EXIT_MS`.
    fn smoke_run() {
        let exit_after = environment_milliseconds("FERROUI_SMOKE_EXIT_MS");

        if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_PAGES") {
            let next = Cell::new(0usize);
            let listed = Cell::new(false);
            let _timer = DispatcherTimer::run(
                move || {
                    let Some(view_model) = main_view_model() else {
                        println!("Waiting for the main view");
                        return true;
                    };
                    let all = pages(&view_model);
                    if !listed.replace(true) {
                        let headers: Vec<String> = all.iter().map(|item| item.header()).collect();
                        println!("Pages ({}): {}", headers.len(), headers.join(", "));
                    }
                    let pages = selected_pages(all);
                    let index = next.get();
                    if let Some(item) = pages.get(index) {
                        println!("Selecting {}", item.header());
                        view_model.navigate_to_item(item);
                        next.set(index + 1);
                        return true;
                    }
                    println!("Selected every page ({})", pages.len());
                    if exit_after.is_none() {
                        std::process::exit(0);
                    }
                    false
                },
                Duration::from_millis(ms),
                DispatcherPriority::NORMAL,
            );
        }

        if let Some(ms) = exit_after {
            println!("Will exit after {ms} ms");
            let _timer = DispatcherTimer::run_once(
                || {
                    println!("Timer fired: exiting");
                    std::process::exit(0);
                },
                Duration::from_millis(ms),
                DispatcherPriority::NORMAL,
            );
        }
    }
}
