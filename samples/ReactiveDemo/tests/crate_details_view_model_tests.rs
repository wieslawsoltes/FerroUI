//! Not a port (the reference has no tests): the properties of [`CrateDetailsViewModel`], its
//! command `OpenPage` and the formatting of the number of downloads.

use super::support::{catalogue_crate, RecordingLauncher};
use crate::crate_search::CrateSearchMetadata;
use crate::{format_thousands, CrateDetailsViewModel};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui_testing::{SchedulerExtensions, TestSchedulerExtensions};

#[test]
fn the_properties_are_those_of_the_crate() {
    let launcher = RecordingLauncher::new();
    let view_model = CrateDetailsViewModel::new(catalogue_crate("serde"), RecordingLauncher::source(&launcher));

    assert_eq!(view_model.name(), "serde");
    assert_eq!(view_model.title().as_deref(), Some("serde"));
    assert_eq!(view_model.version(), "1.0.228");
    assert_eq!(
        view_model.description().as_deref(),
        Some("A generic framework for serializing and deserializing Rust data structures efficiently.")
    );
    assert_eq!(view_model.downloads(), 612_480_315);
    assert_eq!(view_model.repository().as_deref(), Some("https://github.com/serde-rs/serde"));
    assert_eq!(view_model.homepage().as_deref(), Some("https://serde.rs"));
    assert_eq!(view_model.documentation().as_deref(), Some("https://docs.rs/serde"));
    assert_eq!(view_model.page_url(), "https://crates.io/crates/serde");
    // crates.io has no icons: the badge shows the first letter of the name.
    assert_eq!(view_model.initial().as_deref(), Some("S"));
}

#[test]
fn a_crate_without_a_description_has_none() {
    let launcher = RecordingLauncher::new();
    let krate = CrateSearchMetadata { name: "proc-macro2".to_owned(), version: "1.0.101".to_owned(), ..CrateSearchMetadata::default() };
    let view_model = CrateDetailsViewModel::new(krate, RecordingLauncher::source(&launcher));

    assert_eq!(view_model.title().as_deref(), Some("proc-macro2"));
    assert_eq!(view_model.initial().as_deref(), Some("P"));
    assert_eq!(view_model.description(), None);
    assert_eq!(view_model.repository(), None);
    assert_eq!(view_model.downloads(), 0);
}

#[test]
fn open_page_opens_the_page_of_the_crate_on_crates_io_with_the_launcher() {
    TestScheduler::new().with(|scheduler| {
        let launcher = RecordingLauncher::new();
        let view_model = CrateDetailsViewModel::new(catalogue_crate("ferroui-reactiveui"), RecordingLauncher::source(&launcher));
        assert!(launcher.uris().is_empty());

        let _ = view_model.open_page().execute_default().subscribe_next(|_| {});
        scheduler.advance_by_ms(1.0);

        assert_eq!(launcher.uris(), ["https://crates.io/crates/ferroui-reactiveui"]);
    });
}

#[test]
fn the_number_of_downloads_is_written_with_thousands_separators() {
    assert_eq!(format_thousands(0), "0");
    assert_eq!(format_thousands(987), "987");
    assert_eq!(format_thousands(1_000), "1,000");
    assert_eq!(format_thousands(12_345), "12,345");
    assert_eq!(format_thousands(612_480_315), "612,480,315");
    assert_eq!(format_thousands(u64::MAX), "18,446,744,073,709,551,615");
}
