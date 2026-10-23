//! The user interface of the crate browser: components that run again, with their state in
//! hooks.
//!
//! - [`App`]: the router of the two pages, `/` (the search) and `/crate/:name` (a crate).
//! - `SearchPage`: the search box and the results. The search is a future of the trimmed text
//!   ([`use_future`]) that waits [`THROTTLE`] before it asks the service; [`suspense`] shows it.
//! - `DetailsPage`: the page of a crate, inside an [`error_boundary`].
//!
//! What outlives a page (the service, the search text, the crates the results showed) is a
//! [`Browser`] that [`App`] keeps and gives the pages.

use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::utilities::Uri;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_system::threading::CancellationToken;
use ferroui_view::prelude::*;
use reactive_demo::crate_search::{default_search_service, CrateSearchMetadata, ICrateSearchService};
use reactive_demo::format_thousands;
use std::rc::Rc;
use std::time::Duration;

/// How long the search waits after the last change of the text before it runs: the throttle of
/// the pipeline of the reference (`Throttle(TimeSpan.FromMilliseconds(800))`).
pub const THROTTLE: Duration = Duration::from_millis(800);

/// The state of the browser that outlives its pages: the search service, the text of the search
/// box (kept while the page of a crate is shown), and the crates the results showed, which the
/// page of a crate reads.
#[derive(Clone)]
pub struct Browser {
    /// The search of crates: crates.io, or the in-memory catalogue.
    pub service: Rc<dyn ICrateSearchService>,
    /// The text of the search box.
    pub search_text: Signal<String>,
    /// The crates whose results were opened, by name.
    pub seen: Signal<Vec<CrateSearchMetadata>>,
}

impl Browser {
    /// A browser that searches with `service`, with an empty search text. Its signals belong to
    /// the current owner.
    pub fn new(service: Rc<dyn ICrateSearchService>) -> Self {
        Self { service, search_text: signal(String::new()), seen: signal(Vec::new()) }
    }

    /// Keeps `krate` for the page of a crate.
    pub fn remember(&self, krate: CrateSearchMetadata) {
        self.seen.update(|seen| {
            seen.retain(|known| known.name != krate.name);
            seen.push(krate);
        });
    }

    /// The crate named `name`, if a result showed it. The read is not tracked: the page of a
    /// crate reads it again when it is tried again.
    pub fn find(&self, name: &str) -> Option<CrateSearchMetadata> {
        self.seen.with_untracked(|seen| seen.iter().find(|krate| krate.name == name).cloned())
    }
}

impl PartialEq for Browser {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.service, &other.service) && self.search_text == other.search_text && self.seen == other.seen
    }
}

/// The route of the page of the crate `name`.
pub fn details_path(name: &str) -> String {
    format!("/crate/{name}")
}

/// Searches `term` (trimmed already): no crates for an empty term, at once; otherwise the crates
/// of the service, after [`THROTTLE`]. The future of a term the user typed past is dropped while
/// it waits, so that term is never searched.
async fn search_crates(service: Rc<dyn ICrateSearchService>, term: String) -> Result<Vec<CrateSearchMetadata>, String> {
    if term.is_empty() {
        return Ok(Vec::new());
    }
    sleep(THROTTLE).await;
    // The task of the service is a future; it ends on this thread.
    service.search(&term, CancellationToken::none()).await.map_err(|error| error.to_string())
}

/// The application: a router whose routes are the search (`/`) and the page of a crate
/// (`/crate/:name`). `browser` is the state it shows (a new one, with the search service of
/// `reactive-demo`, when it is not given), `path` the route it starts on.
#[component(rerender)]
pub fn App(browser: Option<Browser>, #[prop(default = String::from("/"))] path: String) -> Element {
    let browser = use_hook(move || browser.unwrap_or_else(|| Browser::new(default_search_service())));
    let (search, details) = (browser.clone(), browser);
    let routes = vec![
        route("/", move |_| rsx! { SearchPage { browser: search.clone() } }),
        route("/crate/:name", move |found| {
            let name = found.param("name").unwrap_or_default().to_string();
            rsx! { DetailsPage { browser: details.clone(), name } }
        }),
    ];
    rsx! {
        Grid { margin: 12.0,
            {router(&path, routes)}
        }
    }
}

