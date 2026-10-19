//! The search backend of the sample: what the reference takes from NuGet.Protocol
//! (`SourceRepository`, `PackageSearchResource.SearchAsync` and `IPackageSearchMetadata`).
//!
//! Not an upstream file. The reference searches in `AppViewModel.SearchNuGetPackages` and notes
//! that "ideally, we should extract such code into a separate service, say, INuGetSearchService";
//! the port has that service, [`INuGetSearchService`], so that the tests give the view model a
//! catalogue of their own:
//!
//! - [`NuGetSearchService`], with the feature `online` (the default), sends the query to the search
//!   service of NuGet from a worker thread and gives the result through a task;
//! - [`CatalogueSearchService`] searches an in-memory catalogue of about thirty packages. The
//!   sample uses it without the feature `online`, and the tests always do.

use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::CancellationToken;
use std::rc::Rc;

/// The number of packages a search gives (`SearchAsync(term, filter, skip: 0, take: 10, ..)` of the
/// reference).
pub const TAKE: usize = 10;

/// The metadata of a package that the sample shows (`IPackageSearchMetadata`, the members the
/// reference reads).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageSearchMetadata {
    /// The identifier of the package (`Identity.Id`).
    pub id: String,
    /// The title the package declares, if any.
    pub title: Option<String>,
    /// The description of the package (`Description`).
    pub description: Option<String>,
    /// The address of the icon of the package (`IconUrl`).
    pub icon_url: Option<String>,
    /// The address of the project of the package (`ProjectUrl`).
    pub project_url: Option<String>,
    /// The tags of the package (`Tags`).
    pub tags: Vec<String>,
}

impl PackageSearchMetadata {
    /// The title of the package (`Title`): the declared title, or the identifier for a package that
    /// declares none, as the gallery of NuGet shows it.
    pub fn title(&self) -> String {
        match self.title.as_deref().map(str::trim) {
            Some(title) if !title.is_empty() => title.to_owned(),
            _ => self.id.clone(),
        }
    }

    /// Whether every word of `term` occurs in the identifier, the title, the description or the
    /// tags of the package, without regard to case.
    pub fn matches(&self, term: &str) -> bool {
        let text = format!(
            "{} {} {} {}",
            self.id,
            self.title.as_deref().unwrap_or_default(),
            self.description.as_deref().unwrap_or_default(),
            self.tags.join(" ")
        )
        .to_lowercase();
        let mut words = term.split_whitespace().peekable();
        words.peek().is_some() && words.all(|word| text.contains(&word.to_lowercase()))
    }
}

/// Searches packages (`INuGetSearchService`, the service the reference suggests).
pub trait INuGetSearchService {
    /// The first [`TAKE`] packages that match `term` (`SearchAsync(term, filter, 0, 10, logger,
    /// token)`). The task ends on the calling thread. A search that `token` cancelled before it
    /// started gives a canceled task.
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<PackageSearchMetadata>>;
}

/// The search service of the sample when it runs: NuGet's own with the feature `online`, the
/// in-memory catalogue without it. The tests always use the catalogue.
pub fn default_search_service() -> Rc<dyn INuGetSearchService> {
    #[cfg(all(feature = "online", not(target_arch = "wasm32"), not(test)))]
    let service: Rc<dyn INuGetSearchService> = Rc::new(NuGetSearchService::new());
    #[cfg(not(all(feature = "online", not(target_arch = "wasm32"), not(test))))]
    let service: Rc<dyn INuGetSearchService> = Rc::new(CatalogueSearchService::new());
    service
}

// --- the in-memory catalogue ----------------------------------------------------------------

/// One package of the catalogue: identifier, title, description, project address, tags.
type CatalogueEntry = (&'static str, &'static str, &'static str, &'static str, &'static [&'static str]);

