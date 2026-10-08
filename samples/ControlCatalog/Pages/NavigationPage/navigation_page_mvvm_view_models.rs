//! Port of `Pages/NavigationPage/NavigationPageMvvmViewModels.cs`: the view models of the MVVM
//! sample of the navigation page.

use super::navigation_page_mvvm_navigation::{ISampleNavigationService, NavigationStateChangedEventArgs};
use crate::pages::navigation_demo_helper::parse_color;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_base::input::ICommand;
use ferroui_base::media::Color;
use ferroui_base::threading::DispatcherTask;
use ferroui_controls::ItemsSource;
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The view model of the page of the sample: the state of the navigation and the commands of
/// the side panel.
pub struct NavigationPageMvvmShellViewModel {
    base: ViewModelBase,
    navigation_service: Rc<dyn ISampleNavigationService>,
    current_page_header: RefCell<String>,
    last_action: RefCell<String>,
    navigation_depth: Cell<i32>,
    selected_project: RefCell<Option<Rc<ProjectCardViewModel>>>,
    workspace: Rc<WorkspaceViewModel>,
    open_selected_project_command: Rc<MiniCommand>,
    go_back_command: Rc<MiniCommand>,
    pop_to_root_command: Rc<MiniCommand>,
}

impl PartialEq for NavigationPageMvvmShellViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for NavigationPageMvvmShellViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl NavigationPageMvvmShellViewModel {
    pub(crate) fn new(navigation_service: Rc<dyn ISampleNavigationService>) -> Rc<NavigationPageMvvmShellViewModel> {
        Rc::new_cyclic(|this: &Weak<NavigationPageMvvmShellViewModel>| {
            // The handler belongs to the service, which the view model owns: it holds the
            // view model weakly.
            {
                let this = this.clone();
                navigation_service.state_changed(Rc::new(move |e: &NavigationStateChangedEventArgs| {
                    if let Some(this) = this.upgrade() {
                        this.on_state_changed(e);
                    }
                }));
            }

            let workspace = WorkspaceViewModel::new(Self::create_projects(&navigation_service));
            let selected_project = Some(workspace.projects()[0].clone());

            let open_selected_project_command = {
                let this = this.clone();
                MiniCommand::create_from_task(move || {
                    let this = this.upgrade();
                    async move {
                        if let Some(this) = this {
                            this.open_selected_project_async().await;
                        }
                    }
                })
            };
            let go_back_command = {
                let navigation_service = navigation_service.clone();
                MiniCommand::create_from_task(move || {
                    let task = navigation_service.go_back_async();
                    async move {
                        let _ = task.await;
                    }
                })
            };
            let pop_to_root_command = {
                let navigation_service = navigation_service.clone();
                MiniCommand::create_from_task(move || {
                    let task = navigation_service.pop_to_root_async();
                    async move {
                        let _ = task.await;
                    }
                })
            };

            Self {
                base: ViewModelBase::new(),
                navigation_service,
                current_page_header: RefCell::new(String::from("Not initialized")),
                last_action: RefCell::new(String::from("Waiting for first load")),
                navigation_depth: Cell::new(0),
                selected_project: RefCell::new(selected_project),
                workspace,
                open_selected_project_command,
                go_back_command,
                pop_to_root_command,
            }
        })
    }

    pub(crate) fn workspace(&self) -> Rc<WorkspaceViewModel> {
        self.workspace.clone()
    }

    pub fn projects(&self) -> Rc<Vec<Rc<ProjectCardViewModel>>> {
        self.workspace().projects()
    }

    pub fn open_selected_project_command(&self) -> Rc<MiniCommand> {
        self.open_selected_project_command.clone()
    }

    pub fn go_back_command(&self) -> Rc<MiniCommand> {
        self.go_back_command.clone()
    }

    pub fn pop_to_root_command(&self) -> Rc<MiniCommand> {
        self.pop_to_root_command.clone()
    }

    pub fn current_page_header(&self) -> String {
        self.current_page_header.borrow().clone()
    }

