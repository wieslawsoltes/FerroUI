//! The search backend of the sample: what the reference takes from NuGet.Protocol
//! (`SourceRepository`, `PackageSearchResource.SearchAsync` and `IPackageSearchMetadata`), for the
//! crates of crates.io.
//!
//! Not an upstream file. The reference searches in `AppViewModel.SearchNuGetPackages` and notes
//! that "ideally, we should extract such code into a separate service"; the port has that
//! service, [`ICrateSearchService`], so that the tests give the view model a catalogue of their
//! own:
//!
//! - [`CratesIoSearchService`], with the feature `online` (the default), sends the query to the
//!   API of crates.io from a worker thread and gives the result through a task;
//! - [`CatalogueSearchService`] searches an in-memory catalogue of about thirty crates. The sample
//!   uses it without the feature `online`, and the tests always do.
//!
//! Deviation (DEVIATIONS.md, samples): the reference searches the packages of NuGet; the port
//! searches the crates of crates.io, because FerroUI is a Rust framework.

use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::CancellationToken;
use std::rc::Rc;

/// The number of crates a search gives (`SearchAsync(term, filter, skip: 0, take: 10, ..)` of the
/// reference, `per_page=10` of crates.io).
pub const TAKE: usize = 10;

/// The address of the search of crates.io (`GET /api/v1/crates`).
pub const CRATES_API: &str = "https://crates.io/api/v1/crates";

/// The address of the page of a crate on crates.io, without the name of the crate.
pub const CRATE_PAGE: &str = "https://crates.io/crates/";

/// The address of the search of `term`: the first [`TAKE`] crates whose name or description
/// matches it, with the term percent-encoded.
pub fn search_url(term: &str) -> String {
    format!("{CRATES_API}?q={}&per_page={TAKE}", percent_encode(term))
}

/// `text` percent-encoded for a component of an address: every byte of its UTF-8 form other than
/// the unreserved characters (letters, digits, `-`, `.`, `_` and `~`) is written as `%XX`.
pub fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => encoded.push(char::from(byte)),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// The metadata of a crate that the sample shows (`IPackageSearchMetadata` of the reference, as
/// crates.io describes a crate).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrateSearchMetadata {
    /// The name of the crate (`Identity.Id` of a package).
    pub name: String,
    /// The latest version of the crate (`max_version`, or `newest_version`).
    pub version: String,
    /// The description of the crate (`Description`).
    pub description: Option<String>,
    /// The number of downloads of every version of the crate.
    pub downloads: u64,
    /// The address of the repository of the crate (`ProjectUrl` of a package).
    pub repository: Option<String>,
    /// The address of the home page of the crate.
    pub homepage: Option<String>,
    /// The address of the documentation of the crate.
    pub documentation: Option<String>,
}

impl CrateSearchMetadata {
    /// The page of the crate on crates.io.
    pub fn page_url(&self) -> String {
        format!("{CRATE_PAGE}{}", self.name)
    }

    /// Whether every word of `term` occurs in the name or the description of the crate, without
    /// regard to case.
    pub fn matches(&self, term: &str) -> bool {
        let text = format!("{} {}", self.name, self.description.as_deref().unwrap_or_default()).to_lowercase();
        let mut words = term.split_whitespace().peekable();
        words.peek().is_some() && words.all(|word| text.contains(&word.to_lowercase()))
    }
}

/// Searches crates (`INuGetSearchService`, the service the reference suggests, for crates).
pub trait ICrateSearchService {
    /// The first [`TAKE`] crates that match `term` (`SearchAsync(term, filter, 0, 10, logger,
    /// token)` of the reference). The task ends on the calling thread. A search that `token`
    /// cancelled before it started gives a canceled task.
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<CrateSearchMetadata>>;
}

