//! Port of `ViewModels/MainViewModel.cs`: the application shell view model.

use super::{CommandLabViewModel, DynamicDataViewModel, MetricsViewModel, OverviewViewModel, ViewModelBase};
use crate::services::ILocalMachineMetricsService;
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, IActivatableViewModel, IReactiveObjectExt, IRoutableViewModel, IScreen, NavigateBackCommand, ReactiveCommand, RoutableViewModel,
    RoutingState, ViewModelActivator,
};
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// The commands of the shell as commands of the controls of FerroUI, for the bindings of markup.
// Deviation (DEVIATIONS.md, samples): the reference binds its commands in markup directly; the
// commands of ReactiveUI and of the controls are two traits in the port, so the bindings of markup
// read an adapter of each command (`ReactiveCommandAdapter`), created once.
#[derive(Default)]
struct MarkupCommands {
    show_overview: OnceCell<Rc<dyn FerroCommand>>,
    show_metrics: OnceCell<Rc<dyn FerroCommand>>,
    show_commands: OnceCell<Rc<dyn FerroCommand>>,
    show_dynamic_data: OnceCell<Rc<dyn FerroCommand>>,
    go_back: OnceCell<Rc<dyn FerroCommand>>,
    reset: OnceCell<Rc<dyn FerroCommand>>,
}

/// The pages and the commands of the shell, created after the shell itself
/// (`Initialize(ILocalMachineMetricsService)` and the constructor).
struct Pages {
    overview: Rc<OverviewViewModel>,
    metrics: Rc<MetricsViewModel>,
    commands: Rc<CommandLabViewModel>,
    dynamic_data: Rc<DynamicDataViewModel>,
    show_overview: ReactiveCommand<(), ()>,
    show_metrics: ReactiveCommand<(), ()>,
    show_commands: ReactiveCommand<(), ()>,
    show_dynamic_data: ReactiveCommand<(), ()>,
    reset: ReactiveCommand<(), ()>,
}

/// The application shell view model (`MainViewModel`): the screen whose router the
/// `RoutedViewHost` of the main view follows, its pages and the commands of the sidebar.
///
/// The DynamicData page and its command (`DynamicData`, `ShowDynamicData`) are an addition of the
/// port with no counterpart in the reference.
pub struct MainViewModel {
    base: ViewModelBase,
    /// A weak handle to the shell, for the handlers it registers.
    this: Weak<MainViewModel>,
    /// The router of the screen (`Router`).
    router: Rc<RoutingState>,
    /// The pages and the commands, set once the shell is shared.
    pages: OnceCell<Pages>,
    /// The commands for the bindings of markup.
    markup_commands: MarkupCommands,
    /// The navigation status (`NavigationStatus`).
    navigation_status: RefCell<String>,
    /// The current persistence status (`SessionStatus`).
    session_status: RefCell<String>,
}
reactive_object!(MainViewModel, base);

impl IActivatableViewModel for MainViewModel {
    fn activator(&self) -> &ViewModelActivator {
        self.base.activator()
    }
}

impl IScreen for MainViewModel {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

impl PartialEq for MainViewModel {
    /// The same shell (reference equality, as for the objects of the reference).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MainViewModel {
    /// Creates an initialized shell view model (`Create(ILocalMachineMetricsService)`): the pages,
    /// the commands, and the navigation reset to the overview.
    pub fn create(metrics_service: Rc<dyn ILocalMachineMetricsService>) -> Rc<Self> {
        let view_model = Rc::new_cyclic(|this| Self {
            base: ViewModelBase::new(),
            this: this.clone(),
            router: RoutingState::new(),
            pages: OnceCell::new(),
            markup_commands: MarkupCommands::default(),
            navigation_status: RefCell::new("Navigation is ready.".to_owned()),
            session_status: RefCell::new("Session persistence starts with the desktop lifetime.".to_owned()),
        });
        view_model.initialize(metrics_service);
        view_model
    }