    pub fn set_current_page_header(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.current_page_header, value, "CurrentPageHeader");
    }

    pub fn navigation_depth(&self) -> i32 {
        self.navigation_depth.get()
    }

    pub fn set_navigation_depth(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.navigation_depth, value, "NavigationDepth");
    }

    pub fn last_action(&self) -> String {
        self.last_action.borrow().clone()
    }

    pub fn set_last_action(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.last_action, value, "LastAction");
    }

    pub fn selected_project(&self) -> Option<Rc<ProjectCardViewModel>> {
        self.selected_project.borrow().clone()
    }

    pub fn set_selected_project(&self, value: Option<Rc<ProjectCardViewModel>>) {
        self.base.raise_and_set_if_changed(&self.selected_project, value, "SelectedProject");
    }

    pub fn initialize_async(&self) -> DispatcherTask<()> {
        self.navigation_service.navigate_to_async(self.workspace())
    }

    async fn open_selected_project_async(&self) {
        let Some(selected_project) = self.selected_project() else {
            return;
        };

        let _ = selected_project.open_command_async().await;
    }

    fn on_state_changed(&self, e: &NavigationStateChangedEventArgs) {
        self.set_current_page_header(e.current_page_header().to_string());
        self.set_navigation_depth(e.navigation_depth());
        self.set_last_action(e.last_action().to_string());
    }

    fn create_projects(navigation_service: &Rc<dyn ISampleNavigationService>) -> Rc<Vec<Rc<ProjectCardViewModel>>> {
        Rc::new(vec![
            ProjectCardViewModel::new(
                "Release Radar",
                "Marta Collins",
                "Ready for QA",
                "Coordinate the 11.0 release checklist and lock down the final regression window.",
                "Freeze build on Friday",
                parse_color("#0063B1"),
                navigation_service.clone(),
                &[
                    "Release notes draft updated with accessibility fixes.",
                    "Package validation finished for desktop artifacts.",
                    "Remaining task, confirm browser smoke test coverage.",
                ],
            ),
            ProjectCardViewModel::new(
                "Support Console",
                "Jae Kim",
                "Active Sprint",
                "Consolidate customer incidents into a triage board and route them to platform owners.",
                "Triage review in 2 hours",
                parse_color("#0F7B0F"),
                navigation_service.clone(),
                &[
                    "Five customer reports grouped under input routing.",
                    "Hotfix candidate approved for preview branch.",
                    "Awaiting macOS verification on native embed scenarios.",
                ],
            ),
            ProjectCardViewModel::new(
                "Docs Refresh",
                "Anika Patel",
                "Needs Review",
                "Refresh navigation samples and walkthrough docs so the gallery matches the current API.",
                "Sample review tomorrow",
                parse_color("#8E562E"),
                navigation_service.clone(),
                &[
                    "NavigationPage sample matrix reviewed with design.",
                    "MVVM walkthrough draft linked from the docs backlog.",
                    "Outstanding task, capture one more screenshot for drawer navigation.",
                ],
            ),
        ])
    }
}

// The constructor of the class is internal and `InitializeAsync` returns a task: neither is
// declared for markup.
ferro_markup_type!(class NavigationPageMvvmShellViewModel {
    this: Rc<NavigationPageMvvmShellViewModel>,
    handles: [
        NavigationPageMvvmShellViewModel,
        Rc<NavigationPageMvvmShellViewModel>,
        Option<Rc<NavigationPageMvvmShellViewModel>>
    ],
    properties: [
        // A list a binding delivers to an items source property. The items are the view models
        // themselves (the boxing of a reference type), so that the selected item, which a
        // binding delivers in that form, is found among them.
        Projects: ItemsSource {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| {
                ItemsSource::from(this.projects().iter().map(|project| project.clone() as BoxedValue).collect::<Vec<_>>())
            }
        },
        OpenSelectedProjectCommand: Rc<dyn ICommand> {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.open_selected_project_command().as_command()
        },
        GoBackCommand: Rc<dyn ICommand> {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.go_back_command().as_command()
        },
        PopToRootCommand: Rc<dyn ICommand> {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.pop_to_root_command().as_command()
        },
        CurrentPageHeader: String {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.current_page_header(),
            set: |this: &Rc<NavigationPageMvvmShellViewModel>, value: String| this.set_current_page_header(value)
        },
        NavigationDepth: i32 {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.navigation_depth(),
            set: |this: &Rc<NavigationPageMvvmShellViewModel>, value: i32| this.set_navigation_depth(value)
        },
        LastAction: String {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.last_action(),
            set: |this: &Rc<NavigationPageMvvmShellViewModel>, value: String| this.set_last_action(value)
        },
        SelectedProject: Option<Rc<ProjectCardViewModel>> {
            get: |this: &Rc<NavigationPageMvvmShellViewModel>| this.selected_project(),
            set: |this: &Rc<NavigationPageMvvmShellViewModel>, value: Option<Rc<ProjectCardViewModel>>| {
                this.set_selected_project(value)
            }
        },
    ],
    notify_property_changed: NavigationPageMvvmShellViewModel,
});

