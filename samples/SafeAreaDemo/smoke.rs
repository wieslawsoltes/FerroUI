//! The smoke run of the hosts (not a port: the upstream sample is driven by a person): once the
//! main view shows, the options of the view are changed one after the other, as a person would
//! check and uncheck them, and the state of the view model is reported after each change, so that
//! a run on an emulator, a simulator or a desktop shows what the insets manager and the input pane
//! of the platform answer.

use crate::view_models::MainViewModel;
use crate::views::MainView;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::Thickness;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A change of the smoke run: its name and what it does to the view model.
pub type Step = (&'static str, fn(&MainViewModel));

/// The changes of the smoke run, in their order: each option of the view on and off again, then
/// the three theme variants of the buttons.
pub const STEPS: &[Step] = &[
    ("DisplayEdgeToEdge=true", |view_model| view_model.set_display_edge_to_edge(true)),
    ("HideSystemBars=true", |view_model| view_model.set_hide_system_bars(true)),
    ("HideSystemBars=false", |view_model| view_model.set_hide_system_bars(false)),
    ("AutoSafeAreaPadding=true", |view_model| view_model.set_auto_safe_area_padding(true)),
    ("AutoSafeAreaPadding=false", |view_model| view_model.set_auto_safe_area_padding(false)),
    ("UseSafeArea=false", |view_model| view_model.set_use_safe_area(false)),
    ("UseSafeArea=true", |view_model| view_model.set_use_safe_area(true)),
    ("DisplayEdgeToEdge=false", |view_model| view_model.set_display_edge_to_edge(false)),
    ("Dark", |view_model| view_model.set_dark()),
    ("Light", |view_model| view_model.set_light()),
    ("Default", |view_model| view_model.set_default()),
];

/// The view model of a main view, once the view is loaded (its `OnLoaded` sets it).
pub fn main_view_model(view: &MainView) -> Option<Rc<MainViewModel>> {
    from_markup_value::<Rc<MainViewModel>>(&view.data_context())
}

fn thickness(value: Thickness) -> String {
    format!("{},{},{},{}", value.left, value.top, value.right, value.bottom)
}

/// The state of the view model, on one line.
pub fn describe(view_model: &MainViewModel) -> String {
    format!(
        "SafeAreaPadding={} ViewPadding={} DisplayEdgeToEdge={} HideSystemBars={} AutoSafeAreaPadding={} UseSafeArea={} InputPaneState={:?} CanvasSize={}x{}",
        thickness(view_model.safe_area_padding()),
        thickness(view_model.view_padding()),
        view_model.display_edge_to_edge(),
        view_model.hide_system_bars(),
        view_model.auto_safe_area_padding(),
        view_model.use_safe_area(),
        view_model.input_pane_state(),
        view_model.canvas_size().width,
        view_model.canvas_size().height,
    )
}

/// Runs the smoke run: waits for the view model of the main view (`find`), reports
/// `SAFEAREA SHOWN <state>`, then applies [`STEPS`] one every `step` and reports
/// `SAFEAREA STEP <index> <name>: <state>` for each, the state read just before the next change
/// (the platform answers a change of the insets after the change), and `SAFEAREA DONE` after
/// the last. `stepped` is called after the report of each step with its index and name (a host
/// takes a picture there), and `done` at the end.
pub fn run(
    find: impl Fn() -> Option<Rc<MainViewModel>> + 'static,
    step: Duration,
    note: impl Fn(&str) + 'static,
    stepped: impl Fn(usize, &str) + 'static,
    done: impl Fn() + 'static,
) {
    let view_model: RefCell<Option<Rc<MainViewModel>>> = RefCell::new(None);
    let next = Cell::new(0usize);
    let waited = Cell::new(Duration::ZERO);
    let since_step = Cell::new(Duration::ZERO);
    let tick = Duration::from_millis(250);
    let _timer = DispatcherTimer::run(
        move || {
            let found = view_model.borrow().clone();
            let Some(view_model_ref) = found.or_else(|| {
                let found = find()?;
                note(&format!("SAFEAREA SHOWN {}", describe(&found)));
                *view_model.borrow_mut() = Some(found.clone());
                Some(found)
            }) else {
                waited.set(waited.get() + tick);
                if waited.get().as_millis() % 10_000 == 0 {
                    note(&format!("Waiting for the main view ({} s)", waited.get().as_secs()));
                }
                return true;
            };

            since_step.set(since_step.get() + tick);
            if since_step.get() < step {
                return true;
            }
            since_step.set(Duration::ZERO);

            let index = next.get();
            if index > 0 {
                let name = STEPS[index - 1].0;
                note(&format!("SAFEAREA STEP {} {name}: {}", index - 1, describe(&view_model_ref)));
                stepped(index - 1, name);
            }
            if let Some((name, apply)) = STEPS.get(index) {
                note(&format!("Applying {name}"));
                apply(&view_model_ref);
                next.set(index + 1);
                return true;
            }
            note(&format!("SAFEAREA DONE ({} steps)", STEPS.len()));
            done();
            false
        },
        tick,
        DispatcherPriority::NORMAL,
    )
    .detach();
}