    /// Creates the commands and the routed pages after construction (the constructor and
    /// `Initialize(ILocalMachineMetricsService)`), then resets the navigation to the overview.
    fn initialize(self: &Rc<Self>, metrics_service: Rc<dyn ILocalMachineMetricsService>) {
        let weak = Rc::downgrade(self);
        let base = &self.base;

        let navigate_to = |page: fn(&Self) -> RoutableViewModel| {
            let weak = weak.clone();
            base.track(ReactiveCommand::create(move || {
                if let Some(shell) = weak.upgrade() {
                    shell.navigate(Some(page(&shell)));
                }
            }))
        };
        let show_overview = navigate_to(|shell| shell.overview() as RoutableViewModel);
        let show_metrics = navigate_to(|shell| shell.metrics() as RoutableViewModel);
        let show_commands = navigate_to(|shell| shell.commands() as RoutableViewModel);
        let show_dynamic_data = navigate_to(|shell| shell.dynamic_data() as RoutableViewModel);
        let reset_weak = weak.clone();
        let reset = base.track(ReactiveCommand::create(move || {
            if let Some(shell) = reset_weak.upgrade() {
                shell.reset_navigation();
            }
        }));

        // A router command that fails reports the error through ThrownExceptions as well as to the
        // caller. Nothing observing it sends the error to ReactiveUI's default handler, which
        // throws, so the shell observes all three.
        let thrown = [
            self.router.navigate().thrown_exceptions(),
            self.router.navigate_and_reset().thrown_exceptions(),
            self.router.navigate_back().thrown_exceptions(),
        ];
        for errors in thrown {
            let (on_error, on_error_error) = (weak.clone(), weak.clone());
            let _ = base.track_disposable(errors.subscribe_next_error(
                move |error| with_shell(&on_error, |shell| shell.handle_navigation_error(&error)),
                move |error| with_shell(&on_error_error, |shell| shell.handle_navigation_error(&error)),
            ));
        }

        let screen: Rc<dyn IScreen> = self.clone();
        let overview = OverviewViewModel::new(&screen);
        let metrics = MetricsViewModel::new(&screen, metrics_service);
        let _ = base.track_disposable(metrics.clone());
        let commands = CommandLabViewModel::new(&screen);
        let _ = base.track_disposable(commands.clone());
        let dynamic_data = DynamicDataViewModel::new(&screen);
        let _ = base.track_disposable(dynamic_data.clone());

        let markup = &self.markup_commands;
        let adapter = |command: &ReactiveCommand<(), ()>| ReactiveCommandAdapter::command_of(Rc::new(command.clone()));
        let _ = markup.show_overview.set(adapter(&show_overview));
        let _ = markup.show_metrics.set(adapter(&show_metrics));
        let _ = markup.show_commands.set(adapter(&show_commands));
        let _ = markup.show_dynamic_data.set(adapter(&show_dynamic_data));
        let _ = markup.reset.set(adapter(&reset));
        let _ = markup.go_back.set(ReactiveCommandAdapter::command_of(Rc::new(self.router.navigate_back())));

        let _ = self.pages.set(Pages { overview, metrics, commands, dynamic_data, show_overview, show_metrics, show_commands, show_dynamic_data, reset });
        self.reset_navigation();
    }

    fn pages(&self) -> &Pages {
        self.pages.get().expect("the pages are created with the shell")
    }

    /// The router of the screen (`Router`).
    pub fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }

    /// The overview page view model.
    pub fn overview(&self) -> Rc<OverviewViewModel> {
        self.pages().overview.clone()
    }

    /// The metrics page view model (`Metrics`).
    pub fn metrics(&self) -> Rc<MetricsViewModel> {
        self.pages().metrics.clone()
    }

    /// The commands page view model (`Commands`).
    pub fn commands(&self) -> Rc<CommandLabViewModel> {
        self.pages().commands.clone()
    }

    /// The DynamicData page view model (an addition of the port).
    pub fn dynamic_data(&self) -> Rc<DynamicDataViewModel> {
        self.pages().dynamic_data.clone()
    }

    /// The command that navigates to the overview (`ShowOverview`).
    pub fn show_overview(&self) -> ReactiveCommand<(), ()> {
        self.pages().show_overview.clone()
    }

    /// The command that navigates to the metrics page (`ShowMetrics`).
    pub fn show_metrics(&self) -> ReactiveCommand<(), ()> {
        self.pages().show_metrics.clone()
    }

    /// The command that navigates to the command lab (`ShowCommands`).
    pub fn show_commands(&self) -> ReactiveCommand<(), ()> {
        self.pages().show_commands.clone()
    }

    /// The command that navigates to the DynamicData page (an addition of the port).
    pub fn show_dynamic_data(&self) -> ReactiveCommand<(), ()> {
        self.pages().show_dynamic_data.clone()
    }