/// The search: the search box and, below it, what the search gives. The component runs again
/// as the text changes; the future of the search follows the trimmed text.
#[component(rerender)]
fn SearchPage(browser: Browser) -> Element {
    let router = use_router();
    let text = browser.search_text;
    let term = text.with(|text| text.trim().to_string());
    let service = browser.service.clone();
    let results = use_future(term.clone(), move |term: &String| search_crates(service.clone(), term.clone()));

    rsx! {
        Grid { row_definitions: "Auto,*", column_definitions: "Auto,*",
            TextBlock { text: "Search for: ", font_size: 16.0, font_weight: FontWeight::SemiBold,
                vertical_alignment: VerticalAlignment::Center }
            TextBox { name: "Search", Grid.column: 1, margin: (6.0, 0.0, 0.0, 0.0),
                placeholder_text: "The name of a crate, or words of its description", text: text.two_way() }
            ScrollViewer { Grid.row: 1, Grid.column_span: 2, margin: (0.0, 6.0, 0.0, 0.0),
                horizontal_scroll_bar_visibility: ScrollBarVisibility::Disabled,
                StackPanel {
                    {suspense(
                        results,
                        searching,
                        move |crates| results_view(crates, &term, &browser, router),
                        |error| rsx! {
                            TextBlock { name: "Error", text: format!("The search failed: {error}"),
                                text_wrapping: TextWrapping::Wrap }
                        },
                    )}
                }
            }
        }
    }
}

/// What a search shows while it waits and runs.
fn searching() -> Element {
    rsx! {
        StackPanel { name: "Searching", spacing: 8.0, margin: (0.0, 12.0, 0.0, 0.0),
            ProgressBar { is_indeterminate: true, width: 240.0, horizontal_alignment: HorizontalAlignment::Left }
            TextBlock { name: "Status", text: "Searching...", opacity: 0.7 }
        }
    }
}

/// The results of the search of `term`: a hint when there are none.
fn results_view(crates: Vec<CrateSearchMetadata>, term: &str, browser: &Browser, router: Router) -> Element {
    if crates.is_empty() {
        let hint = if term.is_empty() {
            "Type the name of a crate, or words of its description."
        } else {
            "No crate matches the search."
        };
        return rsx! { TextBlock { name: "Hint", text: hint, opacity: 0.7, margin: (0.0, 6.0, 0.0, 0.0) } };
    }
    rsx! {
        StackPanel { name: "Results", spacing: 2.0,
            for krate in crates {
                {result_item(krate, browser.clone(), router)}
            }
        }
    }
}

/// A result, as the crate view of the reference shows it: the badge, the name and version, the
/// description and the downloads. A click opens the page of the crate.
fn result_item(krate: CrateSearchMetadata, browser: Browser, router: Router) -> Element {
    let name = krate.name.clone();
    let title = format!("{} {}", krate.name, krate.version);
    let description = krate.description.clone().unwrap_or_default();
    let downloads = format!("{} downloads", format_thousands(krate.downloads));
    rsx! {
        Button { key: name.clone(), name: format!("Result-{name}"),
            horizontal_alignment: HorizontalAlignment::Stretch, horizontal_content_alignment: HorizontalAlignment::Stretch,
            background: "Transparent", padding: 0.0,
            on_click: move |_| {
                browser.remember(krate.clone());
                router.navigate(&details_path(&krate.name));
            },
            Grid { column_definitions: "Auto,*",
                {badge(&name, 48.0)}
                StackPanel { Grid.column: 1, margin: 6.0, vertical_alignment: VerticalAlignment::Center,
                    TextBlock { text: title, font_size: 14.0, font_weight: FontWeight::SemiBold }
                    TextBlock { text: description, font_size: 12.0, text_wrapping: TextWrapping::Wrap }
                    TextBlock { text: downloads, font_size: 12.0, opacity: 0.7, margin: (0.0, 2.0, 0.0, 0.0) }
                }
            }
        }
    }
}

