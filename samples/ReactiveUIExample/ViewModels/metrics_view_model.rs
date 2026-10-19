//! Port of `ViewModels/MetricsViewModel.cs`: shows live local process measurements.

use super::page_view_model::page_view_model;
use super::{MetricCardViewModel, PageViewModel};
use crate::models::MachineSnapshot;
use crate::services::ILocalMachineMetricsService;
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    IReactiveObjectExt, IScreen, ObservableAsPropertyHelper, ReactiveCommand, ReactiveUIBindingExtensions, RxSchedulers, ToPropertyMixins,
    ViewModelActivationMixins,
};
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use ferroui_system::{prop, IDisposable, ObservableError};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The maximum percentage value (`MaximumPercentage`).
const MAXIMUM_PERCENTAGE: f64 = 100.0;

/// The interval of the live measurements.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Shows live local process measurements (`MetricsViewModel`).
pub struct MetricsViewModel {
    page: PageViewModel,
    /// The local metrics service (`_metricsService`).
    metrics_service: Rc<dyn ILocalMachineMetricsService>,
    /// The CPU metric card (`ProcessorCard`).
    processor_card: Rc<MetricCardViewModel>,
    /// The memory metric card (`MemoryCard`).
    memory_card: Rc<MetricCardViewModel>,
    /// The heap metric card (`ManagedCard`).
    managed_card: Rc<MetricCardViewModel>,
    /// The command that reads one immediate sample (`SampleOnce`).
    sample_once: OnceCell<ReactiveCommand<(), MachineSnapshot>>,
    /// The command of the controls of FerroUI that runs `SampleOnce`, for the bindings of markup.
    sample_once_command: OnceCell<Rc<dyn FerroCommand>>,
    /// The computed load label (`_loadLabel`).
    load_label: OnceCell<ObservableAsPropertyHelper<String>>,
    /// The latest sample (`Latest`).
    latest: RefCell<MachineSnapshot>,
    /// The CPU warning threshold (`CpuWarningThreshold`).
    cpu_warning_threshold: Cell<f64>,
    /// The number of observed live samples (`SampleCount`).
    sample_count: Cell<i32>,
}
page_view_model!(MetricsViewModel);

impl MetricsViewModel {
    /// The metrics page of `host_screen`, sampling `metrics_service`
    /// (`MetricsViewModel(IScreen, ILocalMachineMetricsService)`).
    pub fn new(host_screen: &Rc<dyn IScreen>, metrics_service: Rc<dyn ILocalMachineMetricsService>) -> Rc<Self> {
        let view_model = Rc::new(Self {
            page: PageViewModel::new(host_screen, "metrics", "Live local metrics", "Samples this process and updates the UI while the page is active."),
            metrics_service,
            processor_card: MetricCardViewModel::new("Process CPU", "0.0%", "Estimated from process CPU time deltas"),
            memory_card: MetricCardViewModel::new("Working set", "0.0 MB", "Current process resident memory"),
            // Deviation (DEVIATIONS.md, samples): the card of the managed heap of the reference shows
            // the heap the allocator of the example counts, and is named for it.
            managed_card: MetricCardViewModel::new("Heap", "0.0 MB", "Heap bytes the allocator of the process holds"),
            sample_once: OnceCell::new(),
            sample_once_command: OnceCell::new(),
            load_label: OnceCell::new(),
            latest: RefCell::new(MachineSnapshot::empty()),
            cpu_warning_threshold: Cell::new(40.0),
            sample_count: Cell::new(0),
        });
        let weak = Rc::downgrade(&view_model);
        let base = view_model.page.base();

        let sample_weak = weak.clone();
        let sample_once = base.track(ReactiveCommand::create_from_task(move || {
            let sample = match sample_weak.upgrade() {
                Some(view_model) => view_model.sample_once_now(),
                None => Err(crate::services::InvalidOperationError::new("The metrics page was disposed.").into_error()),
            };
            async move { sample }
        }));
        let (on_error, on_error_error) = (weak.clone(), weak.clone());
        let _ = base.track_disposable(sample_once.thrown_exceptions().subscribe_next_error(
            move |error| with_view_model(&on_error, |view_model| view_model.apply_error(&error)),
            move |error| with_view_model(&on_error_error, |view_model| view_model.apply_error(&error)),
        ));

        let load_label = view_model
            .when_any_value((prop!(MetricsViewModel => latest), prop!(MetricsViewModel => cpu_warning_threshold)))
            .select(|(snapshot, threshold): (MachineSnapshot, f64)| {
                if snapshot.processor_percent() >= threshold {
                    "CPU is at or above the warning threshold.".to_owned()
                } else {
                    "CPU is below the warning threshold.".to_owned()
                }
            })
            .to_property_with_initial_value(&view_model, prop!(MetricsViewModel => load_label), "Waiting for live data.".to_owned());
        let _ = view_model.load_label.set(load_label);

        let activated = weak;
        ViewModelActivationMixins::when_activated_disposables(&*view_model, move |disposables| {
            let Some(view_model) = activated.upgrade() else {
                return;
            };
            let (on_snapshot, on_error) = (Rc::downgrade(&view_model), Rc::downgrade(&view_model));
            disposables.add(view_model.metrics_service.watch(SAMPLE_INTERVAL).observe_on(RxSchedulers::main_thread_scheduler()).subscribe_next_error(
                move |snapshot| with_view_model(&on_snapshot, |view_model| view_model.apply_snapshot(snapshot)),
                move |error| with_view_model(&on_error, |view_model| view_model.apply_error(&error)),
            ));
        });

        let _ = view_model.sample_once_command.set(ReactiveCommandAdapter::command_of(Rc::new(sample_once.clone())));
        let _ = view_model.sample_once.set(sample_once);
        view_model
    }