/// The packages of the catalogue: the libraries of the ReactiveUI family and well-known packages of
/// the .NET ecosystem.
const CATALOGUE: &[CatalogueEntry] = &[
    ("ReactiveUI", "ReactiveUI", "A composable, cross-platform model-view-viewmodel framework built on reactive extensions.", "https://reactiveui.net/", &["mvvm", "reactive", "rx", "frp"]),
    ("ReactiveUI.WPF", "ReactiveUI.WPF", "The WPF integration of ReactiveUI: reactive windows and user controls, activation and view location.", "https://reactiveui.net/", &["mvvm", "reactive", "wpf"]),
    ("ReactiveUI.WinForms", "ReactiveUI.WinForms", "The Windows Forms integration of ReactiveUI.", "https://reactiveui.net/", &["mvvm", "reactive", "winforms"]),
    ("ReactiveUI.Maui", "ReactiveUI.Maui", "The .NET MAUI integration of ReactiveUI.", "https://reactiveui.net/", &["mvvm", "reactive", "maui"]),
    ("ReactiveUI.Blazor", "ReactiveUI.Blazor", "The Blazor integration of ReactiveUI: reactive components and their view models.", "https://reactiveui.net/", &["mvvm", "reactive", "blazor"]),
    ("ReactiveUI.Testing", "ReactiveUI.Testing", "Helpers for testing view models with test schedulers and builders of test data.", "https://reactiveui.net/", &["mvvm", "reactive", "testing"]),
    ("ReactiveUI.SourceGenerators", "ReactiveUI.SourceGenerators", "Source generators for reactive properties, output properties and commands.", "https://reactiveui.net/", &["mvvm", "reactive", "generators"]),
    ("ReactiveUI.Validation", "ReactiveUI.Validation", "Validation of view model properties with reactive rules.", "https://reactiveui.net/", &["mvvm", "reactive", "validation"]),
    ("ReactiveUI.Fody", "ReactiveUI.Fody", "Weaves the change notifications of reactive properties into view models at build time.", "https://reactiveui.net/", &["mvvm", "reactive", "fody"]),
    ("DynamicData", "Dynamic Data", "Reactive collections based on reactive extensions: observable caches and lists with operators.", "https://dynamic-data.org/", &["reactive", "rx", "collections"]),
    ("Splat", "Splat", "A library to make things cross-platform that should be: service location, logging and images.", "https://github.com/reactiveui/splat", &["di", "logging", "service-location"]),
    ("Akavache", "Akavache", "An asynchronous, persistent key-value store for desktop and mobile applications.", "https://github.com/reactiveui/Akavache", &["cache", "storage", "reactive"]),
    ("Fusillade", "Fusillade", "An opinionated HTTP library for mobile development: request limiting, deduplication and prioritisation.", "https://github.com/reactiveui/Fusillade", &["http", "networking"]),
    ("Punchclock", "Punchclock", "Makes sure that asynchronous operations run in order and with limited concurrency.", "https://github.com/reactiveui/Punchclock", &["async", "queue"]),
    ("Sextant", "Sextant", "A navigation library for ReactiveUI view models.", "https://github.com/reactiveui/Sextant", &["mvvm", "navigation", "reactive"]),
    ("Pharmacist", "Pharmacist", "Builds observables from the events of a library.", "https://github.com/reactiveui/Pharmacist", &["reactive", "events"]),
    ("System.Reactive", "System.Reactive", "Reactive Extensions: composing asynchronous and event-based programs with observable sequences and LINQ-style operators.", "https://github.com/dotnet/reactive", &["rx", "reactive", "linq"]),
    ("Microsoft.Reactive.Testing", "Microsoft.Reactive.Testing", "The test scheduler and the helpers for testing code that uses reactive extensions.", "https://github.com/dotnet/reactive", &["rx", "reactive", "testing"]),
    ("Newtonsoft.Json", "Json.NET", "A popular high-performance JSON framework for .NET.", "https://www.newtonsoft.com/json", &["json", "serialization"]),
    ("Serilog", "Serilog", "Simple .NET logging with fully-structured events.", "https://serilog.net/", &["logging", "structured"]),
    ("NUnit", "NUnit", "A unit-testing framework for all .NET languages.", "https://nunit.org/", &["testing", "unit"]),
    ("xunit", "xUnit.net", "A free, open source, community-focused unit testing tool for .NET.", "https://xunit.net/", &["testing", "unit"]),
    ("Moq", "Moq", "A mocking library for .NET built on LINQ expression trees.", "https://github.com/devlooped/moq", &["testing", "mocking"]),
    ("AutoMapper", "AutoMapper", "A convention-based object-object mapper.", "https://automapper.org/", &["mapping", "objects"]),
    ("Dapper", "Dapper", "A simple object mapper for .NET over ADO.NET connections.", "https://github.com/DapperLib/Dapper", &["orm", "sql", "data"]),
    ("Polly", "Polly", "A resilience and transient-fault-handling library: retry, circuit breaker, timeout and fallback.", "https://github.com/App-vNext/Polly", &["resilience", "retry", "http"]),
    ("FluentValidation", "FluentValidation", "A validation library for .NET that uses a fluent interface to build validation rules.", "https://fluentvalidation.net/", &["validation", "fluent"]),
    ("Humanizer", "Humanizer", "Turns strings, enumerations, dates, times, numbers and quantities into human-friendly text.", "https://github.com/Humanizr/Humanizer", &["strings", "text", "formatting"]),
    ("NodaTime", "Noda Time", "A better date and time API for .NET.", "https://nodatime.org/", &["date", "time"]),
    ("CsvHelper", "CsvHelper", "A library for reading and writing CSV files.", "https://joshclose.github.io/CsvHelper/", &["csv", "data"]),
    ("Bogus", "Bogus", "A simple fake data generator for .NET.", "https://github.com/bchavez/Bogus", &["testing", "data", "fake"]),
    ("NuGet.Protocol", "NuGet.Protocol", "Implements the protocols of NuGet's package sources: service indexes, search and package downloads.", "https://github.com/NuGet/NuGet.Client", &["nuget", "packages", "search"]),
];