/// The view model of the root page: the projects of the workspace.
pub(crate) struct WorkspaceViewModel {
    base: ViewModelBase,
    projects: Rc<Vec<Rc<ProjectCardViewModel>>>,
}

impl INotifyPropertyChanged for WorkspaceViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl WorkspaceViewModel {
    pub(crate) fn new(projects: Rc<Vec<Rc<ProjectCardViewModel>>>) -> Rc<WorkspaceViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), projects })
    }

    pub(crate) fn title(&self) -> &'static str {
        "Team Workspace"
    }

    pub(crate) fn description(&self) -> &'static str {
        "Each card is a project view model with its own command. The command asks ISampleNavigationService to navigate with the next view model, and SamplePageFactory resolves the matching ContentPage."
    }

    pub(crate) fn projects(&self) -> Rc<Vec<Rc<ProjectCardViewModel>>> {
        self.projects.clone()
    }
}

/// The view model of a project card: the data of the project and the command that opens it.
pub struct ProjectCardViewModel {
    base: ViewModelBase,
    navigation_service: Rc<dyn ISampleNavigationService>,
    name: String,
    owner: String,
    status: String,
    summary: String,
    next_milestone: String,
    accent_color: Color,
    activity_items: Rc<Vec<String>>,
    open_command: Rc<MiniCommand>,
}

