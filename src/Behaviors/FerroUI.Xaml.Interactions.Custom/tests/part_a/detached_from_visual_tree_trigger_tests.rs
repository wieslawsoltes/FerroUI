//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/DetachedFromVisualTreeTriggerTests.cs`
//! of Xaml.Behaviors (with its window `DetachedFromVisualTreeTrigger001.xaml` and the binding
//! source of its code file, loaded with the run-time loader).
//!
//! Adapted: the data context is set by the test (a model with the command, which the window of
//! the reference creates in its document); the window is laid out instead of rendering a frame.

use super::command::Command;
use super::support::{find, load, run_jobs, start_app};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model as FerroModel;
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_model, BoxedValue};
use ferroui_controls::{TabControl, Window};
use std::cell::Cell;
use std::rc::Rc;

/// `DetachedTriggerBindingSource`: the data context of the window, with a command that counts
/// its executions.
pub(crate) struct DetachedTriggerBindingSource {
    detached_command: Rc<dyn ICommand>,
    detached_count: Rc<Cell<i32>>,
}

ferro_model!(DetachedTriggerBindingSource, |b| b
    .read_only::<Value<Rc<dyn ICommand>>>("DetachedCommand", |source| source.detached_command.clone()));

impl DetachedTriggerBindingSource {
    fn new() -> Rc<Self> {
        let detached_count = Rc::new(Cell::new(0));
        let count = detached_count.clone();
        let detached_command = Command::new(move |_parameter| count.set(count.get() + 1));
        <DetachedTriggerBindingSource as FerroModel>::new_model(Self { detached_command, detached_count })
    }

    fn detached_count(&self) -> i32 {
        self.detached_count.get()
    }
}

/// `window.CaptureRenderedFrame()`: what the test needs of it, a layout pass and the jobs it
/// queued.
fn capture_rendered_frame(window: &Window) {
    window.update_layout();
    run_jobs();
}

#[test]
fn switching_tabs_executes_bound_detached_command() {
    let _app = start_app();
    let window =
        load::<Window>("DetachedFromVisualTreeTrigger001.xaml", include_str!("DetachedFromVisualTreeTrigger001.xaml"));
    let source = DetachedTriggerBindingSource::new();
    // A model is boxed as the shared object itself.
    window.set_data_context(Some(source.clone() as BoxedValue));

    window.show();
    capture_rendered_frame(&window);

    find::<TabControl>(&window, "TargetTabs").set_selected_index(1);
    run_jobs();
    capture_rendered_frame(&window);

    assert_eq!(1, source.detached_count());
}
