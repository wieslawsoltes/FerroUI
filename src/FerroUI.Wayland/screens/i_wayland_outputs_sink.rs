//! Where the worker sends the outputs (the port of
//! `IWaylandOutputsSink.cs`), and the proxy the worker holds
//! (`WaylandOutputsSinkProxy`, which the reference generates).

use super::wayland_output_snapshot::WaylandOutputsSnapshot;
use crate::server::wayland_marshallers::UiThreadRef;
use ferroui_base::threading::Dispatcher;
use std::rc::Rc;
use std::sync::Arc;

/// UI-thread sink for output snapshots.
pub trait IWaylandOutputsSink {
    fn on_outputs_changed(&self, snapshot: WaylandOutputsSnapshot);
}

/// The sink as the worker holds it: a call is posted to the UI thread at the default priority.
#[derive(Clone)]
pub struct WaylandOutputsSinkProxy {
    target: UiThreadRef<dyn IWaylandOutputsSink>,
}

impl WaylandOutputsSinkProxy {
    /// A proxy of `target`, an object of the calling thread, whose dispatcher `dispatcher` is.
    pub fn new(target: Rc<dyn IWaylandOutputsSink>, dispatcher: Arc<Dispatcher>) -> Self {
        Self { target: UiThreadRef::new(target, dispatcher) }
    }

    pub fn on_outputs_changed(&self, snapshot: WaylandOutputsSnapshot) {
        self.target.post(move |sink| sink.on_outputs_changed(snapshot));
    }
}