/// The packages of the in-memory catalogue that the sample searches without the feature `online`
/// and in its tests (not an upstream type).
#[derive(Debug, Default, Clone, Copy)]
pub struct CatalogueSearchService;

impl CatalogueSearchService {
    /// The service.
    pub fn new() -> Self {
        Self
    }

    /// Every package of the catalogue.
    pub fn packages() -> Vec<PackageSearchMetadata> {
        CATALOGUE
            .iter()
            .map(|(id, title, description, project_url, tags)| PackageSearchMetadata {
                id: (*id).to_owned(),
                title: Some((*title).to_owned()),
                description: Some((*description).to_owned()),
                // The catalogue has no icons: each package shows the default icon of the view
                // model.
                icon_url: None,
                project_url: Some((*project_url).to_owned()),
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            })
            .collect()
    }

    /// The first [`TAKE`] packages of the catalogue that match `term`, in the order of the
    /// catalogue.
    pub fn find(term: &str) -> Vec<PackageSearchMetadata> {
        Self::packages().into_iter().filter(|package| package.matches(term)).take(TAKE).collect()
    }
}

impl INuGetSearchService for CatalogueSearchService {
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<PackageSearchMetadata>> {
        if token.is_cancellation_requested() {
            return Task::from_canceled();
        }
        Task::from_result(Self::find(term))
    }
}

// --- the search service of NuGet -----------------------------------------------------------

#[cfg(all(feature = "online", not(target_arch = "wasm32")))]
pub use online::{download, NuGetSearchService};

#[cfg(all(feature = "online", not(target_arch = "wasm32")))]
mod online {
    use super::{INuGetSearchService, PackageSearchMetadata, TAKE};
    use ferroui_system::threading::tasks::{self, Task};
    use ferroui_system::threading::CancellationToken;
    use serde::Deserialize;
    use std::error::Error;
    use std::sync::OnceLock;

    /// The error of work on a worker thread.
    type WorkError = Box<dyn Error + Send + Sync>;

    /// The address of the search service of nuget.org: the `SearchQueryService` resource of its
    /// service index (`https://api.nuget.org/v3/index.json`).
    // Deviation (DEVIATIONS.md, samples): NuGet.Protocol reads this address from the service
    // index with a request of its own (`SourceRepository.GetResourceAsync<PackageSearchResource>`);
    // the port sends the query to the address the index gives.
    pub const SEARCH_QUERY_SERVICE: &str = "https://azuresearch-usnc.nuget.org/query";

