//! Port of `NugetDetailsViewModel.cs`.

use crate::nuget_search::PackageSearchMetadata;
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

/// This class wraps our NuGet model object into a ViewModel and allows us to have a
/// ReactiveCommand to open the NuGet package URL.
pub struct NugetDetailsViewModel {
    reactive: ReactiveObject,
    metadata: PackageSearchMetadata,
    default_url: String,
    open_page: ReactiveCommand<(), ()>,
}
reactive_object!(NugetDetailsViewModel);

impl PartialEq for NugetDetailsViewModel {
    /// A view model is a reference type: two are equal when they are the same object.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl NugetDetailsViewModel {
    /// The icon of a package that declares none (`_defaultUrl` of the reference).
    pub const DEFAULT_URL: &'static str = "https://git.io/fAlfh";

    /// The view model of the package `metadata`, whose command opens pages with the launcher
    /// `launcher` gives.
    pub fn new(metadata: PackageSearchMetadata, launcher: LauncherSource) -> Rc<Self> {
        let page = Self::page_url_of(&metadata);
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
        Rc::new(Self { reactive: ReactiveObject::new(), metadata, default_url: Self::DEFAULT_URL.to_owned(), open_page })
    }

    /// The page the command opens: the project of the package.
    ///
    /// Deviation (DEVIATIONS.md, samples): for a package that declares no project the reference
    /// fails (`ProjectUrl.ToString()` of null); the port opens the page of the package on the
    /// gallery of NuGet.
    fn page_url_of(metadata: &PackageSearchMetadata) -> String {
        match &metadata.project_url {
            Some(project_url) => project_url.clone(),
            None => format!("https://www.nuget.org/packages/{}", metadata.id),
        }
    }

    /// `IconUrl`: the icon of the package, or the default icon.
    pub fn icon_url(&self) -> String {
        self.metadata.icon_url.clone().unwrap_or_else(|| self.default_url.clone())
    }

    /// `Description`.
    pub fn description(&self) -> Option<String> {
        self.metadata.description.clone()
    }

    /// `ProjectUrl`.
    pub fn project_url(&self) -> Option<String> {
        self.metadata.project_url.clone()
    }

    /// `Title`.
    pub fn title(&self) -> Option<String> {
        Some(self.metadata.title())
    }

    /// The identifier of the package (`Identity.Id` of the metadata the reference wraps).
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// `OpenPage`: ReactiveCommand allows us to execute logic without exposing any of the
    /// implementation details with the View. The generic parameters are the input into the command
    /// and its output. In our case we don't have any input or output so we use `()` which in
    /// Reactive speak means a void type.
    pub fn open_page(&self) -> ReactiveCommand<(), ()> {
        self.open_page.clone()
    }
}