impl PartialEq for ProjectCardViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ProjectCardViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ProjectCardViewModel {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        name: &str,
        owner: &str,
        status: &str,
        summary: &str,
        next_milestone: &str,
        accent_color: Color,
        navigation_service: Rc<dyn ISampleNavigationService>,
        activity_items: &[&str],
    ) -> Rc<ProjectCardViewModel> {
        Rc::new_cyclic(|this: &Weak<ProjectCardViewModel>| {
            // The command belongs to the view model: its callback holds the view model weakly.
            let open_command = {
                let this = this.clone();
                MiniCommand::create_from_task(move || {
                    let this = this.upgrade();
                    async move {
                        if let Some(this) = this {
                            let _ = this.open_command_async().await;
                        }
                    }
                })
            };

            Self {
                base: ViewModelBase::new(),
                navigation_service,
                name: name.to_string(),
                owner: owner.to_string(),
                status: status.to_string(),
                summary: summary.to_string(),
                next_milestone: next_milestone.to_string(),
                accent_color,
                activity_items: Rc::new(activity_items.iter().map(|item| item.to_string()).collect()),
                open_command,
            }
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn next_milestone(&self) -> &str {
        &self.next_milestone
    }

    pub fn accent_color(&self) -> Color {
        self.accent_color
    }

    pub fn activity_items(&self) -> Rc<Vec<String>> {
        self.activity_items.clone()
    }

    pub fn open_command(&self) -> Rc<MiniCommand> {
        self.open_command.clone()
    }

    pub fn open_command_async(self: &Rc<Self>) -> DispatcherTask<()> {
        self.navigation_service
            .navigate_to_async(ProjectDetailViewModel::new(self.clone(), self.navigation_service.clone()))
    }
}

// The constructor of the class is internal and `OpenCommandAsync` returns a task: neither is
// declared for markup.
ferro_markup_type!(class ProjectCardViewModel {
    this: Rc<ProjectCardViewModel>,
    handles: [ProjectCardViewModel, Rc<ProjectCardViewModel>, Option<Rc<ProjectCardViewModel>>],
    properties: [
        Name: String { get: |this: &Rc<ProjectCardViewModel>| this.name().to_string() },
        Owner: String { get: |this: &Rc<ProjectCardViewModel>| this.owner().to_string() },
        Status: String { get: |this: &Rc<ProjectCardViewModel>| this.status().to_string() },
        Summary: String { get: |this: &Rc<ProjectCardViewModel>| this.summary().to_string() },
        NextMilestone: String { get: |this: &Rc<ProjectCardViewModel>| this.next_milestone().to_string() },
        AccentColor: Color { get: |this: &Rc<ProjectCardViewModel>| this.accent_color() },
        // A list a binding delivers to an items source property.
        ActivityItems: ItemsSource {
            get: |this: &Rc<ProjectCardViewModel>| ItemsSource::from_values(this.activity_items().iter().cloned())
        },
        OpenCommand: Rc<dyn ICommand> { get: |this: &Rc<ProjectCardViewModel>| this.open_command().as_command() },
    ],
    notify_property_changed: ProjectCardViewModel,
});

/// The view model of the detail page of a project.
pub(crate) struct ProjectDetailViewModel {
    base: ViewModelBase,
    project: Rc<ProjectCardViewModel>,
    /// Holds the navigation service of the view model (the field `_navigationService` of the
    /// original).
    open_activity_command: Rc<MiniCommand>,
}

impl INotifyPropertyChanged for ProjectDetailViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ProjectDetailViewModel {
    pub(crate) fn new(
        project: Rc<ProjectCardViewModel>,
        navigation_service: Rc<dyn ISampleNavigationService>,
    ) -> Rc<ProjectDetailViewModel> {
        // Nothing but the command of the button of its page refers to the view model, and the
        // view model owns the command: the callback of the command holds what
        // `OpenActivityAsync` reads, not the view model.
        let open_activity_command = {
            let project = project.clone();
            MiniCommand::create_from_task(move || {
                let task = Self::open_activity_async(&project, &navigation_service);
                async move {
                    let _ = task.await;
                }
            })
        };

        Rc::new(Self { base: ViewModelBase::new(), project, open_activity_command })
    }

    pub(crate) fn name(&self) -> &str {
        self.project.name()
    }

    pub(crate) fn owner(&self) -> &str {
        self.project.owner()
    }

    pub(crate) fn status(&self) -> &str {
        self.project.status()
    }

    pub(crate) fn summary(&self) -> &str {
        self.project.summary()
    }

    pub(crate) fn next_milestone(&self) -> &str {
        self.project.next_milestone()
    }

    pub(crate) fn accent_color(&self) -> Color {
        self.project.accent_color()
    }

    pub(crate) fn open_activity_command(&self) -> Rc<MiniCommand> {
        self.open_activity_command.clone()
    }

    fn open_activity_async(
        project: &Rc<ProjectCardViewModel>,
        navigation_service: &Rc<dyn ISampleNavigationService>,
    ) -> DispatcherTask<()> {
        navigation_service.navigate_to_async(ProjectActivityViewModel::new(project))
    }
}

/// The view model of the activity page of a project.
pub(crate) struct ProjectActivityViewModel {
    base: ViewModelBase,
    name: String,
    accent_color: Color,
    items: Rc<Vec<String>>,
}

impl INotifyPropertyChanged for ProjectActivityViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ProjectActivityViewModel {
    pub(crate) fn new(project: &ProjectCardViewModel) -> Rc<ProjectActivityViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            name: project.name().to_string(),
            accent_color: project.accent_color(),
            items: project.activity_items(),
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn accent_color(&self) -> Color {
        self.accent_color
    }

    pub(crate) fn items(&self) -> Rc<Vec<String>> {
        self.items.clone()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::super::navigation_page_mvvm_navigation::SampleViewModel;
    use super::*;
    use ferroui_base::reactive::{Disposable, IDisposable};
    use ferroui_base::utilities::HandlerList;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use mini_mvvm::start_async;

    /// A navigation service that records what it is asked for.
    #[derive(Default)]
    struct RecordingNavigationService {
        state_changed: HandlerList<dyn Fn(&NavigationStateChangedEventArgs)>,
        requests: RefCell<Vec<String>>,
        view_models: RefCell<Vec<SampleViewModel>>,
    }

    impl RecordingNavigationService {
        fn raise(&self, e: &NavigationStateChangedEventArgs) {
            for (_, handler) in self.state_changed.snapshot().iter() {
                handler(e);
            }
        }
    }

    impl ISampleNavigationService for RecordingNavigationService {
        fn state_changed(&self, handler: Rc<dyn Fn(&NavigationStateChangedEventArgs)>) -> Rc<dyn IDisposable> {
            self.state_changed.add(handler);
            Disposable::create(|| {})
        }

        fn navigate_to_async(&self, view_model: SampleViewModel) -> DispatcherTask<()> {
            self.requests.borrow_mut().push(String::from("navigate"));
            self.view_models.borrow_mut().push(view_model);
            start_async(async {})
        }

        fn go_back_async(&self) -> DispatcherTask<()> {
            self.requests.borrow_mut().push(String::from("back"));
            start_async(async {})
        }

        fn pop_to_root_async(&self) -> DispatcherTask<()> {
            self.requests.borrow_mut().push(String::from("root"));
            start_async(async {})
        }
    }

    #[test]
    fn the_shell_starts_with_the_first_project_selected() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let service = Rc::new(RecordingNavigationService::default());
        let shell = NavigationPageMvvmShellViewModel::new(service.clone());

        let projects = shell.projects();
        assert_eq!(vec!["Release Radar", "Support Console", "Docs Refresh"], projects.iter().map(|p| p.name()).collect::<Vec<_>>());
        assert!(shell.selected_project().is_some_and(|selected| Rc::ptr_eq(&selected, &projects[0])));
        assert_eq!("Not initialized", shell.current_page_header());
        assert_eq!(0, shell.navigation_depth());
        assert_eq!("Waiting for first load", shell.last_action());
        assert_eq!(3, projects[2].activity_items().len());
    }

    #[test]
    fn the_shell_follows_the_state_of_the_navigation() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let service = Rc::new(RecordingNavigationService::default());
        let shell = NavigationPageMvvmShellViewModel::new(service.clone());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        shell.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        service.raise(&NavigationStateChangedEventArgs::new("Workspace", 1, "Pushed Workspace"));
        assert_eq!("Workspace", shell.current_page_header());
        assert_eq!(1, shell.navigation_depth());
        assert_eq!("Pushed Workspace", shell.last_action());
        assert_eq!(vec!["CurrentPageHeader", "NavigationDepth", "LastAction"], *seen.borrow());

        // Only what changed is notified.
        service.raise(&NavigationStateChangedEventArgs::new("Workspace", 1, "Already at the root page"));
        assert_eq!(vec!["CurrentPageHeader", "NavigationDepth", "LastAction", "LastAction"], *seen.borrow());
    }

    #[test]
    fn the_commands_ask_the_navigation_service() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let service = Rc::new(RecordingNavigationService::default());
        let shell = NavigationPageMvvmShellViewModel::new(service.clone());

        drop(shell.initialize_async());
        assert!(service.view_models.borrow()[0].clone().downcast::<WorkspaceViewModel>().is_ok());

        shell.set_selected_project(Some(shell.projects()[1].clone()));
        shell.open_selected_project_command().execute(None);
        let detail = service.view_models.borrow()[1].clone().downcast::<ProjectDetailViewModel>();
        let detail = detail.ok().expect("the detail view model of the selected project");
        assert_eq!("Support Console", detail.name());
        assert_eq!("Jae Kim", detail.owner());

        detail.open_activity_command().execute(None);
        let activity = service.view_models.borrow()[2].clone().downcast::<ProjectActivityViewModel>();
        let activity = activity.ok().expect("the activity view model of the project");
        assert_eq!("Support Console", activity.name());
        assert_eq!(3, activity.items().len());

        shell.go_back_command().execute(None);
        shell.pop_to_root_command().execute(None);
        shell.set_selected_project(None);
        shell.open_selected_project_command().execute(None);
        assert_eq!(vec!["navigate", "navigate", "navigate", "back", "root"], *service.requests.borrow());
    }
}