    /// One agent for every request of the process: it keeps the connections alive.
    fn agent() -> &'static ureq::Agent {
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        AGENT.get_or_init(|| ureq::AgentBuilder::new().build())
    }

    /// The response of the search service (the members the sample reads).
    #[derive(Deserialize)]
    struct SearchResponse {
        #[serde(default)]
        data: Vec<SearchResult>,
    }

    /// One package of the response.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SearchResult {
        id: String,
        title: Option<String>,
        description: Option<String>,
        icon_url: Option<String>,
        project_url: Option<String>,
        #[serde(default)]
        tags: Vec<String>,
    }

    /// An empty text as none: the service writes `""` for an address a package does not declare.
    fn non_empty(value: Option<String>) -> Option<String> {
        value.filter(|value| !value.trim().is_empty())
    }

    impl From<SearchResult> for PackageSearchMetadata {
        fn from(result: SearchResult) -> Self {
            Self {
                id: result.id,
                title: non_empty(result.title),
                description: result.description,
                icon_url: non_empty(result.icon_url),
                project_url: non_empty(result.project_url),
                tags: result.tags,
            }
        }
    }

    /// Sends the query of `term` and reads the packages of the response; blocks the thread.
    fn query(term: &str) -> Result<Vec<PackageSearchMetadata>, WorkError> {
        // The query of `PackageSearchResource.SearchAsync(term, new SearchFilter(false), 0, 10,
        // ..)`: no prerelease versions, the first ten packages.
        let response = agent()
            .get(SEARCH_QUERY_SERVICE)
            .query("q", term)
            .query("skip", "0")
            .query("take", &TAKE.to_string())
            .query("prerelease", "false")
            .query("semVerLevel", "2.0.0")
            .call()
            .map_err(|error| error.to_string())?;
        let text = response.into_string().map_err(|error| error.to_string())?;
        let response: SearchResponse = serde_json::from_str(&text).map_err(|error| error.to_string())?;
        Ok(response.data.into_iter().map(PackageSearchMetadata::from).collect())
    }

    /// Downloads the content at `url`; blocks the thread. The icons of the packages are downloaded
    /// with it.
    pub fn download(url: &str) -> Result<Vec<u8>, WorkError> {
        use std::io::Read;

        let response = agent().get(url).call().map_err(|error| error.to_string())?;
        let mut content = Vec::new();
        response.into_reader().read_to_end(&mut content).map_err(|error| error.to_string())?;
        Ok(content)
    }

    /// The search service of nuget.org (`SourceRepository` over
    /// `https://api.nuget.org/v3/index.json` with the providers of the version 3 protocol, and its
    /// `PackageSearchResource`).
    #[derive(Debug, Default, Clone, Copy)]
    pub struct NuGetSearchService;

    impl NuGetSearchService {
        /// The service.
        pub fn new() -> Self {
            Self
        }
    }

    impl INuGetSearchService for NuGetSearchService {
        /// Sends the query from a worker thread (the reference awaits `SearchAsync` with
        /// `ConfigureAwait(false)`); the task ends on the calling thread, through the poster of the
        /// thread (the dispatcher in the application).
        ///
        /// Deviation (DEVIATIONS.md, samples): the token stops a search that has not started; a
        /// request that has started runs to its end, and the reactive pipeline drops its result
        /// when the subscription is gone (NuGet.Protocol hands the token to the HTTP client).
        fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<PackageSearchMetadata>> {
            if token.is_cancellation_requested() {
                return Task::from_canceled();
            }
            let term = term.to_owned();
            tasks::run(move || query(&term))
        }
    }
}

#[cfg(test)]
mod catalogue_tests {
    use super::{CatalogueSearchService, TAKE};

    #[test]
    fn the_catalogue_has_about_thirty_packages() {
        let count = CatalogueSearchService::packages().len();
        assert!((28..=36).contains(&count), "{count}");
    }

    #[test]
    fn every_word_of_the_term_matches_without_regard_to_case() {
        let found: Vec<String> = CatalogueSearchService::find("REACTIVE testing").into_iter().map(|package| package.id).collect();
        assert_eq!(found, ["ReactiveUI.Testing", "Microsoft.Reactive.Testing"]);
    }

    #[test]
    fn a_search_gives_at_most_ten_packages() {
        assert_eq!(CatalogueSearchService::find("reactive").len(), TAKE);
    }

    #[test]
    fn a_blank_term_matches_nothing() {
        assert!(CatalogueSearchService::find("   ").is_empty());
    }
}