/// The search service of the sample when it runs: that of crates.io with the feature `online`,
/// the in-memory catalogue without it. The tests always use the catalogue.
pub fn default_search_service() -> Rc<dyn ICrateSearchService> {
    #[cfg(all(feature = "online", not(target_arch = "wasm32"), not(test)))]
    let service: Rc<dyn ICrateSearchService> = Rc::new(CratesIoSearchService::new());
    #[cfg(not(all(feature = "online", not(target_arch = "wasm32"), not(test))))]
    let service: Rc<dyn ICrateSearchService> = Rc::new(CatalogueSearchService::new());
    service
}

// --- the in-memory catalogue ----------------------------------------------------------------

/// One crate of the catalogue.
struct CatalogueEntry {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    downloads: u64,
    repository: &'static str,
    homepage: Option<&'static str>,
}

/// The version of the crates of FerroUI in the catalogue: that of the workspace.
const FERROUI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The repository of the crates of FerroUI.
const FERROUI_REPOSITORY: &str = "https://github.com/wieslawsoltes/FerroUI";

/// The crates of the catalogue: well-known crates of the Rust ecosystem and the crates of FerroUI
/// that the sample is built on. The versions and the numbers of downloads are a snapshot for the
/// sample, not live data.
const CATALOGUE: &[CatalogueEntry] = &[
    CatalogueEntry { name: "serde", version: "1.0.228", description: "A generic framework for serializing and deserializing Rust data structures efficiently.", downloads: 612_480_315, repository: "https://github.com/serde-rs/serde", homepage: Some("https://serde.rs") },
    CatalogueEntry { name: "tokio", version: "1.47.1", description: "An event-driven, non-blocking I/O platform for writing asynchronous applications.", downloads: 398_207_644, repository: "https://github.com/tokio-rs/tokio", homepage: Some("https://tokio.rs") },
    CatalogueEntry { name: "rand", version: "0.9.2", description: "Random number generators and other randomness functionality.", downloads: 571_036_982, repository: "https://github.com/rust-random/rand", homepage: Some("https://rust-random.github.io/book") },
    CatalogueEntry { name: "regex", version: "1.11.2", description: "Regular expressions with guaranteed linear-time matching on all inputs.", downloads: 487_915_201, repository: "https://github.com/rust-lang/regex", homepage: None },
    CatalogueEntry { name: "clap", version: "4.5.48", description: "A simple to use, efficient and full-featured command-line argument parser.", downloads: 434_662_078, repository: "https://github.com/clap-rs/clap", homepage: None },
    CatalogueEntry { name: "anyhow", version: "1.0.100", description: "A flexible concrete error type for easy, idiomatic error handling in applications.", downloads: 431_855_390, repository: "https://github.com/dtolnay/anyhow", homepage: None },
    CatalogueEntry { name: "thiserror", version: "2.0.17", description: "A derive macro that implements the standard error trait for the error types of libraries.", downloads: 569_120_447, repository: "https://github.com/dtolnay/thiserror", homepage: None },
    CatalogueEntry { name: "rayon", version: "1.11.0", description: "Data parallelism: turns sequential iterators into parallel ones with work stealing.", downloads: 270_544_913, repository: "https://github.com/rayon-rs/rayon", homepage: None },
    CatalogueEntry { name: "reqwest", version: "0.12.23", description: "An ergonomic HTTP client with asynchronous and blocking interfaces.", downloads: 282_330_176, repository: "https://github.com/seanmonstar/reqwest", homepage: None },
    CatalogueEntry { name: "hyper", version: "1.7.0", description: "A protective and efficient HTTP library for clients and servers.", downloads: 381_402_559, repository: "https://github.com/hyperium/hyper", homepage: Some("https://hyper.rs") },
    CatalogueEntry { name: "axum", version: "0.8.6", description: "A web application framework that focuses on ergonomics and modularity.", downloads: 167_093_828, repository: "https://github.com/tokio-rs/axum", homepage: None },
    CatalogueEntry { name: "syn", version: "2.0.106", description: "A parser for streams of Rust tokens into a syntax tree of Rust source code.", downloads: 1_163_847_502, repository: "https://github.com/dtolnay/syn", homepage: None },
    CatalogueEntry { name: "quote", version: "1.0.41", description: "The quasi-quoting macro quote! that turns Rust syntax tree data structures into tokens.", downloads: 802_455_617, repository: "https://github.com/dtolnay/quote", homepage: None },
    CatalogueEntry { name: "proc-macro2", version: "1.0.101", description: "A substitute for the procedural macro API of the compiler, usable outside procedural macros.", downloads: 829_316_042, repository: "https://github.com/dtolnay/proc-macro2", homepage: None },
    CatalogueEntry { name: "itertools", version: "0.14.0", description: "Extra iterator adaptors, iterator methods, free functions and macros.", downloads: 527_684_339, repository: "https://github.com/rust-itertools/itertools", homepage: None },
    CatalogueEntry { name: "chrono", version: "0.4.42", description: "Date and time library for Rust, with time zones.", downloads: 391_770_265, repository: "https://github.com/chronotope/chrono", homepage: None },
    CatalogueEntry { name: "log", version: "0.4.28", description: "A lightweight logging facade for Rust.", downloads: 525_248_906, repository: "https://github.com/rust-lang/log", homepage: None },
    CatalogueEntry { name: "tracing", version: "0.1.41", description: "Application-level tracing for Rust: structured, event-based diagnostics with spans.", downloads: 384_016_730, repository: "https://github.com/tokio-rs/tracing", homepage: Some("https://tokio.rs") },
    CatalogueEntry { name: "futures", version: "0.3.31", description: "Futures and streams with zero-cost abstractions for asynchronous programming.", downloads: 362_551_884, repository: "https://github.com/rust-lang/futures-rs", homepage: Some("https://rust-lang.github.io/futures-rs") },
    CatalogueEntry { name: "bytes", version: "1.10.1", description: "Types and traits for working with buffers of bytes.", downloads: 470_389_157, repository: "https://github.com/tokio-rs/bytes", homepage: None },
    CatalogueEntry { name: "parking_lot", version: "0.12.5", description: "Compact and efficient implementations of the standard synchronization primitives.", downloads: 512_903_448, repository: "https://github.com/Amanieu/parking_lot", homepage: None },
    CatalogueEntry { name: "crossbeam", version: "0.8.4", description: "Tools for concurrent programming: channels, scoped threads and lock-free data structures.", downloads: 121_708_316, repository: "https://github.com/crossbeam-rs/crossbeam", homepage: None },
    CatalogueEntry { name: "once_cell", version: "1.21.3", description: "Single assignment cells and lazy values.", downloads: 693_174_021, repository: "https://github.com/matklad/once_cell", homepage: None },
    CatalogueEntry { name: "libc", version: "0.2.176", description: "Raw bindings to the platform's system libraries, such as libc.", downloads: 794_612_587, repository: "https://github.com/rust-lang/libc", homepage: None },
    CatalogueEntry { name: "bitflags", version: "2.9.4", description: "A macro that generates structures which behave like sets of bit flags.", downloads: 853_027_419, repository: "https://github.com/bitflags/bitflags", homepage: None },
    CatalogueEntry { name: "smallvec", version: "1.15.1", description: "Vectors that store a small number of items inline, without a heap allocation.", downloads: 556_394_870, repository: "https://github.com/servo/rust-smallvec", homepage: None },
    CatalogueEntry { name: "hashbrown", version: "0.16.0", description: "A port of the SwissTable hash table of Google to Rust.", downloads: 778_235_096, repository: "https://github.com/rust-lang/hashbrown", homepage: None },
    CatalogueEntry { name: "indexmap", version: "2.11.4", description: "A hash table that keeps the insertion order of its entries.", downloads: 658_740_133, repository: "https://github.com/indexmap-rs/indexmap", homepage: None },
    CatalogueEntry { name: "uuid", version: "1.18.1", description: "Generates and parses universally unique identifiers (UUIDs).", downloads: 384_591_602, repository: "https://github.com/uuid-rs/uuid", homepage: None },
    CatalogueEntry { name: "url", version: "2.5.7", description: "Parsing and manipulation of addresses, as the WHATWG URL standard describes them.", downloads: 496_118_745, repository: "https://github.com/servo/rust-url", homepage: None },
    CatalogueEntry { name: "ferroui", version: FERROUI_VERSION, description: "A cross-platform user-interface framework for Rust, with markup, styles and themes.", downloads: 2_418, repository: FERROUI_REPOSITORY, homepage: None },
    CatalogueEntry { name: "ferroui-reactive", version: FERROUI_VERSION, description: "Reactive extensions for Rust: observables, operators and schedulers, ported from System.Reactive.", downloads: 1_276, repository: FERROUI_REPOSITORY, homepage: None },
    CatalogueEntry { name: "ferroui-reactiveui", version: FERROUI_VERSION, description: "A model-view-viewmodel framework built on reactive extensions, ported from ReactiveUI.", downloads: 1_093, repository: FERROUI_REPOSITORY, homepage: None },
    CatalogueEntry { name: "ferroui-dynamic-data", version: FERROUI_VERSION, description: "Reactive collections: observable caches and lists with their operators, ported from DynamicData.", downloads: 987, repository: FERROUI_REPOSITORY, homepage: None },
];