    /// The CPU metric card (`ProcessorCard`).
    pub fn processor_card(&self) -> Rc<MetricCardViewModel> {
        self.processor_card.clone()
    }

    /// The memory metric card (`MemoryCard`).
    pub fn memory_card(&self) -> Rc<MetricCardViewModel> {
        self.memory_card.clone()
    }

    /// The heap metric card (`ManagedCard`).
    pub fn managed_card(&self) -> Rc<MetricCardViewModel> {
        self.managed_card.clone()
    }

    /// The command that reads one immediate sample (`SampleOnce`).
    pub fn sample_once(&self) -> ReactiveCommand<(), MachineSnapshot> {
        self.sample_once.get().expect("the command is created with the view model").clone()
    }

    /// The computed load label (`LoadLabel`).
    pub fn load_label(&self) -> String {
        self.load_label.get().map(|helper| helper.value()).unwrap_or_else(|| "Waiting for live data.".to_owned())
    }

    /// The latest sample (`Latest { get; }`).
    pub fn latest(&self) -> MachineSnapshot {
        self.latest.borrow().clone()
    }

    /// Sets the latest sample (`Latest { private set; }`).
    fn set_latest(&self, value: MachineSnapshot) {
        let _ = self.raise_and_set_if_changed(&self.latest, value, "Latest");
    }

    /// The CPU warning threshold (`CpuWarningThreshold { get; }`).
    pub fn cpu_warning_threshold(&self) -> f64 {
        self.cpu_warning_threshold.get()
    }

    /// Sets the CPU warning threshold, clamped between 1 and 100 (`CpuWarningThreshold { set; }`).
    pub fn set_cpu_warning_threshold(&self, value: f64) {
        let _ = self.raise_and_set_if_changed(&self.cpu_warning_threshold, value.clamp(1.0, MAXIMUM_PERCENTAGE), "CpuWarningThreshold");
    }

    /// The number of observed live samples (`SampleCount { get; }`).
    pub fn sample_count(&self) -> i32 {
        self.sample_count.get()
    }

    /// Sets the number of observed live samples (`SampleCount { private set; }`).
    fn set_sample_count(&self, value: i32) {
        let _ = self.raise_and_set_if_changed(&self.sample_count, value, "SampleCount");
    }

    /// Applies a deterministic sample for tests and immediate UI refreshes
    /// (`ApplySnapshot(MachineSnapshot)`).
    pub fn apply_snapshot(&self, snapshot: MachineSnapshot) {
        let (processor, working_set, managed) = (snapshot.processor_percent(), snapshot.working_set_megabytes(), snapshot.managed_megabytes());
        self.set_latest(snapshot);
        self.set_sample_count(self.sample_count() + 1);
        self.processor_card.set_value(format!("{processor:.1}%"));
        self.memory_card.set_value(format!("{working_set:.1} MB"));
        self.managed_card.set_value(format!("{managed:.1} MB"));
    }

    /// Reads and applies one immediate sample (`SampleOnceAsync()`).
    fn sample_once_now(&self) -> Result<MachineSnapshot, ObservableError> {
        let snapshot = self.metrics_service.read_snapshot()?;
        self.apply_snapshot(snapshot.clone());
        Ok(snapshot)
    }

    /// Applies an error sample when the live feed fails (`ApplyError(Exception)`).
    fn apply_error(&self, error: &ObservableError) {
        self.set_latest(self.latest().with_status(error.to_string()));
    }
}

impl IDisposable for MetricsViewModel {
    /// `Dispose()`: the computed label, then the base.
    fn dispose(&self) {
        if let Some(load_label) = self.load_label.get() {
            load_label.dispose();
        }
        self.page.base().dispose();
    }
}

/// Runs `action` on the view model while it is alive.
fn with_view_model(view_model: &Weak<MetricsViewModel>, action: impl FnOnce(&MetricsViewModel)) {
    if let Some(view_model) = view_model.upgrade() {
        action(&view_model);
    }
}

ferro_markup_type!(class MetricsViewModel {
    this: Rc<MetricsViewModel>,
    handles: [MetricsViewModel, Rc<MetricsViewModel>, Option<Rc<MetricsViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<MetricsViewModel>| this.title() },
        Summary: String { get: |this: &Rc<MetricsViewModel>| this.summary() },
        ProcessorCard: Rc<MetricCardViewModel> { get: |this: &Rc<MetricsViewModel>| this.processor_card() },
        MemoryCard: Rc<MetricCardViewModel> { get: |this: &Rc<MetricsViewModel>| this.memory_card() },
        ManagedCard: Rc<MetricCardViewModel> { get: |this: &Rc<MetricsViewModel>| this.managed_card() },
        SampleOnce: Rc<dyn FerroCommand> {
            get: |this: &Rc<MetricsViewModel>| this.sample_once_command.get().expect("the command is created with the view model").clone()
        },
        LoadLabel: String { get: |this: &Rc<MetricsViewModel>| this.load_label() },
        Latest: MachineSnapshot { get: |this: &Rc<MetricsViewModel>| this.latest() },
        CpuWarningThreshold: f64 {
            get: |this: &Rc<MetricsViewModel>| this.cpu_warning_threshold(),
            set: |this: &Rc<MetricsViewModel>, value: f64| this.set_cpu_warning_threshold(value)
        },
        SampleCount: i32 { get: |this: &Rc<MetricsViewModel>| this.sample_count() },
    ],
    notify_property_changed: MetricsViewModel,
});