    /// The command that navigates back (`GoBack`): the router's `NavigateBack`.
    pub fn go_back(&self) -> NavigateBackCommand {
        self.router.navigate_back()
    }

    /// The command that resets navigation (`Reset`).
    pub fn reset(&self) -> ReactiveCommand<(), ()> {
        self.pages().reset.clone()
    }

    /// The navigation status (`NavigationStatus { get; }`).
    pub fn navigation_status(&self) -> String {
        self.navigation_status.borrow().clone()
    }

    /// Sets the navigation status (`NavigationStatus { set; }`).
    pub fn set_navigation_status(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.navigation_status, value, "NavigationStatus");
    }

    /// The current persistence status (`SessionStatus { get; }`).
    pub fn session_status(&self) -> String {
        self.session_status.borrow().clone()
    }

    /// Sets the current persistence status (`SessionStatus { set; }`).
    pub fn set_session_status(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.session_status, value, "SessionStatus");
    }

    /// Navigates to a page (`Navigate(IRoutableViewModel)`); `None` is the null page the router
    /// rejects, whose error becomes the navigation status.
    pub fn navigate(&self, page: Option<RoutableViewModel>) {
        let weak = self.this.clone();
        let on_error = weak.clone();
        let _ = self.router.navigate().execute(page).subscribe_next_error(
            move |routed: RoutableViewModel| {
                with_shell(&weak, |shell| shell.set_navigation_status(format!("Navigated to {}.", routed.url_path_segment().unwrap_or_default())))
            },
            move |error| with_shell(&on_error, |shell| shell.handle_navigation_error(&error)),
        );
    }

    /// Resets the navigation stack to the overview page (`ResetNavigation()`).
    pub fn reset_navigation(&self) {
        let weak = self.this.clone();
        let on_error = weak.clone();
        let overview: RoutableViewModel = self.overview();
        let _ = self.router.navigate_and_reset().execute(Some(overview)).subscribe_next_error(
            move |routed: RoutableViewModel| {
                with_shell(&weak, |shell| shell.set_navigation_status(format!("Reset to {}.", routed.url_path_segment().unwrap_or_default())))
            },
            move |error| with_shell(&on_error, |shell| shell.handle_navigation_error(&error)),
        );
    }

    /// Handles navigation errors (`HandleNavigationError(Exception)`).
    fn handle_navigation_error(&self, error: &ObservableError) {
        self.set_navigation_status(error.to_string());
    }
}

impl IDisposable for MainViewModel {
    /// `Dispose()`: the overview page, then the base, which disposes the other pages and the
    /// commands.
    fn dispose(&self) {
        if self.base.is_disposed() {
            return;
        }
        if let Some(pages) = self.pages.get() {
            pages.overview.dispose();
        }
        self.base.dispose();
    }
}

/// Runs `action` on the shell while it is alive.
fn with_shell(shell: &Weak<MainViewModel>, action: impl FnOnce(&MainViewModel)) {
    if let Some(shell) = shell.upgrade() {
        action(&shell);
    }
}

ferro_markup_type!(class MainViewModel {
    this: Rc<MainViewModel>,
    handles: [MainViewModel, Rc<MainViewModel>, Option<Rc<MainViewModel>>],
    properties: [
        Router: Rc<RoutingState> { get: |this: &Rc<MainViewModel>| this.router() },
        ShowOverview: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.show_overview.get().expect("the commands are created with the shell").clone()
        },
        ShowMetrics: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.show_metrics.get().expect("the commands are created with the shell").clone()
        },
        ShowCommands: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.show_commands.get().expect("the commands are created with the shell").clone()
        },
        ShowDynamicData: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.show_dynamic_data.get().expect("the commands are created with the shell").clone()
        },
        GoBack: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.go_back.get().expect("the commands are created with the shell").clone()
        },
        Reset: Rc<dyn FerroCommand> {
            get: |this: &Rc<MainViewModel>| this.markup_commands.reset.get().expect("the commands are created with the shell").clone()
        },
        NavigationStatus: String { get: |this: &Rc<MainViewModel>| this.navigation_status() },
        SessionStatus: String { get: |this: &Rc<MainViewModel>| this.session_status() },
    ],
    notify_property_changed: MainViewModel,
});