/// The crates of the in-memory catalogue that the sample searches without the feature `online`
/// and in its tests (not an upstream type).
#[derive(Debug, Default, Clone, Copy)]
pub struct CatalogueSearchService;

impl CatalogueSearchService {
    /// The service.
    pub fn new() -> Self {
        Self
    }

    /// Every crate of the catalogue.
    pub fn crates() -> Vec<CrateSearchMetadata> {
        CATALOGUE
            .iter()
            .map(|entry| CrateSearchMetadata {
                name: entry.name.to_owned(),
                version: entry.version.to_owned(),
                description: Some(entry.description.to_owned()),
                downloads: entry.downloads,
                repository: Some(entry.repository.to_owned()),
                homepage: entry.homepage.map(str::to_owned),
                documentation: Some(format!("https://docs.rs/{}", entry.name)),
            })
            .collect()
    }

    /// The first [`TAKE`] crates of the catalogue that match `term`, in the order of the
    /// catalogue.
    pub fn find(term: &str) -> Vec<CrateSearchMetadata> {
        Self::crates().into_iter().filter(|krate| krate.matches(term)).take(TAKE).collect()
    }
}

impl ICrateSearchService for CatalogueSearchService {
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<CrateSearchMetadata>> {
        if token.is_cancellation_requested() {
            return Task::from_canceled();
        }
        Task::from_result(Self::find(term))
    }
}

