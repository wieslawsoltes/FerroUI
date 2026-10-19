//! Port of `NugetDetailsViewModel.cs`, for the crates of crates.io.

use crate::crate_search::CrateSearchMetadata;
use ferroui_base::platform::storage::ILauncher;
use ferroui_base::utilities::Uri;
use ferroui_reactiveui::{reactive_object, ReactiveCommand, ReactiveObject};
use ferroui_system::threading::tasks::Task;
use ferroui_system::ObservableError;
use std::rc::Rc;

/// Gives the launcher of the platform when a page is opened (not an upstream type).
///
/// Deviation (DEVIATIONS.md, samples): the reference opens the page with
/// `Process.Start(new ProcessStartInfo { UseShellExecute = true, .. })`. FerroUI opens addresses
/// with the launcher of the platform (`TopLevel::launcher`), which belongs to a window, so the view
/// model is handed a way to reach it: the main window gives its own, a test gives one that records.
pub type LauncherSource = Rc<dyn Fn() -> Rc<dyn ILauncher>>;

/// This class wraps our crate model object into a ViewModel and allows us to have a
/// ReactiveCommand to open the page of the crate.
pub struct CrateDetailsViewModel {
    reactive: ReactiveObject,
    metadata: CrateSearchMetadata,
    open_page: ReactiveCommand<(), ()>,
}
reactive_object!(CrateDetailsViewModel);

impl PartialEq for CrateDetailsViewModel {
    /// A view model is a reference type: two are equal when they are the same object.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CrateDetailsViewModel {
    /// The view model of the crate `metadata`, whose command opens pages with the launcher
    /// `launcher` gives.
    pub fn new(metadata: CrateSearchMetadata, launcher: LauncherSource) -> Rc<Self> {
        // Deviation (DEVIATIONS.md, samples): the reference opens the project of the package
        // (`ProjectUrl`); the port opens the page of the crate on crates.io, which every crate
        // has.
        let page = metadata.page_url();
        // ReactiveCommand.Create(() => Process.Start(ProjectUrl)).
        let open_page = ReactiveCommand::create(move || {
            let Ok(uri) = Uri::absolute(&page) else {
                return;
            };
            let launcher = launcher();
            // The launcher of the platform opens the address asynchronously; nothing waits for it,
            // as nothing waits for the process the reference starts.
            let _ = Task::spawn_local(async move {
                let _ = launcher.launch_uri_async(&uri).await;
                Ok::<(), ObservableError>(())
            });
        });
        Rc::new(Self { reactive: ReactiveObject::new(), metadata, open_page })
    }

    /// The letter of the badge of the crate: the first character of its name, in upper case.
    ///
    /// Deviation (DEVIATIONS.md, samples): the reference shows the icon of the package
    /// (`IconUrl`, with a default icon for a package that declares none); crates.io has no icons,
    /// so the view shows a badge with this letter.
    pub fn initial(&self) -> Option<String> {
        self.metadata.name.chars().next().map(|first| first.to_uppercase().collect())
    }

    /// `Description`.
    pub fn description(&self) -> Option<String> {
        self.metadata.description.clone()
    }

    /// The repository of the crate (`ProjectUrl` of a package).
    pub fn repository(&self) -> Option<String> {
        self.metadata.repository.clone()
    }

    /// The home page of the crate.
    pub fn homepage(&self) -> Option<String> {
        self.metadata.homepage.clone()
    }

    /// The documentation of the crate.
    pub fn documentation(&self) -> Option<String> {
        self.metadata.documentation.clone()
    }

    /// `Title`: the name of the crate.
    pub fn title(&self) -> Option<String> {
        Some(self.metadata.name.clone())
    }

    /// The name of the crate (`Identity.Id` of the metadata the reference wraps).
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// The latest version of the crate.
    pub fn version(&self) -> String {
        self.metadata.version.clone()
    }

    /// The number of downloads of the crate.
    pub fn downloads(&self) -> u64 {
        self.metadata.downloads
    }

    /// The page of the crate on crates.io, which [`open_page`](Self::open_page) opens.
    pub fn page_url(&self) -> String {
        self.metadata.page_url()
    }

    /// `OpenPage`: ReactiveCommand allows us to execute logic without exposing any of the
    /// implementation details with the View. The generic parameters are the input into the command
    /// and its output. In our case we don't have any input or output so we use `()` which in
    /// Reactive speak means a void type.
    pub fn open_page(&self) -> ReactiveCommand<(), ()> {
        self.open_page.clone()
    }
}

/// `value` with its digits in groups of three, separated by commas (`1,234,567`).
pub fn format_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
