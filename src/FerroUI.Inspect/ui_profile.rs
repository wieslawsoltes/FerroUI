//! A window-scoped capture shared by the inspector and its MCP clients.
use super::{InspectResult, Inspector, NodeId};
use ferroui_base::{
    diagnostics::ui_profile::{Capture, Metric, Phase, Row},
    StyledElement, Visual, WeakRef,
};
use serde_json::{json, Value};
use std::{rc::Rc, time::Duration};

pub(super) const MIN_DURATION_MS: u64 = 100;
pub(super) const MAX_DURATION_MS: u64 = 30_000;

pub(super) struct Recording {
    id: u64,
    window_id: NodeId,
    window: WeakRef<Visual>,
    duration: Duration,
    capture: Rc<Capture>,
}

impl Inspector {
    pub(super) fn start_ui_profile(
        &mut self,
        window_id: NodeId,
        duration_ms: u64,
    ) -> InspectResult {
        let window = self
            .windows()
            .into_iter()
            .find(|window| self.node_id(window) == window_id)
            .ok_or("Choose a live window from list_windows")?;
        let duration = Duration::from_millis(duration_ms);
        let capture = Capture::start(window.presentation_source().clone(), duration)?;
        let id = self
            .profile
            .as_ref()
            .map_or(1, |recording| recording.id + 1);
        self.profile = Some(Recording {
            id,
            window_id,
            window: window.upcast::<Visual>().downgrade(),
            duration,
            capture,
        });
        self.read_ui_profile(Some(id), false)
    }

    pub(super) fn read_ui_profile(&mut self, id: Option<u64>, stop: bool) -> InspectResult {
        let Some(recording) = self.profile.as_ref() else {
            return if id.is_some() {
                Err("The UI profile expired".into())
            } else {
                Ok(Value::Null)
            };
        };
        if id.is_some_and(|id| id != recording.id) {
            return Err("The UI profile was replaced. Read the active capture again.".into());
        }
        let (id, window_id, window, duration, capture) = (
            recording.id,
            recording.window_id,
            recording.window.clone(),
            recording.duration,
            recording.capture.clone(),
        );
        let window_open = self.windows().iter().any(|root| window.points_to(root));
        if stop || !window_open {
            capture.stop();
        }
        let snapshot = capture.snapshot();
        let mut rows = snapshot
            .rows
            .into_iter()
            .enumerate()
            .filter(|(_, row)| row.metrics.iter().any(|metric| metric.calls != 0))
            .collect::<Vec<_>>();
        rows.sort_by_key(|(_, row)| std::cmp::Reverse(own_time(row)));
        let rows = rows
            .into_iter()
            .map(|(index, row)| self.profile_row(index, row, &capture, window_open))
            .collect::<Vec<_>>();
        Ok(json!({
            "id":id, "window_id":window_id, "window_open":window_open,
            "recording":snapshot.recording, "elapsed_ms":ms(snapshot.elapsed),
            "remaining_ms":if snapshot.recording { ms(duration.saturating_sub(snapshot.elapsed)) } else { 0.0 },
            "omitted_calls":snapshot.omitted_calls, "rows":rows
        }))
    }

    fn profile_row(&mut self, index: usize, row: Row, capture: &Capture, open: bool) -> Value {
        let node = row
            .visual
            .as_ref()
            .and_then(WeakRef::upgrade)
            .filter(|visual| open && capture.contains(visual) && !self.is_overlay(visual));
        let phases = Phase::ALL
            .into_iter()
            .zip(row.metrics)
            .filter(|(_, metric)| metric.calls != 0)
            .map(|(phase, metric)| phase_json(phase, metric))
            .collect::<Vec<_>>();
        json!({
            "row_id":index,
            "kind":if row.is_component {"component"} else {"control"},
            "label":row.name,
            "node_id":node.as_ref().map(|node| self.node_id(node)),
            "name":node.as_ref().and_then(|node| node.cast::<StyledElement>()?.name()),
            "has_source":node.as_ref().is_some_and(|node| super::source::has_location(node)),
            "self_ms":ms(own_time(&row)), "phases":phases
        })
    }
}

fn own_time(row: &Row) -> Duration {
    row.metrics.iter().map(|metric| metric.own).sum()
}

fn phase_json(phase: Phase, metric: Metric) -> Value {
    let name = match phase {
        Phase::Measure => "measure",
        Phase::Arrange => "arrange",
        Phase::Render => "render",
        Phase::Reconcile => "reconcile",
    };
    json!({"phase":name,"calls":metric.calls,"self_ms":ms(metric.own),
        "total_ms":ms(metric.inclusive),"max_self_ms":ms(metric.longest_own)})
}

fn ms(value: Duration) -> f64 {
    value.as_secs_f64() * 1000.0
}