/// The badge of a crate: crates.io has no icons, so it is the first letter of the name in a
/// rounded square (the icon of the package in the reference).
fn badge(name: &str, size: f64) -> Element {
    let letter: String = name.chars().next().map(|first| first.to_uppercase().collect()).unwrap_or_default();
    rsx! {
        Border { margin: 6.0, width: size, height: size, corner_radius: 8.0, background: "#B7410E",
            horizontal_alignment: HorizontalAlignment::Center, vertical_alignment: VerticalAlignment::Center,
            TextBlock { text: letter, font_size: size * 0.45, font_weight: FontWeight::SemiBold, foreground: "White",
                horizontal_alignment: HorizontalAlignment::Center, vertical_alignment: VerticalAlignment::Center }
        }
    }
}

/// The page of the crate `name`, inside an error boundary: a crate that no result showed (a
/// route typed by hand, a state that was lost) is an error of the page, which the boundary
/// replaces with the error and a way to try again.
#[component(rerender)]
fn DetailsPage(browser: Browser, name: String) -> Element {
    let router = use_router();
    let back = router;
    rsx! {
        Grid {
            {error_boundary(
                move || {
                    let krate = browser.find(&name).unwrap_or_else(|| {
                        panic!("The crate `{name}` is not among the results of a search: search for it first.")
                    });
                    crate_details(krate, back)
                },
                move |error| failure(error, router),
            )}
        }
    }
}

/// The page of a crate: the details of the crate view of the reference, larger, with its links
/// and a button back to the search.
fn crate_details(krate: CrateSearchMetadata, router: Router) -> Element {
    let page = krate.page_url();
    let description = krate.description.clone().unwrap_or_else(|| String::from("No description."));
    let links: Vec<Element> = [
        ("Repository", &krate.repository),
        ("Home page", &krate.homepage),
        ("Documentation", &krate.documentation),
    ]
    .into_iter()
    .filter_map(|(label, address)| address.as_deref().map(|address| link(label, address)))
    .collect();

    rsx! {
        StackPanel { name: "Details", spacing: 8.0,
            Button { name: "Back", horizontal_alignment: HorizontalAlignment::Left,
                on_click: move |_| {
                    // Back to the page before, or to the search when the browser started here.
                    if !router.navigate_back() {
                        router.navigate_and_reset("/");
                    }
                },
                "Back" }
            Grid { column_definitions: "Auto,*",
                {badge(&krate.name, 64.0)}
                StackPanel { Grid.column: 1, margin: 6.0, vertical_alignment: VerticalAlignment::Center,
                    TextBlock { name: "CrateName", text: krate.name.clone(), font_size: 24.0, font_weight: FontWeight::SemiBold }
                    TextBlock { name: "Version", text: format!("Version {}", krate.version), font_size: 14.0 }
                }
            }
            TextBlock { name: "Description", text: description, text_wrapping: TextWrapping::Wrap }
            TextBlock { name: "Downloads", text: format!("{} downloads", format_thousands(krate.downloads)), opacity: 0.7 }
            HyperlinkButton { name: "Page", navigate_uri: Uri::absolute(&page).ok(), padding: 0.0,
                {format!("Open on crates.io: {page}")} }
            {links}
        }
    }
}

/// A link of the page of a crate.
fn link(label: &str, address: &str) -> Element {
    rsx! {
        HyperlinkButton { key: label, name: label.replace(' ', ""), navigate_uri: Uri::absolute(address).ok(), padding: 0.0,
            {format!("{label}: {address}")} }
    }
}

/// What the error boundary of the page of a crate shows: the error, a button that tries the page
/// again, and one back to the search.
fn failure(error: ErrorInfo, router: Router) -> Element {
    let message = error.message().to_string();
    rsx! {
        StackPanel { name: "Failure", spacing: 8.0,
            TextBlock { text: "The page of the crate could not be shown.", font_size: 16.0, font_weight: FontWeight::SemiBold }
            TextBlock { name: "ErrorMessage", text: message, text_wrapping: TextWrapping::Wrap }
            StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
                Button { name: "TryAgain", on_click: move |_| error.reset(), "Try again" }
                Button { name: "BackToSearch", on_click: move |_| router.navigate_and_reset("/"), "Back to the search" }
            }
        }
    }
}
