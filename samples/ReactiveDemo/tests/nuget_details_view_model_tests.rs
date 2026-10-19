//! Not a port (the reference has no tests): the properties of [`NugetDetailsViewModel`] and its
//! command `OpenPage`.

use super::support::{catalogue_package, RecordingLauncher};
use crate::nuget_search::PackageSearchMetadata;
use crate::NugetDetailsViewModel;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui_testing::{SchedulerExtensions, TestSchedulerExtensions};

#[test]
fn the_properties_are_those_of_the_package() {
    let launcher = RecordingLauncher::new();
    let view_model = NugetDetailsViewModel::new(catalogue_package("Newtonsoft.Json"), RecordingLauncher::source(&launcher));

    assert_eq!(view_model.id(), "Newtonsoft.Json");
    assert_eq!(view_model.title().as_deref(), Some("Json.NET"));
    assert_eq!(view_model.description().as_deref(), Some("A popular high-performance JSON framework for .NET."));
    assert_eq!(view_model.project_url().as_deref(), Some("https://www.newtonsoft.com/json"));
    // A package without an icon shows the default one.
    assert_eq!(view_model.icon_url(), NugetDetailsViewModel::DEFAULT_URL);
}

#[test]
fn a_package_without_a_title_is_titled_by_its_identifier() {
    let launcher = RecordingLauncher::new();
    let package = PackageSearchMetadata {
        id: "Some.Package".to_owned(),
        title: Some("  ".to_owned()),
        icon_url: Some("https://example.org/icon.png".to_owned()),
        ..PackageSearchMetadata::default()
    };
    let view_model = NugetDetailsViewModel::new(package, RecordingLauncher::source(&launcher));

    assert_eq!(view_model.title().as_deref(), Some("Some.Package"));
    assert_eq!(view_model.icon_url(), "https://example.org/icon.png");
    assert_eq!(view_model.description(), None);
}

#[test]
fn open_page_opens_the_project_with_the_launcher() {
    TestScheduler::new().with(|scheduler| {
        let launcher = RecordingLauncher::new();
        let view_model = NugetDetailsViewModel::new(catalogue_package("ReactiveUI"), RecordingLauncher::source(&launcher));
        assert!(launcher.uris().is_empty());

        let _ = view_model.open_page().execute_default().subscribe_next(|_| {});
        scheduler.advance_by_ms(1.0);

        assert_eq!(launcher.uris(), ["https://reactiveui.net/"]);
    });
}

#[test]
fn open_page_opens_the_gallery_page_of_a_package_without_a_project() {
    TestScheduler::new().with(|scheduler| {
        let launcher = RecordingLauncher::new();
        let package = PackageSearchMetadata { id: "Some.Package".to_owned(), ..PackageSearchMetadata::default() };
        let view_model = NugetDetailsViewModel::new(package, RecordingLauncher::source(&launcher));

        let _ = view_model.open_page().execute_default().subscribe_next(|_| {});
        scheduler.advance_by_ms(1.0);

        assert_eq!(launcher.uris(), ["https://www.nuget.org/packages/Some.Package"]);
    });
}