// --- the search of crates.io ----------------------------------------------------------------

#[cfg(all(feature = "online", not(target_arch = "wasm32")))]
pub use online::{CratesIoSearchService, USER_AGENT};

#[cfg(all(feature = "online", not(target_arch = "wasm32")))]
mod online {
    use super::{search_url, CrateSearchMetadata, ICrateSearchService};
    use ferroui_system::threading::tasks::{self, Task};
    use ferroui_system::threading::CancellationToken;
    use serde::Deserialize;
    use std::error::Error;
    use std::sync::OnceLock;

    /// The error of work on a worker thread.
    type WorkError = Box<dyn Error + Send + Sync>;

    /// The `User-Agent` of the requests: crates.io refuses requests without one that names the
    /// client and a way to reach its authors (its data access policy).
    pub const USER_AGENT: &str = "ferroui-reactive-demo (https://github.com/wieslawsoltes/FerroUI)";

    /// One agent for every request of the process: it keeps the connections alive.
    fn agent() -> &'static ureq::Agent {
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        AGENT.get_or_init(|| ureq::AgentBuilder::new().build())
    }

    /// The response of the search (the members the sample reads).
    #[derive(Deserialize)]
    struct SearchResponse {
        #[serde(default)]
        crates: Vec<SearchResult>,
    }

    /// One crate of the response.
    #[derive(Deserialize)]
    struct SearchResult {
        name: String,
        description: Option<String>,
        max_version: Option<String>,
        newest_version: Option<String>,
        #[serde(default)]
        downloads: u64,
        repository: Option<String>,
        homepage: Option<String>,
        documentation: Option<String>,
    }

    /// An empty text as none.
    fn non_empty(value: Option<String>) -> Option<String> {
        value.filter(|value| !value.trim().is_empty())
    }

    impl From<SearchResult> for CrateSearchMetadata {
        fn from(result: SearchResult) -> Self {
            // `max_version` is `0.0.0` for a crate whose every version is yanked.
            let version = non_empty(result.max_version)
                .filter(|version| version != "0.0.0")
                .or_else(|| non_empty(result.newest_version))
                .unwrap_or_default();
            Self {
                name: result.name,
                version,
                description: non_empty(result.description).map(|description| description.trim().to_owned()),
                downloads: result.downloads,
                repository: non_empty(result.repository),
                homepage: non_empty(result.homepage),
                documentation: non_empty(result.documentation),
            }
        }
    }

    /// Sends the query of `term` and reads the crates of the response; blocks the thread.
    fn query(term: &str) -> Result<Vec<CrateSearchMetadata>, WorkError> {
        let response = agent()
            .get(&search_url(term))
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/json")
            .call()
            .map_err(|error| error.to_string())?;
        let text = response.into_string().map_err(|error| error.to_string())?;
        let response: SearchResponse = serde_json::from_str(&text).map_err(|error| error.to_string())?;
        Ok(response.crates.into_iter().map(CrateSearchMetadata::from).collect())
    }

    /// The search of crates.io (`SourceRepository` over the service index of nuget.org and its
    /// `PackageSearchResource` in the reference).
    #[derive(Debug, Default, Clone, Copy)]
    pub struct CratesIoSearchService;

    impl CratesIoSearchService {
        /// The service.
        pub fn new() -> Self {
            Self
        }
    }

    impl ICrateSearchService for CratesIoSearchService {
        /// Sends the query from a worker thread (the reference awaits `SearchAsync` with
        /// `ConfigureAwait(false)`); the task ends on the calling thread, through the poster of the
        /// thread (the dispatcher in the application).
        ///
        /// Deviation (DEVIATIONS.md, samples): the token stops a search that has not started; a
        /// request that has started runs to its end, and the reactive pipeline drops its result
        /// when the subscription is gone (NuGet.Protocol hands the token to the HTTP client).
        fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<CrateSearchMetadata>> {
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
    use super::{search_url, CatalogueSearchService, TAKE};

    #[test]
    fn the_catalogue_has_about_thirty_crates() {
        let count = CatalogueSearchService::crates().len();
        assert!((30..=40).contains(&count), "{count}");
    }

    #[test]
    fn every_word_of_the_term_matches_without_regard_to_case() {
        let found: Vec<String> = CatalogueSearchService::find("Http CLIENT").into_iter().map(|krate| krate.name).collect();
        assert_eq!(found, ["reqwest", "hyper"]);
    }

    #[test]
    fn a_search_gives_at_most_ten_crates() {
        let all = CatalogueSearchService::crates().iter().filter(|krate| krate.matches("e")).count();
        assert!(all > TAKE, "{all}");
        assert_eq!(CatalogueSearchService::find("e").len(), TAKE);
    }

    #[test]
    fn a_blank_term_matches_nothing() {
        assert!(CatalogueSearchService::find("   ").is_empty());
    }

    #[test]
    fn the_search_address_has_the_term_percent_encoded() {
        assert_eq!(search_url("serde"), "https://crates.io/api/v1/crates?q=serde&per_page=10");
        assert_eq!(search_url("async web/http & tls"), "https://crates.io/api/v1/crates?q=async%20web%2Fhttp%20%26%20tls&per_page=10");
        assert_eq!(search_url("proc-macro2_x.y~z"), "https://crates.io/api/v1/crates?q=proc-macro2_x.y~z&per_page=10");
        assert_eq!(search_url("caf\u{e9}"), "https://crates.io/api/v1/crates?q=caf%C3%A9&per_page=10");
    }
}
